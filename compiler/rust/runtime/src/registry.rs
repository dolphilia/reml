use crate::stage::{StageId, StageRequirement};
use std::fmt;

/// Capability を検証するための簡易レジストリ。
#[derive(Debug, Clone)]
pub struct CapabilityRegistry;

impl CapabilityRegistry {
  pub fn registry() -> Self {
    Self
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
        format!(
          "required {:?} but runtime is {:?}",
          requirement, actual
        ),
      ))
    }
  }
}

/// Capability 検証に失敗した場合のエラー。
#[derive(Debug, Clone)]
pub struct CapabilityError {
  code: &'static str,
  detail: String,
}

impl CapabilityError {
  pub fn new(code: &'static str, detail: impl Into<String>) -> Self {
    Self {
      code,
      detail: detail.into(),
    }
  }

  pub fn code(&self) -> &'static str {
    self.code
  }

  pub fn detail(&self) -> &str {
    &self.detail
  }
}

impl fmt::Display for CapabilityError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}: {}", self.code, self.detail)
  }
}

impl std::error::Error for CapabilityError {}
