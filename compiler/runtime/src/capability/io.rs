use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::descriptor::CapabilityDescriptor;

/// IO Capability のハンドル。
#[derive(Debug, Clone)]
pub struct IoCapability {
    descriptor: CapabilityDescriptor,
    metadata: IoCapabilityMetadata,
}

impl IoCapability {
    pub fn new(descriptor: CapabilityDescriptor, metadata: IoCapabilityMetadata) -> Self {
        Self {
            descriptor,
            metadata,
        }
    }

    pub fn descriptor(&self) -> &CapabilityDescriptor {
        &self.descriptor
    }

    pub fn metadata(&self) -> &IoCapabilityMetadata {
        &self.metadata
    }
}

/// IO メタデータ。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct IoCapabilityMetadata {
    pub adapters: Vec<IoAdapterKind>,
    pub operations: Vec<IoOperationKind>,
    pub supports_async: bool,
}

impl Default for IoCapabilityMetadata {
    fn default() -> Self {
        Self {
            adapters: vec![IoAdapterKind::FileSystem],
            operations: vec![IoOperationKind::Read, IoOperationKind::Write],
            supports_async: false,
        }
    }
}

/// IO アダプタ種別。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum IoAdapterKind {
    FileSystem,
    Watcher,
    PathSecurity,
    Network,
    Custom(String),
}

/// IO 操作種別。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum IoOperationKind {
    Read,
    Write,
    Metadata,
    Symlink,
    Watcher,
    Custom(String),
}
