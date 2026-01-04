use once_cell::sync::Lazy;
use reml_runtime::{
    capability::{
        io::{IoCapability, IoCapabilityMetadata},
        registry::{reset_for_tests, CapabilityRegistry},
        CapabilityDescriptor, CapabilityHandle, CapabilityProvider,
    },
    StageId,
};
use std::sync::Mutex;

static CAPABILITY_TEST_GUARD: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));
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
    let _guard = CAPABILITY_TEST_GUARD.lock().unwrap();
    reset_for_tests();
    let registry = CapabilityRegistry::registry();
    let base_len = registry.describe_all().len();
    registry
        .register(sample_io_capability("test.io.fs.read"))
        .expect("initial registration should succeed");

    let handle = registry
        .get("test.io.fs.read")
        .expect("handle should be retrievable");
    assert_eq!(handle.descriptor().id, "test.io.fs.read");

    let descriptor = registry
        .describe("test.io.fs.read")
        .expect("descriptor should exist");
    assert_eq!(descriptor.stage(), StageId::Beta);

    let all = registry.describe_all();
    assert_eq!(all.len(), base_len + 1);
    assert_eq!(all.last().unwrap().id, "test.io.fs.read");
}

#[test]
fn duplicate_registration_is_rejected() {
    let _guard = CAPABILITY_TEST_GUARD.lock().unwrap();
    reset_for_tests();
    let registry = CapabilityRegistry::registry();
    registry
        .register(sample_io_capability("test.io.dup"))
        .expect("first registration should succeed");
    let error = registry
        .register(sample_io_capability("test.io.dup"))
        .expect_err("second registration should fail");
    assert_eq!(error.code(), "runtime.capability.already_registered");
    assert!(error.detail().contains("already registered"));
}

#[test]
fn missing_capability_reports_unknown_error() {
    let _guard = CAPABILITY_TEST_GUARD.lock().unwrap();
    reset_for_tests();
    let registry = CapabilityRegistry::registry();
    let err = registry
        .get("missing.capability")
        .expect_err("get should fail for missing capability");
    assert_eq!(err.code(), "runtime.capability.unknown");
    let describe_err = registry
        .describe("missing.capability")
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
