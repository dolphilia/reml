use std::{collections::HashMap, path::Path, sync::Mutex};

use crate::{
    capability::CapabilityRegistry,
    config::manifest::ManifestCapabilities,
    runtime::{
        plugin::{
            record_revoke_audit, BundleContext, PluginBundleRegistration, PluginError,
            PluginLoader, SignatureStatus, VerificationPolicy,
        },
        plugin_bridge::{PluginExecutionBridge, PluginInstance, PluginLoadRequest},
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
        let signature_status = self.loader.verify_bundle_signature(&bundle, policy)?;
        let context = BundleContext::new(&bundle, signature_status.clone());
        let mut registered_capabilities = Vec::new();
        let mut registrations = Vec::new();
        let mut capabilities_by_plugin = HashMap::new();

        for manifest in &bundle.plugins {
            let registration = match self
                .loader
                .register_manifest_with_context(manifest, Some(&context))
            {
                Ok(registration) => registration,
                Err(err) => {
                    self.rollback_capabilities(&registered_capabilities);
                    return Err(PluginError::BundleInstallFailed {
                        message: err.to_string(),
                        capability_error: None,
                    });
                }
            };
            registered_capabilities.extend(registration.capabilities.iter().cloned());
            capabilities_by_plugin.insert(
                registration.plugin_id.clone(),
                registration.capabilities.clone(),
            );

            let manifest_caps = match ManifestCapabilities::from_manifest(manifest) {
                Ok(manifest_caps) => manifest_caps,
                Err(err) => {
                    self.rollback_capabilities(&registered_capabilities);
                    return Err(PluginError::BundleInstallFailed {
                        message: err.to_string(),
                        capability_error: None,
                    });
                }
            };
            for (capability_id, record) in manifest_caps.iter() {
                if let Err(err) = self.registry.verify_capability_stage(
                    capability_id,
                    record.stage,
                    &record.declared_effects,
                ) {
                    self.rollback_capabilities(&registered_capabilities);
                    return Err(PluginError::BundleInstallFailed {
                        message: err.detail().to_string(),
                        capability_error: Some(err),
                    });
                }
            }

            registrations.push(registration);
        }

        let registration = PluginBundleRegistration {
            bundle_id: bundle.bundle_id.clone(),
            bundle_version: bundle.bundle_version.clone(),
            plugins: registrations,
            signature_status: signature_status.clone(),
        };

        let mut instances: HashMap<String, PluginInstance> = HashMap::new();
        let mut load_error: Option<PluginError> = None;

        for manifest in &bundle.plugins {
            let plugin_id = manifest.project.name.0.clone();
            let module_info = bundle.module_info_for(&plugin_id);
            let request = PluginLoadRequest {
                manifest,
                bundle_hash: bundle.bundle_hash.as_deref(),
                module_path: module_info.map(|info| info.module_path.as_path()),
            };
            match self.bridge.load(request) {
                Ok(instance) => {
                    instances.insert(plugin_id, instance);
                }
                Err(err) => {
                    load_error = Some(err);
                    break;
                }
            }
        }

        if load_error.is_some() {
            for (_, instance) in instances.drain() {
                let _ = self.bridge.unload(instance);
            }
        }

        let load_failed = load_error.is_some();
        let mut state_guard = self
            .state
            .lock()
            .expect("PluginRuntimeManager.state poisoned");
        for plugin in &registration.plugins {
            let handle = PluginRuntimeHandle {
                bundle_id: registration.bundle_id.clone(),
                plugin_id: plugin.plugin_id.clone(),
            };
            let instance = if load_failed {
                None
            } else {
                instances.remove(&plugin.plugin_id)
            };
            let record = PluginRuntimeRecord {
                state: if load_failed {
                    PluginRuntimeState::Failed
                } else if instance.is_some() {
                    PluginRuntimeState::Loaded
                } else {
                    PluginRuntimeState::Unloaded
                },
                handle,
                bundle_version: registration.bundle_version.clone(),
                signature_status: registration.signature_status.clone(),
                capabilities: if load_failed {
                    Vec::new()
                } else {
                    capabilities_by_plugin
                        .get(&plugin.plugin_id)
                        .cloned()
                        .unwrap_or_default()
                },
                instance,
            };
            state_guard.insert(plugin.plugin_id.clone(), record);
        }

        if let Some(error) = load_error {
            self.rollback_capabilities(&registered_capabilities);
            return Err(PluginError::BundleInstallFailed {
                message: error.to_string(),
                capability_error: None,
            });
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

    fn rollback_capabilities(&self, capabilities: &[String]) {
        for capability in capabilities {
            let _ = self.registry.unregister(capability);
        }
    }
}
