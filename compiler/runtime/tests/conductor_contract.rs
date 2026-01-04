use std::path::PathBuf;

use reml_runtime::{
    capability::{registry::reset_for_tests, CapabilityRegistry, ConductorCapabilityContract},
    config::load_manifest,
};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/manifest")
        .join(name)
}

fn contract_from_fixture() -> ConductorCapabilityContract {
    let manifest_path = fixture("capability_contract.toml");
    let manifest = load_manifest(&manifest_path).expect("load manifest");
    manifest
        .conductor_capability_contract()
        .expect("build contract")
}

#[test]
fn verify_conductor_contract_succeeds_for_manifest() {
    reset_for_tests();
    let contract = contract_from_fixture();
    CapabilityRegistry::registry()
        .verify_conductor_contract(contract)
        .expect("contract valid");
}

#[test]
fn verify_conductor_contract_reports_stage_mismatch_with_span() {
    reset_for_tests();
    let mut contract = contract_from_fixture();
    contract.manifest_path = Some(fixture("capability_contract_beta.toml"));
    let err = CapabilityRegistry::registry()
        .verify_conductor_contract(contract)
        .expect_err("stage mismatch expected");
    match err {
        reml_runtime::CapabilityError::ContractViolation {
            manifest_path,
            source_span,
            ..
        } => {
            let manifest_path = manifest_path.expect("manifest path present");
            assert!(
                manifest_path.ends_with("capability_contract_beta.toml"),
                "unexpected manifest path: {manifest_path:?}"
            );
            let span = source_span.expect("source span present");
            assert_eq!(span.start, 10);
            assert_eq!(span.end, 24);
        }
        other => panic!("unexpected error variant: {other:?}"),
    }
}

#[test]
fn verify_conductor_contract_reports_missing_manifest_entry() {
    reset_for_tests();
    let mut contract = contract_from_fixture();
    contract.manifest_path = Some(fixture("capability_contract_empty.toml"));
    let err = CapabilityRegistry::registry()
        .verify_conductor_contract(contract)
        .expect_err("missing manifest entry expected");
    match err {
        reml_runtime::CapabilityError::ContractViolation { .. } => {}
        other => panic!("unexpected error variant: {other:?}"),
    }
}
