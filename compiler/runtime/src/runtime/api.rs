use crate::{
    capability::registry::{CapabilityError, CapabilityRegistry},
    stage::{StageId, StageRequirement},
};

/// Capability 検証の結果を保持するガード。
#[derive(Debug, Clone, Copy)]
pub struct CapabilityGuard {
    capability: &'static str,
    requirement: StageRequirement,
    actual_stage: StageId,
}

impl CapabilityGuard {
    /// 検証した Capability ID。
    pub fn capability(&self) -> &'static str {
        self.capability
    }

    /// 要求された Stage 要件。
    pub fn requirement(&self) -> StageRequirement {
        self.requirement
    }

    /// 実際に許可された Stage。
    pub fn actual_stage(&self) -> StageId {
        self.actual_stage
    }

    /// Stage 要件を満たしているかを返す。
    pub fn satisfies(&self) -> bool {
        self.requirement.matches(self.actual_stage)
    }
}

/// `Core.Runtime` API から Capability を検証するエントリポイント。
pub fn guard_capability(
    capability: &'static str,
    requirement: StageRequirement,
    required_effects: &[&str],
) -> Result<CapabilityGuard, CapabilityError> {
    let effects = required_effects
        .iter()
        .map(|effect| effect.to_string())
        .collect::<Vec<_>>();
    guard_capability_inner(capability, requirement, effects)
}

/// 動的に構築した効果タグ集合を用いて Capability を検証する。
pub fn guard_capability_with_owned_effects(
    capability: &'static str,
    requirement: StageRequirement,
    required_effects: &[String],
) -> Result<CapabilityGuard, CapabilityError> {
    guard_capability_inner(capability, requirement, required_effects.to_vec())
}

/// Core.IO で使用する Capability を検証するヘルパ。
pub fn guard_io_capability(
    capability: &'static str,
    requirement: StageRequirement,
    required_effects: &[&str],
) -> Result<CapabilityGuard, CapabilityError> {
    guard_capability(capability, requirement, required_effects)
}

/// Core.Time で使用する Capability を検証するヘルパ。
pub fn guard_time_capability(
    capability: &'static str,
    requirement: StageRequirement,
    required_effects: &[&str],
) -> Result<CapabilityGuard, CapabilityError> {
    guard_capability(capability, requirement, required_effects)
}

/// Core.Async で使用する Capability を検証するヘルパ。
pub fn guard_async_capability(
    capability: &'static str,
    requirement: StageRequirement,
    required_effects: &[&str],
) -> Result<CapabilityGuard, CapabilityError> {
    guard_capability(capability, requirement, required_effects)
}

fn guard_capability_inner(
    capability: &'static str,
    requirement: StageRequirement,
    required_effects: Vec<String>,
) -> Result<CapabilityGuard, CapabilityError> {
    let registry = CapabilityRegistry::registry();
    registry
        .verify_capability_stage(capability, requirement, &required_effects)
        .map(|actual_stage| CapabilityGuard {
            capability,
            requirement,
            actual_stage,
        })
}
