use std::{collections::HashMap, fmt, sync::Mutex, time::SystemTime};

use once_cell::sync::OnceCell;

use crate::{
    capability_handle::CapabilityHandle,
    capability_metadata::{CapabilityDescriptor, CapabilityId, StageId, StageRequirement},
};

/// Registry 内の Capability を格納するシングルトン。
pub struct CapabilityRegistry {
    handles: Mutex<HashMap<CapabilityId, CapabilityHandle>>,
}

impl CapabilityRegistry {
    /// グローバルインスタンスを取得する。
    pub fn registry() -> &'static CapabilityRegistry {
        static INSTANCE: OnceCell<CapabilityRegistry> = OnceCell::new();
        INSTANCE.get_or_init(|| CapabilityRegistry {
            handles: Mutex::new(HashMap::new()),
        })
    }

    /// Descriptor を登録する。
    pub fn register(&self, handle: CapabilityHandle) -> Result<(), CapabilityError> {
        let id = handle.descriptor().id.clone();
        let mut lock = self
            .handles
            .lock()
            .expect("CapabilityRegistry mutex がロックできません");
        if lock.contains_key(&id) {
            return Err(CapabilityError::AlreadyRegistered { id });
        }
        lock.insert(id, handle);
        Ok(())
    }

    /// 登録済み Capability を取得する（クローン）。
    pub fn get(&self, id: &CapabilityId) -> Option<CapabilityDescriptor> {
        let lock = self
            .handles
            .lock()
            .expect("CapabilityRegistry mutex がロックできません");
        lock.get(id).map(|handle| handle.descriptor().clone())
    }

    /// Descriptor を返す。
    pub fn describe(&self, id: &CapabilityId) -> Option<CapabilityDescriptor> {
        self.get(id)
    }

    /// Stage 要件と効果スコープを検証し、ハンドルを返す。
    pub fn verify_capability_stage(
        &self,
        id: impl AsRef<str>,
        requirement: StageRequirement,
        required_effects: &[String],
    ) -> Result<CapabilityHandle, CapabilityError> {
        let mut lock = self
            .handles
            .lock()
            .expect("CapabilityRegistry mutex がロックできません");

        let key = id.as_ref();
        let requested_id = key.to_string();
        let handle = lock
            .get_mut(key)
            .ok_or_else(|| CapabilityError::MissingCapability {
                id: requested_id.clone(),
            })?;

        {
            let descriptor = handle.descriptor();
            if !requirement.matches(descriptor.stage) {
                return Err(CapabilityError::StageViolation {
                    id: descriptor.id.clone(),
                    required: requirement,
                    actual: descriptor.stage,
                });
            }

            let actual_scope = descriptor.effect_scope.clone();
            let missing_effects: Vec<String> = required_effects
                .iter()
                .filter(|effect| !actual_scope.contains(effect))
                .cloned()
                .collect();
            if !missing_effects.is_empty() {
                return Err(CapabilityError::EffectViolation {
                    id: descriptor.id.clone(),
                    required: required_effects.to_vec(),
                    missing: missing_effects,
                    actual_scope,
                });
            }
        }

        handle.descriptor_mut().last_verified_at = Some(SystemTime::now());
        Ok(handle.clone())
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
    EffectViolation {
        id: CapabilityId,
        required: Vec<String>,
        missing: Vec<String>,
        actual_scope: Vec<String>,
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
            CapabilityError::EffectViolation {
                id,
                missing,
                actual_scope,
                ..
            } => write!(
                f,
                "Capability '{}' に required effects {} が含まれていません (available={})",
                id,
                missing.join(", "),
                actual_scope.join(", ")
            ),
        }
    }
}

impl std::error::Error for CapabilityError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability_handle::CapabilityHandle;
    use crate::capability_metadata::{CapabilityDescriptor, CapabilityProvider, StageId};

    fn new_gc_handle(id: &str, stage: StageId) -> CapabilityHandle {
        CapabilityHandle::gc(CapabilityDescriptor::new(
            id,
            stage,
            vec!["ffi".into()],
            CapabilityProvider::Core,
        ))
    }

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
        let handle = new_gc_handle("ffi.capability", StageId::Beta);
        let id = handle.descriptor().id.clone();
        let _ = registry.handles.lock().expect("lock").remove(&id);

        registry
            .register(handle.clone())
            .expect("登録に失敗しました");

        let required_effects = vec!["ffi".into()];
        let verified = registry
            .verify_capability_stage(
                &id,
                StageRequirement::Exact(StageId::Beta),
                &required_effects,
            )
            .expect("stage 検証に失敗");
        assert_eq!(verified.descriptor().stage, StageId::Beta);
        assert!(verified.as_gc().is_some());
    }

    #[test]
    fn stage_violation_error() {
        let registry = CapabilityRegistry::registry();
        let handle = new_gc_handle("ffi.stage-test", StageId::Experimental);
        let id = handle.descriptor().id.clone();
        let _ = registry.handles.lock().expect("lock").remove(&id);
        registry.register(handle).expect("登録失敗");

        let required_effects: &[String] = &[];
        let result = registry.verify_capability_stage(
            &id,
            StageRequirement::Exact(StageId::Beta),
            required_effects,
        );
        assert!(matches!(
            result,
            Err(CapabilityError::StageViolation { .. })
        ));
    }
    #[test]
    fn effect_violation_error() {
        let registry = CapabilityRegistry::registry();
        let handle = new_gc_handle("ffi.effect-test", StageId::Beta);
        let id = handle.descriptor().id.clone();
        let _ = registry.handles.lock().expect("lock").remove(&id);
        registry.register(handle).expect("登録失敗");

        let required_effects = vec!["audit".into()];
        let result = registry.verify_capability_stage(
            &id,
            StageRequirement::Exact(StageId::Beta),
            &required_effects,
        );
        assert!(matches!(
            result,
            Err(CapabilityError::EffectViolation { .. })
        ));
    }
}
