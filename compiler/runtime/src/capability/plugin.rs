use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::descriptor::CapabilityDescriptor;

/// プラグイン Capability ハンドル。
#[derive(Debug, Clone)]
pub struct PluginCapability {
    descriptor: CapabilityDescriptor,
    metadata: PluginCapabilityMetadata,
}

impl PluginCapability {
    pub fn new(descriptor: CapabilityDescriptor, metadata: PluginCapabilityMetadata) -> Self {
        Self {
            descriptor,
            metadata,
        }
    }

    pub fn descriptor(&self) -> &CapabilityDescriptor {
        &self.descriptor
    }

    pub fn metadata(&self) -> &PluginCapabilityMetadata {
        &self.metadata
    }
}

/// プラグイン公開情報。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PluginCapabilityMetadata {
    pub package: String,
    pub version: Option<String>,
    pub bundle_id: Option<String>,
    pub bundle_version: Option<String>,
    pub exposed_capabilities: Vec<String>,
}

impl PluginCapabilityMetadata {
    pub fn new(
        package: impl Into<String>,
        version: Option<impl Into<String>>,
        exposed_capabilities: Vec<String>,
    ) -> Self {
        Self {
            package: package.into(),
            version: version.map(Into::into),
            bundle_id: None,
            bundle_version: None,
            exposed_capabilities,
        }
    }
}

impl Default for PluginCapabilityMetadata {
    fn default() -> Self {
        Self {
            package: "plugin".to_string(),
            version: None,
            bundle_id: None,
            bundle_version: None,
            exposed_capabilities: Vec::new(),
        }
    }
}
