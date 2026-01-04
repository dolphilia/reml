use reml_runtime::{
    capability::{
        io::{IoCapability, IoCapabilityMetadata},
        registry, CapabilityDescriptor, CapabilityHandle, CapabilityProvider,
    },
    stage::{StageId, StageRequirement},
    CapabilityRegistry,
};

#[test]
fn capability_text_audit_is_reported_stable() {
    registry::reset_for_tests();
    let registry = CapabilityRegistry::registry();
    registry
        .register(sample_text_capability())
        .expect("should register core.text.audit capability");
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

fn sample_text_capability() -> CapabilityHandle {
    IoCapability::new(
        CapabilityDescriptor::new(
            "core.text.audit",
            StageId::Stable,
            ["audit"],
            CapabilityProvider::Core,
        ),
        IoCapabilityMetadata::default(),
    )
    .into()
}
