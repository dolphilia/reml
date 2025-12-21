use crate::config::manifest::{Manifest, ManifestCapabilities};
use crate::runtime::bridge::RuntimeBridgeRegistry;
use crate::runtime::plugin::PluginError;
use crate::stage::{StageId, StageRequirement};

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

pub trait PluginExecutionBridge: Send + Sync {
    fn load(&self, manifest: &Manifest) -> Result<PluginInstance, PluginError>;
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
    fn load(&self, manifest: &Manifest) -> Result<PluginInstance, PluginError> {
        let capabilities = ManifestCapabilities::from_manifest(manifest).map_err(|err| {
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
            plugin_id: manifest.project.name.0.clone(),
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
            "plugin.noop" => Ok(PluginInvokeResponse { payload: Vec::new() }),
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

fn stage_from_requirement(requirement: StageRequirement) -> StageId {
    match requirement {
        StageRequirement::Exact(stage) | StageRequirement::AtLeast(stage) => stage,
    }
}

fn io_error(message: &str) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::Other, message)
}
