# Runtime Capability Stage ログ

Core.Runtime の Capability で Stage 要件や監査メタデータの扱いに差分が生じた場合、本ログに記録する。`collect-iterator-audit-metrics.py` で検証する KPI とリンクし、Phase3 self-host 判定時に参照する。

## 2025-12-30 Capability ドキュメント同期（Run ID: 20251230-capability-doc-sync）
- `scripts/capability/generate_md.py --json reports/spec-audit/ch3/capability_list-20251205.json --output docs/spec/3-8-core-runtime-capability.md` を実行し、`reml_capability list --format json` の結果を仕様本文のテーブルへ再反映した。README のスナップショットと同じ JSON を用いることで、Stage/EffectScope/Provider 情報が 3.8 章・リポジトリ索引・CI レポート間で揃う。
- `docs/spec/3-0-core-library-overview.md` と `docs/spec/1-0-language-core-overview.md` に Capability Registry／`effects.contract.stage_mismatch` サンプルへの導線を追記した。`reports/spec-audit/ch3/capability_stage-mismatch-20251206.json` を参照すれば Stage 差分の監査ログをそのまま確認できる。
- `docs/plans/bootstrap-roadmap/assets/capability-stage-flow.mmd` をもとに `docs/plans/bootstrap-roadmap/assets/capability-stage-flow.svg` を書き出し（Mermaid 図を手動で SVG 化）、RunConfig→CapabilityRegistry→StageAuditPayload→Diagnostics/Audit/KPI の経路を図式化した。図の再生成は `mmdc -i capability-stage-flow.mmd -o capability-stage-flow.svg`（Mermaid CLI が利用可能な環境）または今回追加した SVG を直接編集して行う。

## 2025-12-06 Core.Diagnostics Stage mismatch
- 対象 Capability: `console`（`effects.contract.stage_mismatch` を再現する `examples/core_diagnostics/pipeline_branch.reml`）
- Stage 要件: `StageRequirement::AtLeast(StageId::Beta)`（`perform Console` が `RuntimeBridgeRegistry` の `core.console` と突き合わせられる）
- 診断・監査メタデータ:
  - `examples/core_diagnostics/pipeline_branch.expected.diagnostic.json` に `capability.id=console` / `capability.expected_stage=at_least:beta` / `capability.actual_stage=at_least:stable` を保持し、`effects.contract.stage_trace` と `bridge.stage.trace` が CLI/LSP/Audit 共通で同一配列を指すことを確認。
  - 監査ログは `examples/core_diagnostics/pipeline_branch.expected.audit.jsonl`（2 行）に保存し、`pipeline.outcome=success` / `pipeline.exit_code=failure` が stage mismatch 発生時でも揃うことを確認。
- 観測方法:
  - `tooling/examples/run_examples.sh --suite core_diagnostics --with-audit --update-golden`（内部で `target/debug/reml_frontend --output json --emit-audit-log .../pipeline_branch.reml` を実行）で Run ID `80b0d934-6b51-4718-9fc4-dcff8c57b849` を取得し、結果を `reports/spec-audit/ch3/capability_stage-mismatch-20251206.json` に集約。
  - `scripts/validate-diagnostic-json.sh reports/spec-audit/ch3/capability_stage-mismatch-20251206.json --effect-tag runtime` を実行して `capability.*` / `effect.stage.*` / `effects.contract.stage_trace` / `pipeline.*` の必須キーが欠落していないことを確認。
- フォローアップ:
  - ✅ 2025-12-06: `tooling/examples/run_examples.sh --suite core_diagnostics --with-audit` を再実行し、`pipeline_success`（run_id=`06c6a78e-be71-4323-a6fd-23e74515bf34`）/`pipeline_branch`（run_id=`ec456a62-42bc-4cf6-9fed-5858fdc9fc83`）の audit メタデータが Capability マトリクス更新後も変化していないことを確認。Run 情報は `docs/plans/bootstrap-roadmap/pipeline_branch-stage-mismatch-plan.md#run-id-ec456a62-42bc-4cf6-9fed-5858fdc9fc83` に追記済み。
- 関連ドキュメント: `docs/plans/bootstrap-roadmap/pipeline_branch-stage-mismatch-plan.md`（§6 実施ログ）、`docs/plans/bootstrap-roadmap/3-8-core-runtime-capability-plan.md#5.2-実施結果`

## 2025-12-08 Core.Time / Timezone
- 対象 Capability: `core.time.timezone.local`, `core.time.timezone.lookup`
- Stage 要件: `StageRequirement::AtLeast(StageId::Beta)`（`compiler/runtime/src/time/timezone.rs::verify_capability`）
- 診断・監査メタデータ:  
  - `extensions["time"].platform` / `audit.metadata["time.platform"]` に `std::env::consts::OS` を記録  
  - `extensions["time"].timezone` / `audit.metadata["time.timezone"]` に解決対象の TZ 名 (`UTC±HH:MM`) を記録  
  - Capability 検証に失敗した場合は `TimeError::system_clock_unavailable(...).with_capability_context(...)` で `time.capability` / `time.required_stage` / `time.actual_stage` を書き込む
- 観測方法: `python3 tooling/ci/collect-iterator-audit-metrics.py --section numeric_time --scenario timezone_lookup --tz-source tests/data/time/timezone_cases.json`。`time.timezone.lookup_consistency` KPI が 1.0 を下回った場合、本ログにプラットフォーム差分と再現手順を追記する。
- 関連ドキュメント: `docs/plans/bootstrap-roadmap/3-4-core-numeric-time-plan.md` §4.2, `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` `time.timezone.lookup_consistency` 行。

## 2025-12-11 Core.Diagnostics / Metrics.Emit
- 対象 Capability: `metrics.emit`
- Stage 要件: `StageRequirement::Exact(StageId::Stable)`（`compiler/runtime/src/diagnostics/metric_point.rs::emit_metric`）
- 診断・監査メタデータ:
  - 成功時: `metric_point.*` / `effect.stage.required = "stable"` / `effect.stage.actual = "stable"` / `effect.required_effects = ["audit"]`
  - 失敗時: `effects.contract.stage_mismatch` 診断を返し、`extensions["effects.contract.stage.*"]` と `AuditEnvelope.metadata["effect.stage.*"]` の双方へ Capability/Stage/Required effects を記録
- 観測方法:
  - `cargo test --manifest-path compiler/runtime/Cargo.toml --features core_numeric,core_time metrics_capability`
  - `python3 tooling/ci/collect-iterator-audit-metrics.py --section numeric_time --scenario emit_metric --metric-source tests/data/metrics/metric_point_cases.json`
  - Stage mismatch の再現ログは `reports/dual-write/metrics-stage-mismatch.json`
- 関連ドキュメント: `docs/plans/bootstrap-roadmap/3-4-core-numeric-time-plan.md` §5.3, `docs/notes/runtime/runtime-metrics-capability.md`

## 2025-12-15 Core.IO / Path Security & Watcher
- 対象 Capability: `io.fs.read`, `io.fs.write`, `fs.permissions.read`, `fs.permissions.modify`, `fs.symlink.query`, `fs.symlink.modify`, `fs.watcher.native`, `fs.watcher.recursive`, `security.fs.policy`
- Stage 要件:
  - `io.fs.*`: `StageRequirement::AtLeast(StageId::Beta)`（IO API 共有基盤）。`compiler/runtime/src/io/{reader.rs,writer.rs}` から `CapabilityRegistry::verify_capability_stage("io.fs", ..)` を呼び出す設計。
  - `fs.permissions.*` / `security.fs.policy`: `StageRequirement::Exact(StageId::Stable)`。`SecurityCapability` を経由して `effect.stage.required = "stable"` を診断へ転写。
  - `fs.symlink.*`: `StageRequirement::AtLeast(StageId::Beta)`（Windows 開発者モードや POSIX `lstat` 依存のため）。
  - `fs.watcher.*`: `StageRequirement::AtLeast(StageId::Beta)`（`watch_with_limits` の recursive 対応は Stable 昇格条件付き）。
- 診断・監査メタデータ:
  - すべての IO API で `IoContext` に `metadata.io.capability`, `metadata.io.operation`, `metadata.io.path`, `metadata.security.policy_digest`（policy 利用時）を記録する。
  - Stage 取得結果は `effect.stage.required` / `effect.stage.actual` / `effects.contract.stage_mismatch`（不一致時）へ転写し、Watcher 系は加えて `AuditEnvelope.metadata["io.watch.queue_size"]`, `["io.watch.delay_ns"]` を必須化する。
  - `security.fs.policy` 経由の拒否は `IoErrorKind::SecurityViolation` → `diagnostic("core.path.security.violation")` を生成し、`metadata.security.tripped_capability` に Capability ID を書き込む。
- 接続状況:
  - `compiler/runtime/src/io/adapters.rs` に `FsAdapter`/`WatcherAdapter` を追加し、`Reader`/`Writer` から `FsAdapter::ensure_{read,write}` を呼び出すことで `io.fs.*` Capability を `verify_capability_stage` に渡している。
  - Watcher Capability はまだ実装呼び出しが無いため、`WatcherAdapter` で Stage のキャッシュだけを提供し、監視 API 実装時に `ensure_{native,recursive}` を利用する。
- 観測方法:
  - `docs/plans/bootstrap-roadmap/assets/core-io-capability-map.md` を基準に `python3 tooling/ci/collect-iterator-audit-metrics.py --section core_io --scenario capability_matrix --matrix docs/plans/bootstrap-roadmap/assets/core-io-capability-map.md --output reports/spec-audit/ch3/core_io_capabilities.json --require-success` を実行して Capability / Stage / effect ラベルの整合を確認する。
  - `scripts/validate-diagnostic-json.sh --pattern core.io --pattern core.path.security --pattern core.io.watcher` を用いて診断メタデータが欠落していないことを検証し、結果を `reports/spec-audit/ch3/core_io_summary-YYYYMMDD.md` に記録する。
  - Watcher 実装後は `RuntimeBridgeRegistry` の `describe_bridge("native.fs.watch")` 出力を `docs/notes/runtime/runtime-bridges-roadmap.md` に添付し、Stage mismtach が出た場合は本ログにも Run ID を追記する。
- ✅ 2025-12-19: `compiler/runtime/src/path/security.rs` で `SecurityPolicy` / `PathSecurityError` を導入し、`validate_path` / `sandbox_path` / `is_safe_symlink` が `FsAdapter::ensure_security_policy()`・`ensure_symlink_query()` を呼び出すように更新。`cargo test --manifest-path compiler/runtime/Cargo.toml path_security` と `tests/data/core_path/security/*.json` で `core.path.security.invalid`/`violation`/`symlink` 診断に `metadata.security.reason`, `effect.security`, `effect.stage.required = "stable"` が含まれることを確認した。`core-io-capability-map` と `core-io-effects-matrix` にも Rust 実装の検証ポイントを追記済み。
- 関連ドキュメント: `docs/plans/bootstrap-roadmap/assets/core-io-capability-map.md`, `docs/plans/bootstrap-roadmap/3-5-core-io-path-plan.md` §1.3, `docs/plans/bootstrap-roadmap/3-8-core-runtime-capability-plan.md`（Runbook 追記）、`docs/guides/runtime/runtime-bridges.md`

## 2025-12-06 Core.IO Capability マトリクス（Run ID: 20251206-core-io-capability-matrix）
- 対象 Capability: `io.fs.*`, `fs.permissions.*`, `fs.symlink.*`, `fs.watcher.*`, `watcher.*`, `security.fs.policy`, `memory.buffered_io`
- 目的: `docs/plans/bootstrap-roadmap/assets/core-io-capability-map.md` に Stage/Provider/効果スコープ列を追加し、CI で `collect-iterator-audit-metrics.py --section core_io --scenario capability_matrix --matrix docs/plans/bootstrap-roadmap/assets/core-io-capability-map.md --output reports/spec-audit/ch3/core_io_capabilities.json --require-success` を実行するための基準票を整備する。
- 成果物:
  - `reports/spec-audit/ch3/core_io_capabilities.json` に `core_io.capability_matrix_pass_rate`（pass_rate=1.0, total=13）を保存し、`watcher.fschange`/`watcher.recursive` の `platform:*` 行が OS 不一致を検知できることを確認。
  - `reports/spec-audit/ch3/io_bridge-capability-sync-20251206.md` を追加し、`RuntimeBridgeRegistry` と Capability Registry の同期状況（Watcher Stage trace と Capability Hook の対応）を記録。
  - `tests/capabilities/core_io_registry.json` / `compiler/runtime/tests/core_io_capabilities.rs` を更新し、`cargo test -p reml_runtime core_io_capability_matrix` が `fs.symlink.modify` / `fs.watcher.*` / `watcher.resource_limits` を網羅するようにした。
- Rust ランタイムの `RuntimeBridgeRegistry` を `runtime::bridge` モジュールとして実装し、`FsAdapter::ensure_*` などの Stage 検証で `record_stage_probe` が呼ばれるように更新。`BRIDGE_STAGE_RECORDS_PATH=reports/spec-audit/ch3/runtime_bridge-stage-records-20251206.json cargo test -p reml_runtime stage_records_are_accessible_after_fs_operations -- --nocapture` を実行すると JSON スナップショットを取得でき、io_bridge-capability-sync レポートから Rust 側 Stage 記録を引用可能になった。
- 関連ドキュメント: `docs/plans/bootstrap-roadmap/3-8-core-runtime-capability-plan.md#5.5`（Runbook 追加）、`docs/plans/bootstrap-roadmap/3-5-core-io-path-plan.md`（Capability 整合セクション）、`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md`（`core_io.capability_matrix_pass_rate` KPI 追加）、`docs/plans/bootstrap-roadmap/assets/core-io-capability-map.md`

## 2025-12-21 Core.IO Watcher クロスプラットフォーム Capability
- 対象 Capability: `watcher.fschange`, `watcher.recursive`, `watcher.resource_limits`（`fs.watcher.*` に対する OS サポート層）
- Stage 要件:
  - `watcher.fschange`: `StageRequirement::AtLeast(StageId::Beta)`。Linux/macOS/Windows のみ対応。Registry 上は `fs.watcher.native` と同値だが、非対応 OS は `UnsupportedPlatform`.
  - `watcher.recursive`: `StageRequirement::Exact(StageId::Stable)`（Linux/Windows）。macOS は `StageId::Beta` 扱い。その他 OS は `UnsupportedPlatform`.
  - `watcher.resource_limits`: `StageRequirement::AtLeast(StageId::Beta)`（`WatcherAdapter::ensure_resource_limit_capability`）。`WatchLimits` が有効な OS のみサポート。
- 診断・監査メタデータ:
  - `IoErrorKind::UnsupportedPlatform` に `IoError::with_platform` / `with_feature` を追加し、`extensions["io"].platform`, `extensions["io"].feature` および `AuditEnvelope.metadata["io.platform"]`, `["io.feature"]` を必須化。
  - `watch_with_limits` では `WatchLimits::uses_resource_limits()` を検知して Capability チェック後に OS 判定 (`ensure_watcher_feature`) を行い、失敗時は `core.io.unsupported_platform` 診断に `metadata.io.capability = "watcher.resource_limits"` を含める。
  - `watcher.rs` が `ensure_watcher_feature` で `std::env::consts::OS` を測定し、`watcher_audit` 出力 (`reports/spec-audit/ch3/io_watcher-simple_case.jsonl`) に `io.watch.*` と同時に `io.platform`/`io.feature` を記録。
- 観測方法:
  - `python3 tooling/ci/collect-iterator-audit-metrics.py --section core_io --scenario watcher_audit --check recursive` で `watcher.recursive` の pass/fail を確認。
  - `scripts/validate-diagnostic-json.sh --pattern core.io.watcher --pattern core.io.unsupported_platform` で `metadata.io.platform` / `metadata.io.feature` / `audit["io.capability"]` が欠落しないことを検証。
  - 非対応 OS（CI の cross target など）では `watch` コマンドが即座に `IoErrorKind::UnsupportedPlatform` を返し、Run ID ごとのログを `reports/spec-audit/ch3/io_watcher-unsupported_platform.md` に追加する運用とする。
- 関連ドキュメント: `docs/plans/bootstrap-roadmap/assets/core-io-capability-map.md`（Watcher 新行）、`docs/plans/bootstrap-roadmap/3-5-core-io-path-plan.md` §5.3 進捗ログ、`docs/plans/bootstrap-roadmap/3-8-core-runtime-capability-plan.md` §5.5（Runbook 追記）

## 2029-12-21 Capability Handle Inventory（Run ID: 20291221-core-runtime-capability）
- 目的: `3-8-core-runtime-capability.md` §1.1 に列挙された Capability ハンドルの実装状況を Rust 側で棚卸しし、Stage 判定や監査統合の欠落を共有する。
- 成果物:
  - `docs/plans/bootstrap-roadmap/assets/capability-handle-inventory.csv`: Gc/Io/Async/Audit など 14 項目の状態（未実装/Stage 検証のみ/部分実装）と参照ファイル（例: `compiler/runtime/src/io/adapters.rs#L27-L233`, `compiler/runtime/src/audit/mod.rs`）を記載。
- `docs/plans/bootstrap-roadmap/assets/capability-stage-field-gap.csv`: `CapabilityDescriptor.provider` や `StageRequirement::satisfies` など Stage/Effect 項目の欠落を Diagnostic/Audit キーと紐付けて記録。
- `docs/plans/bootstrap-roadmap/assets/core-runtime-capability-init.md`: RunConfig→Manifest→CapabilityRegistry→StageAuditPayload の初期化順序と `collect-iterator-audit-metrics --section runtime` への接続を記述。
- `docs/plans/bootstrap-roadmap/assets/capability-error-matrix.csv`: `CapabilityError` バリアント別にトリガー条件・診断コード・監査イベント・実装状況を整理。
- 測定: `python3 tooling/ci/collect-iterator-audit-metrics.py --section runtime --dry-run` を実行し、`reports/runtime-capabilities-validation.json` ベースの `runtime.capability_validation` KPI を `docs/plans/bootstrap-roadmap/assets/metrics/runtime-capability-stage.csv` に保存（pass_rate=1.0、候補 3 つ）。
- 依存更新: `docs/plans/rust-migration/2-1-runtime-integration.md` と `docs/plans/bootstrap-roadmap/3-7-core-config-data-plan.md#3.3` へインベントリと初期化メモの参照を追加済み。Phase 3 以降の実装で `StageAuditPayload` が `CapabilityDescriptor` を受け取るよう設計変更を検討する。

## Capability List Update
- 2025-12-05 22:45:42 UTC: CLI `compiler/runtime/target/debug/reml_capability` → JSON `reports/spec-audit/ch3/capability_list-20251205.json`、docs `docs/spec/3-8-core-runtime-capability.md`, `docs/plans/bootstrap-roadmap/README.md` を更新
