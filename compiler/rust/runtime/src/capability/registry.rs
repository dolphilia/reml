use std::fmt;

use once_cell::sync::OnceLock;

use crate::stage::{StageId, StageRequirement};

static REGISTRY: OnceLock<CapabilityRegistry> = OnceLock::new();

/// Capability を検証するためのレジストリ。
#[derive(Debug)]
pub struct CapabilityRegistry {
    _private: (),
}

impl CapabilityRegistry {
    /// シングルトンのレジストリを取得する。
    pub fn registry() -> &'static Self {
        REGISTRY.get_or_init(Self::new)
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
    REGISTRY.take();
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
