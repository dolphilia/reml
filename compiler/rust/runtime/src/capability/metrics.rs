use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::descriptor::CapabilityDescriptor;

/// Metrics Capability ハンドル。
#[derive(Debug, Clone)]
pub struct MetricsCapability {
    descriptor: CapabilityDescriptor,
    metadata: MetricsCapabilityMetadata,
}

impl MetricsCapability {
    pub fn new(descriptor: CapabilityDescriptor, metadata: MetricsCapabilityMetadata) -> Self {
        Self {
            descriptor,
            metadata,
        }
    }

    pub fn descriptor(&self) -> &CapabilityDescriptor {
        &self.descriptor
    }

    pub fn metadata(&self) -> &MetricsCapabilityMetadata {
        &self.metadata
    }
}

/// Metrics 出力に関する情報。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct MetricsCapabilityMetadata {
    pub exporters: Vec<MetricsExporterKind>,
    pub supports_histogram: bool,
    pub supports_sampling: bool,
}

impl Default for MetricsCapabilityMetadata {
    fn default() -> Self {
        Self {
            exporters: vec![MetricsExporterKind::Json],
            supports_histogram: true,
            supports_sampling: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MetricsExporterKind {
    Json,
    Prometheus,
    Otel,
    Custom(String),
}
