use std::fs;
use std::path::PathBuf;

use reml_runtime::config::{
    load_manifest, update_dsl_signature, validate_manifest, CapabilityId, DslEntry, DslExportRef,
    DslExportSignature, Manifest, PackageName, ProjectSection, ProjectStage, SemanticVersion,
};
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
