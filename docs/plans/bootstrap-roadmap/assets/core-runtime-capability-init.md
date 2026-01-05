# Core Runtime Capability 初期化手順メモ（Run ID: 20291221-core-runtime-capability）

`docs/plans/bootstrap-roadmap/3-8-core-runtime-capability-plan.md#1.3` の整理結果。RunConfig → Manifest → CapabilityRegistry → StageAuditPayload の依存を明文化し、Config/Data（[3-7-plan](../3-7-core-config-data-plan.md) §3.3）と共有する。

## 初期化シーケンス
1. **RunConfig 準備**  
   - `reml_frontend`/CLI で受け取った `--effect-stage`・`--runtime-capabilities-json` を `RunConfig.extensions["effect_stage"]` に書き込む。  
   - Manifest を指定した場合は `compiler/runtime/src/run_config.rs::apply_manifest_overrides` で `run.target.capabilities[]` を `ConductorCapabilityRequirement` に変換する。

2. **ConfigManifest → Capability 契約**  
   - `compiler/runtime/src/config/manifest.rs` で `run.target.capabilities` を解析して `ConductorCapabilityRequirement { id, requirement, declared_effects, source_span }` を構築する。  
   - `docs/plans/bootstrap-roadmap/assets/capability-stage-field-gap.csv` で欠落している `source_span` は TODO。生成された契約は `CapabilityRegistry` 呼び出し前に 1 つの構造体にまとめておく。

3. **CapabilityRegistry 呼び出し順序**  
   - Registry はまだ `register()` を持たないため、暫定的に `CapabilityRegistry::registry().verify_capability_stage(id, requirement, effects)` のみを呼び出している。  
   - 最低限 `FsAdapter`/`WatcherAdapter`/`MetricsStageGuard` の順で Stage を検証し、検証結果を `StageAuditPayload` に渡す。  
   - 今後 `register` → `describe_all` の経路を実装する際は本メモの順序に画像（`capability-stage-flow.mmd`）を添えて更新する。

4. **StageAuditPayload 生成**  
   - `compiler/frontend/src/diagnostic/effects.rs` の `StageAuditPayload::record(capability_id, requirement, result)` を唯一の窓口にし、`Diagnostic.extensions["effect.stage.*"]` と `AuditEnvelope.metadata["effect.stage.*"]` の両方へ転写する。  
   - ここで `CapabilityDescriptor` の `provider`/`manifest_path`/`effect_scope` を受け取れるようにするのが差分解消タスク。

5. **監査ログと KPI**  
   - `RuntimeBridgeRegistry`・`FsAdapter` など Stage を観測する箇所では `record_bridge_stage_probe` を呼び `AuditEnvelope.metadata["bridge.stage.*"]` を埋める。  
   - `python3 tooling/ci/collect-iterator-audit-metrics.py --section runtime --dry-run` の出力を `docs/plans/bootstrap-roadmap/assets/metrics/runtime-capability-stage.csv` に追記し、`runtime.capability_validation` KPI を `docs/guides/tooling/audit-metrics.md` へ記録する。

参考図: `docs/plans/bootstrap-roadmap/assets/capability-stage-flow.mmd`
