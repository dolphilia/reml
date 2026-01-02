# IO Bridge Capability Sync (2025-12-06)

## 目的
- Core.IO Capability マトリクス（`docs/plans/bootstrap-roadmap/assets/core-io-capability-map.md`）と Runtime Bridge / Capability Registry の整合を確認し、Watcher/Console 系 Stage トレースが `AuditEnvelope.metadata` へ漏れなく転写されているかを検証する。
- Plan 3.8 §5.5 の Runbook で要求される `capability_matrix` シナリオと `RuntimeBridgeRegistry` の同期ログを 1 つのレポートで参照できるようにする。

## 実行手順
1. Core Diagnostics サンプル再実行：
   ```bash
   tooling/examples/run_examples.sh --suite core_diagnostics --with-audit
   ```
   - `pipeline_success` Run ID: `06c6a78e-be71-4323-a6fd-23e74515bf34`
   - `pipeline_branch` Run ID: `ec456a62-42bc-4cf6-9fed-5858fdc9fc83`
2. Capability マトリクス検証：
   ```bash
   python3 tooling/ci/collect-iterator-audit-metrics.py \
     --section core_io \
     --scenario capability_matrix \
     --matrix docs/plans/bootstrap-roadmap/assets/core-io-capability-map.md \
     --output reports/spec-audit/ch3/core_io_capabilities.json \
     --require-success
   ```
3. Capability Registry テスト：
   ```bash
   cargo test -p reml_runtime core_io_capability_matrix
   ```

## 観測結果
- `core_io_capabilities.json` で 13 行すべてが `Stage`/`Provider`/`Effect Scope` を満たし、`pass_rate=1.0` を記録。Watcher 派生行（`watcher.fschange` / `watcher.recursive`）は `platform:linux|macos|windows` として保持され、`metadata.io.feature` が `IoErrorKind::UnsupportedPlatform` に転写されることを確認済み。
- `pipeline_branch` 監査ログ（`examples/core_diagnostics/pipeline_branch.expected.audit.jsonl`）では `bridge.stage.trace[*]` と `effects.contract.stage_trace[*]` が同一配列であり、Capability Registry → RuntimeBridgeRegistry の Stage 要件が一致している。
- `cargo test -p reml_runtime core_io_capability_matrix` で `fs.symlink.modify` / `fs.watcher.*` / `watcher.resource_limits` を含む Stage 検証が成功。失敗ケース（`io.fs.read` に `Exact(StageId::Alpha)` を要求）も `capability.stage.mismatch` として検知され、Registry 側の Stage メタデータがテストで参照できることを確認。
- Rust ランタイム内でも Bridge 記録が参照できるようになり、`cargo test -p reml_runtime stage_records_are_accessible_after_fs_operations -- --nocapture` を `BRIDGE_STAGE_RECORDS_PATH=reports/spec-audit/ch3/runtime_bridge-stage-records-20251206.json` と共に実行すると、`RuntimeBridgeRegistry` が収集した Stage プローブを JSON として書き出せる。

## 生成物
- `reports/spec-audit/ch3/core_io_capabilities.json`
- `reports/spec-audit/ch3/runtime_bridge-stage-records-20251206.json`
- `examples/core_diagnostics/pipeline_branch.expected.{diagnostic.json,audit.jsonl}`（差分なし、Run ID のみ確認）
- `tests/capabilities/core_io_registry.json`（Watcher/Permissions 行を追加）

## フォローアップ
- `docs/plans/bootstrap-roadmap/3-8-core-runtime-capability-plan.md#5.5` に Run ID `20251206-core-io-capability-matrix` を追記。
- `docs/notes/runtime/runtime-capability-stage-log.md#2025-12-06-coreio-capability-マトリクスrun-id-20251206-core-io-capability-matrix` で本レポートを参照できるようにした。
