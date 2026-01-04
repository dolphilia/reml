use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::descriptor::CapabilityDescriptor;

/// プロセス Capability ハンドル。
#[derive(Debug, Clone)]
pub struct ProcessCapability {
    descriptor: CapabilityDescriptor,
    metadata: ProcessCapabilityMetadata,
}

impl ProcessCapability {
    pub fn new(descriptor: CapabilityDescriptor, metadata: ProcessCapabilityMetadata) -> Self {
        Self {
            descriptor,
            metadata,
        }
    }

    pub fn descriptor(&self) -> &CapabilityDescriptor {
        &self.descriptor
    }

    pub fn metadata(&self) -> &ProcessCapabilityMetadata {
        &self.metadata
    }
}

/// プロセス制御に関する情報。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ProcessCapabilityMetadata {
    pub supports_spawn: bool,
    pub supports_kill: bool,
    pub spawn_strategy: ProcessSpawnStrategy,
}

impl Default for ProcessCapabilityMetadata {
    fn default() -> Self {
        Self {
            supports_spawn: true,
            supports_kill: true,
            spawn_strategy: ProcessSpawnStrategy::Blocking,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProcessSpawnStrategy {
    Blocking,
    Async,
    ExternalBridge,
    Custom(String),
}
