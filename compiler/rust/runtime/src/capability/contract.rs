use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use super::descriptor::CapabilityId;
use crate::stage::StageRequirement;

/// Manifest で記録される要求スパン情報。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityContractSpan {
    pub start: u32,
    pub end: u32,
}

impl CapabilityContractSpan {
    pub const fn new(start: u32, end: u32) -> Self {
        if end < start {
            Self { start, end: start }
        } else {
            Self { start, end }
        }
    }
}

/// Conductor/DSL が宣言する Capability 要件の 1 件。
#[derive(Debug, Clone)]
pub struct ConductorCapabilityRequirement {
    pub id: CapabilityId,
    pub stage: StageRequirement,
    pub declared_effects: Vec<String>,
    pub source_span: Option<CapabilityContractSpan>,
}

impl ConductorCapabilityRequirement {
    pub fn new(
        id: impl Into<String>,
        stage: StageRequirement,
        declared_effects: impl IntoIterator<Item = impl Into<String>>,
        source_span: Option<CapabilityContractSpan>,
    ) -> Self {
        Self {
            id: id.into(),
            stage,
            declared_effects: declared_effects.into_iter().map(Into::into).collect(),
            source_span,
        }
    }
}

/// `verify_conductor_contract` に渡す要求集合。
#[derive(Debug, Clone)]
pub struct ConductorCapabilityContract {
    pub requirements: Vec<ConductorCapabilityRequirement>,
    pub manifest_path: Option<PathBuf>,
}

impl ConductorCapabilityContract {
    pub fn new(requirements: Vec<ConductorCapabilityRequirement>) -> Self {
        Self {
            requirements,
            manifest_path: None,
        }
    }

    pub fn with_manifest_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.manifest_path = Some(path.into());
        self
    }
}
