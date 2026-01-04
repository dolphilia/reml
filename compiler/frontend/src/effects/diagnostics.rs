use crate::diagnostic::effects::ensure_object;
use crate::typeck::StageRequirement;
use serde::Serialize;
use serde_json::{json, Map, Value};

/// 効果診断で Stage/Capability 差分を伝搬するためのユーティリティ。
pub struct EffectDiagnostic;

#[derive(Debug, Clone, Serialize)]
pub struct CapabilityMismatch {
    capability: String,
    required: StageRequirement,
    actual: StageRequirement,
}

impl CapabilityMismatch {
    pub fn new(
        capability: impl Into<String>,
        required: StageRequirement,
        actual: StageRequirement,
    ) -> Self {
        Self {
            capability: capability.into(),
            required,
            actual,
        }
    }

    pub fn capability(&self) -> &str {
        &self.capability
    }

    pub fn required(&self) -> &StageRequirement {
        &self.required
    }

    pub fn actual(&self) -> &StageRequirement {
        &self.actual
    }

    pub fn required_label(&self) -> String {
        self.required.label()
    }

    pub fn actual_label(&self) -> String {
        self.actual.label()
    }
}

impl EffectDiagnostic {
    /// Stage/Capability 不一致情報を `extensions` と `audit_metadata` に埋め込む。
    pub fn apply_stage_violation(
        mismatch: &CapabilityMismatch,
        extensions: &mut Map<String, Value>,
        metadata: &mut Map<String, Value>,
    ) {
        let capability = mismatch.capability().to_string();
        let required_label = mismatch.required_label();
        let actual_label = mismatch.actual_label();

        let stage_difference = json!({
            "expected": required_label,
            "actual": actual_label,
        });

        let effects_entry = extensions
            .entry("effects".to_string())
            .or_insert_with(|| Value::Object(Map::new()));
        let effects_object = ensure_object(effects_entry);
        effects_object.insert("capability".to_string(), json!(capability.clone()));
        let stage_entry = effects_object
            .entry("stage".to_string())
            .or_insert_with(|| Value::Object(Map::new()));
        let stage_object = ensure_object(stage_entry);
        stage_object.insert("required".to_string(), json!(required_label.clone()));
        stage_object.insert("actual".to_string(), json!(actual_label.clone()));
        stage_object.insert("capability".to_string(), json!(capability.clone()));
        stage_object.insert("mismatch".to_string(), stage_difference.clone());

        extensions.insert("effect.capability".to_string(), json!(capability.clone()));
        extensions.insert(
            "effect.stage.required".to_string(),
            json!(required_label.clone()),
        );
        extensions.insert(
            "effect.stage.actual".to_string(),
            json!(actual_label.clone()),
        );

        metadata.insert("effect.capability".to_string(), json!(capability.clone()));
        metadata.insert(
            "effect.stage.required".to_string(),
            json!(required_label.clone()),
        );
        metadata.insert(
            "effect.stage.actual".to_string(),
            json!(actual_label.clone()),
        );
        metadata.insert("capability.id".to_string(), json!(capability.clone()));
        metadata.insert(
            "capability.expected_stage".to_string(),
            json!(required_label.clone()),
        );
        metadata.insert(
            "capability.actual_stage".to_string(),
            json!(actual_label.clone()),
        );
        metadata.insert("capability.mismatch".to_string(), stage_difference);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::typeck::StageId;

    fn requirement_exact(stage: StageId) -> StageRequirement {
        StageRequirement::Exact(stage)
    }

    #[test]
    fn stage_violation_applies_metadata() {
        let mismatch = CapabilityMismatch::new(
            "core.iterator.collect",
            requirement_exact(StageId::beta()),
            requirement_exact(StageId::stable()),
        );
        let mut extensions = Map::new();
        let mut metadata = Map::new();

        EffectDiagnostic::apply_stage_violation(&mismatch, &mut extensions, &mut metadata);

        let effects = extensions
            .get("effects")
            .and_then(|value| value.get("capability").cloned());
        assert_eq!(
            effects,
            Some(Value::String("core.iterator.collect".to_string()))
        );
        assert_eq!(
            extensions.get("effect.stage.required"),
            Some(&json!("beta"))
        );
        assert_eq!(
            metadata.get("capability.expected_stage"),
            Some(&json!("beta"))
        );
        assert!(metadata.contains_key("capability.mismatch"));
    }
}
