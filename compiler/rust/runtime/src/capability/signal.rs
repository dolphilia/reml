use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::descriptor::CapabilityDescriptor;

/// Signal Capability ハンドル。
#[derive(Debug, Clone)]
pub struct SignalCapability {
    descriptor: CapabilityDescriptor,
    metadata: SignalCapabilityMetadata,
}

impl SignalCapability {
    pub fn new(descriptor: CapabilityDescriptor, metadata: SignalCapabilityMetadata) -> Self {
        Self {
            descriptor,
            metadata,
        }
    }

    pub fn descriptor(&self) -> &CapabilityDescriptor {
        &self.descriptor
    }

    pub fn metadata(&self) -> &SignalCapabilityMetadata {
        &self.metadata
    }
}

/// シグナル監視情報。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SignalCapabilityMetadata {
    pub handled_signals: Vec<String>,
    pub supports_subscribe: bool,
    pub requires_native_support: bool,
}

impl Default for SignalCapabilityMetadata {
    fn default() -> Self {
        Self {
            handled_signals: Vec::new(),
            supports_subscribe: true,
            requires_native_support: true,
        }
    }
}
