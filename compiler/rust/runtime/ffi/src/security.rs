use crate::{
    audit::{AuditContext, AuditError, AuditSink},
    capability_handle::SecurityCapability,
    capability_metadata::{CapabilityDescriptor, StageId, StageRequirement},
};
use serde_json::json;
use std::fmt;

/// セキュリティポリシー。
#[derive(Debug, Clone)]
pub struct SecurityPolicy {
    pub required_stage: StageRequirement,
    pub required_effects: Vec<String>,
}

impl SecurityPolicy {
    pub fn new(required_stage: StageRequirement, required_effects: Vec<String>) -> Self {
        Self {
            required_stage,
            required_effects,
        }
    }

    /// Capability の descriptor を元に検証を行う。
    pub fn verify(&self, descriptor: &CapabilityDescriptor) -> Result<(), SecurityError> {
        if !self.required_stage.matches(descriptor.stage) {
            return Err(SecurityError::StageViolation {
                required: self.required_stage,
                actual: descriptor.stage,
                capability: descriptor.id.clone(),
            });
        }

        let missing: Vec<String> = self
            .required_effects
            .iter()
            .filter(|effect| !descriptor.effect_scope.contains(effect))
            .cloned()
            .collect();
        if !missing.is_empty() {
            return Err(SecurityError::EffectViolation {
                capability: descriptor.id.clone(),
                missing,
            });
        }

        Ok(())
    }
}

impl SecurityCapability {
    pub fn enforce(&self, policy: &SecurityPolicy) -> Result<(), SecurityError> {
        policy.verify(&self.descriptor)
    }
}

/// FFI 呼び出しのオプション。
#[derive(Debug, Clone)]
pub struct CallOptions {
    pub audit_sink: AuditSink,
    pub security_policy: SecurityPolicy,
    pub stage_requirement: StageRequirement,
}

impl CallOptions {
    pub fn new(
        audit_sink: AuditSink,
        security_policy: SecurityPolicy,
        stage_requirement: StageRequirement,
    ) -> Self {
        Self {
            audit_sink,
            security_policy,
            stage_requirement,
        }
    }

    pub fn new_context(&self, symbol: &str) -> Result<AuditContext, AuditError> {
        let stage_meta = json!({ "stage_requirement": self.stage_requirement.to_string() });
        let ctx = AuditContext::new("ffi", symbol, self.audit_sink.clone())?;
        Ok(ctx.with_metadata(stage_meta.as_object().cloned().unwrap_or_default()))
    }
}

/// セキュリティ違反エラー。
#[derive(Debug)]
pub enum SecurityError {
    StageViolation {
        required: StageRequirement,
        actual: StageId,
        capability: String,
    },
    EffectViolation {
        capability: String,
        missing: Vec<String>,
    },
}

impl fmt::Display for SecurityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SecurityError::StageViolation {
                required,
                actual,
                capability,
            } => write!(
                f,
                "SecurityPolicy: '{}' は stage {} を要求していますが、実際は {} です",
                capability, required, actual
            ),
            SecurityError::EffectViolation {
                capability,
                missing,
            } => write!(
                f,
                "SecurityPolicy: '{}' に required effects {} がありません",
                capability,
                missing.join(", ")
            ),
        }
    }
}

impl std::error::Error for SecurityError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability_metadata::{CapabilityDescriptor, CapabilityProvider, StageId};

    #[test]
    fn security_policy_effect_missing() {
        let desc = CapabilityDescriptor::new(
            "ffi.cap",
            StageId::Stable,
            vec!["ffi".into()],
            CapabilityProvider::Core,
        );
        let policy = SecurityPolicy::new(
            StageRequirement::Exact(StageId::Stable),
            vec!["audit".into(), "ffi".into()],
        );
        let err = policy
            .verify(&desc)
            .expect_err("missing effect を検出できるはず");
        assert!(matches!(err, SecurityError::EffectViolation { .. }));
    }

    #[test]
    fn call_options_context_metadata() {
        let sink = AuditSink::new();
        let policy =
            SecurityPolicy::new(StageRequirement::AtLeast(StageId::Beta), vec!["ffi".into()]);
        let options = CallOptions::new(
            sink.clone(),
            policy,
            StageRequirement::AtLeast(StageId::Beta),
        );
        let ctx = options.new_context("symbol-name").unwrap();
        assert!(ctx.log("test", json!({"ok": true})).is_ok());
        let entries = sink.entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].metadata["stage_requirement"], "at_least(beta)");
    }
}
