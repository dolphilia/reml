use insta::assert_yaml_snapshot;
use serde_json::json;

use reml_runtime_ffi::core_prelude::iter::Iter;

fn render_stage(
    stage: reml_runtime_ffi::core_prelude::iter::IteratorStageSnapshot,
) -> serde_json::Value {
    json!({
        "required": {
            "mode": stage.required.mode,
            "stage": stage.required.stage,
        },
        "actual": stage.actual,
        "capability": stage.capability,
        "kind": stage.kind,
        "source": stage.source,
    })
}

fn render_effects(labels: reml_runtime_ffi::core_prelude::iter::EffectLabels) -> serde_json::Value {
    json!({
        "mem": labels.mem,
        "mutating": labels.mutating,
        "debug": labels.debug,
        "async_pending": labels.async_pending,
        "audit": labels.audit,
        "mem_bytes": labels.mem_bytes,
        "predicate_calls": labels.predicate_calls,
    })
}

fn map_pipeline_case() -> serde_json::Value {
    let iter = Iter::from_list(vec![1, 2, 3, 4]).map(|value| value * 10);
    let (core_vec, _) = iter
        .clone()
        .collect_vec()
        .expect("VecCollector should not fail")
        .into_parts();
    let values = core_vec.into_inner();
    let stage = iter.stage_snapshot("iter.map_pipeline");
    let effects = iter.effect_labels();
    json!({
        "case": "map_pipeline",
        "stage": render_stage(stage),
        "effects": render_effects(effects),
        "value": values,
    })
}

fn filter_effect_case() -> serde_json::Value {
    let iter = Iter::from_list(vec![1, 2, 3, 4]).filter(|value| *value % 2 == 0);
    let (core_vec, _) = iter
        .clone()
        .collect_vec()
        .expect("VecCollector should not fail")
        .into_parts();
    let values = core_vec.into_inner();
    let stage = iter.stage_snapshot("iter.filter_effect");
    let effects = iter.effect_labels();
    json!({
        "case": "filter_effect",
        "stage": render_stage(stage),
        "effects": render_effects(effects),
        "value": values,
    })
}

fn map_filter_chain_panic_guard_case() -> serde_json::Value {
    let iter = Iter::from_list(vec![-1i64, 0, 1, 2])
        .map(|value| value.checked_sub(1).ok_or("underflow"))
        .filter(|result| result.is_ok())
        .map(|result| result.unwrap_or_default());
    let (core_vec, _) = iter
        .clone()
        .collect_vec()
        .expect("VecCollector should not fail")
        .into_parts();
    let values = core_vec.into_inner();
    let stage = iter.stage_snapshot("iter.map_filter_chain_panic_guard");
    let effects = iter.effect_labels();
    json!({
        "case": "map_filter_chain_panic_guard",
        "stage": render_stage(stage),
        "effects": render_effects(effects),
        "value": values,
    })
}

#[test]
fn core_iter_adapters_snapshot() {
    let cases = vec![
        map_pipeline_case(),
        filter_effect_case(),
        map_filter_chain_panic_guard_case(),
    ];
    assert_yaml_snapshot!("core_iter_adapters", cases);
}
