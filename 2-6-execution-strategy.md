# 2.6 実行戦略（Execution Strategy）

> 目的：**最小コアで高い実用性能**（線形時間・ゼロコピー・良質な診断）を実現し、**左再帰・ストリーム**・**インクリメンタル**も無理なく扱える実行系を定義する。
> 前提：2.1 の `State/Reply{consumed, committed}`、2.2 の合成規則、2.3/1.4 の Unicode/入力モデルと整合。

---

## A. ランナーとモード

### A-1. ランナー API（外部インターフェイス）

```reml
fn run<T>(p: Parser<T>, src: String, cfg: RunConfig = {}) -> Result<(T, Span), ParseError>
fn run_partial<T>(p: Parser<T>, src: String, cfg: RunConfig = {}) -> Result<(T, Input, Span), ParseError>
fn run_stream<T>(p: Parser<T>, feeder: Feeder, cfg: RunConfig = {}) -> Result<StreamOutcome<T>, ParseError>
fn resume<T>(k: Continuation<T>, more: Bytes) -> Result<StreamOutcome<T>, ParseError>
```

* `run_stream` は **逐次供給**（ファイル・ソケット）向けで、`StreamOutcome` が Pending の場合は追加データが必要。
* `StreamOutcome<T>` と `Feeder` / `Continuation<T>` の定義は [2.1 パーサ型](2-1-parser-type.md) のランナー節を参照。
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

```reml
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
  stream_buffer_bytes: Option<usize> = Some(64 * 1024),
  gc: Option<GcConfig> = None
}

type GcConfig = {
  policy: GcPolicy = "Incremental",
  heap_max_bytes: Option<usize>,
  pause_target_ms: Option<f64>,
  profile: Option<GcProfileId>
}

type GcPolicy = "Rc" | "Incremental" | "Generational" | "Region"

type GcProfileId = "game" | "ide" | "web" | "data" | String
```

* `exec_mode` は Normal / Packrat / Hybrid / Streaming の各モードを切替える（既定は `normal`）。
* `packrat` と `left_recursion` はメモ化と seed-growing 左再帰を手動で調整する。
* `fuel_max_steps` / `fuel_on_empty_loop` は停止性の安全弁として機能する。
* `packrat_window_bytes` / `memo_max_entries` はキャッシュのメモリ上限。
* `stream_buffer_bytes` はストリーム入力のリングバッファ既定サイズ。
* `gc` を指定すると実行時に GC Capability へ通知され、ポリシー・ヒープ上限・停止時間目標を伝える（§G-1）。

* **空成功の繰返し**検出は必須（2.2 に準拠）。
* `fuel_max_steps` 超過は `E_FUEL` としてエラー化（位置・直近ルール列を提示）。

### B-3. 期待集合の早期確定

* `cut_here()` を通過したら **親の期待集合を破棄**し、その地点からの期待を再構築（2.5 と同一）。

---

## C. メモ化（Packrat）と左再帰

### C-1. メモ化キーと値

```reml
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

```reml
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
* Feeder は `pull(hint: DemandHint)` を受け取り、`FeederYield::Chunk` でチャンクを返し、`::Await` / `::Closed` / `::Error` でバックプレッシャや終了を通知。
* 文字モデル（1.4）の **境界表**（コードポイント/グラフェム）は **スライディングで増分更新**。

### F-2. `StreamOutcome`・継続とデマンドヒント

```reml
type StreamOutcome<T> =
  | Completed { value: T, span: Span, meta: StreamMeta }
  | Pending { continuation: Continuation<T>, demand: DemandHint, meta: StreamMeta }
```

* `StreamMeta` は累積消費量や再開回数、バックプレッシャ関連指標をまとめた統計で、監査ログ `parser.stream` に添付する。
* `Pending.demand` は次に必要な供給量のヒントを示し、Feeder／`FlowController` が入力バッチを調整する指針になる。

```reml
type DemandHint = {
  min_bytes: usize,
  preferred_bytes: Option<usize>,
  frame_boundary: Option<TokenClass>
}

type Continuation<T> = {
  state: Opaque,
  meta: ContinuationMeta
}

type ContinuationMeta = {
  commit_watermark: usize,
  buffered: Input,
  resume_hint: Option<DemandHint>,
  expected_tokens: Set<Expectation>,
  last_checkpoint: Option<Span>,
  trace_label: Option<String>
}
```

* `commit_watermark` 以前のメモ化エントリは安全に破棄できる。`buffered` はリングバッファ内の未消費入力で、`resume` 時に再利用される。
* `expected_tokens` / `last_checkpoint` は IDE 補完やロールバック処理のガイドとなる。`trace_label` は SpanTrace（2.5）と連動し、ログ上で継続の出処を追跡しやすくする。
* `run_stream` が `Pending` を返した場合は `Continuation` と `DemandHint` を使って供給戦略を決め、`resume` に追加バイトを渡す。`Completed` のときは `StreamMeta` を監査・テレメトリへ記録する。

### F-3. インクリメンタル再パース（IDE）

* \*\*編集差分（byte range + delta）\*\*を受け取り、

  1. その範囲を跨ぐメモを無効化、
  2. **依存グラフ**（`ParserId`→呼出）で影響範囲を再評価、
  3. 変更境界から**局所再パース**。
* AST ノードは `Span` を鍵に **ロープ**状に結び直す（ゼロコピー維持）。

### F-4. `StreamDriver` とフロー制御（ドラフト）

```reml
type StreamDriver<T, Sink> = {
  parser: Parser<T>,
  feeder: Feeder,
  sink: Sink,
  flow: FlowController,
  on_diagnostic: StreamDiagnosticHook,
  state: Option<Continuation<T>>,
  meta: StreamMeta
}

type FlowController = {
  mode: FlowMode,
  high_watermark: usize,
  low_watermark: usize,
  policy: FlowPolicy
}

type FlowMode = "push" | "pull" | "hybrid"

type FlowPolicy =
  | Manual { on_demand: fn() -> Demand }
  | Auto { backpressure: BackpressureSpec }

type Demand = { bytes: usize, frames: usize }

type BackpressureSpec = {
  max_lag: Option<Duration>,
  debounce: Option<Duration>,
  throttle: Option<Duration>
}
```

* `StreamDriver::pump()` は `run_stream` を 1 ステップ進め、`StreamOutcome` を `sink` へ渡す。`Pending` の場合は `state` に継続を保持し、`FlowController` の判定に従って再開タイミングを決定する。
* `FlowController.mode` で IDE 向けの **pull**（差分が届いたときのみ `resume`）、リアルタイム処理の **push**、両者を組み合わせる **hybrid** を切り替える。
* `Auto` ポリシーは `StreamMeta.lag_nanos` やバッファ充填率を監視し、しきい値を越えた場合にスロットリング／ドレインなどの内部イベントを発火させ、供給側へ通知する。

### F-5. ストリーム診断フック

```reml
type StreamDiagnosticHook = fn(StreamEvent) -> ()

type StreamEvent =
  | Progress { consumed: usize, produced: usize, lap: Duration }
  | Pending { reason: PendingReason, meta: ContinuationMeta }
  | Error { diagnostic: ParseError, continuation: Option<ContinuationMeta> }

type PendingReason = "Backpressure" | "InputExhausted" | "FeederAwait" | "FeederClosed"
```

* `Progress` はテレメトリや IDE ステータスバーに活用し、解析の進捗をリアルタイムで可視化する。
* `Pending` イベントは `ContinuationMeta` を添付するため、補完候補提示や復旧 UI にそのまま渡せる。
* `Error` は `ParseError` を構造化ログに出力しつつ、継続があれば添付する。`audit.log("parser.stream.error", …)` などで監査フローに統合する。

---

## G. 並列性・再入性

* パーサ値は **不変**・**スレッドセーフ**。
* `State` は実行ごとに分離。`MemoTable` も run 単位。
* **分割統治**（ファイル複数・モジュール複数）は **上位で並列**に回す想定（同一入力内での並列実行は非推奨：メモが競合する）。

### G-1. GC 制御フロー（ドラフト）

```reml
type GcCapability = {
  configure: fn(GcConfig) -> (),
  register_root: fn(RootSet) -> (),
  unregister_root: fn(RootSet) -> (),
  write_barrier: fn(Object, Field) -> (),
  metrics: fn() -> GcMetrics,
  trigger: fn(GcReason) -> ()
}

type RootSet = {
  stack_roots: List<Ptr<Object>>,
  global_roots: List<Ptr<Object>>
}

type GcMetrics = {
  heap_bytes: usize,
  heap_limit: usize,
  last_pause_ms: f64,
  total_collections: u64,
  policy: GcPolicy
}

type GcReason = "Manual" | "Threshold" | "Idle" | "Emergency"
```

* ランナーは `RunConfig.gc` が指定されていれば初期化時に `configure` を呼び、`RootSet` を登録する。パーサ評価中にローターンスレッドが変更される場合は `register_root`/`unregister_root` を再調整する。
* 書き込みバリアは `Parser` が持つ mutable state から参照型を更新する際に呼び出し、世代間ポインタを GC へ通知する。
* `metrics()` は `guides/runtime-bridges.md` で定義する `gc.stats` 監査ログと一致するメトリクスを返す。
* `trigger` はポリシー固有のコレクションを明示的に走らせるためのフックであり、`pause_target_ms` を守れない場合は `GcReason::Emergency` として呼び出す。

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
  この実行戦略で、Reml のパーサは **小さなコア**のまま現実的な大規模入力・対話・言語処理に耐える。


## K. ツール統合オプション

IDE/LSP 連携や CLI/監査ツールとの統合に向けたランナー拡張仕様をここにまとめる。

### K-1. LSP / IDE メタデータ出力

* `RunConfig.lsp = { highlight = true, completion = true, codeActions = true }` のような設定で、構文ハイライトや補完情報を生成。
* `run_with_lsp(parser, src, cfg)` ヘルパを提供し、`to_lsp_diagnostics`（2.5節）と組み合わせて IDE へ送出。

### K-2. 構造化ログ / CLI 連携

* `RunConfig.log_format = "json"` により、実行イベントを JSON で出力。
* `reml-run lint config.ks --format json` のような CLI コマンド例を提示し、CI/CD での利用を想定。

### K-3. ホットリロード API

```reml
fn reload<T>(parser: Parser<T>, state: ReloadState<T>, diff: SchemaDiff<Old, New>)
  -> Result<ReloadState<T>, ReloadError>
```

* `state` には前回の継続・キャッシュを保持。
* `diff` を適用後に `audit` ログへ記録し、失敗時はロールバック情報を返す。

```reml
type ReloadState<T> = {
  continuation: Option<Continuation<T>>,  // Pending セッションがあれば保存
  memo: MemoTable,                        // Packrat キャッシュのスナップショット
  version: SemVer                         // 適用済み設定のバージョン
}

type ReloadError =
  | ValidationFailed(List<Diagnostic>)
  | ApplyFailed { reason: String, rollback: RollbackInfo }
  | IncompatibleVersion { running: SemVer, incoming: SemVer }

type RollbackInfo = {
  audit_id: Uuid,
  actions: List<String>
}
```

CLI `reml-run reload` は `ReloadError` を exit code `5`（Incompatible）、`6`（Validation）、`7`（ApplyFailed）に割り当て、`rollback` サブコマンドと同じフォーマットで `RollbackInfo` を出力する。

### K-4. 監査フック

* `RunConfig.audit = Some(|event| audit_log(event))` で診断や差分を収集。
* `audit` 効果と連携し、エラー発生時に自動で `audit_id` を付与。


#
