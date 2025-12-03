use reml_runtime::audit::{AuditEnvelope, AuditEvent};
use serde_json::{Map, Number, Value};
use std::{fs, path::PathBuf};
use uuid::Uuid;

fn expected_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/expected/audit")
        .join(name)
}

fn load_expected(name: &str) -> Value {
    let path = expected_path(name);
    let text =
        fs::read_to_string(&path).unwrap_or_else(|err| panic!("failed to read {path:?}: {err}"));
    serde_json::from_str(&text)
        .unwrap_or_else(|err| panic!("failed to parse expected JSON {path:?}: {err}"))
}

fn snapshot_event(redacted: bool) -> Value {
    let mut metadata = Map::new();
    metadata.insert("schema.version".into(), Value::String("3.0.0-alpha".into()));
    metadata.insert(
        "event.kind".into(),
        Value::String("pipeline_completed".into()),
    );
    metadata.insert(
        "pipeline.id".into(),
        Value::String("pipeline::core_diagnostics".into()),
    );
    metadata.insert(
        "pipeline.dsl_id".into(),
        Value::String("core.diagnostics".into()),
    );
    metadata.insert("pipeline.node".into(), Value::String("cli".into()));
    metadata.insert("pipeline.outcome".into(), Value::String("success".into()));
    metadata.insert("pipeline.count".into(), Value::Number(Number::from(1_i64)));
    metadata.insert(
        "timestamp".into(),
        Value::String("2025-07-05T12:00:00Z".into()),
    );
    if redacted {
        metadata.insert("privacy.redacted".into(), Value::Bool(true));
    }
    let envelope = AuditEnvelope::from_parts(
        metadata,
        Some(Uuid::parse_str("00000000-0000-0000-0000-00000000aaaa").unwrap()),
        None,
        Some("core.diagnostics".into()),
    );
    let event = AuditEvent::new("2025-07-05T12:05:00Z", envelope);
    serde_json::to_value(event).expect("serialize audit event")
}

#[test]
fn audit_snapshot_without_privacy_flag_matches_fixture() {
    let actual = snapshot_event(false);
    assert_eq!(
        actual,
        load_expected("privacy_plain.json"),
        "privacy flagなしの監査イベントがフィクスチャと一致していません"
    );
}

#[test]
fn audit_snapshot_with_privacy_flag_matches_fixture() {
    let actual = snapshot_event(true);
    assert_eq!(
        actual,
        load_expected("privacy_redacted.json"),
        "privacy.redacted=true の監査イベントがフィクスチャと一致していません"
    );
}
