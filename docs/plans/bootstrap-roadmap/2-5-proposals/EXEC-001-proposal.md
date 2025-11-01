# EXEC-001 ストリーミング実行 PoC 計画

## 1. 背景と症状
- 仕様は `run`/`run_partial` に加え、入力チャンクを扱う `run_stream` / `resume` / `DemandHint` を定義している（docs/spec/2-6-execution-strategy.md:10-24, docs/spec/2-7-core-parse-streaming.md:22-84）。  
- 現行 OCaml 実装はバッチ解析用ランナーのみを提供し（compiler/ocaml/src/parser_driver.ml:15-43）、ストリーミング API・バックプレッシャ制御・`RunConfig.extensions["stream"]` が未実装。  
- Phase 3 の self-host では `run_stream` 互換性がゴール条件に含まれており、仕様差分がセルフホストのスケジュールを阻害するリスクが高い。

## 2. Before / After
### Before
- `RunConfig` から `extensions["stream"]` を参照する経路がなく、チャンク単位での再開・バックプレッシャ制御を提供できない。  
- CLI/LSP でインクリメンタル解析を試みる際、バッチランナーを再呼び出しするしかなく、期待される性能と診断品質を満たせない。

### After
- `Core.Parse.Streaming`（新モジュール）を実装し、`run_stream` / `resume` / `StreamOutcome` / `Continuation` を仕様通りに提供する。  
- `RunConfig.extensions["stream"]` に `checkpoint` / `resume_hint` / `DemandHint` を格納し、バッチランナーとストリーミングランナーの結果が一致することを保証する。  
- PoC 段階ではチャンク投入・バックプレッシャの最小限機能を実装し、Phase 2-7 `execution-strategy` タスクへ本実装のロードマップを渡す。

#### API スケッチ
```ocaml
type feeder = unit -> chunk_result
type chunk_result =
  | Chunk of string
  | Closed
  | Pending of Demand_hint.t

val run_stream :
  parser:'a Core_parse.parser ->
  feeder:feeder ->
  cfg:streaming_config ->
  'a stream_outcome
```

## 3. 影響範囲と検証
- **一致性テスト**: `docs/spec/2-7-core-parse-streaming.md:254-267` に定義されたテスト計画を PoC へ反映し、`run` と `run_stream` の結果一致をゴールデン比較で確認。  
- **CI**: `tooling/ci` にストリーミング用テストシナリオを追加し、`RunConfig.extensions["stream"]` が埋まっているかをメトリクスで監視。  
- **CLI/LSP**: インクリメンタル解析を利用する CLI モードを追加し、`DemandHint` が動作するか手動検証。
- **OCaml テスト**: `compiler/ocaml/tests/streaming_runner_tests.ml` を新設し、`resume` や `Pending DemandHint::Pause` のフローが期待通りに推移するかステップ単位で検証する。

## 4. 実装ステップ

### Step 0. 仕様・実装差分の棚卸し（1.5日）
- 目的: 既存ランナーと仕様 (docs/spec/2-6-execution-strategy.md, docs/spec/2-7-core-parse-streaming.md) の差異を列挙し、PoC 範囲を明確化する。
- 主な作業:
  - `parser_driver.ml` と `parser_run_config.ml` の現状 API/拡張マップを確認し、`extensions["stream"]` の流入経路を洗い出す。
  - Phase 2-5 の進捗（`PARSER-002`, `ERR-001`, `ERR-002`）とクロスリファレンスし、ストリーミングが再利用すべき診断・RunConfig・Recover 設定を一覧化する。
- 調査・検証:
  - `docs/guides/core-parse-streaming.md` と `docs/plans/bootstrap-roadmap/2-5-review-log.md` の該当エントリを読み、仕様上の必須メタデータ（`DemandHint`, `ContinuationMeta`, 指標名）を再確認。
  - `tooling/ci/collect-iterator-audit-metrics.py` の Packrat/Recover 指標を調査し、ストリーミング PoC で観測すべきメトリクスを決める。

### Step 1. Core.Parse.Streaming モジュール骨格の抽出（2日）
- 目的: バッチランナーから共通処理を切り出し、新規モジュール `core_parse_streaming.{ml,mli}`（仮）に PoC 用の最小骨格を定義する。
- 主な作業:
  - `Core_parse`／`Parser_diag_state` の初期化・診断集約ロジックを再利用できる形で関数化し、ストリーミング側から呼べるようにする。
  - Packrat 状態 (`Core_parse.State`) と `RunConfig` を引き回すインターフェイスを整理し、`Core_parse.Streaming` の公開シグネチャを設計する。
- 調査・検証:
  - `parser_driver.ml:120-320` 辺りの初期化順序を確認し、必要な引数（`lex_pack`, `config`, `diag_state`）を抜け漏れなくモジュールへ伝達する。
  - `docs/spec/2-6-execution-strategy.md` §B のトランポリン要件と整合するかチェックする。

### Step 2. Feeder / Continuation / DemandHint モデル設計（2日）
- 目的: ストリーミング固有の型 (`Feeder`, `StreamOutcome`, `DemandHint`, `ContinuationMeta`) を PoC で表現し、診断との接点を定義する。
- 主な作業:
  - 仕様準拠の型定義を OCaml 版で表現し、`RunConfig.extensions["stream"]` から `resume_hint` 等を復元する処理を実装する。
  - `ContinuationMeta.expected_tokens` に `ERR-001` で導入した `ExpectationSummary` を流し込む接合部を作成し、CLI/LSP と同一の期待集合表示を保証する。
- 調査・検証:
  - `docs/spec/2-7-core-parse-streaming.md` §B-§C の契約を再読し、最低限 PoC で必要なフィールド（`min_bytes`, `commit_watermark`, `last_checkpoint` 等）を確定。
  - `parser_diag_state.ml` の `farthest_snapshot`・`span_trace_pairs` を参照し、継続に載せるデータ量の制約を調査する。

### Step 3. ストリーミング制御ループ PoC（3日）
- 目的: `run_stream` / `resume` を実装し、チャンク読み取り・バックプレッシャ・診断発火を最小限動作させる。
- 主な作業:
  - `Feeder.pull` → `Core_parse.Streaming.step` → `StreamOutcome` への制御フローを構築し、`Pending` と `Completed` の遷移を確認。
  - Packrat のコミットウォーターマークと入力バッファのライフサイクルを管理し、`Pending` 返却時に `DemandHint` を算出する。
  - `RunConfig extensions["stream"]` のプレースホルダ（`checkpoint`, `resume_hint`, `flow_mode` 等）を読み取るロジックを追加。
- 調査・検証:
  - `Core_parse.State.record_packrat_access` の呼び出しタイミングをバッチと比較し、Packrat 指標の整合を確認。
  - `docs/guides/core-parse-streaming.md` §5-§7 のドライバ例を PoC に当てはめ、欠落 API を洗い出す。

### Step 4. CLI/LSP/CI 連携と検証ケースの整備（2.5日）
- 目的: 新ランナーを CLI/LSP/テストに統合し、PoC の振る舞いを自動検証できる状態にする。
- 主な作業:
  - `compiler/ocaml/src/main.ml`, `tooling/lsp/run_config_loader.ml` へ `--streaming` 相当のハンドラを追加し、`RunConfig` 経由でストリーミング設定を配線。
  - `compiler/ocaml/tests/streaming_runner_tests.ml`（新設）で `run` と `run_stream` の結果一致、`Pending DemandHint::Pause` の分岐、`resume` の往復をテスト。
  - `tooling/ci/collect-iterator-audit-metrics.py` と `scripts/validate-diagnostic-json.sh` にストリーミング用メトリクス/サンプルを追加し、CI で欠落を検知できるようにする。
- 調査・検証:
  - 既存ゴールデン（`parser-runconfig-packrat.json.golden`）を基にストリーミング版フィクスチャを作成し、差分測定方法を決める。
  - LSP 側で既に実装済みのインクリメンタル診断フローを確認し、PoC との整合課題を洗い出す。

### Step 5. ドキュメント・フォローアップ登録（1.5日）
- 目的: PoC の制限・今後の課題を記録し、Phase 2-7 以降のフル実装に繋げる。
- 主な作業:
  - `docs/guides/core-parse-streaming.md`、`docs/spec/2-6-execution-strategy.md`、`docs/spec/2-7-core-parse-streaming.md` に PoC 状態と既知制限を脚注追加。
  - `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` と `docs/notes/runtime-bridges.md` へ制限/連携要件を追記。
  - `0-3-audit-and-metrics.md` に新規指標（例: `parser.stream.outcome_consistency`, `parser.stream.demandhint_coverage`）を登録し、計測結果を記録する。
- 調査・検証:
  - Phase 3 計画書におけるストリーミング要件を確認し、PoC で満たせていない項目を列挙。
  - CLI/LSP チームとレビューを実施し、PoC 公開チャネル（`-Zstreaming` フラグ等）の合意形成状況をまとめる。

## 5. フォローアップ
- PoC の仕様制限（例: チャンクサイズ固定、`Pending` の暗黙タイムアウト未実装）を `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` に記録し、本実装フェーズへ引き継ぐ。  
- `docs/guides/core-parse-streaming.md` のサンプルを PoC API で動作させ、差分を脚注として追加。  
- Phase 3 の self-host 計画書で `run_stream` をクリティカルパスに含めるため、進捗を `0-3-audit-and-metrics.md` に定期記録する。
- `docs/notes/runtime-bridges.md` にストリーミング API と Runtime Bridge の連携要件を追記し、バックプレッシャ信号の橋渡し方法を明文化する。
- **タイミング**: PARSER-001/002/LEXER-002 が揃った Phase 2-5 後半に PoC 実装へ着手し、Phase 2-6 開始前までに最小機能のストリーミングランナーを完成させる。

## 6. 残課題
- Feeder API とバックプレッシャの初期値（`DemandHint::Continue` / `::Pause`) をどの程度細分化するか決定が必要。  
- PoC をどのリリースチャネルに公開するか（`-Zstreaming` フラグの導入有無）を Phase 2-7 チームと合意したい。
