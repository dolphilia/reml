#![cfg(feature = "schema")]

use reml_runtime::config::{
    apply_manifest_overrides, compat::TrailingCommaMode, ensure_schema_version_compatibility,
    load_manifest, validate_manifest, ApplyManifestOverridesArgs, ConfigFormat,
};
use reml_runtime::data::schema::Schema;
use reml_runtime::run_config::RunConfigManifestOverrides;
use reml_runtime::stage::StageId;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/config_integration")
}

fn read_schema(fixture: &PathBuf) -> Schema {
    let raw = fs::read_to_string(fixture).expect("schema fixture");
    serde_json::from_str(&raw).expect("schema json")
}

fn apply_overrides(manifest_path: &PathBuf) -> RunConfigManifestOverrides {
    let manifest = load_manifest(manifest_path).expect("manifest should load");
    validate_manifest(&manifest).expect("manifest validations pass");
    let schema = read_schema(&fixtures_dir().join("schema.json"));
    ensure_schema_version_compatibility(&manifest, &schema).expect("schema compatible");
    apply_manifest_overrides(ApplyManifestOverridesArgs {
        manifest: &manifest,
        format: ConfigFormat::Json,
        stage: StageId::Beta,
    })
}

#[test]
fn manifest_schema_runconfig_integration() {
    let manifest_path = fixtures_dir().join("reml.toml");
    let overrides = apply_overrides(&manifest_path);
    let manifest_extension = Value::Object(overrides.manifest_extension.clone());
    assert_eq!(
        manifest_extension
            .pointer("/source")
            .and_then(Value::as_str),
        Some("manifest")
    );
    assert_eq!(
        manifest_extension
            .pointer("/project/name")
            .and_then(Value::as_str),
        Some("config-integration")
    );
    assert_eq!(
        manifest_extension
            .pointer("/project/stage")
            .and_then(Value::as_str),
        Some("beta")
    );
    assert_eq!(
        manifest_extension
            .pointer("/compatibility_profile")
            .and_then(Value::as_str),
        Some("json-relaxed")
    );
    let compatibility_layer = overrides.compatibility_layer.expect("compatibility layer");
    assert_eq!(
        compatibility_layer.profile_label.as_deref(),
        Some("json-relaxed")
    );
    assert_eq!(
        compatibility_layer.compatibility.trailing_comma,
        TrailingCommaMode::ArraysAndObjects
    );
}
