use insta::assert_yaml_snapshot;
use serde_json::json;

use reml_runtime_ffi::core_prelude::{
    collectors::{CollectorAuditTrail, VecCollector},
    iter::{BufferStrategy, EffectLabels, Iter, IteratorStageSnapshot},
    Collector,
};

fn render_stage(stage: IteratorStageSnapshot) -> serde_json::Value {
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

fn render_effects(labels: EffectLabels) -> serde_json::Value {
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

fn render_collector(audit: &CollectorAuditTrail) -> serde_json::Value {
    json!({
        "kind": audit.kind.as_str(),
        "stage": {
            "required": {
                "mode": audit.stage.required.mode,
                "stage": audit.stage.required.stage,
            },
            "actual": audit.stage.actual,
            "capability": audit.stage.capability,
            "kind": audit.stage.kind,
            "source": audit.stage.source,
        },
        "effects": {
            "mem": audit.effects.mem,
            "mutating": audit.effects.mutating,
            "debug": audit.effects.debug,
            "async_pending": audit.effects.async_pending,
            "audit": audit.effects.audit,
            "predicate_calls": audit.effects.predicate_calls,
            "mem_bytes": audit.effects.mem_bytes,
        },
        "markers": {
            "mem_reservation": audit.markers.mem_reservation,
            "reserve": audit.markers.reserve,
            "finish": audit.markers.finish,
        },
    })
}

fn render_list_case(case: &str, iter: Iter<i64>) -> serde_json::Value {
    let outcome = iter
        .clone()
        .collect_list()
        .expect("ListCollector should not fail");
    let stage = iter.stage_snapshot(format!("core_iter_pipeline::{case}"));
    let effects = iter.effect_labels();
    let (list, audit) = outcome.into_parts();
    json!({
        "case": case,
        "stage": render_stage(stage),
        "effects": render_effects(effects),
        "collector": render_collector(&audit),
        "value": list.into_vec(),
    })
}

fn render_vec_case<T>(
    case: &str,
    iter: Iter<T>,
    mut value_renderer: impl FnMut(Vec<T>) -> serde_json::Value,
) -> serde_json::Value {
    let outcome = iter
        .clone()
        .collect_vec()
        .expect("VecCollector should not fail");
    let stage = iter.stage_snapshot(format!("core_iter_pipeline::{case}"));
    let effects = iter.effect_labels();
    let (vec, audit) = outcome.into_parts();
    let values = vec.into_inner();
    json!({
        "case": case,
        "stage": render_stage(stage),
        "effects": render_effects(effects),
        "collector": render_collector(&audit),
        "value": value_renderer(values),
    })
}

fn list_roundtrip() -> serde_json::Value {
    render_list_case("list_roundtrip", Iter::from_list(vec![1, 2, 3, 4]))
}

fn map_filter_vec() -> serde_json::Value {
    let iter = Iter::from_list(vec![1, 2, 3, 4])
        .map(|value| value * 10)
        .filter(|value| *value >= 20);
    render_vec_case("map_filter_vec", iter, |vec| json!(vec))
}

fn zip_collect_list() -> serde_json::Value {
    let colors = vec!["red".to_string(), "green".to_string(), "blue".to_string()];
    let iter = Iter::from_list(vec![1, 2, 3]).zip(Iter::from_list(colors));
    render_vec_case("zip_collect_list", iter, |vec| json!(vec))
}

fn buffered_mem_case() -> serde_json::Value {
    let iter = Iter::from_list(vec![0, 1, 2, 3, 4]).buffered(2, BufferStrategy::DropOldest);
    render_vec_case("buffered_mem_case", iter, |vec| json!(vec))
}

fn from_iter_and_into_iter() -> serde_json::Value {
    let iter: Iter<i64> = (10..13).collect();
    let stage = iter.stage_snapshot("core_iter_pipeline::from_iter_and_into_iter".to_string());
    let effects = iter.effect_labels();
    let outcome = iter.collect_list().expect("ListCollector should not fail");
    let (list, audit) = outcome.into_parts();

    let iter_into: Iter<i64> = (20..23).collect();
    let std_values: Vec<i64> = iter_into.into_iter().collect();

    json!({
        "case": "from_iter_and_into_iter",
        "stage": render_stage(stage),
        "effects": render_effects(effects),
        "collector": render_collector(&audit),
        "value": list.into_vec(),
        "std_into_iter": std_values,
    })
}

fn try_collect_success() -> serde_json::Value {
    let iter: Iter<Result<i64, &'static str>> = Iter::from_list(vec![Ok(1), Ok(2), Ok(3), Ok(4)]);
    let stage = iter.stage_snapshot("core_iter_pipeline::try_collect_success".to_string());
    let effects = iter.effect_labels();
    let audit_outcome = iter
        .clone()
        .map(|result| result.expect("試験用 clone は成功結果のみを想定"))
        .collect_vec()
        .expect("VecCollector should not fail for audit collection");
    let (_audit_vec, audit) = audit_outcome.into_parts();
    let values = iter
        .try_collect(VecCollector::new())
        .expect("try_collect should succeed")
        .into_inner();
    json!({
        "case": "try_collect_success",
        "stage": render_stage(stage),
        "effects": render_effects(effects),
        "collector": render_collector(&audit),
        "value": values,
    })
}

#[test]
fn core_iter_pipeline_snapshot() {
    let cases = vec![
        list_roundtrip(),
        map_filter_vec(),
        zip_collect_list(),
        buffered_mem_case(),
        from_iter_and_into_iter(),
        try_collect_success(),
    ];
    assert_yaml_snapshot!("core_iter_pipeline", cases);
}
