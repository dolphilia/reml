use std::{fmt, time::SystemTime};

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
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum StageId {
    Experimental,
    Beta,
    Stable,
}

impl fmt::Display for StageId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            StageId::Experimental => "experimental",
            StageId::Beta => "beta",
            StageId::Stable => "stable",
        };
        write!(f, "{}", label)
    }
}

/// Stage 要件。Exact/AtLeast をサポート。
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
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
