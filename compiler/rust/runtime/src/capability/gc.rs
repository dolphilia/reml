use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::descriptor::CapabilityDescriptor;

/// GC Capability のハンドル。
#[derive(Debug, Clone)]
pub struct GcCapability {
    descriptor: CapabilityDescriptor,
    metadata: GcCapabilityMetadata,
}

impl GcCapability {
    pub fn new(descriptor: CapabilityDescriptor, metadata: GcCapabilityMetadata) -> Self {
        Self {
            descriptor,
            metadata,
        }
    }

    pub fn descriptor(&self) -> &CapabilityDescriptor {
        &self.descriptor
    }

    pub fn metadata(&self) -> &GcCapabilityMetadata {
        &self.metadata
    }
}

/// GC 実装に関するメタデータ。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct GcCapabilityMetadata {
    pub strategy: GcStrategy,
    pub supports_compaction: bool,
    pub concurrent: bool,
}

impl Default for GcCapabilityMetadata {
    fn default() -> Self {
        Self {
            strategy: GcStrategy::MarkSweep,
            supports_compaction: true,
            concurrent: false,
        }
    }
}

/// GC 方式。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", tag = "kind", content = "detail")]
pub enum GcStrategy {
    MarkSweep,
    Generational,
    ReferenceCounting,
    Immix,
    Custom(String),
}
