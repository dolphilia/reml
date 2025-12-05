use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::descriptor::CapabilityDescriptor;

/// Memory Capability ハンドル。
#[derive(Debug, Clone)]
pub struct MemoryCapability {
    descriptor: CapabilityDescriptor,
    metadata: MemoryCapabilityMetadata,
}

impl MemoryCapability {
    pub fn new(descriptor: CapabilityDescriptor, metadata: MemoryCapabilityMetadata) -> Self {
        Self {
            descriptor,
            metadata,
        }
    }

    pub fn descriptor(&self) -> &CapabilityDescriptor {
        &self.descriptor
    }

    pub fn metadata(&self) -> &MemoryCapabilityMetadata {
        &self.metadata
    }
}

/// メモリ管理に関する情報。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct MemoryCapabilityMetadata {
    pub model: MemoryModel,
    pub max_allocation_bytes: Option<u64>,
    pub supports_guard_pages: bool,
}

impl Default for MemoryCapabilityMetadata {
    fn default() -> Self {
        Self {
            model: MemoryModel::VirtualMemory,
            max_allocation_bytes: None,
            supports_guard_pages: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MemoryModel {
    VirtualMemory,
    Arena,
    Region,
    Custom(String),
}
