use reml_runtime::{
    stage::{StageId, StageRequirement},
    CapabilityRegistry,
};

#[test]
fn metrics_capability_accepts_stable_requirement() {
    let registry = CapabilityRegistry::registry();
    let effects = vec!["audit".to_string()];
    let stage = registry
        .verify_capability_stage(
            "metrics.emit",
            StageRequirement::Exact(StageId::Stable),
            &effects,
        )
        .expect("metrics.emit should be available at stable");
    assert_eq!(stage, StageId::Stable);
}

#[test]
fn metrics_capability_reports_actual_stage_on_violation() {
    let registry = CapabilityRegistry::registry();
    let effects = vec!["audit".to_string()];
    let error = registry
        .verify_capability_stage(
            "metrics.emit",
            StageRequirement::Exact(StageId::Beta),
            &effects,
        )
        .expect_err("beta requirement should fail for metrics.emit");
    assert_eq!(error.code(), "capability.stage.mismatch");
    assert_eq!(error.actual_stage(), Some(StageId::Stable));
    assert!(
        error.detail().contains("runtime is"),
        "detail should explain mismatch"
    );
}
