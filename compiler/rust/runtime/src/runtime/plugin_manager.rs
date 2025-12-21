use std::{collections::HashMap, path::Path, sync::Mutex};

use crate::{
    capability::CapabilityRegistry,
    runtime::{
        plugin::{
            record_revoke_audit, PluginBundleRegistration, PluginError, PluginLoader,
            SignatureStatus, VerificationPolicy,
        },
        plugin_bridge::{PluginExecutionBridge, PluginInstance},
    },
};

/// 実行時のプラグイン状態。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginRuntimeState {
    Loaded,
    Failed,
    Unloaded,
}

/// プラグインの識別ハンドル。
#[derive(Debug, Clone)]
pub struct PluginRuntimeHandle {
    pub bundle_id: String,
    pub plugin_id: String,
}

#[derive(Debug)]
struct PluginRuntimeRecord {
    state: PluginRuntimeState,
    handle: PluginRuntimeHandle,
    bundle_version: String,
    signature_status: SignatureStatus,
    capabilities: Vec<String>,
    instance: Option<PluginInstance>,
}

/// 実行時プラグインのロード/アンロードを管理する。
pub struct PluginRuntimeManager {
    loader: PluginLoader,
    bridge: Box<dyn PluginExecutionBridge>,
    registry: &'static CapabilityRegistry,
    state: Mutex<HashMap<String, PluginRuntimeRecord>>,
}

impl PluginRuntimeManager {
    pub fn new(loader: PluginLoader, bridge: Box<dyn PluginExecutionBridge>) -> Self {
        Self {
            loader,
            bridge,
            registry: CapabilityRegistry::registry(),
            state: Mutex::new(HashMap::new()),
        }
    }

    pub fn load_bundle_and_attach(
        &self,
        path: impl AsRef<Path>,
        policy: VerificationPolicy,
    ) -> Result<PluginBundleRegistration, PluginError> {
        let bundle = self.loader.load_bundle_manifest(path)?;
        {
            let state_guard = self
                .state
                .lock()
                .expect("PluginRuntimeManager.state poisoned");
            for manifest in &bundle.plugins {
                let plugin_id = manifest.project.name.0.clone();
                if matches!(
                    state_guard.get(&plugin_id).map(|record| record.state),
                    Some(PluginRuntimeState::Loaded)
                ) {
                    return Err(PluginError::AlreadyLoaded { plugin_id });
                }
            }
        }
        let registration = self.loader.register_bundle(bundle.clone(), policy)?;

        let capabilities_by_plugin: HashMap<String, Vec<String>> = registration
            .plugins
            .iter()
            .map(|plugin| (plugin.plugin_id.clone(), plugin.capabilities.clone()))
            .collect();

        let mut instances: HashMap<String, PluginInstance> = HashMap::new();
        let mut load_error: Option<PluginError> = None;

        for manifest in &bundle.plugins {
            let plugin_id = manifest.project.name.0.clone();
            match self.bridge.load(manifest) {
                Ok(instance) => {
                    instances.insert(plugin_id, instance);
                }
                Err(err) => {
                    load_error = Some(err);
                    break;
                }
            }
        }

        let mut state_guard = self
            .state
            .lock()
            .expect("PluginRuntimeManager.state poisoned");
        for plugin in &registration.plugins {
            let plugin_id = plugin.plugin_id.clone();
            let handle = PluginRuntimeHandle {
                bundle_id: registration.bundle_id.clone(),
                plugin_id: plugin.plugin_id.clone(),
            };
            let instance = instances.remove(&plugin.plugin_id);
            let record = PluginRuntimeRecord {
                state: if instance.is_some() {
                    PluginRuntimeState::Loaded
                } else if load_error.is_some() {
                    PluginRuntimeState::Failed
                } else {
                    PluginRuntimeState::Unloaded
                },
                handle,
                bundle_version: registration.bundle_version.clone(),
                signature_status: registration.signature_status.clone(),
                capabilities: capabilities_by_plugin
                    .get(&plugin.plugin_id)
                    .cloned()
                    .unwrap_or_default(),
                instance,
            };
            state_guard.insert(plugin.plugin_id.clone(), record);
        }

        if let Some(error) = load_error {
            return Err(error);
        }

        Ok(registration)
    }

    pub fn unload(&self, plugin_id: &str) -> Result<(), PluginError> {
        let mut state_guard = self
            .state
            .lock()
            .expect("PluginRuntimeManager.state poisoned");
        let record = match state_guard.get_mut(plugin_id) {
            Some(record) => record,
            None => {
                return Err(PluginError::NotLoaded {
                    plugin_id: plugin_id.to_string(),
                })
            }
        };

        if let Some(instance) = record.instance.take() {
            self.bridge.unload(instance)?;
        }

        for capability in &record.capabilities {
            self.registry.unregister(capability)?;
        }

        record.state = PluginRuntimeState::Unloaded;
        record_revoke_audit(
            plugin_id,
            &record.capabilities,
            Some(&record.handle.bundle_id),
            Some(&record.bundle_version),
            Some(&record.signature_status),
        );
        Ok(())
    }

    pub fn state_of(&self, plugin_id: &str) -> Option<PluginRuntimeState> {
        let state_guard = self
            .state
            .lock()
            .expect("PluginRuntimeManager.state poisoned");
        state_guard.get(plugin_id).map(|record| record.state)
    }

}
