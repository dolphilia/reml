use std::fs;
use std::path::PathBuf;

use reml_runtime::config::{
    ensure_schema_version_compatibility, load_manifest, update_dsl_signature, validate_manifest,
    CapabilityId, DslEntry, DslExportRef, DslExportSignature, DslSignatureStageBounds, Manifest,
    PackageName, ProjectSection, ProjectStage, SemanticVersion,
};
use reml_runtime::data::{Schema, SchemaVersion};
use serde_json::{Map, Value};
use tempfile::tempdir;

fn sample_project(stage: ProjectStage) -> ProjectSection {
    let mut project = ProjectSection::default();
    project.name = PackageName("demo-app".into());
    project.version = SemanticVersion("0.1.0".into());
    project.stage = stage;
    project
}

fn manifest_with_project(stage: ProjectStage) -> Manifest {
    Manifest::builder().project(sample_project(stage)).finish()
}

#[test]
fn load_manifest_fails_when_entry_missing() {
    let dir = tempdir().expect("tempdir");
    let manifest_path = dir.path().join("reml.toml");
    fs::write(
        &manifest_path,
        r#"
[project]
name = "demo"
version = "0.1.0"

[dsl.demo]
entry = "dsl/demo.reml"
exports = ["Demo"]
"#,
    )
    .expect("write manifest");

    let err = load_manifest(&manifest_path).expect_err("missing entry");
    assert_eq!(err.code, "manifest.entry.missing");
}

#[test]
fn load_manifest_succeeds_when_entry_exists() {
    let dir = tempdir().expect("tempdir");
    let manifest_path = dir.path().join("reml.toml");
    let dsl_dir = dir.path().join("dsl");
    fs::create_dir_all(&dsl_dir).expect("dsl dir");
    fs::write(dsl_dir.join("demo.reml"), "dsl Demo").expect("dsl file");
    fs::write(
        &manifest_path,
        r#"
[project]
name = "demo"
version = "0.1.0"

[dsl.demo]
entry = "dsl/demo.reml"
exports = ["Demo"]
"#,
    )
    .expect("write manifest");

    let manifest = load_manifest(&manifest_path).expect("load manifest");
    assert_eq!(
        manifest
            .manifest_path()
            .map(|path| path.ends_with("reml.toml")),
        Some(true)
    );
}

#[test]
fn validate_manifest_reports_invalid_stage() {
    let manifest = manifest_with_project(ProjectStage::Unknown("future".into()));
    let err = validate_manifest(&manifest).expect_err("stage check");
    assert_eq!(err.code, "config.invalid_stage");
}

#[test]
fn update_dsl_signature_applies_capabilities() {
    let mut manifest = manifest_with_project(ProjectStage::Stable);
    let mut entry = DslEntry::default();
    entry.entry = PathBuf::from("dsl/demo.reml");
    entry.exports = vec![DslExportRef {
        name: "Demo".into(),
        signature: None,
    }];
    manifest.dsl.insert("demo".into(), entry);
    manifest = manifest.with_manifest_path("/tmp/reml/reml.toml");

    let signature = DslExportSignature {
        name: "Demo".into(),
        category: Some("language".into()),
        allows_effects: vec!["io.fs".into()],
        requires_capabilities: vec![
            CapabilityId("core.demo".into()),
            CapabilityId("core.demo".into()),
        ],
        stage_bounds: None,
        extra: Map::<String, Value>::new(),
    };

    let updated = update_dsl_signature(manifest, "demo", signature).expect("update");
    let entry = updated.dsl.get("demo").expect("dsl entry");
    assert_eq!(entry.capabilities.len(), 1);
    assert_eq!(entry.capabilities[0].0, "core.demo");
    let export = entry
        .exports
        .iter()
        .find(|export| export.name == "Demo")
        .expect("export");
    assert!(export.signature.is_some());
}

#[test]
fn update_dsl_signature_records_stage_bounds() {
    let mut manifest = manifest_with_project(ProjectStage::Stable);
    let mut entry = DslEntry::default();
    entry.entry = PathBuf::from("dsl/core_config.reml");
    entry.exports = vec![DslExportRef {
        name: "CoreConfigExport".into(),
        signature: None,
    }];
    manifest.dsl.insert("core_config".into(), entry);

    let stage_bounds = DslSignatureStageBounds {
        minimum: Some(ProjectStage::Beta),
        maximum: Some(ProjectStage::Stable),
        current: Some(ProjectStage::Beta),
    };
    let signature = DslExportSignature {
        name: "CoreConfigExport".into(),
        category: Some("runtime_bridge".into()),
        allows_effects: vec!["audit.emit".into()],
        requires_capabilities: vec![CapabilityId("core.audit".into())],
        stage_bounds: Some(stage_bounds),
        extra: Map::<String, Value>::new(),
    };

    let updated =
        update_dsl_signature(manifest, "core_config", signature).expect("signature update");
    let entry = updated
        .dsl
        .get("core_config")
        .expect("core_config entry present");
    assert_eq!(
        entry.expect_effects_stage,
        Some(ProjectStage::Beta),
        "stage projection should follow signature bounds"
    );
    let export = entry
        .exports
        .iter()
        .find(|export| export.name == "CoreConfigExport")
        .expect("core_config export present");
    let signature_json = export.signature.as_ref().expect("signature stored");
    let stage_bounds = signature_json
        .get("stage_bounds")
        .and_then(|value| value.as_object())
        .expect("stage bounds serialized");
    assert_eq!(
        stage_bounds.get("current").and_then(|value| value.as_str()),
        Some("beta")
    );
    assert_eq!(
        stage_bounds.get("maximum").and_then(|value| value.as_str()),
        Some("stable")
    );
}

fn schema_with_version(name: &str, version: (u32, u32, u32)) -> Schema {
    Schema::builder(name)
        .version(SchemaVersion::new(version.0, version.1, version.2))
        .finish()
}

#[test]
fn schema_version_check_passes_when_manifest_is_newer() {
    let mut manifest = manifest_with_project(ProjectStage::Stable);
    manifest.project.version = SemanticVersion("1.2.3".into());
    let schema = schema_with_version("core.config", (1, 1, 0));
    ensure_schema_version_compatibility(&manifest, &schema).expect("compatible versions");
}

#[test]
fn schema_version_check_fails_on_major_mismatch() {
    let mut manifest = manifest_with_project(ProjectStage::Stable);
    manifest.project.version = SemanticVersion("1.0.0".into());
    let schema = schema_with_version("core.config", (2, 0, 0));
    let err =
        ensure_schema_version_compatibility(&manifest, &schema).expect_err("should fail on major");
    assert_eq!(err.code, "config.schema.version_incompatible");
}

#[test]
fn schema_version_check_fails_when_schema_is_newer() {
    let mut manifest = manifest_with_project(ProjectStage::Stable);
    manifest.project.version = SemanticVersion("1.1.0".into());
    let schema = schema_with_version("core.config", (1, 2, 0));
    let err =
        ensure_schema_version_compatibility(&manifest, &schema).expect_err("schema minor is ahead");
    assert_eq!(err.code, "config.schema.version_incompatible");
}

#[test]
fn schema_version_check_skips_when_schema_version_absent() {
    let mut manifest = manifest_with_project(ProjectStage::Stable);
    manifest.project.version = SemanticVersion("1.0.0".into());
    let schema = Schema::builder("core.config").finish();
    ensure_schema_version_compatibility(&manifest, &schema).expect("no schema version");
}

#[test]
fn schema_version_check_errors_on_invalid_manifest_version() {
    let mut manifest = manifest_with_project(ProjectStage::Stable);
    manifest.project.version = SemanticVersion("next".into());
    let schema = schema_with_version("core.config", (1, 0, 0));
    let err = ensure_schema_version_compatibility(&manifest, &schema)
        .expect_err("manifest version parse error");
    assert_eq!(err.code, "config.project.version_invalid");
}
