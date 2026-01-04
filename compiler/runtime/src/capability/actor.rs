use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::descriptor::CapabilityDescriptor;

/// Actor Capability ハンドル。
#[derive(Debug, Clone)]
pub struct ActorCapability {
    descriptor: CapabilityDescriptor,
    metadata: ActorCapabilityMetadata,
}

impl ActorCapability {
    pub fn new(descriptor: CapabilityDescriptor, metadata: ActorCapabilityMetadata) -> Self {
        Self {
            descriptor,
            metadata,
        }
    }

    pub fn descriptor(&self) -> &CapabilityDescriptor {
        &self.descriptor
    }

    pub fn metadata(&self) -> &ActorCapabilityMetadata {
        &self.metadata
    }
}

/// Actor 実装に関する情報。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ActorCapabilityMetadata {
    pub scheduler: ActorSchedulerKind,
    pub supports_remote_mailbox: bool,
    pub mailbox_capacity: Option<u32>,
}

impl Default for ActorCapabilityMetadata {
    fn default() -> Self {
        Self {
            scheduler: ActorSchedulerKind::Local,
            supports_remote_mailbox: false,
            mailbox_capacity: Some(1024),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ActorSchedulerKind {
    Local,
    Distributed,
    Custom(String),
}
