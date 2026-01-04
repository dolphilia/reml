use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::descriptor::CapabilityDescriptor;

/// Hardware Capability ハンドル。
#[derive(Debug, Clone)]
pub struct HardwareCapability {
    descriptor: CapabilityDescriptor,
    metadata: HardwareCapabilityMetadata,
}

impl HardwareCapability {
    pub fn new(descriptor: CapabilityDescriptor, metadata: HardwareCapabilityMetadata) -> Self {
        Self {
            descriptor,
            metadata,
        }
    }

    pub fn descriptor(&self) -> &CapabilityDescriptor {
        &self.descriptor
    }

    pub fn metadata(&self) -> &HardwareCapabilityMetadata {
        &self.metadata
    }
}

/// ハードウェアサポート情報。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct HardwareCapabilityMetadata {
    pub accelerators: Vec<HardwareAcceleratorKind>,
    pub supports_simd: bool,
    pub supports_gpu: bool,
}

impl Default for HardwareCapabilityMetadata {
    fn default() -> Self {
        Self {
            accelerators: Vec::new(),
            supports_simd: false,
            supports_gpu: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HardwareAcceleratorKind {
    Gpu,
    Tpu,
    Cryptography,
    Custom(String),
}
