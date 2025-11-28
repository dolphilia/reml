//! MetricPoint 由来の監査メタデータを `AuditEnvelope.metadata` と
//! `Diagnostic.extensions` に統一フォーマットで書き込むためのヘルパ。
//!
//! `docs/spec/3-4-core-numeric-time.md` §4 と
//! `docs/spec/3-6-core-diagnostics-audit.md` §4 の要件を満たす形で
//! `metric_point.*` / `effect.*` キーを展開する。

use std::collections::BTreeMap;

use serde_json::{Map, Number, Value};

use crate::stage::{StageId, StageRequirement};

use super::{metric_point::MetricPoint, stage_guard::METRIC_CAPABILITY_ID};

/// 指定したメトリクス情報を `AuditEnvelope.metadata` へ転写する。
pub(crate) fn attach_audit(
    metadata: &mut Map<String, Value>,
    metric: &MetricPoint,
    stage_requirement: StageRequirement,
    actual_stage: StageId,
    required_effects: &[String],
) {
    metadata.insert(
        "metric_point.name".into(),
        Value::String(metric.name.clone()),
    );
    metadata.insert(
        "metric_point.kind".into(),
        Value::String(metric.value.kind_label().into()),
    );
    metadata.insert("metric_point.value".into(), metric.value.metadata_value());
    metadata.insert(
        "metric_point.timestamp.seconds".into(),
        Value::Number(Number::from(metric.timestamp.seconds())),
    );
    metadata.insert(
        "metric_point.timestamp.nanos".into(),
        Value::Number(Number::from(i64::from(metric.timestamp.nanos()))),
    );
    if let Some(audit_id) = metric.audit_id.as_ref() {
        metadata.insert(
            "metric_point.audit_id".into(),
            Value::String(audit_id.clone()),
        );
    }
    with_metric_tags(metadata, &metric.tags);
    metadata.insert(
        "effect.capability".into(),
        Value::String(METRIC_CAPABILITY_ID.into()),
    );
    metadata.insert(
        "effect.stage.required".into(),
        Value::String(stage_requirement_label(stage_requirement)),
    );
    metadata.insert(
        "effect.stage.actual".into(),
        Value::String(actual_stage.as_str().into()),
    );
    metadata.insert(
        "effect.required_capabilities".into(),
        Value::Array(vec![Value::String(METRIC_CAPABILITY_ID.into())]),
    );
    metadata.insert(
        "effect.actual_capabilities".into(),
        Value::Array(vec![Value::String(METRIC_CAPABILITY_ID.into())]),
    );
    if !required_effects.is_empty() {
        metadata.insert(
            "effect.required_effects".into(),
            Value::Array(
                required_effects
                    .iter()
                    .cloned()
                    .map(Value::String)
                    .collect(),
            ),
        );
    }
}

/// `metric_point.tag.*` と `metric_point.tags` の両方を整形する。
pub(crate) fn with_metric_tags(metadata: &mut Map<String, Value>, tags: &BTreeMap<String, String>) {
    if tags.is_empty() {
        return;
    }
    let mut object = Map::new();
    for (key, value) in tags {
        metadata.insert(
            format!("metric_point.tag.{key}"),
            Value::String(value.clone()),
        );
        object.insert(key.clone(), Value::String(value.clone()));
    }
    metadata.insert("metric_point.tags".into(), Value::Object(object));
}

/// `MetricPoint` を監査ログへ挿入する際のメタデータを生成する。
pub(crate) fn metric_audit_metadata(
    metric: &MetricPoint,
    stage_requirement: StageRequirement,
    actual_stage: StageId,
    required_effects: &[String],
) -> Map<String, Value> {
    let mut metadata = Map::new();
    attach_audit(
        &mut metadata,
        metric,
        stage_requirement,
        actual_stage,
        required_effects,
    );
    metadata
}

pub(crate) fn stage_requirement_label(requirement: StageRequirement) -> String {
    match requirement {
        StageRequirement::Exact(stage) => stage.as_str().into(),
        StageRequirement::AtLeast(stage) => format!("at_least {}", stage.as_str()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        diagnostics::metric_point,
        stage::{StageId, StageRequirement},
    };
    use serde_json::json;

    #[test]
    fn attach_audit_merges_with_existing_metadata() {
        let required_effects = vec!["audit".to_string()];
        let metric = metric_point::metric_point("latency.mean", 42.0_f64)
            .with_tag("unit", "ms")
            .with_audit_id("audit-latency");
        let mut metadata = Map::new();
        metadata.insert("schema.version".into(), Value::String("3.0.0-alpha".into()));
        attach_audit(
            &mut metadata,
            &metric,
            StageRequirement::Exact(StageId::Stable),
            StageId::Stable,
            &required_effects,
        );
        assert_eq!(
            metadata
                .get("schema.version")
                .and_then(Value::as_str)
                .unwrap(),
            "3.0.0-alpha"
        );
        assert_eq!(
            metadata
                .get("metric_point.name")
                .and_then(Value::as_str)
                .unwrap(),
            "latency.mean"
        );
        assert_eq!(
            metadata
                .get("metric_point.tag.unit")
                .and_then(Value::as_str)
                .unwrap(),
            "ms"
        );
        assert_eq!(
            metadata
                .get("effect.capability")
                .and_then(Value::as_str)
                .unwrap(),
            METRIC_CAPABILITY_ID
        );
        assert_eq!(
            metadata
                .get("effect.required_effects")
                .and_then(Value::as_array)
                .map(|array| array.len()),
            Some(1)
        );
    }

    #[test]
    fn with_metric_tags_constructs_tag_object() {
        let mut tags = BTreeMap::new();
        tags.insert("env".into(), "ci".into());
        tags.insert("component".into(), "runtime".into());
        let mut metadata = Map::new();
        with_metric_tags(&mut metadata, &tags);
        let tag_object = metadata
            .get("metric_point.tags")
            .and_then(Value::as_object)
            .expect("tags object");
        assert_eq!(tag_object.get("env"), Some(&Value::String("ci".into())));
        assert_eq!(
            tag_object.get("component"),
            Some(&Value::String("runtime".into()))
        );
        assert_eq!(metadata.get("metric_point.tag.env"), Some(&json!("ci")));
        assert_eq!(
            metadata.get("metric_point.tag.component"),
            Some(&json!("runtime"))
        );
    }
}
