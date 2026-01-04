use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::descriptor::CapabilityDescriptor;

/// Async/Actor ランタイム Capability ハンドル。
#[derive(Debug, Clone)]
pub struct AsyncCapability {
    descriptor: CapabilityDescriptor,
    metadata: AsyncCapabilityMetadata,
}

impl AsyncCapability {
    pub fn new(descriptor: CapabilityDescriptor, metadata: AsyncCapabilityMetadata) -> Self {
        Self {
            descriptor,
            metadata,
        }
    }

    pub fn descriptor(&self) -> &CapabilityDescriptor {
        &self.descriptor
    }

    pub fn metadata(&self) -> &AsyncCapabilityMetadata {
        &self.metadata
    }
}

/// Async ランタイム設定。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct AsyncCapabilityMetadata {
    pub scheduler: AsyncSchedulerKind,
    pub supports_backpressure: bool,
    pub supports_blocking_adapter: bool,
}

impl Default for AsyncCapabilityMetadata {
    fn default() -> Self {
        Self {
            scheduler: AsyncSchedulerKind::MultiThread,
            supports_backpressure: true,
            supports_blocking_adapter: false,
        }
    }
}

/// スケジューラ種別。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AsyncSchedulerKind {
    SingleThread,
    MultiThread,
    Actor,
    Custom(String),
}
