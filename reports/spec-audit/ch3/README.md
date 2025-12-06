# ch3 - Chapter 3 監査ログ

- 対象: `docs/spec/3-0-core-library-overview.md`〜`3-10-core-env.md`, `docs/spec/3-6-core-diagnostics-audit.md`, `docs/spec/3-8-core-runtime-capability.md`, `docs/guides/runtime-bridges.md`。
- 保存物: Rust Runtime/Adapter テスト結果、`collect-iterator-audit-metrics.py --section diagnostics|effects` の出力、`audit` JSON スナップショット。
- 手順: `cargo test --manifest-path compiler/rust/runtime/ffi/Cargo.toml`, `cargo test --manifest-path compiler/rust/adapter/Cargo.toml`, `python3 tooling/ci/collect-iterator-audit-metrics.py --section diagnostics --require-success` を実行し、標準出力を貼付する。
- 更新責任者: Rust Runtime WG（#rust-runtime）。
- `metric_point-emit_metric.json` には `tooling/ci/collect-iterator-audit-metrics.py --section numeric_time --scenario emit_metric --metric-source tests/data/metrics/metric_point_cases.json` の結果を保存し、`MetricPoint` → `AuditSink` 連携の監査メタデータ確認に利用する。
- `core_io_capabilities.json` には `core_io.capability_matrix_pass_rate` のメトリクス（`--scenario capability_matrix`）を保存し、`docs/plans/bootstrap-roadmap/assets/core-io-capability-map.md` の Stage/Provider/効果スコープ欠落を検証する。
- `io_bridge-capability-sync-*.md` には Runtime Bridge vs Capability Registry の同期結果（Watcher Stage trace / `metadata.io.watch.*` / `effects.contract.stage_mismatch` 整合）を記録する。
- `runtime_bridge-stage-records-*.json` には `BRIDGE_STAGE_RECORDS_PATH=<path> cargo test -p reml_runtime stage_records_are_accessible_after_fs_operations -- --nocapture` を実行した際に `RuntimeBridgeRegistry` が出力した Stage プローブのスナップショットを保存し、Rust ランタイム側の Bridge 記録を参照可能にする。
