# Runtime Capability Stage ログ

Core.Runtime の Capability で Stage 要件や監査メタデータの扱いに差分が生じた場合、本ログに記録する。`collect-iterator-audit-metrics.py` で検証する KPI とリンクし、Phase3 self-host 判定時に参照する。

## 2025-12-08 Core.Time / Timezone
- 対象 Capability: `core.time.timezone.local`, `core.time.timezone.lookup`
- Stage 要件: `StageRequirement::AtLeast(StageId::Beta)`（`compiler/rust/runtime/src/time/timezone.rs::verify_capability`）
- 診断・監査メタデータ:  
  - `extensions["time"].platform` / `audit.metadata["time.platform"]` に `std::env::consts::OS` を記録  
  - `extensions["time"].timezone` / `audit.metadata["time.timezone"]` に解決対象の TZ 名 (`UTC±HH:MM`) を記録  
  - Capability 検証に失敗した場合は `TimeError::system_clock_unavailable(...).with_capability_context(...)` で `time.capability` / `time.required_stage` / `time.actual_stage` を書き込む
- 観測方法: `python3 tooling/ci/collect-iterator-audit-metrics.py --section numeric_time --scenario timezone_lookup --tz-source tests/data/time/timezone_cases.json`。`time.timezone.lookup_consistency` KPI が 1.0 を下回った場合、本ログにプラットフォーム差分と再現手順を追記する。
- 関連ドキュメント: `docs/plans/bootstrap-roadmap/3-4-core-numeric-time-plan.md` §4.2, `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` `time.timezone.lookup_consistency` 行。

## 2025-12-11 Core.Diagnostics / Metrics.Emit
- 対象 Capability: `metrics.emit`
- Stage 要件: `StageRequirement::Exact(StageId::Stable)`（`compiler/rust/runtime/src/diagnostics/metric_point.rs::emit_metric`）
- 診断・監査メタデータ:
  - 成功時: `metric_point.*` / `effect.stage.required = "stable"` / `effect.stage.actual = "stable"` / `effect.required_effects = ["audit"]`
  - 失敗時: `effects.contract.stage_mismatch` 診断を返し、`extensions["effects.contract.stage.*"]` と `AuditEnvelope.metadata["effect.stage.*"]` の双方へ Capability/Stage/Required effects を記録
- 観測方法:
  - `cargo test --manifest-path compiler/rust/runtime/Cargo.toml --features core_numeric,core_time metrics_capability`
  - `python3 tooling/ci/collect-iterator-audit-metrics.py --section numeric_time --scenario emit_metric --metric-source tests/data/metrics/metric_point_cases.json`
  - Stage mismatch の再現ログは `reports/dual-write/metrics-stage-mismatch.json`
- 関連ドキュメント: `docs/plans/bootstrap-roadmap/3-4-core-numeric-time-plan.md` §5.3, `docs/notes/runtime-metrics-capability.md`
