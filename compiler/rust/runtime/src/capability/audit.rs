use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::descriptor::CapabilityDescriptor;

/// Audit Capability ハンドル。
#[derive(Debug, Clone)]
pub struct AuditCapability {
    descriptor: CapabilityDescriptor,
    metadata: AuditCapabilityMetadata,
}

impl AuditCapability {
    pub fn new(descriptor: CapabilityDescriptor, metadata: AuditCapabilityMetadata) -> Self {
        Self {
            descriptor,
            metadata,
        }
    }

    pub fn descriptor(&self) -> &CapabilityDescriptor {
        &self.descriptor
    }

    pub fn metadata(&self) -> &AuditCapabilityMetadata {
        &self.metadata
    }
}

/// Audit 出力に関する情報。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct AuditCapabilityMetadata {
    pub transport: AuditTransport,
    pub schema_version: String,
    pub persists_history: bool,
}

impl Default for AuditCapabilityMetadata {
    fn default() -> Self {
        Self {
            transport: AuditTransport::JsonLines,
            schema_version: "3.0.0-alpha".to_string(),
            persists_history: true,
        }
    }
}

/// Audit 伝送方式。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AuditTransport {
    JsonLines,
    Stdout,
    File,
    ExternalBridge,
}
