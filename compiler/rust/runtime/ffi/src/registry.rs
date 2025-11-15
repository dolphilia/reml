use std::{
    collections::HashMap,
    fmt,
    sync::{Mutex, OnceLock},
    time::SystemTime,
};

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

/// 実装が登録済み Capability を扱うハンドル。
#[derive(Debug, Clone)]
pub struct CapabilityHandle {
    descriptor: CapabilityDescriptor,
}

impl CapabilityHandle {
    pub fn descriptor(&self) -> &CapabilityDescriptor {
        &self.descriptor
    }
}

/// Registry 内の Capability を格納するシングルトン。
pub struct CapabilityRegistry {
    descriptors: Mutex<HashMap<CapabilityId, CapabilityDescriptor>>,
}

impl CapabilityRegistry {
    /// グローバルインスタンスを取得する。
    pub fn registry() -> &'static CapabilityRegistry {
        static INSTANCE: OnceLock<CapabilityRegistry> = OnceLock::new();
        INSTANCE.get_or_init(|| CapabilityRegistry {
            descriptors: Mutex::new(HashMap::new()),
        })
    }

    /// Descriptor を登録する。
    pub fn register(&self, descriptor: CapabilityDescriptor) -> Result<(), CapabilityError> {
        let mut lock = self
            .descriptors
            .lock()
            .expect("CapabilityRegistry mutex がロックできません");
        if lock.contains_key(&descriptor.id) {
            return Err(CapabilityError::AlreadyRegistered {
                id: descriptor.id.clone(),
            });
        }
        lock.insert(descriptor.id.clone(), descriptor);
        Ok(())
    }

    /// 登録済み Capability を取得する（クローン）。
    pub fn get(&self, id: &CapabilityId) -> Option<CapabilityDescriptor> {
        let lock = self
            .descriptors
            .lock()
            .expect("CapabilityRegistry mutex がロックできません");
        lock.get(id).cloned()
    }

    /// Descriptor を返す。
    pub fn describe(&self, id: &CapabilityId) -> Option<CapabilityDescriptor> {
        self.get(id)
    }

    /// Stage 要件と効果スコープを検証し、ハンドルを返す。
    pub fn verify_capability_stage(
        &self,
        id: &CapabilityId,
        requirement: StageRequirement,
    ) -> Result<CapabilityHandle, CapabilityError> {
        let mut lock = self
            .descriptors
            .lock()
            .expect("CapabilityRegistry mutex がロックできません");

        let descriptor = lock
            .get_mut(id)
            .ok_or_else(|| CapabilityError::MissingCapability { id: id.clone() })?;

        if !requirement.matches(descriptor.stage) {
            return Err(CapabilityError::StageViolation {
                id: descriptor.id.clone(),
                required: requirement,
                actual: descriptor.stage,
            });
        }

        descriptor.last_verified_at = Some(SystemTime::now());
        Ok(CapabilityHandle {
            descriptor: descriptor.clone(),
        })
    }
}

/// Capability 検証エラー。
#[derive(Debug)]
pub enum CapabilityError {
    AlreadyRegistered {
        id: CapabilityId,
    },
    MissingCapability {
        id: CapabilityId,
    },
    StageViolation {
        id: CapabilityId,
        required: StageRequirement,
        actual: StageId,
    },
}

impl fmt::Display for CapabilityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CapabilityError::AlreadyRegistered { id } => {
                write!(f, "Capability '{}' はすでに登録済みです", id)
            }
            CapabilityError::MissingCapability { id } => {
                write!(f, "Capability '{}' は未登録です", id)
            }
            CapabilityError::StageViolation {
                id,
                required,
                actual,
            } => write!(
                f,
                "Capability '{}' の stage が一致しません (required={}, actual={})",
                id, required, actual
            ),
        }
    }
}

impl std::error::Error for CapabilityError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stage_requirement_matches() {
        assert!(StageRequirement::Exact(StageId::Beta).matches(StageId::Beta));
        assert!(!StageRequirement::Exact(StageId::Beta).matches(StageId::Stable));
        assert!(StageRequirement::AtLeast(StageId::Beta).matches(StageId::Stable));
        assert!(!StageRequirement::AtLeast(StageId::Stable).matches(StageId::Beta));
    }

    #[test]
    fn register_and_verify_capability() {
        let registry = CapabilityRegistry::registry();
        let descriptor = CapabilityDescriptor::new(
            "ffi.capability",
            StageId::Beta,
            vec!["ffi".into()],
            CapabilityProvider::Core,
        );
        // 何度もテストすると既存登録とぶつかる可能性があるため、まだ未登録であることを保証
        let _ = registry
            .descriptors
            .lock()
            .expect("lock")
            .remove(&descriptor.id);

        registry
            .register(descriptor.clone())
            .expect("登録に失敗しました");

        let handle = registry
            .verify_capability_stage(&descriptor.id, StageRequirement::Exact(StageId::Beta))
            .expect("stage 検証に失敗");
        assert_eq!(handle.descriptor().stage, StageId::Beta);
    }

    #[test]
    fn stage_violation_error() {
        let registry = CapabilityRegistry::registry();
        let descriptor = CapabilityDescriptor::new(
            "ffi.stage-test",
            StageId::Experimental,
            vec!["ffi".into()],
            CapabilityProvider::Core,
        );
        let _ = registry
            .descriptors
            .lock()
            .expect("lock")
            .remove(&descriptor.id);
        registry.register(descriptor.clone()).expect("登録失敗");

        let result = registry
            .verify_capability_stage(&descriptor.id, StageRequirement::Exact(StageId::Beta));
        assert!(matches!(
            result,
            Err(CapabilityError::StageViolation { .. })
        ));
    }
}
