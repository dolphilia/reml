# Manifest Capability Contract（Run ID: 20260225-manifest-capability-contract）

- コマンド: `cargo test -p reml_runtime manifest_validation`
- 目的: Manifest (`run.target.capabilities`) の Stage / Effect / manifest_path 情報が `Manifest::conductor_capability_contract()` と `ManifestCapabilities::from_manifest()` に正しく転写され、Capability Registry 側の契約検証に渡せることを確認する。
- 結果:
  - `tests/manifest_validation.rs::conductor_capability_contract_round_trip` で `StageRequirement::AtLeast(StageId::Beta)` / `declared_effects=["console","console.effect"]` / `manifest_path=/virtual/workspace/reml.toml` / `source_span=(10,24)` を保持した要求が生成されることを確認。
  - `tests/manifest_validation.rs::manifest_capabilities_detect_duplicate_ids` で Capability ID 重複時に `ManifestCapabilityError::DuplicateCapability { capability: "console" }` を返すこと、診断導線と同じメタデータを返却できることを検証。
- 参照: `docs/plans/bootstrap-roadmap/3-8-core-runtime-capability-plan.md#5.3`、`docs/plans/bootstrap-roadmap/3-7-core-config-data-plan.md#5.1`。
