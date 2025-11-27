use reml_runtime::registry::CapabilityRegistry;
use reml_runtime::stage::{StageId, StageRequirement};

#[test]
fn capability_text_audit_is_reported_stable() {
    let registry = CapabilityRegistry::registry();
    let result = registry.verify_capability_stage(
        "core.text.audit",
        StageRequirement::Exact(StageId::Stable),
        &["audit".to_string()],
    );
    assert!(
        result.is_ok(),
        "core.text.audit capability should be accepted at Stable stage"
    );
}
