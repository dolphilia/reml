use reml_runtime::{
    capability::{
        io::{IoCapability, IoCapabilityMetadata},
        registry::{reset_for_tests, CapabilityRegistry},
        CapabilityDescriptor, CapabilityHandle, CapabilityProvider,
    },
    StageId,
};
use static_assertions::assert_impl_all;

#[test]
fn capability_registry_traits() {
    assert_impl_all!(CapabilityRegistry: Send, Sync);
    assert!(std::ptr::eq(
        CapabilityRegistry::registry(),
        CapabilityRegistry::registry()
    ));
}

#[test]
fn register_and_retrieve_capability() {
    reset_for_tests();
    let registry = CapabilityRegistry::registry();
    registry
        .register(sample_io_capability("io.fs.read"))
        .expect("initial registration should succeed");

    let handle = registry
        .get("io.fs.read")
        .expect("handle should be retrievable");
    assert_eq!(handle.descriptor().id, "io.fs.read");

    let descriptor = registry
        .describe("io.fs.read")
        .expect("descriptor should exist");
    assert_eq!(descriptor.stage(), StageId::Beta);

    let all = registry.describe_all();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].id, "io.fs.read");
}

#[test]
fn duplicate_registration_is_rejected() {
    reset_for_tests();
    let registry = CapabilityRegistry::registry();
    registry
        .register(sample_io_capability("io.fs.read"))
        .expect("first registration should succeed");
    let error = registry
        .register(sample_io_capability("io.fs.read"))
        .expect_err("second registration should fail");
    assert_eq!(error.code(), "runtime.capability.already_registered");
    assert!(error.detail().contains("already registered"));
}

#[test]
fn missing_capability_reports_unknown_error() {
    reset_for_tests();
    let registry = CapabilityRegistry::registry();
    let err = registry
        .get("io.fs.write")
        .expect_err("get should fail for missing capability");
    assert_eq!(err.code(), "runtime.capability.unknown");
    let describe_err = registry
        .describe("io.fs.write")
        .expect_err("describe should fail for missing capability");
    assert_eq!(describe_err.code(), "runtime.capability.unknown");
}

fn sample_io_capability(id: &str) -> CapabilityHandle {
    IoCapability::new(
        CapabilityDescriptor::new(id, StageId::Beta, ["effect.io"], CapabilityProvider::Core),
        IoCapabilityMetadata::default(),
    )
    .into()
}
