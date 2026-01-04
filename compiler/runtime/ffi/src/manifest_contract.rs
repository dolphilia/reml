use std::{
    collections::HashMap,
    fmt, fs,
    path::{Path, PathBuf},
    str::FromStr,
};

use toml::Value;
use serde::{Deserialize, Serialize};
use crate::{capability_metadata::StageRequirement, CapabilityId};

/// Manifest で保持する Capability 要求スパン。
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

    pub const fn len(&self) -> u32 {
        self.end.saturating_sub(self.start)
    }
}

/// Conductor 契約の単位要件。
#[derive(Debug, Clone)]
pub struct ConductorCapabilityRequirement {
    pub id: CapabilityId,
    pub stage: StageRequirement,
    pub declared_effects: Vec<String>,
    pub source_span: Option<CapabilityContractSpan>,
}

/// DSL/Conductor から渡される契約全体。
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

/// `reml.toml` の `run.target.capabilities` から読み取った登録情報。
#[derive(Debug, Clone)]
pub struct ManifestCapabilities {
    entries: HashMap<CapabilityId, ManifestCapabilityEntry>,
}

impl ManifestCapabilities {
    /// ファイルパスを読み取り、構造化された能力データを返す。
    pub fn load(path: impl AsRef<Path>) -> Result<Self, ManifestError> {
        let text = fs::read_to_string(path.as_ref()).map_err(ManifestError::from)?;
        let value: Value = toml::from_str(&text).map_err(ManifestError::from)?;
        Self::from_toml(value)
    }

    fn from_toml(value: Value) -> Result<Self, ManifestError> {
        let capabilities = value
            .get("run")
            .and_then(|run| run.get("target"))
            .and_then(|target| target.get("capabilities"))
            .and_then(|caps| caps.as_array())
            .ok_or(ManifestError::MissingCapabilities)?;

        let mut entries = HashMap::new();
        for entry in capabilities {
            let table = entry.as_table().ok_or_else(|| {
                ManifestError::InvalidCapabilityEntry("テーブルではありません".into())
            })?;

            let id = table
                .get("id")
                .and_then(|value| value.as_str())
                .ok_or_else(|| ManifestError::InvalidCapabilityEntry("id".into()))?
                .to_string();

            let stage_str = table
                .get("stage")
                .and_then(|value| value.as_str())
                .ok_or_else(|| ManifestError::InvalidCapabilityEntry("stage".into()))?;
            let stage = StageRequirement::from_str(stage_str)
                .map_err(|err| ManifestError::InvalidStage(err.to_string()))?;

            let declared_effects = table
                .get("declared_effects")
                .and_then(|value| value.as_array())
                .map(|array| {
                    array
                        .iter()
                        .filter_map(|item| item.as_str().map(|s| s.to_string()))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();

            let source_span = table.get("source_span").and_then(parse_span);

            if entries.contains_key(&id) {
                return Err(ManifestError::DuplicateCapability(id));
            }

            entries.insert(
                id,
                ManifestCapabilityEntry {
                    stage,
                    declared_effects,
                    source_span,
                },
            );
        }

        Ok(Self { entries })
    }

    /// ID で参照される entry を取得する。
    pub fn get(&self, id: &CapabilityId) -> Option<&ManifestCapabilityEntry> {
        self.entries.get(id)
    }
}

/// 単一 Capability に関する manifest 情報。
#[derive(Debug, Clone)]
pub struct ManifestCapabilityEntry {
    pub stage: StageRequirement,
    pub declared_effects: Vec<String>,
    pub source_span: Option<CapabilityContractSpan>,
}

/// マニフェスト読み込み・解析時のエラー。
#[derive(Debug, Clone)]
pub enum ManifestError {
    Io(String),
    Parse(String),
    MissingCapabilities,
    InvalidCapabilityEntry(String),
    DuplicateCapability(CapabilityId),
    InvalidStage(String),
}

impl fmt::Display for ManifestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ManifestError::Io(err) => write!(f, "マニフェスト読み込みエラー: {}", err),
            ManifestError::Parse(err) => write!(f, "TOML 解析エラー: {}", err),
            ManifestError::MissingCapabilities => {
                write!(f, "run.target.capabilities セクションが見つかりません")
            }
            ManifestError::InvalidCapabilityEntry(field) => {
                write!(f, "Capability エントリが不正 ({})", field)
            }
            ManifestError::DuplicateCapability(id) => {
                write!(f, "Capability が重複しています: {}", id)
            }
            ManifestError::InvalidStage(reason) => {
                write!(f, "Stage 要件の解析に失敗: {}", reason)
            }
        }
    }
}

impl std::error::Error for ManifestError {}

impl From<std::io::Error> for ManifestError {
    fn from(value: std::io::Error) -> Self {
        ManifestError::Io(value.to_string())
    }
}

impl From<toml::de::Error> for ManifestError {
    fn from(value: toml::de::Error) -> Self {
        ManifestError::Parse(value.to_string())
    }
}

fn parse_span(value: &Value) -> Option<CapabilityContractSpan> {
    let table = value.as_table()?;
    let start = table.get("start")?.as_integer()?;
    let end = table
        .get("end")
        .and_then(|value| value.as_integer())
        .or_else(|| {
            table
                .get("length")
                .and_then(|value| value.as_integer())
                .map(|len| start + len)
        })?;
    let start = to_u32(start)?;
    let end = to_u32(end)?;
    Some(CapabilityContractSpan::new(start, end))
}

fn to_u32(value: i64) -> Option<u32> {
    if value < 0 || value > u32::MAX as i64 {
        return None;
    }
    Some(value as u32)
}
