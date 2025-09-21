# 2.6 実行戦略（Execution Strategy）

> 目的：**最小コアで高い実用性能**（線形時間・ゼロコピー・良質な診断）を実現し、**左再帰・ストリーム**・**インクリメンタル**も無理なく扱える実行系を定義する。
> 前提：2.1 の `State/Reply{consumed, committed}`、2.2 の合成規則、2.3/1.4 の Unicode/入力モデルと整合。

---

## A. ランナーとモード

### A-1. ランナー API（外部インターフェイス）

```reml
fn run<T>(p: Parser<T>, src: String, cfg: RunConfig = {}) -> ParseResult<T>
fn run_partial<T>(p: Parser<T>, src: String, cfg: RunConfig = {}) -> ParseResultWithRest<T>
```

* `ParseResult` は 2.1 節と同様、成功時の値と診断をまとめて返す。
* `ParseResultWithRest` は未消費入力を併せて返し、REPL やインクリメンタル更新に備える。
* ストリーミング処理や継続再開は拡張モジュール `Core.Parse.Streaming` に委ねる（§F 参照）。

### A-2. 実行モードと拡張の棲み分け

コア仕様の `RunConfig` はバッチ解析を前提とし、Packrat や左再帰の切替、追跡の有無など最小限の選択肢のみを持ちます。ストリーミング処理やハイブリッド実行、差分再利用といった高度な戦略は拡張モジュールで定義され、コアからは opt-in で利用します。

---

## B. コアエンジン（ステップ実行・安全弁）

### B-1. トランポリン & 末尾最適化

* すべてのコンビネータは **ループ化**／**トランポリン**で実装し、**スタック深度は O(1)**。
* 再帰下降は `call(rule_id)` → `jump`（継続渡し）で表現。

### B-2. RunConfig のコアスイッチ

`RunConfig` はバッチ解析に必要な最小限のスイッチだけを提供し、燃料制御や追加の安全弁は拡張モジュール側で定義する。

```reml
type RunConfig = {
  require_eof: Bool = false,
  packrat: Bool = false,
  left_recursion: "off" | "on" | "auto" = "auto",
  trace: Bool = false,
  merge_warnings: Bool = true,
  legacy_result: Bool = false,
  extensions: RunConfigExtensions = {}
}

type RunConfigExtensions = Map<Str, Any>
```

* `require_eof` で余剰入力を拒否するかどうかを切り替える。
* `packrat` と `left_recursion` はメモ化と seed-growing 左再帰を制御する主要スイッチ。
* `trace` は SpanTrace を収集し、`merge_warnings` は回復警告をまとめてノイズを抑制する。
* `legacy_result` は旧来の戻り値形式を要求するツールチェーンとの互換用スイッチ。
* 追加の燃料制御や GC 連携、ストリーミング用バッファ、LSP 設定などは `extensions` に格納されるモジュール固有設定として扱い、必要なときだけ読み込む（推奨ネームスペースは [2-1](2-1-parser-type.md) を参照）。

* **空成功の繰返し**検出は必須（2.2 に準拠）。

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

* 実装は **前方最遠コミット水位**（`commit_watermark`）を基準に古いエントリを段階的に破棄し、メモリ使用量を制御する。
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
* **閾値**と**上限**は実装側の LRU などポリシーに委ねる。
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

コア仕様ではストリーミングおよび差分適用の詳細を定義しません。これらの機能は `Core.Parse.Streaming` 拡張に委ねられ、`Parser` の意味論と互換な `run_stream`/`resume` API、継続メタデータ、バックプレッシャ制御を個別に定義します。詳細は [Core.Parse.Streaming 拡張ガイド](guides/core-parse-streaming.md) を参照してください。

ここでは以下の契約のみを前提とします。

* ストリーミングランナーは `run`/`run_partial` と同じ `ParseResult`/`Diagnostic` 形式を再利用する。
* インクリメンタル再パースは `commit_watermark` と `ParserId` 依存グラフを利用して影響範囲を絞り込む。
* Feeder や DemandHint などの詳細型は拡張側で定義され、コアからは不透明。

---

## G. 並列性・再入性

* パーサ値は **不変**であり、`State` は実行ごとに分離される。共有を行う場合は拡張側でスレッド安全性を保証する。
* 同一入力内での並列実行は推奨しないが、モジュール単位の分割統治は上位レイヤで並列化できる。
* GC やランタイム統合に関する詳細なコールバックは `guides/runtime-bridges.md` に委ね、コア仕様では純粋性と境界の明示のみを要求する。


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
* 大規模入力・REPL・LSP 連携が必要な場合は `Core.Parse.Streaming` 拡張を利用する。

---

## J. 仕様チェックリスト

* [ ] `run` / `run_partial` が `ParseResult` / `ParseResultWithRest` を返し、診断・未消費入力を一貫して扱う。
* [ ] `RunConfig` のコアスイッチ（require_eof / packrat / left_recursion / trace / merge_warnings）を実装し、既定値を確認する。
* [ ] Packrat メモ化：キー `(ParserId, byte_off)`、`commit_watermark` に基づく掃除、実装依存の窓上限を備える。
* [ ] 左再帰は seed-growing で解決し、`left_recursion=auto` と `precedence` の協調を確認する。
* [ ] 最遠エラー統合と `cut` による期待リセットが期待どおりに動作する。
* [ ] `trace` と `merge_warnings` の挙動をテストし、診断ノイズを制御する。
* [ ] インクリメンタル処理やストリーミングを提供する場合は `Core.Parse.Streaming` 拡張の契約に従うことを文書化する。

---

### まとめ

* 既定は **前進解析 + cut/label による制御可能なバックトラック**で、Packrat と左再帰サポートをスイッチ可能にする。
* `RunConfig` は最小限のスイッチに留め、燃料制御・ストリーミング・GC 連携などは拡張モジュールで opt-in する。
* 診断品質（最遠エラー、SpanTrace、警告集約）とゼロコピー入力を中核に据え、DSL から大規模入力まで一貫した挙動を提供する。
