use insta::assert_yaml_snapshot;

use reml_runtime::config::{
    compatibility_profile, compatibility_profile_for_stage, resolve_compat, ConfigCompatibility,
    ConfigCompatibilitySource, ConfigFormat, KeyPolicy, ResolveCompatOptions, TrailingCommaMode,
};
use reml_runtime::config::{CompatibilityLayer, Manifest};
use reml_runtime::stage::StageId;

#[test]
fn config_compat_strict_json_snapshot() {
    let compat = ConfigCompatibility::strict_json();
    assert_yaml_snapshot!("config_compat_strict_json", compat);
}

#[test]
fn config_compat_toml_relaxed_snapshot() {
    let compat = ConfigCompatibility::relaxed_toml();
    assert_yaml_snapshot!("config_compat_toml_relaxed", compat);
}

#[test]
fn compatibility_profile_helper_handles_aliases() {
    let compat = compatibility_profile("toml-relaxed").expect("profile");
    assert_eq!(compat.unquoted_key, KeyPolicy::AllowAlphaNumeric);
    assert_eq!(compat.trailing_comma, TrailingCommaMode::ArraysAndObjects);
    let err = compatibility_profile("unknown-profile").expect_err("should reject unknown profile");
    assert_eq!(err.requested(), "unknown-profile");
}

#[test]
fn resolve_compat_prefers_cli_over_other_layers() {
    let cli_layer = CompatibilityLayer::new(
        ConfigCompatibility::relaxed_json(),
        Some("cli-relaxed".to_string()),
    );
    let env_layer = CompatibilityLayer::new(
        ConfigCompatibility::strict_json(),
        Some("env-strict".to_string()),
    );
    let manifest_layer = CompatibilityLayer::new(
        ConfigCompatibility::relaxed_json(),
        Some("manifest-relaxed".to_string()),
    );
    let resolved = resolve_compat(ResolveCompatOptions {
        format: ConfigFormat::Json,
        stage: StageId::Stable,
        cli: Some(cli_layer),
        env: Some(env_layer),
        manifest: Some(manifest_layer),
    });
    assert_eq!(resolved.source, ConfigCompatibilitySource::Cli);
    assert_eq!(resolved.profile_label.as_deref(), Some("cli-relaxed"));
    assert_eq!(
        resolved.compatibility.trailing_comma,
        TrailingCommaMode::ArraysAndObjects
    );
}

#[test]
fn resolve_compat_falls_back_to_stage_profile() {
    let resolved = resolve_compat(ResolveCompatOptions {
        format: ConfigFormat::Toml,
        stage: StageId::Alpha,
        cli: None,
        env: None,
        manifest: None,
    });
    assert_eq!(resolved.source, ConfigCompatibilitySource::Default);
    assert_eq!(
        resolved.compatibility.trailing_comma,
        compatibility_profile_for_stage(ConfigFormat::Toml, StageId::Alpha).trailing_comma
    );
}

#[test]
fn manifest_config_compat_is_applied() {
    let raw = r#"
[project]
name = "demo"
version = "0.1.0"

[config.compatibility.json]
profile = "json-relaxed"
trailing_comma = "arrays"
feature_guard = ["json5"]
"#;
    let manifest = Manifest::parse_toml(raw).expect("manifest");
    let layer = manifest
        .compatibility_layer(ConfigFormat::Json, StageId::Stable)
        .expect("layer");
    assert_eq!(layer.profile_label.as_deref(), Some("json-relaxed"));
    assert_eq!(
        layer.compatibility.trailing_comma,
        TrailingCommaMode::Arrays
    );
    assert!(layer
        .compatibility
        .feature_guard
        .iter()
        .any(|value| value == "json5"));
}
