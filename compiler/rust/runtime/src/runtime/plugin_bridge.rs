use crate::config::manifest::Manifest;
use crate::runtime::plugin::PluginError;

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
