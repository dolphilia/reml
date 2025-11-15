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
fn effect_extension_includes_contract_metadata() {
    let context = build_effect_context();
    let mut extensions = serde_json::Map::new();
    effects::apply_extensions(&context, &mut extensions);

    assert_eq!(
        extensions
            .get("effects.contract.stage.required")
            .and_then(Value::as_str),
        Some("at_least:beta")
    );
    assert_eq!(
        extensions
            .get("effects.contract.stage.actual")
            .and_then(Value::as_str),
        Some("at_least:stable")
    );
    assert_eq!(
        extensions
            .get("effects.contract.capability")
            .and_then(Value::as_str),
        Some("core.iterator.collect")
    );
    let trace = extensions
        .get("effects.contract.stage_trace")
        .and_then(Value::as_array)
        .expect("stage_trace should be an array");
    assert!(trace
        .iter()
        .any(|entry| entry.get("source").and_then(Value::as_str) == Some("cli_option")));
    assert!(trace
        .iter()
        .any(|entry| entry.get("source").and_then(Value::as_str) == Some("runtime")));
}
