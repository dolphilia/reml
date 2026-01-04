use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::descriptor::CapabilityDescriptor;

/// Security Capability ハンドル。
#[derive(Debug, Clone)]
pub struct SecurityCapability {
    descriptor: CapabilityDescriptor,
    metadata: SecurityCapabilityMetadata,
}

impl SecurityCapability {
    pub fn new(descriptor: CapabilityDescriptor, metadata: SecurityCapabilityMetadata) -> Self {
        Self {
            descriptor,
            metadata,
        }
    }

    pub fn descriptor(&self) -> &CapabilityDescriptor {
        &self.descriptor
    }

    pub fn metadata(&self) -> &SecurityCapabilityMetadata {
        &self.metadata
    }
}

/// セキュリティポリシーの概要。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SecurityCapabilityMetadata {
    pub policies: Vec<SecurityPolicyKind>,
    pub enforces_path_sandbox: bool,
    pub tracks_manifest: bool,
}

impl Default for SecurityCapabilityMetadata {
    fn default() -> Self {
        Self {
            policies: vec![SecurityPolicyKind::FsSandbox],
            enforces_path_sandbox: true,
            tracks_manifest: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SecurityPolicyKind {
    FsSandbox,
    CapabilityWhitelist,
    ManifestContract,
    Custom(String),
}
