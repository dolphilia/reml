use reml_frontend::diagnostic::effects;
use reml_frontend::typeck::{RuntimeCapability, StageContext, StageId, StageRequirement};
use serde_json::Value;

fn build_effect_context() -> effects::EffectAuditContext {
    let stage_context = StageContext {
        runtime: StageRequirement::AtLeast(StageId::stable()),
        capability: StageRequirement::AtLeast(StageId::beta()),
    };
    let capabilities = vec![RuntimeCapability::new("core.iterator.collect", StageId::beta())];
    effects::EffectAuditContext::from_stage_context(&stage_context, &capabilities)
}

#[test]
fn effect_audit_metadata_exports_contract_keys() {
    let context = build_effect_context();
    let mut metadata = serde_json::Map::new();
    effects::apply_audit_metadata(&context, &mut metadata);

    assert_eq!(
        metadata
            .get("effects.contract.stage.required")
            .and_then(Value::as_str),
        Some("at_least:beta")
    );
    assert_eq!(
        metadata
            .get("effects.contract.stage.actual")
            .and_then(Value::as_str),
        Some("at_least:stable")
    );
    assert_eq!(
        metadata
            .get("effects.contract.capability")
            .and_then(Value::as_str),
        Some("core.iterator.collect")
    );
    let trace = metadata
        .get("effects.contract.stage_trace")
        .and_then(Value::as_array)
        .expect("stage_trace should exist");
    assert!(trace
        .iter()
        .any(|entry| entry.get("source").and_then(Value::as_str) == Some("runtime_capability")));
}
