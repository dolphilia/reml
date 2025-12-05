use std::{collections::BTreeSet, path::PathBuf};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::stage::StageId;

/// Capability を識別する ID。
pub type CapabilityId = String;

/// Capability で要求・提供される効果タグ。
pub type EffectTag = String;

/// Capability の公開メタデータ。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CapabilityDescriptor {
    pub id: CapabilityId,
    pub stage: StageId,
    pub effect_scope: BTreeSet<EffectTag>,
    pub provider: CapabilityProvider,
    pub manifest_path: Option<PathBuf>,
    pub last_verified_at: Option<CapabilityTimestamp>,
}

impl CapabilityDescriptor {
    /// 新しい Descriptor を生成する。
    pub fn new(
        id: impl Into<String>,
        stage: StageId,
        effect_scope: impl IntoIterator<Item = impl Into<String>>,
        provider: CapabilityProvider,
    ) -> Self {
        Self {
            id: id.into(),
            stage,
            effect_scope: effect_scope.into_iter().map(Into::into).collect(),
            provider,
            manifest_path: None,
            last_verified_at: None,
        }
    }

    pub fn with_manifest_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.manifest_path = Some(path.into());
        self
    }

    pub fn with_last_verified_at(mut self, timestamp: CapabilityTimestamp) -> Self {
        self.last_verified_at = Some(timestamp);
        self
    }

    pub fn stage(&self) -> StageId {
        self.stage
    }

    pub fn effect_scope(&self) -> &BTreeSet<EffectTag> {
        &self.effect_scope
    }
}

/// Capability を提供する主体。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CapabilityProvider {
    Core,
    Plugin {
        package: String,
        version: Option<String>,
    },
    ExternalBridge {
        name: String,
        version: Option<String>,
    },
    RuntimeComponent {
        name: String,
    },
}

/// Descriptor で使用する Timestamp の軽量表現。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CapabilityTimestamp {
    pub seconds: i64,
    pub nanos: i32,
}

impl CapabilityTimestamp {
    pub const fn new(seconds: i64, nanos: i32) -> Self {
        Self { seconds, nanos }
    }
}

#[cfg(any(feature = "core_time", feature = "metrics"))]
impl From<crate::time::Timestamp> for CapabilityTimestamp {
    fn from(value: crate::time::Timestamp) -> Self {
        CapabilityTimestamp::new(value.seconds(), value.nanos())
    }
}
