use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::descriptor::CapabilityDescriptor;

#[derive(Debug, Clone)]
pub struct NativeCapability {
    descriptor: CapabilityDescriptor,
    metadata: NativeCapabilityMetadata,
}

impl NativeCapability {
    pub fn new(descriptor: CapabilityDescriptor, metadata: NativeCapabilityMetadata) -> Self {
        Self {
            descriptor,
            metadata,
        }
    }

    pub fn descriptor(&self) -> &CapabilityDescriptor {
        &self.descriptor
    }

    pub fn metadata(&self) -> &NativeCapabilityMetadata {
        &self.metadata
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
pub struct NativeCapabilityMetadata {}
