//! 簡易 Stage/Capability 判定モデル。

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};

/// ランタイムが扱う Stage ID。
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "lowercase")]
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

impl fmt::Display for StageId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for StageId {
    type Err = StageParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "experimental" => Ok(StageId::Experimental),
            "alpha" => Ok(StageId::Alpha),
            "beta" => Ok(StageId::Beta),
            "stable" => Ok(StageId::Stable),
            other => Err(StageParseError::new(format!("unknown StageId '{other}'"))),
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

impl fmt::Display for StageRequirement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StageRequirement::Exact(stage) => write!(f, "exact({stage})"),
            StageRequirement::AtLeast(stage) => write!(f, "at_least({stage})"),
        }
    }
}

impl FromStr for StageRequirement {
    type Err = StageParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let normalized = s.trim();
        if normalized.is_empty() {
            return Err(StageParseError::new("stage requirement cannot be empty"));
        }
        if let Some(value) = normalized.strip_prefix("exact:") {
            return StageId::from_str(value).map(StageRequirement::Exact);
        }
        if let Some(value) = normalized.strip_prefix("at_least:") {
            return StageId::from_str(value).map(StageRequirement::AtLeast);
        }
        StageId::from_str(normalized).map(StageRequirement::Exact)
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

/// Stage 解析時のエラー。
#[derive(Debug, Clone)]
pub struct StageParseError {
    details: String,
}

impl StageParseError {
    pub fn new(details: impl Into<String>) -> Self {
        Self {
            details: details.into(),
        }
    }
}

impl fmt::Display for StageParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid stage value: {}", self.details)
    }
}

impl std::error::Error for StageParseError {}
