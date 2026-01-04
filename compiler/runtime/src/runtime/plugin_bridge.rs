use crate::config::manifest::{Manifest, ManifestCapabilities};
use crate::runtime::bridge::{BridgeMetadata, RuntimeBridgeRegistry};
use crate::runtime::plugin::PluginError;
use crate::stage::{StageId, StageRequirement};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use wasmtime::{Engine, Instance, Module, Store};

#[derive(Debug, Clone)]
pub struct PluginInstance {
    pub plugin_id: String,
}

#[derive(Debug, Clone)]
pub struct PluginInvokeRequest {
    pub entrypoint: String,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct PluginInvokeResponse {
    pub payload: Vec<u8>,
}

pub struct PluginLoadRequest<'a> {
    pub manifest: &'a Manifest,
    pub bundle_hash: Option<&'a str>,
    pub module_path: Option<&'a Path>,
}

pub trait PluginExecutionBridge: Send + Sync {
    fn load(&self, request: PluginLoadRequest) -> Result<PluginInstance, PluginError>;
    fn invoke(
        &self,
        instance: &PluginInstance,
        request: PluginInvokeRequest,
    ) -> Result<PluginInvokeResponse, PluginError>;
    fn unload(&self, instance: PluginInstance) -> Result<(), PluginError>;
}

pub struct NativePluginExecutionBridge {
    registry: &'static RuntimeBridgeRegistry,
}

impl NativePluginExecutionBridge {
    pub fn new() -> Self {
        Self {
            registry: RuntimeBridgeRegistry::global(),
        }
    }
}

impl Default for NativePluginExecutionBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginExecutionBridge for NativePluginExecutionBridge {
    fn load(&self, request: PluginLoadRequest) -> Result<PluginInstance, PluginError> {
        let capabilities =
            ManifestCapabilities::from_manifest(request.manifest).map_err(|err| {
                PluginError::VerificationFailed {
                    message: err.to_string(),
                }
            })?;

        for (capability_id, record) in capabilities.iter() {
            let actual = stage_from_requirement(record.stage);
            self.registry
                .record_stage_probe(capability_id.as_str(), record.stage, actual);
        }

        Ok(PluginInstance {
            plugin_id: request.manifest.project.name.0.clone(),
        })
    }

    fn invoke(
        &self,
        _instance: &PluginInstance,
        request: PluginInvokeRequest,
    ) -> Result<PluginInvokeResponse, PluginError> {
        match request.entrypoint.as_str() {
            "plugin.echo" => Ok(PluginInvokeResponse {
                payload: request.payload,
            }),
            "plugin.noop" => Ok(PluginInvokeResponse {
                payload: Vec::new(),
            }),
            "plugin.io_error" => Err(PluginError::Io {
                message: io_error("native bridge io error").to_string(),
            }),
            "plugin.fail" => Err(PluginError::VerificationFailed {
                message: "native bridge verify failure".to_string(),
            }),
            other => Err(PluginError::VerificationFailed {
                message: format!("unknown entrypoint: {other}"),
            }),
        }
    }

    fn unload(&self, _instance: PluginInstance) -> Result<(), PluginError> {
        Ok(())
    }
}

pub struct PluginWasmBridge {
    registry: &'static RuntimeBridgeRegistry,
    engine: Engine,
    modules: Mutex<HashMap<String, WasmModuleRecord>>,
}

impl PluginWasmBridge {
    pub fn new() -> Self {
        Self {
            registry: RuntimeBridgeRegistry::global(),
            engine: Engine::default(),
            modules: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for PluginWasmBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginExecutionBridge for PluginWasmBridge {
    fn load(&self, request: PluginLoadRequest) -> Result<PluginInstance, PluginError> {
        let module_path = request
            .module_path
            .ok_or_else(|| PluginError::VerificationFailed {
                message: "wasm module path is missing".to_string(),
            })?;

        let module_bytes = fs::read(module_path).map_err(|err| PluginError::Io {
            message: err.to_string(),
        })?;
        let module_hash = compute_module_hash(&module_bytes);
        let module = Module::new(&self.engine, &module_bytes).map_err(|err| {
            PluginError::VerificationFailed {
                message: err.to_string(),
            }
        })?;

        let capabilities =
            ManifestCapabilities::from_manifest(request.manifest).map_err(|err| {
                PluginError::VerificationFailed {
                    message: err.to_string(),
                }
            })?;
        let bundle_hash = request.bundle_hash.map(str::to_string);
        for (capability_id, record) in capabilities.iter() {
            let actual = stage_from_requirement(record.stage);
            self.registry.record_stage_probe_with_metadata(
                capability_id.as_str(),
                record.stage,
                actual,
                BridgeMetadata::wasm(bundle_hash.clone(), Some(module_hash.clone())),
            );
        }

        let plugin_id = request.manifest.project.name.0.clone();
        let mut guard = self
            .modules
            .lock()
            .expect("PluginWasmBridge.modules poisoned");
        guard.insert(
            plugin_id.clone(),
            WasmModuleRecord {
                module,
                module_path: module_path.to_path_buf(),
                module_hash,
                bundle_hash,
            },
        );

        Ok(PluginInstance { plugin_id })
    }

    fn invoke(
        &self,
        instance: &PluginInstance,
        request: PluginInvokeRequest,
    ) -> Result<PluginInvokeResponse, PluginError> {
        let module = {
            let guard = self
                .modules
                .lock()
                .expect("PluginWasmBridge.modules poisoned");
            guard
                .get(&instance.plugin_id)
                .cloned()
                .ok_or_else(|| PluginError::NotLoaded {
                    plugin_id: instance.plugin_id.clone(),
                })?
        };

        let mut store = Store::new(&self.engine, ());
        let instance =
            Instance::new(&mut store, &module.module, &[]).map_err(|err| PluginError::Bridge {
                message: err.to_string(),
            })?;
        let memory =
            instance
                .get_memory(&mut store, "memory")
                .ok_or_else(|| PluginError::Bridge {
                    message: "wasm memory export not found".to_string(),
                })?;

        let payload_len = request.payload.len();
        memory
            .write(&mut store, 0, &request.payload)
            .map_err(|err| PluginError::Bridge {
                message: err.to_string(),
            })?;

        let func = instance
            .get_func(&mut store, request.entrypoint.as_str())
            .ok_or_else(|| PluginError::VerificationFailed {
                message: format!("unknown entrypoint: {}", request.entrypoint),
            })?;
        let typed = func
            .typed::<(i32, i32), i32>(&store)
            .map_err(|err| PluginError::Bridge {
                message: err.to_string(),
            })?;
        let response_len = typed
            .call(&mut store, (0, payload_len as i32))
            .map_err(|err| PluginError::Bridge {
                message: err.to_string(),
            })?;

        let mut response = vec![0u8; response_len as usize];
        memory
            .read(&mut store, 0, &mut response)
            .map_err(|err| PluginError::Bridge {
                message: err.to_string(),
            })?;

        Ok(PluginInvokeResponse { payload: response })
    }

    fn unload(&self, instance: PluginInstance) -> Result<(), PluginError> {
        let mut guard = self
            .modules
            .lock()
            .expect("PluginWasmBridge.modules poisoned");
        guard.remove(&instance.plugin_id);
        Ok(())
    }
}

#[derive(Clone)]
struct WasmModuleRecord {
    module: Module,
    module_path: PathBuf,
    module_hash: String,
    bundle_hash: Option<String>,
}

fn stage_from_requirement(requirement: StageRequirement) -> StageId {
    match requirement {
        StageRequirement::Exact(stage) | StageRequirement::AtLeast(stage) => stage,
    }
}

fn compute_module_hash(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    format!("sha256:{}", bytes_to_hex(digest.as_slice()))
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for value in bytes {
        out.push(hex_nibble(value >> 4));
        out.push(hex_nibble(value & 0x0f));
    }
    out
}

fn hex_nibble(value: u8) -> char {
    match value {
        0..=9 => (b'0' + value) as char,
        10..=15 => (b'a' + (value - 10)) as char,
        _ => '0',
    }
}

fn io_error(message: &str) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::Other, message)
}
