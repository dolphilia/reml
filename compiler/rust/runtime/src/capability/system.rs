use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::descriptor::CapabilityDescriptor;

/// System Capability ハンドル。
#[derive(Debug, Clone)]
pub struct SystemCapability {
    descriptor: CapabilityDescriptor,
    metadata: SystemCapabilityMetadata,
}

impl SystemCapability {
    pub fn new(descriptor: CapabilityDescriptor, metadata: SystemCapabilityMetadata) -> Self {
        Self {
            descriptor,
            metadata,
        }
    }

    pub fn descriptor(&self) -> &CapabilityDescriptor {
        &self.descriptor
    }

    pub fn metadata(&self) -> &SystemCapabilityMetadata {
        &self.metadata
    }
}

/// 実行環境に関する情報。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SystemCapabilityMetadata {
    pub os: Option<String>,
    pub arch: Option<String>,
    pub profile: Option<String>,
}

impl Default for SystemCapabilityMetadata {
    fn default() -> Self {
        Self {
            os: None,
            arch: None,
            profile: None,
        }
    }
}
