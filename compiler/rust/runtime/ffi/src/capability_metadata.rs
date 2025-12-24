use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr, time::SystemTime};

/// Capability の識別子。
pub type CapabilityId = String;

/// Capability の提供者種別。
#[derive(Debug, Clone)]
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

impl fmt::Display for CapabilityProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CapabilityProvider::Core => write!(f, "core"),
            CapabilityProvider::Plugin { package, version } => {
                write!(f, "plugin/{}", package)?;
                if let Some(version) = version {
                    write!(f, "@{}", version)?;
                }
                Ok(())
            }
            CapabilityProvider::ExternalBridge { name, version } => {
                write!(f, "bridge/{}", name)?;
                if let Some(version) = version {
                    write!(f, "@{}", version)?;
                }
                Ok(())
            }
            CapabilityProvider::RuntimeComponent { name } => write!(f, "runtime/{}", name),
        }
    }
}

/// Stage の識別子。順序付き。
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum StageId {
    Experimental,
    Alpha,
    Beta,
    Stable,
}

impl StageId {
    /// 仕様で定義される文字列表現を返す。
    pub fn as_str(&self) -> &'static str {
        match self {
            StageId::Experimental => "experimental",
            StageId::Alpha => "alpha",
            StageId::Beta => "beta",
            StageId::Stable => "stable",
        }
    }
}

/// Stage の解析が失敗した場合のエラー。
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
        write!(f, "不正な Stage 値: {}", self.details)
    }
}

impl std::error::Error for StageParseError {}

impl FromStr for StageId {
    type Err = StageParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "experimental" => Ok(StageId::Experimental),
            "alpha" => Ok(StageId::Alpha),
            "beta" => Ok(StageId::Beta),
            "stable" => Ok(StageId::Stable),
            other => Err(StageParseError::new(format!("未知の StageId '{}'", other))),
        }
    }
}

impl fmt::Display for StageId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            StageId::Experimental => "experimental",
            StageId::Alpha => "alpha",
            StageId::Beta => "beta",
            StageId::Stable => "stable",
        };
        write!(f, "{}", label)
    }
}

/// Stage 要件。Exact/AtLeast をサポート。
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StageRequirement {
    Exact(StageId),
    AtLeast(StageId),
}

impl StageRequirement {
    /// 実際の Stage を受け取り、要件を満たすか判定する。
    pub fn matches(self, actual: StageId) -> bool {
        match self {
            StageRequirement::Exact(expected) => actual == expected,
            StageRequirement::AtLeast(minimum) => actual >= minimum,
        }
    }
}

impl FromStr for StageRequirement {
    type Err = StageParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let normalized = s.trim();
        if let Some(value) = normalized.strip_prefix("exact:") {
            let stage = StageId::from_str(value)?;
            return Ok(StageRequirement::Exact(stage));
        }
        if let Some(value) = normalized.strip_prefix("at_least:") {
            let stage = StageId::from_str(value)?;
            return Ok(StageRequirement::AtLeast(stage));
        }
        if normalized.is_empty() {
            return Err(StageParseError::new("空文字列"));
        }
        StageId::from_str(normalized).map(StageRequirement::Exact)
    }
}

impl fmt::Display for StageRequirement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StageRequirement::Exact(stage) => write!(f, "exact({})", stage),
            StageRequirement::AtLeast(stage) => write!(f, "at_least({})", stage),
        }
    }
}

/// Capability の公開メタデータ。
#[derive(Debug, Clone)]
pub struct CapabilityDescriptor {
    pub id: CapabilityId,
    pub stage: StageId,
    pub effect_scope: Vec<String>,
    pub provider: CapabilityProvider,
    pub manifest_path: Option<String>,
    pub last_verified_at: Option<SystemTime>,
}

/// プラグイン公開情報。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginCapabilityMetadata {
    pub package: String,
    pub version: Option<String>,
    pub bundle_id: Option<String>,
    pub bundle_version: Option<String>,
    pub exposed_capabilities: Vec<String>,
}

impl PluginCapabilityMetadata {
    pub fn new(
        package: impl Into<String>,
        version: Option<impl Into<String>>,
        exposed_capabilities: Vec<String>,
    ) -> Self {
        Self {
            package: package.into(),
            version: version.map(Into::into),
            bundle_id: None,
            bundle_version: None,
            exposed_capabilities,
        }
    }
}

impl Default for PluginCapabilityMetadata {
    fn default() -> Self {
        Self {
            package: "plugin".to_string(),
            version: None,
            bundle_id: None,
            bundle_version: None,
            exposed_capabilities: Vec::new(),
        }
    }
}

impl CapabilityDescriptor {
    /// 単純な構築ヘルパ。
    pub fn new(
        id: impl Into<CapabilityId>,
        stage: StageId,
        effect_scope: Vec<String>,
        provider: CapabilityProvider,
    ) -> Self {
        Self {
            id: id.into(),
            stage,
            effect_scope,
            provider,
            manifest_path: None,
            last_verified_at: None,
        }
    }
}
