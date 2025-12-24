use std::fs;
use std::path::{Path, PathBuf};

use tempfile::tempdir;

use reml_runtime::{
    capability::registry::reset_for_tests,
    capability::CapabilityRegistry,
    runtime::{
        bridge::RuntimeBridgeRegistry,
        plugin::PluginLoader,
        plugin_bridge::{
            PluginExecutionBridge, PluginInvokeRequest, PluginLoadRequest, PluginWasmBridge,
        },
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

fn write_wasm_module(dir: &Path) -> PathBuf {
    let module_path = dir.join("plugin").join("plugin.wasm");
    let wasm = wat::parse_str(
        r#"(module
  (memory (export "memory") 1)
  (func (export "plugin.echo") (param i32 i32) (result i32)
    local.get 1)
)"#,
    )
    .expect("valid wat");
    fs::write(&module_path, wasm).expect("write wasm");
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

fn compute_hash(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut out = String::with_capacity(digest.len() * 2);
    for value in digest.as_slice() {
        out.push(hex_nibble(value >> 4));
        out.push(hex_nibble(value & 0x0f));
    }
    format!("sha256:{out}")
}

fn hex_nibble(value: u8) -> char {
    match value {
        0..=9 => (b'0' + value) as char,
        10..=15 => (b'a' + (value - 10)) as char,
        _ => '0',
    }
}

#[test]
fn wasm_bridge_loads_bundle_and_invokes() {
    let _guard = reml_runtime::test_support::lock();
    reset_for_tests();
    RuntimeBridgeRegistry::global().clear();

    let dir = tempdir().expect("tempdir");
    write_plugin_manifest(dir.path());
    let module_path = write_wasm_module(dir.path());
    let bundle_path = write_bundle(dir.path());

    let loader = PluginLoader::new();
    let bundle = loader
        .load_bundle_manifest(&bundle_path)
        .expect("bundle should load");
    let manifest = bundle.plugins.get(0).expect("plugin manifest should exist");
    loader
        .register_manifest(manifest)
        .expect("capabilities should register");
    let module = bundle
        .module_info_for(&manifest.project.name.0)
        .expect("module info should exist");

    let bridge = PluginWasmBridge::new();
    let instance = bridge
        .load(PluginLoadRequest {
            manifest,
            bundle_hash: bundle.bundle_hash.as_deref(),
            module_path: Some(module.module_path.as_path()),
        })
        .expect("wasm bridge load should succeed");

    let response = bridge
        .invoke(
            &instance,
            PluginInvokeRequest {
                entrypoint: "plugin.echo".to_string(),
                payload: vec![1, 2, 3],
            },
        )
        .expect("wasm invoke should succeed");
    assert_eq!(response.payload, vec![1, 2, 3]);

    let record = RuntimeBridgeRegistry::global()
        .latest_stage_record("plugin.demo.audit")
        .expect("bridge stage record should exist");
    assert_eq!(record.kind.as_deref(), Some("wasm"));
    assert_eq!(record.engine.as_deref(), Some("wasmtime"));
    assert_eq!(record.bundle_hash.as_deref(), bundle.bundle_hash.as_deref());
    let bytes = fs::read(&module_path).expect("module bytes should exist");
    assert_eq!(
        record.module_hash.as_deref(),
        Some(compute_hash(&bytes).as_str())
    );

    let descriptor = CapabilityRegistry::registry()
        .describe("plugin.demo.audit")
        .expect("plugin capability should be registered");
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
