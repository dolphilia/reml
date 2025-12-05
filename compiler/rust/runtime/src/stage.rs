//! 簡易 Stage/Capability 判定モデル。

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// ランタイムが扱う Stage ID。
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
pub enum StageId {
    Experimental,
    Alpha,
    Beta,
    Stable,
}

impl StageId {
    pub fn as_str(&self) -> &'static str {
        match self {
            StageId::Experimental => "experimental",
            StageId::Alpha => "alpha",
            StageId::Beta => "beta",
            StageId::Stable => "stable",
        }
    }
}

/// Capability Registry で使用する Stage 要件。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum StageRequirement {
    Exact(StageId),
    AtLeast(StageId),
}

impl StageRequirement {
    pub fn matches(&self, actual: StageId) -> bool {
        (*self).satisfies(actual)
    }

    /// 仕様で定義される `satisfies` 判定。
    pub const fn satisfies(self, actual: StageId) -> bool {
        match self {
            StageRequirement::Exact(expected) => stage_rank(actual) == stage_rank(expected),
            StageRequirement::AtLeast(threshold) => stage_rank(actual) >= stage_rank(threshold),
        }
    }
}

const fn stage_rank(stage: StageId) -> u8 {
    match stage {
        StageId::Experimental => 0,
        StageId::Alpha => 1,
        StageId::Beta => 2,
        StageId::Stable => 3,
    }
}
