use std::{
    collections::HashMap,
    fmt,
    sync::{Mutex, OnceLock},
    time::SystemTime,
};

use crate::{
    capability_handle::CapabilityHandle,
    capability_metadata::{
        CapabilityDescriptor, CapabilityId, CapabilityProvider, StageId, StageRequirement,
    },
};

/// Registry 内の Capability を格納するシングルトン。
pub struct CapabilityRegistry {
    handles: Mutex<HashMap<CapabilityId, CapabilityHandle>>,
}

impl CapabilityRegistry {
    /// グローバルインスタンスを取得する。
    pub fn registry() -> &'static CapabilityRegistry {
        static INSTANCE: OnceLock<CapabilityRegistry> = OnceLock::new();
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
        id: &CapabilityId,
        requirement: StageRequirement,
    ) -> Result<CapabilityHandle, CapabilityError> {
        let mut lock = self
            .handles
            .lock()
            .expect("CapabilityRegistry mutex がロックできません");

        let handle = lock
            .get_mut(id)
            .ok_or_else(|| CapabilityError::MissingCapability { id: id.clone() })?;

        if !requirement.matches(handle.descriptor().stage) {
            return Err(CapabilityError::StageViolation {
                id: handle.descriptor().id.clone(),
                required: requirement,
                actual: handle.descriptor().stage,
            });
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

        let verified = registry
            .verify_capability_stage(&id, StageRequirement::Exact(StageId::Beta))
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

        let result = registry.verify_capability_stage(&id, StageRequirement::Exact(StageId::Beta));
        assert!(matches!(
            result,
            Err(CapabilityError::StageViolation { .. })
        ));
    }
}
