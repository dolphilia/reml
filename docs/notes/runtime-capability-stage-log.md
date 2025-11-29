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

## 2025-12-15 Core.IO / Path Security & Watcher
- 対象 Capability: `io.fs.read`, `io.fs.write`, `fs.permissions.read`, `fs.permissions.modify`, `fs.symlink.query`, `fs.symlink.modify`, `fs.watcher.native`, `fs.watcher.recursive`, `security.fs.policy`
- Stage 要件:
  - `io.fs.*`: `StageRequirement::AtLeast(StageId::Beta)`（IO API 共有基盤）。`compiler/rust/runtime/src/io/{reader.rs,writer.rs}` から `CapabilityRegistry::verify_capability_stage("io.fs", ..)` を呼び出す設計。
  - `fs.permissions.*` / `security.fs.policy`: `StageRequirement::Exact(StageId::Stable)`。`SecurityCapability` を経由して `effect.stage.required = "stable"` を診断へ転写。
  - `fs.symlink.*`: `StageRequirement::AtLeast(StageId::Beta)`（Windows 開発者モードや POSIX `lstat` 依存のため）。
  - `fs.watcher.*`: `StageRequirement::AtLeast(StageId::Beta)`（`watch_with_limits` の recursive 対応は Stable 昇格条件付き）。
- 診断・監査メタデータ:
  - すべての IO API で `IoContext` に `metadata.io.capability`, `metadata.io.operation`, `metadata.io.path`, `metadata.security.policy_digest`（policy 利用時）を記録する。
  - Stage 取得結果は `effect.stage.required` / `effect.stage.actual` / `effects.contract.stage_mismatch`（不一致時）へ転写し、Watcher 系は加えて `AuditEnvelope.metadata["io.watch.queue_size"]`, `["io.watch.delay_ns"]` を必須化する。
  - `security.fs.policy` 経由の拒否は `IoErrorKind::SecurityViolation` → `diagnostic("core.path.security.violation")` を生成し、`metadata.security.tripped_capability` に Capability ID を書き込む。
- 観測方法:
  - `docs/plans/bootstrap-roadmap/assets/core-io-capability-map.md` を基準に `python3 tooling/ci/collect-iterator-audit-metrics.py --section core_io --scenario capability_matrix --matrix docs/plans/bootstrap-roadmap/assets/core-io-capability-map.md --output reports/spec-audit/ch3/core_io_capabilities.json --require-success` を実行して Capability / Stage / effect ラベルの整合を確認する。
  - `scripts/validate-diagnostic-json.sh --pattern core.io --pattern core.path.security --pattern core.io.watcher` を用いて診断メタデータが欠落していないことを検証し、結果を `reports/spec-audit/ch3/core_io_summary-YYYYMMDD.md` に記録する。
  - Watcher 実装後は `RuntimeBridgeRegistry` の `describe_bridge("native.fs.watch")` 出力を `docs/notes/runtime-bridges-roadmap.md` に添付し、Stage mismtach が出た場合は本ログにも Run ID を追記する。
- 関連ドキュメント: `docs/plans/bootstrap-roadmap/assets/core-io-capability-map.md`, `docs/plans/bootstrap-roadmap/3-5-core-io-path-plan.md` §1.3, `docs/plans/bootstrap-roadmap/3-8-core-runtime-capability-plan.md`（Runbook 追記）、`docs/guides/runtime-bridges.md`
