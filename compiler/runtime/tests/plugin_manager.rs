use std::fs;
use std::path::{Path, PathBuf};

use tempfile::tempdir;

use reml_runtime::{
    capability::registry::{reset_for_tests, CapabilityError, CapabilityRegistry},
    runtime::{
        bridge::RuntimeBridgeRegistry,
        plugin::{take_plugin_audit_events, PluginLoader, VerificationPolicy},
        plugin_bridge::NativePluginExecutionBridge,
        plugin_manager::PluginRuntimeManager,
    },
    stage::StageRequirement,
};

fn write_plugin_manifest(dir: &Path) -> PathBuf {
    let plugin_dir = dir.join("plugin");
    fs::create_dir_all(&plugin_dir).expect("plugin dir");
    let manifest_path = plugin_dir.join("reml.toml");
    fs::write(
        &manifest_path,
        r#"
[project]
name = "plugin.demo"
version = "0.1.0"
kind = "plugin"

[run.target]
capabilities = [
  { id = "plugin.demo.audit", stage = "beta", declared_effects = ["audit"] }
]
"#,
    )
    .expect("write manifest");
    manifest_path
}

fn write_module(dir: &Path) -> PathBuf {
    let module_path = dir.join("plugin").join("plugin.wasm");
    fs::write(&module_path, b"wasm-demo").expect("write module");
    module_path
}

fn write_bundle(dir: &Path) -> PathBuf {
    let bundle_path = dir.join("bundle.json");
    fs::write(
        &bundle_path,
        r#"
{
  "bundle_id": "bundle.demo",
  "bundle_version": "0.1.0",
  "plugins": [
    { "manifest_path": "plugin/reml.toml", "module_path": "plugin/plugin.wasm" }
  ]
}
"#,
    )
    .expect("write bundle");
    bundle_path
}

fn setup_manager() -> PluginRuntimeManager {
    PluginRuntimeManager::new(
        PluginLoader::new(),
        Box::new(NativePluginExecutionBridge::new()),
    )
}

#[test]
fn plugin_manager_records_signature_and_install_audit() {
    let _guard = reml_runtime::test_support::lock();
    reset_for_tests();
    RuntimeBridgeRegistry::global().clear();
    let _ = take_plugin_audit_events();

    let dir = tempdir().expect("tempdir");
    write_plugin_manifest(dir.path());
    write_module(dir.path());
    let bundle_path = write_bundle(dir.path());
    let manager = setup_manager();
    manager
        .load_bundle_and_attach(&bundle_path, VerificationPolicy::Permissive)
        .expect("bundle load should succeed");

    let events = take_plugin_audit_events();
    let has_verify_signature = events.iter().any(|event| {
        event
            .envelope
            .metadata
            .get("event.kind")
            .and_then(|value| value.as_str())
            == Some("plugin.verify_signature")
    });
    let has_install = events.iter().any(|event| {
        event
            .envelope
            .metadata
            .get("event.kind")
            .and_then(|value| value.as_str())
            == Some("plugin.install")
    });

    assert!(
        has_verify_signature,
        "plugin.verify_signature should be recorded"
    );
    assert!(has_install, "plugin.install should be recorded");
    let verify_signature = events
        .iter()
        .find(|event| {
            event
                .envelope
                .metadata
                .get("event.kind")
                .and_then(|value| value.as_str())
                == Some("plugin.verify_signature")
        })
        .expect("plugin.verify_signature event should be recorded");
    assert!(
        verify_signature
            .envelope
            .metadata
            .get("plugin.modules")
            .is_some(),
        "plugin.modules should be recorded"
    );
    let install = events
        .iter()
        .find(|event| {
            event
                .envelope
                .metadata
                .get("event.kind")
                .and_then(|value| value.as_str())
                == Some("plugin.install")
        })
        .expect("plugin.install event should be recorded");
    assert!(
        install
            .envelope
            .metadata
            .get("plugin.module_hash")
            .is_some(),
        "plugin.module_hash should be recorded"
    );
}

#[test]
fn plugin_manager_stage_record_matches_capability_descriptor() {
    let _guard = reml_runtime::test_support::lock();
    reset_for_tests();
    RuntimeBridgeRegistry::global().clear();

    let dir = tempdir().expect("tempdir");
    write_plugin_manifest(dir.path());
    write_module(dir.path());
    let bundle_path = write_bundle(dir.path());
    let manager = setup_manager();
    manager
        .load_bundle_and_attach(&bundle_path, VerificationPolicy::Permissive)
        .expect("bundle load should succeed");

    let registry = CapabilityRegistry::registry();
    let descriptor = registry
        .describe("plugin.demo.audit")
        .expect("plugin capability should be registered");
    let record = RuntimeBridgeRegistry::global()
        .latest_stage_record("plugin.demo.audit")
        .expect("bridge stage record should exist");
    assert_eq!(
        record.required,
        StageRequirement::Exact(descriptor.stage()),
        "bridge stage requirement should align with capability stage"
    );
    assert_eq!(
        record.actual,
        descriptor.stage(),
        "bridge stage record should match capability stage"
    );
}

#[test]
fn plugin_manager_unload_allows_reload() {
    let _guard = reml_runtime::test_support::lock();
    reset_for_tests();
    RuntimeBridgeRegistry::global().clear();

    let dir = tempdir().expect("tempdir");
    write_plugin_manifest(dir.path());
    write_module(dir.path());
    let bundle_path = write_bundle(dir.path());
    let manager = setup_manager();
    manager
        .load_bundle_and_attach(&bundle_path, VerificationPolicy::Permissive)
        .expect("bundle load should succeed");

    manager
        .unload("plugin.demo")
        .expect("plugin unload should succeed");

    let registry = CapabilityRegistry::registry();
    let err = registry
        .describe("plugin.demo.audit")
        .expect_err("capability should be unregistered");
    match err {
        CapabilityError::NotRegistered { .. } => {}
        other => panic!("unexpected error: {other:?}"),
    }

    manager
        .load_bundle_and_attach(&bundle_path, VerificationPolicy::Permissive)
        .expect("reload should succeed");
}
