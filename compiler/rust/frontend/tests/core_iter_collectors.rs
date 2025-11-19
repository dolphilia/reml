//! `ListCollector`/`VecCollector` の効果ログを固定するスナップショットテスト。

use serde_json::json;

use reml_runtime_ffi::core_prelude::collectors::{CollectOutcome, ListCollector, VecCollector};

fn render_snapshot<T>(
    outcome: CollectOutcome<T>,
    value_renderer: impl FnOnce(T) -> serde_json::Value,
) -> serde_json::Value {
    let (value, audit) = outcome.into_parts();
    let stage = &audit.stage;
    let stage_kind = stage.kind.clone();
    let stage_source = stage.source.clone();
    json!({
        "kind": audit.kind.as_str(),
        "stage": {
            "required": {
                "mode": stage.required.mode,
                "stage": stage.required.stage,
            },
            "actual": stage.actual,
            "capability": stage.capability,
            "kind": stage_kind,
            "source": stage_source,
        },
        "effects": {
            "mem": audit.effects.mem,
            "mutating": audit.effects.mutating,
            "debug": audit.effects.debug,
            "async_pending": audit.effects.async_pending,
        },
        "markers": {
            "mem_reservation": audit.markers.mem_reservation,
            "reserve": audit.markers.reserve,
            "finish": audit.markers.finish,
        },
        "value": value_renderer(value),
    })
}

fn collect_list_baseline() -> String {
    let mut collector = ListCollector::new();
    collector.push(1).unwrap();
    collector.push(2).unwrap();
    collector.push(3).unwrap();
    let snapshot = render_snapshot(collector.finish(), |list| json!(list.into_vec()));
    serde_json::to_string_pretty(&snapshot).unwrap()
}

fn collect_vec_mem_reservation() -> String {
    let mut collector = VecCollector::with_capacity(4);
    collector.push(10).unwrap();
    collector.push(20).unwrap();
    collector.reserve(2).unwrap();
    collector.push(30).unwrap();
    let snapshot = render_snapshot(collector.finish(), |vec| json!(vec));
    serde_json::to_string_pretty(&snapshot).unwrap()
}

#[test]
fn core_iter_collectors_snapshot() {
    let cases = vec![
        ("collect_list_baseline", collect_list_baseline()),
        ("collect_vec_mem_reservation", collect_vec_mem_reservation()),
    ];
    let actual = cases
        .into_iter()
        .map(|(name, value)| format!("{name}: {value}"))
        .collect::<Vec<_>>()
        .join("\n");

    const SNAPSHOT: &str = include_str!("__snapshots__/core_iter_collectors.snap");
    let expected = SNAPSHOT.trim_end_matches('\n');
    assert_eq!(
        actual, expected,
        "core_iter_collectors snap が変更されました"
    );
}
