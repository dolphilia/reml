# Conductor Contract Verification（Run ID: 20240712-conductor-contract）

- コマンド: `cargo test conductor_contract` (`compiler/rust/runtime`)
- 目的: `reml_runtime::config::manifest` が生成した `ConductorCapabilityContract` を `CapabilityRegistry::verify_conductor_contract` で検証し、Stage/Effect/Manifest 整合性を確認する。
- 結果: `verify_conductor_contract_*` 3 ケース（正常・manifest stage mismatch・manifest entry 欠落）がすべて成功。違反時は `CapabilityError::ContractViolation` に `manifest_path` と `source_span` が含まれることを `tests/conductor_contract.rs` で検証済み。
- 参照: `docs/plans/bootstrap-roadmap/3-8-core-runtime-capability-plan.md#3-3-verify_conductor_contract`、`docs/spec/3-8-core-runtime-capability.md`。
