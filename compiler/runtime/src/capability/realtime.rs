use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::descriptor::CapabilityDescriptor;

/// Realtime Capability ハンドル。
#[derive(Debug, Clone)]
pub struct RealtimeCapability {
    descriptor: CapabilityDescriptor,
    metadata: RealtimeCapabilityMetadata,
}

impl RealtimeCapability {
    pub fn new(descriptor: CapabilityDescriptor, metadata: RealtimeCapabilityMetadata) -> Self {
        Self {
            descriptor,
            metadata,
        }
    }

    pub fn descriptor(&self) -> &CapabilityDescriptor {
        &self.descriptor
    }

    pub fn metadata(&self) -> &RealtimeCapabilityMetadata {
        &self.metadata
    }
}

/// リアルタイムスケジューリング情報。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RealtimeCapabilityMetadata {
    pub latency_budget_ns: Option<u64>,
    pub supports_deadlines: bool,
    pub clock_source: RealtimeClockSource,
}

impl Default for RealtimeCapabilityMetadata {
    fn default() -> Self {
        Self {
            latency_budget_ns: None,
            supports_deadlines: false,
            clock_source: RealtimeClockSource::Monotonic,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RealtimeClockSource {
    Monotonic,
    ExternalPps,
    Custom(String),
}
