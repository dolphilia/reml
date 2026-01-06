# 調査メモ: 第22章 テスト戦略

## 対象モジュール

- `compiler/runtime/src/test/mod.rs`
- `compiler/runtime/src/test/dsl/mod.rs`
- `docs/spec/3-11-core-test.md`
- `docs/guides/tooling/testing.md`
- `tooling/examples/run_examples.sh`
- `tests/reml_e2e/tests/scenario.rs`
- `compiler/frontend/tests/`
- `compiler/runtime/tests/`
- `tests/`

## 入口と全体像

- Core.Test の Rust 実装は `compiler/runtime/src/test/mod.rs` が入口で、スナップショットやテーブル駆動、ファジングの最小機能を提供している。
  - `compiler/runtime/src/test/mod.rs:1-258`
- DSL 向けテストキット（`Core.Test.Dsl` 相当）は `compiler/runtime/src/test/dsl/mod.rs` にまとまっており、`test_parser` が Parser の出力検証・ゴールデン更新を担う。
  - `compiler/runtime/src/test/dsl/mod.rs:1-259`
- スナップショット運用やゴールデンの配置はガイドで定義され、Phase4 の回帰シナリオと同期する設計が明記されている。
  - `docs/guides/tooling/testing.md:1-77`

## Core.Test 実装の要点

- `TestError` / `TestErrorKind` / `SnapshotPolicy` / `SnapshotMode` が API の中心で、`TestError::into_diagnostic` で `test.failed` 診断を生成する。
  - `compiler/runtime/src/test/mod.rs:30-127`
- スナップショットはプロセス内メモリに保持し、`verify/update/record` を `SnapshotMode` で切り替える。最大サイズ超過は `HarnessFailure` 扱い。
  - `compiler/runtime/src/test/mod.rs:165-321`
- スナップショット更新時は `AuditEvent::SnapshotUpdated` を記録し、`snapshot.name/hash/mode/bytes` を metadata に載せる。
  - `compiler/runtime/src/test/mod.rs:329-359`
- `test_with` はケース名を `TestError` に付与して診断を記録し、`take_test_diagnostics` / `take_test_audit_events` で回収する。
  - `compiler/runtime/src/test/mod.rs:208-387`
- ファジングは `FuzzGenerator` の簡易 RNG を用いて、panic を `FuzzCrash` として収束させる。
  - `compiler/runtime/src/test/mod.rs:238-421`

## DSL Test Kit とゴールデン

- `DslCase` と `DslExpectation` がテストケース定義、`AstMatcher` が AST の一致判定を担う（`Exact/Any/Pattern/List/Record`）。
  - `compiler/runtime/src/test/dsl/mod.rs:15-62`
- `test_parser` は `run_with_default` を使って `ParseResult` を評価し、AST/診断/ゴールデンの期待値に応じて分岐する。
  - `compiler/runtime/src/test/dsl/mod.rs:122-259`
- エラー比較は診断コード + 位置 + メッセージ部分一致を評価し、`parser.unexpected_eof` を EOF 判定に寄せる補正ロジックがある。
  - `compiler/runtime/src/test/dsl/mod.rs:174-299`
- ゴールデンケースは `*.input` / `*.ast` / `*.error` の3点セットを前提にし、`SnapshotPolicy` に応じて更新/検証/記録する。
  - `compiler/runtime/src/test/dsl/mod.rs:77-412`

## 仕様とガイドの整合ポイント

- `Core.Test` の目的・API・SnapshotPolicy・FuzzConfig・診断/監査の取り扱いは spec に定義されている。
  - `docs/spec/3-11-core-test.md:1-118`
- `Core.Test.Dsl` の構文・`AtSpec`・診断コードの最小セットは spec の DSL Test Kit で整理されている。
  - `docs/spec/3-11-core-test.md:119-219`
- ガイドは `snapshot.name` と `scenario_id` の一致、`update` の使用制限、ゴールデンの配置規約を明記している。
  - `docs/guides/tooling/testing.md:58-77`

## 回帰テストと実行スイート

- `tooling/examples/run_examples.sh` は spec/practical/language_impl の Phase4 スイートと、`core_*` サンプルを実行するユーティリティ。
  - `tooling/examples/run_examples.sh:19-286`
- `--update-golden` では `reml_frontend` の JSON 出力を `*.expected.diagnostic.json` / `*.expected.audit.jsonl` に更新する。
  - `tooling/examples/run_examples.sh:204-266`
- E2E テストは `tests/reml_e2e/tests/scenario.rs` が `tooling/examples/run_phase4_suite.py` を呼び出してスイートを実行する。
  - `tests/reml_e2e/tests/scenario.rs:50-96`

## テスト配置の概要

- フロントエンドの Rust テストは `compiler/frontend/tests/` にあり、`__snapshots__` や `snapshots/` ディレクトリでスナップショットを管理している。
- ランタイムの統合テストは `compiler/runtime/tests/` に集中し、`expected/` / `fixtures/` / `golden/` / `snapshots/` などで期待値を分離している。
- リポジトリ直下の `tests/` は E2E やデータセット（`data/` / `expected/` / `capabilities/`）の置き場になっている。

## TODO / 不明点

- `Core.Test` のスナップショットは現在メモリのみで、`docs/spec/3-11-core-test.md` が想定するファイル I/O 連携は未実装（後続フェーズ対応）。
- `tooling/examples/run_phase4_suite.py` 側の更新ポリシーやディレクトリ構成の詳細は別途確認が必要。
