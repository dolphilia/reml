# Runtime Capability テスト・CI チェックリスト

本書は `docs/plans/bootstrap-roadmap/3-8-core-runtime-capability-plan.md` §7 の作業を実施するための共通手順をまとめたメモである。`CapabilityRegistry` に関わる単体テスト、`core_runtime_capability` 例題スイート、CI 連携、KPI 記録を 1 つの資料で辿れるようにする。Phase 3 の Rust Runtime ジョブと `pipeline_branch` Stage mismatch 計画（`docs/plans/bootstrap-roadmap/pipeline_branch-stage-mismatch-plan.md`）の双方から参照する。

## 1. 単体テスト整備（7.1 用）
| # | 内容 | 手順 | 成果物 | 備考 |
| --- | --- | --- | --- | --- |
| 1 | `capability_registry::*` テストを CI 必須に昇格 | `cargo test -p reml_runtime capability_registry -- --nocapture` を最小構成で実行し、`tests/capability_registry.rs` の `CapabilityError` メッセージを `insta` スナップショット (`tests/snapshots/capability_registry__*.snap`) で固定する | `reports/spec-audit/ch3/runtime_capability-unit-YYYYMMDD.md` に実行ログと差分を保存 | スナップショット差分が出たら `insta review` を CI ではなく手元で実行し、承認済み snapshot のみコミットする。 |
| 2 | 多重登録負荷テスト | `cargo nextest run -p reml_runtime --run-ignored capability_registry_load` を `#[ignore]` テストで実行し、`tests/capability_registry_load.rs` に 1,000 件登録を行うケースを追加 | `reports/spec-audit/ch3/runtime_capability-load-YYYYMMDD.log` に nextest ログを保存 | `nextest.toml` で `profile.ci` に `retries = 0` を設定し、性能回帰を即時検知する。 |
| 3 | JSON フィクスチャ共有 | `compiler/rust/runtime/tests/fixtures/capabilities/*.json` へ `StageRequirement`・`effect_scope`・`provider` 等のサンプルを保存し、テストヘルパ `fixtures::load_capability("console")` を経由して参照する | `tests/fixtures/README.md` にフォーマットを記載し、`docs/spec/3-8-core-runtime-capability.md#capabilitydescriptor` の表と差分が出ないよう `scripts/capability/generate_md.py` を参照 | 既存の `examples/core_diagnostics/*.reml` ゴールデンを JSON 化して流用すると Stage 情報を重複管理せずに済む。 |

## 2. 統合テストと例題スイート（7.2 用）
- `examples/core_runtime_capability/README.md`（新設予定）で `core_runtime_capability/*.reml` を一覧化する。初期セットは `registry_success.reml`（登録→参照成功）と `registry_stage_violation.reml`（`CapabilityError::StageViolation`）の 2 件を最低ラインにする。例題は `tooling/examples/run_examples.sh --suite core_runtime_capability --with-audit` で実行し、CLI（stdout）と audit（stderr）をそれぞれ `*.expected.{diagnostic.json,audit.jsonl}` に保存する。
- `tooling/examples/run_examples.sh --suite core_runtime_capability --with-audit --update-golden` を追加し、`examples/core_diagnostics` と同じ `allowed failure` ハンドリングを導入する。`suite_config/core_runtime_capability.env`（新設）で `ALLOW_FAILURE=registry_stage_violation` を明示する。
- `scripts/poc_dualwrite_compare.sh --runtime-capability <example>` を追加し、OCaml 実装のランタイムと差分比較する。差分ログは `reports/dual-write/runtime_capability/` に保存し、`docs/plans/rust-migration/2-1-runtime-integration.md` から参照する。
- `collect-iterator-audit-metrics.py --section runtime --scenario capability_registry --diagnostic-source examples/core_runtime_capability/*.expected.diagnostic.json --audit-source ...audit.jsonl --require-success` を新設し、`reports/spec-audit/ch3/runtime_capability-suite-YYYYMMDD.json` に `capability_registry_pass_rate` とテストケース一覧を出力する。`docs/notes/runtime-capability-stage-log.md#runtime-capability-suite` で run_id を管理する。

## 3. CI 連携（7.3 用）
1. `.github/workflows/rust-runtime.yaml` で `runtime-capability-unit`（`cargo test -p reml_runtime capability_registry`）、`runtime-capability-nextest`、`core-runtime-capability-suite` の 3 ジョブを並列化する。Linux/macOS/Windows のマトリクスを共有し、`cargo nextest` ジョブのみ Linux x86_64 に限定してもよい。
2. CI 失敗時に `scripts/ci/post_failure_runtime_capability.sh` を呼び出し、以下を記録する。
   - `docs/plans/bootstrap-roadmap/0-4-risk-handling.md#runtime-capability` の表へ Run ID / 失敗ジョブ / Git SHA / 主要診断キー (`capability.error.code`) を追記
   - `reports/audit/dashboard/runtime_capability-ci-YYYYMMDD.md` にコンソールログの抜粋と `collect-iterator-audit-metrics` の結果を保存
3. `docs/plans/bootstrap-roadmap/assets/metrics/runtime-capability-ci.csv` に `run_id,date,job,pass_rate,failures,artifact,notes` を追記し、`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の `runtime.capability_ci_pass_rate` 指標から参照する。`job` には `runtime-capability-unit` / `runtime-capability-nextest` / `core-runtime-capability-suite` を使用する。
4. KPI 収集が成功した場合は `collect-iterator-audit-metrics.py --section runtime --scenario capability_registry` の結果を `reports/spec-audit/ch3/runtime_capability-ci-summary-YYYYMMDD.json` に保存し、CI アーティファクトへ添付する。

## 4. 相互参照
- `docs/plans/bootstrap-roadmap/pipeline_branch-stage-mismatch-plan.md#7-テスト・ci-反映` では `core_diagnostics` スイートの allowed failure ロジックが `core_runtime_capability` スイートにも適用される旨を共有する。
- `docs/plans/bootstrap-roadmap/3-6-core-diagnostics-audit-plan.md` に Stage mismatch のサンプルとして `core_runtime_capability` の audit ゴールデンを引用し、`collect-iterator-audit-metrics` の `--scenario capability_registry` が `Pipeline` 指標と連動していることを明示する。
- `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` と `docs/plans/bootstrap-roadmap/0-4-risk-handling.md` から本チェックリストにリンクし、CI 停止時のトリアージを一本化する。
