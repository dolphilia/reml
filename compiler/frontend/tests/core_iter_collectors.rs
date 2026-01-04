//! `ListCollector`/`VecCollector` の効果ログを固定するスナップショットテスト。

use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};
use uuid::Uuid;

use reml_frontend::pipeline::{AuditEmitter, PipelineDescriptor};
use reml_runtime::collections::persistent::btree::PersistentMap;
use reml_runtime::config::{
    ConfigCompatibility, ConfigCompatibilitySource, ConfigFormat, ResolvedConfigCompatibility,
};
use reml_runtime_ffi::core_prelude::{
    collectors::{
        CollectErrorKind, CollectOutcome, CollectorAuditTrail, CollectorEffectMarkers,
        CollectorKind, CollectorStageProfile, ListCollector, MapCollector, SetCollector,
        StringCollector, TableCollector, VecCollector,
    },
    iter::{EffectLabels, Iter},
    Collector, GuardDiagnostic, IntoDiagnostic,
};

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
            "audit": audit.effects.audit,
            "cell": audit.effects.cell,
            "rc": audit.effects.rc,
            "rc_ops": audit.effects.rc_ops,
            "unicode": audit.effects.unicode,
            "io": audit.effects.io,
            "io_blocking": audit.effects.io_blocking,
            "io_async": audit.effects.io_async,
            "security": audit.effects.security,
            "transfer": audit.effects.transfer,
            "fs_sync": audit.effects.fs_sync,
            "time": audit.effects.time,
            "time_calls": audit.effects.time_calls,
            "io_blocking_calls": audit.effects.io_blocking_calls,
            "io_async_calls": audit.effects.io_async_calls,
            "fs_sync_calls": audit.effects.fs_sync_calls,
            "security_events": audit.effects.security_events,
            "predicate_calls": audit.effects.predicate_calls,
            "mem_bytes": audit.effects.mem_bytes,
        },
        "markers": {
            "mem_reservation": audit.markers.mem_reservation,
            "reserve": audit.markers.reserve,
            "finish": audit.markers.finish,
            "cell_mutations": audit.markers.cell_mutations,
            "time_calls": audit.effects.time_calls,
            "io_blocking_ops": audit.markers.io_blocking_ops,
            "io_async_ops": audit.markers.io_async_ops,
            "security_checks": audit.markers.security_checks,
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
    let snapshot = render_snapshot(collector.finish(), |vec| json!(vec.into_inner()));
    serde_json::to_string_pretty(&snapshot).unwrap()
}

fn render_error_snapshot(diag: GuardDiagnostic) -> serde_json::Value {
    diag.into_json()
}

fn collect_map_duplicate() -> String {
    let mut collector = MapCollector::new();
    collector.push(("dup".to_string(), 1)).unwrap();
    let err = collector.push(("dup".to_string(), 2)).unwrap_err();
    let diag = err.into_diagnostic();
    serde_json::to_string_pretty(&render_error_snapshot(diag)).unwrap()
}

fn collect_set_stage() -> String {
    let mut collector = SetCollector::new();
    collector.push(5).unwrap();
    collector.push(1).unwrap();
    collector.push(3).unwrap();
    let snapshot = render_snapshot(collector.finish(), |set| {
        json!(set.into_set().into_iter().collect::<Vec<_>>())
    });
    serde_json::to_string_pretty(&snapshot).unwrap()
}

fn collect_string_invalid() -> String {
    let mut collector = StringCollector::new();
    collector.push(0xC3).unwrap();
    let err = collector.push(0x28).unwrap_err();
    let diag = err.into_diagnostic();
    serde_json::to_string_pretty(&render_error_snapshot(diag)).unwrap()
}

fn collect_table_baseline() -> String {
    let mut collector = TableCollector::new();
    collector.push(("first".to_string(), 10)).unwrap();
    collector.push(("second".to_string(), 20)).unwrap();
    let snapshot = render_snapshot(collector.finish(), |table| json!(table.into_entries()));
    serde_json::to_string_pretty(&snapshot).unwrap()
}

fn collect_table_duplicate() -> String {
    let mut collector = TableCollector::new();
    collector.push(("dup".to_string(), 1)).unwrap();
    let err = collector.push(("dup".to_string(), 2)).unwrap_err();
    let diag = err.into_diagnostic();
    serde_json::to_string_pretty(&render_error_snapshot(diag)).unwrap()
}

fn config_compat_audit_event() -> String {
    let descriptor = PipelineDescriptor::new(
        Path::new("tests/snapshots/config_compat/sample.reml"),
        Uuid::nil(),
        "parse",
        "cli",
        "reml_frontend",
        "reml_frontend --config-compat json-relaxed sample.reml",
        "3.0.0-alpha",
    );
    let resolved = ResolvedConfigCompatibility {
        format: ConfigFormat::Json,
        source: ConfigCompatibilitySource::Cli,
        profile_label: Some("json-relaxed".to_string()),
        compatibility: ConfigCompatibility::relaxed_json(),
    };
    let mut emitter = AuditEmitter::new(Vec::new(), true);
    emitter
        .config_compat_changed(&descriptor, &resolved)
        .expect("emit audit event");
    let buffer = emitter.into_inner().unwrap_or_default();
    let payload = String::from_utf8(buffer).expect("utf-8 payload");
    let event_line = payload.lines().next().unwrap_or_default();
    let mut event: Value = serde_json::from_str(event_line).expect("audit json");
    if let Some(obj) = event.as_object_mut() {
        obj.insert("timestamp".to_string(), json!("2025-01-01T00:00:00Z"));
        if let Some(envelope) = obj.get_mut("envelope").and_then(Value::as_object_mut) {
            envelope.insert(
                "audit_id".to_string(),
                json!("00000000-0000-0000-0000-000000000000"),
            );
            if let Some(metadata) = envelope.get_mut("metadata").and_then(Value::as_object_mut) {
                metadata.insert("timestamp".to_string(), json!("2025-01-01T00:00:00Z"));
            }
        }
    }
    serde_json::to_string_pretty(&event).unwrap()
}

fn flatten_config_tree(value: &Value) -> BTreeMap<String, Value> {
    fn flatten_value(value: &Value, path: &str, out: &mut BTreeMap<String, Value>) {
        match value {
            Value::Object(entries) => {
                if entries.is_empty() {
                    let key = if path.is_empty() { "$" } else { path };
                    out.insert(key.to_string(), Value::Object(serde_json::Map::new()));
                }
                for (key, child) in entries {
                    let next = if path.is_empty() {
                        key.clone()
                    } else {
                        format!("{path}.{key}")
                    };
                    flatten_value(child, &next, out);
                }
            }
            Value::Array(items) => {
                if items.is_empty() {
                    let key = if path.is_empty() { "$" } else { path };
                    out.insert(key.to_string(), Value::Array(vec![]));
                    return;
                }
                for (index, child) in items.iter().enumerate() {
                    let next = if path.is_empty() {
                        format!("[{index}]")
                    } else {
                        format!("{path}[{index}]")
                    };
                    flatten_value(child, &next, out);
                }
            }
            _ => {
                let key = if path.is_empty() { "$" } else { path };
                out.insert(key.to_string(), value.clone());
            }
        }
    }

    let mut flattened = BTreeMap::new();
    flatten_value(value, "", &mut flattened);
    flattened
}

fn config_diff_report() -> String {
    let base: Value = serde_json::from_str(include_str!(
        "../../../examples/core_config/cli/config_old.json"
    ))
    .expect("config_old fixture");
    let target: Value = serde_json::from_str(include_str!(
        "../../../examples/core_config/cli/config_new.json"
    ))
    .expect("config_new fixture");
    let base_map = PersistentMap::from_map(flatten_config_tree(&base));
    let target_map = PersistentMap::from_map(flatten_config_tree(&target));
    let change_set = base_map
        .diff_change_set(&target_map)
        .expect("change_set diff");
    let summary = change_set.summary();
    let payload = json!({
        "kind": "config_diff",
        "summary": {
            "added": summary.added,
            "removed": summary.removed,
            "updated": summary.updated,
            "total": summary.total()
        },
        "change_set": change_set.to_value()
    });
    serde_json::to_string_pretty(&payload).unwrap()
}

fn custom_snapshot(
    kind: CollectorKind,
    source: &'static str,
    effects: EffectLabels,
    mut markers: CollectorEffectMarkers,
    value: Value,
) -> Value {
    markers.record_finish();
    let stage = CollectorStageProfile::for_kind(kind);
    debug_assert!(effects.cell || effects.rc || effects.audit || effects.mem || effects.mutating);
    let audit = CollectorAuditTrail::new(kind, stage.snapshot(source), effects, markers);
    debug_assert_eq!(effects.cell, audit.effects.cell, "cell effect mismatch");
    debug_assert_eq!(effects.rc, audit.effects.rc, "rc effect mismatch");
    debug_assert_eq!(effects.rc_ops, audit.effects.rc_ops, "rc ops mismatch");
    debug_assert_eq!(
        markers.cell_mutations, audit.markers.cell_mutations,
        "cell mutation markers mismatch"
    );
    render_snapshot(CollectOutcome::new(value, audit), |val| val)
}

fn collect_cell_ref_effects() -> String {
    let effects = EffectLabels {
        mem: false,
        mutating: true,
        debug: false,
        async_pending: false,
        audit: false,
        cell: true,
        rc: true,
        unicode: false,
        io: false,
        io_blocking: false,
        io_async: false,
        security: false,
        transfer: false,
        fs_sync: false,
        mem_bytes: 0,
        predicate_calls: 0,
        rc_ops: 2,
        time: false,
        time_calls: 0,
        io_blocking_calls: 0,
        io_async_calls: 0,
        fs_sync_calls: 0,
        security_events: 0,
    };
    let mut markers = CollectorEffectMarkers::default();
    markers.record_cell_op();
    markers.record_cell_op();
    let snapshot = custom_snapshot(
        CollectorKind::Custom("cell_ref"),
        "CellRefCollector::finish",
        effects,
        markers,
        json!({
            "cells": [
                {"id": "config", "value": 1},
                {"id": "tunable", "value": 2}
            ],
            "refs": [
                {"id": "shared_config", "state": "borrowed_mut"},
                {"id": "readonly_cache", "state": "borrowed"}
            ]
        }),
    );
    serde_json::to_string_pretty(&snapshot).unwrap()
}

fn table_csv_import() -> String {
    let effects = EffectLabels {
        mem: true,
        mutating: true,
        debug: false,
        async_pending: false,
        audit: true,
        cell: false,
        rc: false,
        unicode: false,
        io: true,
        io_blocking: false,
        io_async: false,
        security: false,
        transfer: false,
        fs_sync: false,
        mem_bytes: 128,
        predicate_calls: 0,
        rc_ops: 0,
        time: false,
        time_calls: 0,
        io_blocking_calls: 0,
        io_async_calls: 0,
        fs_sync_calls: 0,
        security_events: 0,
    };
    let markers = CollectorEffectMarkers::default();
    let mut snapshot = custom_snapshot(
        CollectorKind::Table,
        "TableCollector::csv_import",
        effects,
        markers,
        json!({
            "rows": [
                {"id": "alpha", "columns": ["alpha", 10]},
                {"id": "beta", "columns": ["beta", 20]},
                {"id": "gamma", "columns": ["gamma", 30]}
            ]
        }),
    );
    if let Some(obj) = snapshot.as_object_mut() {
        obj.insert(
            "metrics".into(),
            json!({
                "table": {
                    "insert_per_sec": 2048.0,
                    "insert_total": 2048,
                    "insert_duration_ms": 1000.0
                },
                "csv": {
                    "load_latency_ms": 120.0,
                    "rows": 2048
                }
            }),
        );
    }
    serde_json::to_string_pretty(&snapshot).unwrap()
}

#[test]
fn core_iter_collectors_snapshot() {
    let cases = vec![
        ("collect_list_baseline", collect_list_baseline()),
        ("collect_vec_mem_reservation", collect_vec_mem_reservation()),
        ("collect_map_duplicate", collect_map_duplicate()),
        ("collect_set_stage", collect_set_stage()),
        ("collect_string_invalid", collect_string_invalid()),
        ("collect_table_baseline", collect_table_baseline()),
        ("collect_table_duplicate", collect_table_duplicate()),
        ("collect_cell_ref_effects", collect_cell_ref_effects()),
        ("table_csv_import", table_csv_import()),
        ("config_compat_event", config_compat_audit_event()),
        ("config_diff_report", config_diff_report()),
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

#[test]
fn iter_collect_map_round_trip() {
    let iter = Iter::from_list(vec![("alpha".to_string(), 10), ("beta".to_string(), 20)]);
    let (map, _) = iter.collect_map().unwrap().into_parts();
    let converted: BTreeMap<String, i32> = map.into_map();
    assert_eq!(converted.get("alpha"), Some(&10));
    assert_eq!(converted.get("beta"), Some(&20));
}

#[test]
fn iter_collect_map_duplicate_key() {
    let iter = Iter::from_list(vec![("dup".to_string(), 1), ("dup".to_string(), 2)]);
    let err = iter.collect_map().unwrap_err();
    assert_eq!(err.kind(), &CollectErrorKind::DuplicateKey);
}

#[test]
fn iter_collect_set_round_trip() {
    let iter = Iter::from_list(vec![5, 1, 3]);
    let (set, _) = iter.collect_set().unwrap().into_parts();
    let converted: BTreeSet<i32> = set.into_set();
    assert_eq!(converted.len(), 3);
    assert!(converted.contains(&1));
    assert!(converted.contains(&5));
}

#[test]
fn iter_collect_set_duplicate_value() {
    let iter = Iter::from_list(vec![2, 2]);
    let err = iter.collect_set().unwrap_err();
    assert_eq!(err.kind(), &CollectErrorKind::DuplicateKey);
}
