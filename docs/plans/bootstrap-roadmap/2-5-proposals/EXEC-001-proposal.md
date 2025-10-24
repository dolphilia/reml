# EXEC-001 ストリーミング実行 PoC 提案

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

## 4. フォローアップ
- PoC の仕様制限（例: チャンクサイズ固定、`Pending` の暗黙タイムアウト未実装）を `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` に記録し、本実装フェーズへ引き継ぐ。  
- `docs/guides/core-parse-streaming.md` のサンプルを PoC API で動作させ、差分を脚注として追加。  
- Phase 3 の self-host 計画書で `run_stream` をクリティカルパスに含めるため、進捗を `0-3-audit-and-metrics.md` に定期記録する。
- `docs/notes/runtime-bridges.md` にストリーミング API と Runtime Bridge の連携要件を追記し、バックプレッシャ信号の橋渡し方法を明文化する。
- **タイミング**: PARSER-001/002/LEXER-002 が揃った Phase 2-5 後半に PoC 実装へ着手し、Phase 2-6 開始前までに最小機能のストリーミングランナーを完成させる。

## 確認事項
- Feeder API とバックプレッシャの初期値（`DemandHint::Continue` / `::Pause`) をどの程度細分化するか決定が必要。  
- PoC をどのリリースチャネルに公開するか（`-Zstreaming` フラグの導入有無）を Phase 2-7 チームと合意したい。
