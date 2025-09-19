# 2.6 実行戦略（Execution Strategy）

> 目的：**最小コアで高い実用性能**（線形時間・ゼロコピー・良質な診断）を実現し、**左再帰・ストリーム**・**インクリメンタル**も無理なく扱える実行系を定義する。
> 前提：2.1 の `State/Reply{consumed, committed}`、2.2 の合成規則、2.3/1.4 の Unicode/入力モデルと整合。

---

## A. ランナーとモード

### A-1. ランナー API（外部インターフェイス）

```kestrel
fn run<T>(p: Parser<T>, src: String, cfg: RunConfig = {}) -> Result<(T, Span), ParseError>
fn run_partial<T>(p: Parser<T>, src: String, cfg: RunConfig = {}) -> Result<(T, Input, Span), ParseError>
fn run_stream<T>(p: Parser<T>, feeder: Feeder, cfg: RunConfig = {}) -> Result<StreamOutcome<T>, ParseError>
fn resume<T>(k: Continuation<T>, more: Bytes) -> Result<StreamOutcome<T>, ParseError>
```

* `run_stream` は **逐次供給**（ファイル・ソケット）向けで、`StreamOutcome` が Pending の場合は追加データが必要。
* `StreamOutcome<T>` と `Feeder` / `Continuation<T>` の定義は [2.1 パーサ型](2-1-perser-type.md) のランナー節を参照。
* `resume` は Pending となった **継続**を受け取り、追加バイトで再開（§F）。

### A-2. 実行モード（`cfg.exec_mode`）

* `Normal`（既定）：前進解析（LL(\*) 相当）＋必要箇所のみバックトラック。
* `Packrat`：**メモ化**で PEG 風の **O(n)**。左再帰には §C を推奨。
* `Hybrid`：**スライディング窓**と**選択的メモ化**（ホットルールのみ）でメモリを節制。
* `Streaming`：リングバッファ上で§Fの継続実行（インクリメンタル）。

---

## B. コアエンジン（ステップ実行・安全弁）

### B-1. トランポリン & 末尾最適化

* すべてのコンビネータは **ループ化**／**トランポリン**で実装し、**スタック深度は O(1)**。
* 再帰下降は `call(rule_id)` → `jump`（継続渡し）で表現。

### B-2. 実行燃料（fuel）

`RunConfig` に燃料を設け、**停止性と DoS 耐性**を確保。

```kestrel
type RunConfig = {
  exec_mode: "normal" | "packrat" | "hybrid" | "streaming" = "normal",
  require_eof: Bool = false,
  packrat: Bool = false,
  left_recursion: "off" | "on" | "auto" = "auto",
  fuel_max_steps: Option<usize> = None,
  fuel_on_empty_loop: "error" | "warn" = "error",
  packrat_window_bytes: Option<usize> = Some(1 << 20),
  memo_max_entries: Option<usize> = Some(1 << 20),
  trace: Bool = false,
  merge_warnings: Bool = true,
  stream_buffer_bytes: Option<usize> = Some(64 * 1024)
}
```

* `exec_mode` は Normal / Packrat / Hybrid / Streaming の各モードを切替える（既定は `normal`）。
* `packrat` と `left_recursion` はメモ化と seed-growing 左再帰を手動で調整する。
* `fuel_max_steps` / `fuel_on_empty_loop` は停止性の安全弁として機能する。
* `packrat_window_bytes` / `memo_max_entries` はキャッシュのメモリ上限。
* `stream_buffer_bytes` はストリーム入力のリングバッファ既定サイズ。

* **空成功の繰返し**検出は必須（2.2 に準拠）。
* `fuel_max_steps` 超過は `E_FUEL` としてエラー化（位置・直近ルール列を提示）。

### B-3. 期待集合の早期確定

* `cut_here()` を通過したら **親の期待集合を破棄**し、その地点からの期待を再構築（2.5 と同一）。

---

## C. メモ化（Packrat）と左再帰

### C-1. メモ化キーと値

```kestrel
type ParserId = u32
type MemoKey = (ParserId, byte_off)
type MemoVal<T> = Reply<T>  // Ok/Err 丸ごと
```

* 値は**スパン・consumed/committed**を含む `Reply` を**丸ごと**キャッシュ。
* **命中時は入力を進めず**、`Reply` をそのまま返す。

### C-2. メモの窓（スライディング）

* `packrat_window_bytes` で **前方最遠コミット水位**（`commit_watermark`）より**古いオフセット**のエントリを**段階的に破棄**。
* `commit_watermark` は **最後に `committed=true` で確定した `byte_off` の最大値**。→ `cut` によって**安全に掃除**できる。

### C-3. 左再帰（seed-growing）

* `left_recursion = on|auto` かつ Packrat 有効のとき、**Warth et al.** の **種成長**を実装：

  1. `(A, pos)` に「**評価中**」フラグと \*\*種（失敗）\*\*を入れる。
  2. 右辺を評価し、**より遠く進めた**結果が得られたら **更新**して再試行。
  3. 進捗がなくなるまで繰返し、最終結果を確定。
* `auto` は **`precedence` 使用時は無効**（必要ない）／**直接左再帰を検知**したルールのみ有効化。
* **非終端 A の複数定義**（演算子階層など）には `precedence` を推奨（高速）。

---

## D. 選択的メモ化（Hybrid）

* **ホットルール自動検出**：短時間に同位置で頻出する `ParserId` を **ホット**とみなし、それのみメモ化。
* **閾値**と**上限**は `memo_max_entries`／LRU。
* PEG 的線形性を緩く保ちつつ、**メモリ消費を数十 MB に抑制**できる。

---

## E. エラー・トレース・計測

### E-1. 最遠エラー集約

* 2.5 の **farthest-first** を実装：`byte_end` → `committed` → `expected ∪`。
* `then` で失敗したら **直前の `rule/label` を `context` に積む**。

### E-2. トレース・プロファイル（オプション）

```kestrel
type TraceEvent =
  | Enter(ParserId, Input)
  | ExitOk(ParserId, Span)
  | ExitErr(ParserId, ParseError)

fn with_trace<T>(p: Parser<T>, on_event: TraceEvent -> ()) -> Parser<T>
```

* `cfg.trace=true` で **SpanTrace** と **イベントフック**を活性化。
* 最小限のオーバーヘッド（ビルド時に NOP へ落ちる）。

### E-3. カバレッジ

* どの分岐が一度も走っていないかを `ParserId` 単位で可視化（テスト補助）。

---

## F. ストリーミング＆インクリメンタル

### F-1. 入力リングバッファ

* `Input.bytes` は **固定サイズリング**（既定 64KB〜任意）。
* **先読み**が窓を越える場合は **ブロック（継続待ち）**。
* Feeder は `pull(max_bytes)` で `FeederSignal::Ready` によりチャンクを返し、`::Await` / `::Closed` / `::Error` でバックプレッシャや終了を通知。
* 文字モデル（1.4）の **境界表**（コードポイント/グラフェム）は **スライディングで増分更新**。

### F-2. 継続（Continuation）

```kestrel
type Continuation<T> = {
  state: Opaque,           // メモ/位置/進行中ルールの縮約スナップショット
  commit_watermark: usize, // 掃除可能基準
  buffered: Input           // リングバッファに残っている未消費入力
}
```

* `run_stream` は **入力不足**で停止すると `StreamOutcome::Pending`（`Continuation` 付き）を返し、`resume` で再開。
* **Fix**：`attempt` の境界より前のメモは **破棄可能**、`commit_watermark` より前は**安全に破棄**。`buffered` には再開時に利用する未消費入力が格納される。

### F-3. インクリメンタル再パース（IDE）

* \*\*編集差分（byte range + delta）\*\*を受け取り、

  1. その範囲を跨ぐメモを無効化、
  2. **依存グラフ**（`ParserId`→呼出）で影響範囲を再評価、
  3. 変更境界から**局所再パース**。
* AST ノードは `Span` を鍵に **ロープ**状に結び直す（ゼロコピー維持）。

---

## G. 並列性・再入性

* パーサ値は **不変**・**スレッドセーフ**。
* `State` は実行ごとに分離。`MemoTable` も run 単位。
* **分割統治**（ファイル複数・モジュール複数）は **上位で並列**に回す想定（同一入力内での並列実行は非推奨：メモが競合する）。

---

## H. パフォーマンス方針（実装規約）

1. **ホット経路を手でループ化**

   * `many`, `takeWhile`, `string`, `symbol` は **分配関数呼び出しを避け**、ASCII 最適化。
2. **Arena（解放一括）**

   * 一時ノードは **ランナー専用アリーナ**へ確保→終了時に一括解放。
3. **ゼロコピー**

   * `Str` は親 `String` を参照共有（SSO/RC）。
4. **境界キャッシュ**

   * コードポイント/グラフェム境界は **遅延構築**かつ **ビュー共有**。
5. **メモ掃除**

   * `cut` と `commit_watermark` を利用して **前方を積極的に破棄**。
6. **オペランド後 Cut**

   * `precedence` は **演算子読取時に cut** を自動挿入（右項欠落の診断改善＋バックトラック削減）。

---

## I. 互換と移行

* **`precedence` を使う限り左再帰は不要**（デフォルト `left_recursion=auto` がそれを尊重）。
* 既存 PEG ルールを移植する場合は `packrat=true` を推奨、メモ窓でメモリをコントロール。
* 大規模入力・REPL・LSP 連携は `run_stream`/`resume` を使う。

---

## J. 仕様チェックリスト

* [ ] **モード**：Normal / Packrat / Hybrid / Streaming。
* [ ] **トランポリン**＋**燃料**で停止性確保。
* [ ] **Packrat**：キー `(ParserId, byte_off)`、窓/LRU、**cut で掃除**。
* [ ] **左再帰**：seed-growing（on/auto）。`precedence` 併用で不要化。
* [ ] **最遠エラー**：farthest-first、`cut` で期待再初期化。
* [ ] **トレース**：Enter/ExitOk/ExitErr フック、SpanTrace。
* [ ] **ストリーミング**：リングバッファ、Continuation、resume。
* [ ] **インクリメンタル**：差分無効化→局所再パース。
* [ ] **性能規約**：ASCII 高速・アリーナ・境界キャッシュ・ゼロコピー。

---

### まとめ

* 既定は **前進解析 + cut/label による制御可能な BT**。
* 必要に応じて **Packrat（線形化）**・**左再帰 seed-growing**・**スライディング窓**で実用性能とメモリのバランスを取る。
* **ストリーミング/インクリメンタル**と **高品位エラー**が最初から設計に入っており、IDE/LSP にも直結できる。
  この実行戦略で、Kestrel のパーサは **小さなコア**のまま現実的な大規模入力・対話・言語処理に耐える。
