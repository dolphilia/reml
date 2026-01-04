use reml_runtime::{
    capability::{
        io::{IoCapability, IoCapabilityMetadata},
        registry::{reset_for_tests, CapabilityError, CapabilityRegistry},
        CapabilityDescriptor, CapabilityHandle, CapabilityProvider, PluginCapabilityMetadata,
    },
    StageId, StageRequirement,
};

fn sample_handle(id: &str, stage: StageId, effects: &[&str]) -> CapabilityHandle {
    IoCapability::new(
        CapabilityDescriptor::new(id, stage, effects.iter().copied(), CapabilityProvider::Core),
        IoCapabilityMetadata::default(),
    )
    .into()
}

#[test]
fn verify_capability_succeeds_with_matching_stage_and_effects() {
    reset_for_tests();
    let registry = CapabilityRegistry::registry();
    registry
        .register(sample_handle(
            "sample.io.audit",
            StageId::Stable,
            &["audit"],
        ))
        .expect("registration should succeed");
    let required_effects = vec!["audit".to_string()];
    let handle = registry
        .verify_capability(
            "sample.io.audit",
            StageRequirement::AtLeast(StageId::Beta),
            &required_effects,
        )
        .expect("capability should satisfy requirement");
    assert_eq!(handle.descriptor().stage(), StageId::Stable);
    let events = registry.capability_checks();
    assert!(!events.is_empty());
    let metadata = &events.last().unwrap().envelope.metadata;
    assert_eq!(
        metadata
            .get("capability.result")
            .and_then(|value| value.as_str()),
        Some("success")
    );
}

#[test]
fn verify_capability_reports_stage_violation() {
    reset_for_tests();
    let registry = CapabilityRegistry::registry();
    registry
        .register(sample_handle(
            "sample.io.beta_only",
            StageId::Beta,
            &["audit"],
        ))
        .expect("registration should succeed");
    let err = registry
        .verify_capability(
            "sample.io.beta_only",
            StageRequirement::Exact(StageId::Stable),
            &["audit".to_string()],
        )
        .expect_err("stage mismatch must surface an error");
    assert_eq!(err.code(), "capability.stage.mismatch");
    assert_eq!(err.actual_stage(), Some(StageId::Beta));
}

#[test]
fn verify_capability_reports_effect_scope_mismatch() {
    reset_for_tests();
    let registry = CapabilityRegistry::registry();
    registry
        .register(sample_handle(
            "sample.io.partial",
            StageId::Stable,
            &["audit"],
        ))
        .expect("registration should succeed");
    let err = registry
        .verify_capability(
            "sample.io.partial",
            StageRequirement::AtLeast(StageId::Beta),
            &["audit".to_string(), "mem".to_string()],
        )
        .expect_err("missing effects should be reported");
    assert_eq!(err.code(), "capability.effect_scope.mismatch");
    let missing = err.missing_effects().unwrap();
    assert_eq!(missing, &["mem"]);
}

#[test]
fn verify_capability_reports_not_registered_error() {
    reset_for_tests();
    let registry = CapabilityRegistry::registry();
    let err = registry
        .verify_capability(
            "missing.capability",
            StageRequirement::AtLeast(StageId::Experimental),
            &[],
        )
        .expect_err("unknown capability should produce an error");
    assert_eq!(err.code(), "runtime.capability.unknown");
    assert!(matches!(err, CapabilityError::NotRegistered { .. }));
}

#[test]
fn verify_capability_records_plugin_provider_metadata() {
    reset_for_tests();
    let registry = CapabilityRegistry::registry();
    registry
        .register_plugin_capability(
            "plugin.demo.audit",
            StageId::Beta,
            &["audit"],
            PluginCapabilityMetadata::new("plugin.demo", Some("1.2.3"), vec!["audit".to_string()]),
        )
        .expect("plugin capability registration should succeed");
    registry
        .verify_capability(
            "plugin.demo.audit",
            StageRequirement::Exact(StageId::Beta),
            &["audit".to_string()],
        )
        .expect("plugin capability should satisfy requirement");
    let events = registry.capability_checks();
    let metadata = events
        .iter()
        .rev()
        .find(|event| {
            event
                .envelope
                .metadata
                .get("capability.id")
                .and_then(|value| value.as_str())
                == Some("plugin.demo.audit")
        })
        .expect("plugin capability audit event should exist")
        .envelope
        .metadata
        .clone();
    assert_eq!(
        metadata
            .get("capability.provider.kind")
            .and_then(|value| value.as_str()),
        Some("plugin")
    );
    assert_eq!(
        metadata
            .get("capability.provider")
            .and_then(|value| value.as_str()),
        Some("plugin:plugin.demo@1.2.3")
    );
    assert_eq!(
        metadata
            .get("plugin.package")
            .and_then(|value| value.as_str()),
        Some("plugin.demo")
    );
    assert_eq!(
        metadata
            .get("plugin.version")
            .and_then(|value| value.as_str()),
        Some("1.2.3")
    );
}
