use insta::assert_yaml_snapshot;
use serde_json::json;

use reml_runtime_ffi::core_prelude::{
    collectors::{MapCollector, VecCollector},
    iter::{BufferStrategy, EffectLabels, Iter, IteratorStageSnapshot, TryCollectError},
    Collector, GuardDiagnostic, IntoDiagnostic,
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

fn effect_case(case: &str, iter: Iter<i64>) -> serde_json::Value {
    let stage = iter.stage_snapshot(format!("core_iter_effects::{case}"));
    let effects = iter.effect_labels();
    json!({
        "case": case,
        "stage": render_stage(stage),
        "effects": render_effects(effects),
    })
}

fn pure_effects() -> serde_json::Value {
    effect_case("pure_effects", Iter::from_list(vec![1, 2, 3]))
}

fn buffered_effects() -> serde_json::Value {
    let iter = Iter::from_list(vec![1, 2, 3, 4]).buffered(3, BufferStrategy::DropOldest);
    effect_case("buffered_effects", iter)
}

fn try_unfold_effects() -> serde_json::Value {
    let iter = Iter::try_unfold(0, |state| -> Result<Option<(i64, i64)>, ()> {
        if state >= 2 {
            Ok(None)
        } else {
            Ok(Some((state, state + 1)))
        }
    });
    effect_case("try_unfold_effects", iter)
}

#[test]
fn core_iter_effect_labels_snapshot() {
    let cases = vec![pure_effects(), buffered_effects(), try_unfold_effects()];
    assert_yaml_snapshot!("core_iter_effect_labels", cases);
}

fn try_collect_error_case(case: &str, diag: serde_json::Value) -> serde_json::Value {
    json!({
        "case": case,
        "error": diag,
    })
}

fn guard_to_json(diag: GuardDiagnostic) -> serde_json::Value {
    diag.into_json()
}

fn try_collect_item_error() -> serde_json::Value {
    let iter: Iter<Result<i64, &'static str>> = Iter::from_list(vec![Ok(1), Err("boom"), Ok(3)]);
    match iter
        .try_collect(VecCollector::new())
        .expect_err("should capture item error")
    {
        TryCollectError::Item(err) => {
            try_collect_error_case("item_error", json!({ "kind": "item", "value": err }))
        }
        other => panic!("unexpected error variant: {:?}", other),
    }
}

fn try_collect_collector_error() -> serde_json::Value {
    let iter: Iter<Result<(String, i32), &'static str>> =
        Iter::from_list(vec![Ok(("dup".to_string(), 1)), Ok(("dup".to_string(), 2))]);
    match iter
        .try_collect(MapCollector::new())
        .expect_err("should capture collector error")
    {
        TryCollectError::Collector(err) => {
            let diag = err.into_diagnostic();
            try_collect_error_case("collector_error", guard_to_json(diag))
        }
        other => panic!("unexpected error variant: {:?}", other),
    }
}

#[test]
fn core_iter_try_collect_errors_snapshot() {
    let cases = vec![try_collect_item_error(), try_collect_collector_error()];
    assert_yaml_snapshot!("core_iter_try_collect_errors", cases);
}
