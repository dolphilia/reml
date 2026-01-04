use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::descriptor::CapabilityDescriptor;

/// Core.Collections で利用する Capability。
#[derive(Debug, Clone)]
pub struct CollectionsCapability {
    descriptor: CapabilityDescriptor,
    metadata: CollectionsCapabilityMetadata,
}

impl CollectionsCapability {
    pub fn new(descriptor: CapabilityDescriptor, metadata: CollectionsCapabilityMetadata) -> Self {
        Self {
            descriptor,
            metadata,
        }
    }

    pub fn descriptor(&self) -> &CapabilityDescriptor {
        &self.descriptor
    }

    pub fn metadata(&self) -> &CollectionsCapabilityMetadata {
        &self.metadata
    }
}

/// Ref/Cell など内部可変性を伴うコレクション Capability の付帯情報。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CollectionsCapabilityMetadata {
    /// `collector.effect.*` で監査する効果ラベル。
    pub collector_effects: Vec<String>,
    /// 内部可変性 (`effect {mut}`) を追跡するかどうか。
    pub tracks_mutation: bool,
    /// 参照カウント (`effect {rc}`) を追跡するかどうか。
    pub tracks_reference_count: bool,
}

impl Default for CollectionsCapabilityMetadata {
    fn default() -> Self {
        Self {
            collector_effects: vec![],
            tracks_mutation: true,
            tracks_reference_count: true,
        }
    }
}
