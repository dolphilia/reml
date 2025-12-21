use std::path::Path;

use thiserror::Error;

use crate::{
    capability::{CapabilityError, CapabilityRegistry, PluginCapabilityMetadata},
    config::manifest::{
        load_manifest, Manifest, ManifestCapabilities, ManifestCapabilityError, ProjectKind,
    },
    stage::{StageId, StageRequirement},
};

/// プラグイン登録結果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginRegistration {
    pub plugin_id: String,
    pub capabilities: Vec<String>,
}

/// プラグインローダのエラー。
#[derive(Debug, Error)]
pub enum PluginLoadError {
    #[error("manifest project.kind が plugin ではありません: {kind}")]
    NotPluginProject { kind: String },
    #[error("manifest 読み込みに失敗しました: {message}")]
    ManifestLoad { message: String },
    #[error("manifest capability 解析に失敗しました: {0}")]
    ManifestCapability(#[from] ManifestCapabilityError),
    #[error("capability 登録に失敗しました: {0}")]
    CapabilityRegistration(#[from] CapabilityError),
}

/// プラグイン登録のローダ。
#[derive(Debug, Clone)]
pub struct PluginLoader {
    registry: &'static CapabilityRegistry,
}

impl PluginLoader {
    pub fn new() -> Self {
        Self {
            registry: CapabilityRegistry::registry(),
        }
    }

    /// マニフェストファイルからプラグイン Capability を登録する。
    pub fn register_manifest_path(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<PluginRegistration, PluginLoadError> {
        let manifest = load_manifest(path).map_err(|diagnostic| PluginLoadError::ManifestLoad {
            message: diagnostic.message,
        })?;
        self.register_manifest(&manifest)
    }

    /// 既に読み込まれたマニフェストからプラグイン Capability を登録する。
    pub fn register_manifest(
        &self,
        manifest: &Manifest,
    ) -> Result<PluginRegistration, PluginLoadError> {
        if !matches!(manifest.project.kind, ProjectKind::Plugin) {
            return Err(PluginLoadError::NotPluginProject {
                kind: manifest.project.kind.as_str().to_string(),
            });
        }

        let capabilities = ManifestCapabilities::from_manifest(manifest)?;
        let capability_ids = capabilities.ids();
        let package = manifest.project.name.0.clone();
        let version = normalize_version(&manifest.project.version.0);
        let metadata = PluginCapabilityMetadata::new(
            package.clone(),
            version.clone(),
            capability_ids.clone(),
        );

        for capability_id in &capability_ids {
            if let Some(record) = capabilities.get(capability_id) {
                let stage = stage_from_requirement(record.stage);
                let effects: Vec<&str> =
                    record.declared_effects.iter().map(String::as_str).collect();
                self.registry.register_plugin_capability(
                    capability_id,
                    stage,
                    &effects,
                    metadata.clone(),
                )?;
            }
        }

        Ok(PluginRegistration {
            plugin_id: package,
            capabilities: capability_ids,
        })
    }
}

fn normalize_version(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn stage_from_requirement(requirement: StageRequirement) -> StageId {
    match requirement {
        StageRequirement::Exact(stage) | StageRequirement::AtLeast(stage) => stage,
    }
}
