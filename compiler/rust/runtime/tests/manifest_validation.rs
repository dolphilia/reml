use insta::assert_yaml_snapshot;
use reml_runtime::capability::contract::CapabilityContractSpan;
use reml_runtime::config::OptimizeLevel;
use reml_runtime::config::{
    manifest::{
        BuildProfile, CapabilityId, DslCategory, DslEntry, DslExportRef, DslExportSignature,
        DslSignatureStageBounds, Manifest, ManifestBuilder, ManifestCapabilities,
        ManifestCapabilityError, PackageName, ProjectSection, RunCapabilityEntry, RunSection,
        RunTargetSection,
    },
    update_dsl_signature, ProjectStage, SemanticVersion,
};
use reml_runtime::prelude::ensure::GuardDiagnostic;
use reml_runtime::stage::{StageId, StageRequirement};
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

const TEST_MANIFEST_PATH: &str = "/virtual/workspace/reml.toml";

fn base_project() -> ProjectSection {
    let mut project = ProjectSection::default();
    project.name = PackageName("demo-app".into());
    project.version = SemanticVersion("0.1.0".into());
    project.stage = ProjectStage::Beta;
    project
}

fn base_dsl_entry() -> DslEntry {
    let mut entry = DslEntry::default();
    entry.entry = PathBuf::from("dsl/demo.reml");
    entry.exports = vec![DslExportRef {
        name: "Demo".into(),
        signature: None,
    }];
    entry.expect_effects = BTreeSet::from(["config.load".to_string()]);
    entry.expect_effects_stage = Some(ProjectStage::Stable);
    entry
}

fn base_manifest() -> Manifest {
    ManifestBuilder::default()
        .project(base_project())
        .dsl_entry("demo", base_dsl_entry())
        .finish()
        .with_manifest_path(TEST_MANIFEST_PATH)
}

fn snapshot_diag(diag: GuardDiagnostic) -> Value {
    diag.into_json()
}

fn expect_manifest_error(name: &str, manifest: Manifest) {
    let diag = reml_runtime::config::validate_manifest(&manifest).expect_err(name);
    assert_yaml_snapshot!(name, snapshot_diag(diag));
}

fn capability_entry(id: &str, stage: &str) -> RunCapabilityEntry {
    RunCapabilityEntry {
        id: CapabilityId(id.into()),
        stage: Some(stage.into()),
        declared_effects: vec![format!("{id}.effect"), id.into()],
        source_span: Some(CapabilityContractSpan::new(10, 24)),
        provider: Some("core.runtime".into()),
    }
}

#[test]
fn manifest_requires_project_name() {
    let mut manifest = base_manifest();
    manifest.project.name = PackageName(String::new());
    expect_manifest_error("manifest_missing_project_name", manifest);
}

#[test]
fn manifest_requires_project_version() {
    let mut manifest = base_manifest();
    manifest.project.version = SemanticVersion(String::new());
    expect_manifest_error("manifest_missing_project_version", manifest);
}

#[test]
fn manifest_rejects_unknown_project_kind() {
    let mut manifest = base_manifest();
    manifest.project.kind = reml_runtime::config::ProjectKind::Unknown("service".into());
    expect_manifest_error("manifest_unknown_project_kind", manifest);
}

#[test]
fn manifest_rejects_unknown_stage() {
    let mut manifest = base_manifest();
    manifest.project.stage = ProjectStage::Unknown("next".into());
    expect_manifest_error("manifest_unknown_project_stage", manifest);
}

#[test]
fn manifest_rejects_unknown_build_optimize() {
    let mut manifest = base_manifest();
    manifest.build.optimize = OptimizeLevel::Unknown("turbo".into());
    expect_manifest_error("manifest_unknown_build_optimize", manifest);
}

#[test]
fn manifest_rejects_unknown_profile_optimize() {
    let mut manifest = base_manifest();
    manifest.build.profiles.insert(
        "custom".into(),
        BuildProfile {
            optimize: Some(OptimizeLevel::Unknown("hyper".into())),
            ..BuildProfile::default()
        },
    );
    expect_manifest_error("manifest_unknown_profile_optimize", manifest);
}

#[test]
fn manifest_requires_dsl_entry_path() {
    let mut manifest = base_manifest();
    if let Some(entry) = manifest.dsl.get_mut("demo") {
        entry.entry = PathBuf::new();
    }
    expect_manifest_error("manifest_missing_dsl_entry", manifest);
}

#[test]
fn manifest_rejects_unknown_dsl_kind() {
    let mut manifest = base_manifest();
    if let Some(entry) = manifest.dsl.get_mut("demo") {
        entry.kind = DslCategory::Unknown("custom_bridge".into());
    }
    expect_manifest_error("manifest_unknown_dsl_kind", manifest);
}

#[test]
fn manifest_rejects_unknown_effect_stage() {
    let mut manifest = base_manifest();
    if let Some(entry) = manifest.dsl.get_mut("demo") {
        entry.expect_effects_stage = Some(ProjectStage::Unknown("gamma".into()));
    }
    expect_manifest_error("manifest_unknown_effect_stage", manifest);
}

#[test]
fn update_dsl_signature_validates_stage_bounds() {
    let manifest = base_manifest();
    let mut signature = DslExportSignature {
        name: "Demo".into(),
        category: Some("runtime_bridge".into()),
        allows_effects: vec!["config.load".into()],
        requires_capabilities: vec![],
        stage_bounds: Some(DslSignatureStageBounds {
            minimum: Some(ProjectStage::Unknown("legacy".into())),
            maximum: None,
            current: None,
        }),
        extra: serde_json::Map::new(),
    };
    signature
        .extra
        .insert("stage".into(), Value::String("beta".into()));
    let err = update_dsl_signature(manifest, "demo", signature).expect_err("invalid bounds");
    assert_yaml_snapshot!("manifest_stage_bounds_invalid", snapshot_diag(err));
}

#[test]
fn conductor_capability_contract_round_trip() {
    let mut manifest = base_manifest();
    manifest.run = RunSection {
        target: RunTargetSection {
            capabilities: vec![capability_entry("console", "at_least:beta")],
        },
    };
    let contract = manifest
        .conductor_capability_contract()
        .expect("contract generated");
    assert_eq!(contract.requirements.len(), 1);
    assert_eq!(
        contract.manifest_path.as_deref(),
        Some(Path::new(TEST_MANIFEST_PATH))
    );
    let requirement = &contract.requirements[0];
    assert_eq!(requirement.id, "console");
    assert_eq!(requirement.stage, StageRequirement::AtLeast(StageId::Beta));
    assert_eq!(
        requirement.declared_effects,
        vec!["console", "console.effect"]
    );
    let span = requirement
        .source_span
        .as_ref()
        .expect("source span should be present");
    assert_eq!(span.start, 10);
    assert_eq!(span.end, 24);
}

#[test]
fn manifest_capabilities_detect_duplicate_ids() {
    let mut manifest = base_manifest();
    manifest.run.target.capabilities = vec![
        capability_entry("console", "at_least:beta"),
        capability_entry("console", "stable"),
    ];
    let err =
        ManifestCapabilities::from_manifest(&manifest).expect_err("duplicate capability expected");
    match err {
        ManifestCapabilityError::DuplicateCapability { capability } => {
            assert_eq!(capability, "console");
        }
        other => panic!("unexpected error: {other:?}"),
    }
}
