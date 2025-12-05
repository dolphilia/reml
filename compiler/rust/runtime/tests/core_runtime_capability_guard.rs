use once_cell::sync::OnceCell;

use reml_runtime::{
    capability::{
        async_runtime::{AsyncCapability, AsyncCapabilityMetadata},
        descriptor::{CapabilityDescriptor, CapabilityProvider},
        handle::CapabilityHandle,
        registry::{CapabilityError, CapabilityRegistry},
    },
    runtime::api::{guard_async_capability, guard_io_capability, guard_time_capability},
    stage::{StageId, StageRequirement},
};

#[test]
fn io_capability_guard_reports_stable_stage() {
    let guard = guard_io_capability(
        "io.fs.read",
        StageRequirement::AtLeast(StageId::Beta),
        &["io", "fs.read"],
    )
    .expect("io.fs.read capability should be registered");
    assert_eq!(
        guard.actual_stage(),
        StageId::Stable,
        "io.fs.read is registered as Stable"
    );
    assert!(guard.satisfies(), "Stage guard must satisfy requirement");
}

#[test]
fn time_capability_guard_uses_time_effect_scope() {
    let guard = guard_time_capability(
        "core.time.timezone.lookup",
        StageRequirement::AtLeast(StageId::Beta),
        &["time"],
    )
    .expect("core.time.timezone.lookup capability should exist");
    assert_eq!(
        guard.actual_stage(),
        StageId::Beta,
        "timezone lookup is a Beta capability"
    );
}

#[test]
fn async_capability_guard_supports_registered_runtime() {
    ensure_async_capability_registered();
    let guard = guard_async_capability(
        "core.async.scheduler",
        StageRequirement::AtLeast(StageId::Beta),
        &["runtime", "async"],
    )
    .expect("test async capability should be registered");
    assert!(
        guard.satisfies(),
        "registered async capability should meet requirement"
    );
}

fn ensure_async_capability_registered() {
    static REGISTER: OnceCell<()> = OnceCell::new();
    REGISTER.get_or_init(|| {
        let registry = CapabilityRegistry::registry();
        let descriptor = CapabilityDescriptor::new(
            "core.async.scheduler",
            StageId::Stable,
            ["runtime", "async"].into_iter(),
            CapabilityProvider::RuntimeComponent {
                name: "core-runtime-capability-guard-test".into(),
            },
        );
        let handle = CapabilityHandle::from(AsyncCapability::new(
            descriptor,
            AsyncCapabilityMetadata::default(),
        ));
        if let Err(err) = registry.register(handle) {
            match err {
                CapabilityError::AlreadyRegistered { .. } => {}
                other => panic!("failed to register async capability for tests: {other}"),
            }
        }
    });
}
