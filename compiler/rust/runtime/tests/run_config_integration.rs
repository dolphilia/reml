use reml_runtime::config::{manifest::Manifest, ConfigFormat};
use reml_runtime::run_config::{
    apply_manifest_overrides, ApplyManifestOverridesArgs, RunConfigManifestOverrides,
};
use reml_runtime::stage::StageId;

fn sample_manifest() -> Manifest {
    let raw = r#"
[project]
name = "demo"
version = "0.1.0"

[config.compatibility.toml]
profile = "toml-relaxed"
trailing_comma = "arrays"
"#;
    Manifest::parse_toml(raw)
        .expect("manifest")
        .with_manifest_path("/tmp/reml/reml.toml")
}

#[test]
fn manifest_overrides_include_path_and_stage() {
    let manifest = sample_manifest();
    let overrides = run_overrides(&manifest, ConfigFormat::Toml, StageId::Stable);
    let manifest_extension = overrides.manifest_extension;
    assert_eq!(
        manifest_extension.get("source").and_then(|v| v.as_str()),
        Some("manifest")
    );
    assert_eq!(
        manifest_extension
            .get("runtime_stage")
            .and_then(|v| v.as_str()),
        Some("stable")
    );
    let project = manifest_extension
        .get("project")
        .and_then(|value| value.as_object())
        .expect("project payload");
    assert!(project.contains_key("name"));
    assert!(project.contains_key("version"));
    assert!(
        manifest_extension.contains_key("compatibility"),
        "compatibility field missing: {manifest_extension:?}"
    );
}

fn run_overrides(
    manifest: &reml_runtime::config::Manifest,
    format: ConfigFormat,
    stage: StageId,
) -> RunConfigManifestOverrides {
    apply_manifest_overrides(ApplyManifestOverridesArgs {
        manifest,
        format,
        stage,
    })
}
