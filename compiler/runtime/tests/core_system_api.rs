use std::convert::TryFrom;

use reml_runtime::{
    capability::registry::{reset_for_tests, CapabilityRegistry},
    env as core_env,
    path::PathBuf,
    prelude::ensure::IntoDiagnostic,
    runtime::{SignalErrorKind, SignalInfo},
    system::{env as system_env, process, signal},
};

#[test]
fn process_returns_unsupported_when_capability_missing() {
    reset_for_tests();
    let registry = CapabilityRegistry::registry();
    registry
        .unregister("core.process")
        .expect("core.process should be registered by default");

    let program = PathBuf::try_from("noop").expect("path should be valid");
    let command = process::Command::new(program);
    let err = process::spawn(command, process::SpawnOptions::default())
        .expect_err("missing capability should return an error");
    assert_eq!(err.kind, process::ProcessErrorKind::Unsupported);
    let diagnostic = err.clone().into_diagnostic();
    assert_eq!(diagnostic.code, "system.capability.missing");
}

#[test]
fn signal_returns_unsupported_when_capability_missing() {
    reset_for_tests();
    let registry = CapabilityRegistry::registry();
    registry
        .unregister("core.signal")
        .expect("core.signal should be registered by default");

    let err = signal::raise(9).expect_err("missing capability should return an error");
    assert_eq!(err.kind, SignalErrorKind::Unsupported);
    let diagnostic = err.clone().into_diagnostic();
    assert_eq!(diagnostic.code, "system.capability.missing");
}

#[test]
fn signal_detail_from_runtime_info_sets_defaults() {
    let info = SignalInfo::new(15, 42);
    let detail = signal::from_runtime_info(info);
    assert_eq!(detail.info, info);
    assert_eq!(detail.source_pid, Some(42));
    assert!(detail.timestamp.is_none());
    assert!(detail.payload.is_none());
    assert!(detail.raw_code.is_none());
}

#[test]
fn core_env_alias_returns_same_values() {
    let key = "REML_TEST_CORE_ENV_ALIAS";
    let value = "enabled";
    let _ = system_env::remove_env(key);

    system_env::set_env(key, value).expect("env set should succeed");
    let core_value = core_env::get_env(key).expect("core env read should succeed");
    let system_value = system_env::get_env(key).expect("system env read should succeed");
    assert_eq!(core_value, Some(value.to_string()));
    assert_eq!(system_value, Some(value.to_string()));

    system_env::remove_env(key).expect("env cleanup should succeed");
}
