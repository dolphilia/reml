use std::{
    fmt,
    sync::{RwLock, RwLockReadGuard},
};

use once_cell::sync::Lazy;

use crate::stage::{StageId, StageRequirement};

static REGISTRY: Lazy<RwLock<Option<&'static CapabilityRegistry>>> =
    Lazy::new(|| RwLock::new(None));

/// Capability を検証するためのレジストリ。
#[derive(Debug)]
pub struct CapabilityRegistry {
    _private: (),
}

impl CapabilityRegistry {
    /// シングルトンのレジストリを取得する。
    pub fn registry() -> &'static Self {
        if let Some(instance) = Self::try_get_cached(REGISTRY.read().unwrap()) {
            return instance;
        }
        let mut guard = REGISTRY.write().unwrap();
        if let Some(instance) = *guard {
            return instance;
        }
        let leaked: &'static CapabilityRegistry = Box::leak(Box::new(Self::new()));
        *guard = Some(leaked);
        leaked
    }

    fn try_get_cached(
        guard: RwLockReadGuard<'_, Option<&'static CapabilityRegistry>>,
    ) -> Option<&'static Self> {
        *guard
    }

    fn new() -> Self {
        Self { _private: () }
    }

    pub fn verify_capability_stage(
        &self,
        _capability: &str,
        requirement: StageRequirement,
        _required_effects: &[String],
    ) -> Result<StageId, CapabilityError> {
        // 現状の PoC ではすべての Capability が stable とみなされる。
        let actual = StageId::Stable;
        if requirement.matches(actual) {
            Ok(actual)
        } else {
            Err(CapabilityError::new(
                "capability.stage.mismatch",
                format!("required {:?} but runtime is {:?}", requirement, actual),
            )
            .with_actual_stage(actual))
        }
    }

    /// Core.IO アダプタ向けの Stage 検証ヘルパ。
    pub fn verify_stage_for_io(
        &self,
        capability: &'static str,
        requirement: StageRequirement,
    ) -> Result<StageId, CapabilityError> {
        self.verify_capability_stage(capability, requirement, &[])
    }
}

/// Capability 検証に失敗した場合のエラー。
#[derive(Debug, Clone)]
pub struct CapabilityError {
    code: &'static str,
    detail: String,
    actual_stage: Option<StageId>,
}

impl CapabilityError {
    pub fn new(code: &'static str, detail: impl Into<String>) -> Self {
        Self {
            code,
            detail: detail.into(),
            actual_stage: None,
        }
    }

    pub fn code(&self) -> &'static str {
        self.code
    }

    pub fn detail(&self) -> &str {
        &self.detail
    }

    pub fn actual_stage(&self) -> Option<StageId> {
        self.actual_stage
    }

    pub fn with_actual_stage(mut self, stage: StageId) -> Self {
        self.actual_stage = Some(stage);
        self
    }
}

impl fmt::Display for CapabilityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.detail)
    }
}

impl std::error::Error for CapabilityError {}

#[cfg(test)]
pub(crate) fn reset_for_tests() {
    if let Some(instance) = REGISTRY.write().unwrap().take() {
        unsafe {
            drop(Box::from_raw(
                instance as *const CapabilityRegistry as *mut CapabilityRegistry,
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_returns_same_instance() {
        let first = CapabilityRegistry::registry() as *const CapabilityRegistry;
        let second = CapabilityRegistry::registry() as *const CapabilityRegistry;
        assert_eq!(first, second);
    }

    #[test]
    fn reset_for_tests_clears_instance() {
        let first = CapabilityRegistry::registry() as *const CapabilityRegistry;
        reset_for_tests();
        let second = CapabilityRegistry::registry() as *const CapabilityRegistry;
        assert_ne!(first, second);
    }
}
