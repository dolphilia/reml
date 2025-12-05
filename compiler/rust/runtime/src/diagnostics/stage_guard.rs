use crate::{
    capability::registry::{CapabilityError, CapabilityRegistry},
    stage::{StageId, StageRequirement},
};

/// Metrics/Audit API が要求する Capability ID。
pub(crate) const METRIC_CAPABILITY_ID: &str = "metrics.emit";
/// Metrics API が要求する Stage 。
pub(crate) const METRIC_STAGE_REQUIREMENT: StageRequirement =
    StageRequirement::Exact(StageId::Stable);
const METRIC_REQUIRED_EFFECTS: [&str; 1] = ["audit"];

/// Metrics API で必須となる効果タグ。
pub(crate) fn metric_required_effects() -> Vec<String> {
    METRIC_REQUIRED_EFFECTS
        .iter()
        .map(|value| value.to_string())
        .collect()
}

/// `metrics.emit` Capability の Stage を検証した結果を保持するガード。
#[derive(Debug, Clone)]
pub struct MetricsStageGuard {
    requirement: StageRequirement,
    actual_stage: StageId,
    required_effects: Vec<String>,
}

impl MetricsStageGuard {
    /// 指定した Stage 要件で Capability を検証する。
    pub(crate) fn verify(
        requirement: StageRequirement,
        required_effects: &[String],
    ) -> Result<Self, CapabilityError> {
        let registry = CapabilityRegistry::registry();
        let actual_stage = registry.verify_capability_stage(
            METRIC_CAPABILITY_ID,
            requirement,
            required_effects,
        )?;
        Ok(Self {
            requirement,
            actual_stage,
            required_effects: required_effects.to_vec(),
        })
    }

    /// 検証時に使用した Stage 要件。
    pub(crate) fn requirement(&self) -> StageRequirement {
        self.requirement
    }

    /// 実際に許可された Stage。
    pub(crate) fn actual_stage(&self) -> StageId {
        self.actual_stage
    }

    /// 必須効果タグ。
    pub(crate) fn required_effects(&self) -> &[String] {
        &self.required_effects
    }
}
