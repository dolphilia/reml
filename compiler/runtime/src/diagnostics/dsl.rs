use crate::parse::{InputPosition, Span};
use crate::prelude::ensure::GuardDiagnostic;
use serde_json::{Map, Value};

fn position_to_json(position: InputPosition) -> Value {
    let mut obj = Map::new();
    obj.insert("byte".into(), Value::from(position.byte as u64));
    obj.insert("line".into(), Value::from(position.line as u64));
    obj.insert("column".into(), Value::from(position.column as u64));
    Value::Object(obj)
}

fn span_to_json(span: Span) -> Value {
    let mut obj = Map::new();
    obj.insert("start".into(), position_to_json(span.start));
    obj.insert("end".into(), position_to_json(span.end));
    Value::Object(obj)
}

pub fn apply_dsl_metadata(
    diagnostic: &mut GuardDiagnostic,
    dsl_id: &str,
    parent_id: Option<&str>,
    span: Span,
) {
    let span_payload = span_to_json(span);
    diagnostic
        .extensions
        .insert("source_dsl".into(), Value::String(dsl_id.to_string()));
    let mut dsl_extension = Map::new();
    dsl_extension.insert("id".into(), Value::String(dsl_id.to_string()));
    dsl_extension.insert(
        "parent_id".into(),
        parent_id
            .map(|value| Value::String(value.to_string()))
            .unwrap_or(Value::Null),
    );
    dsl_extension.insert("embedding_span".into(), span_payload.clone());
    diagnostic
        .extensions
        .insert("dsl".into(), Value::Object(dsl_extension));

    diagnostic
        .audit_metadata
        .insert("dsl.id".into(), Value::String(dsl_id.to_string()));
    if let Some(parent_id) = parent_id {
        diagnostic
            .audit_metadata
            .insert("dsl.parent_id".into(), Value::String(parent_id.to_string()));
    }
    diagnostic
        .audit_metadata
        .insert("dsl.embedding.span".into(), span_payload);
}
