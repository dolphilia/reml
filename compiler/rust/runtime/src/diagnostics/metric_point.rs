use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Number, Value};

use crate::{
    capability::registry::CapabilityError,
    prelude::ensure::{DiagnosticSeverity, GuardDiagnostic},
    stage::{StageId, StageRequirement},
    time::{self, Duration, Timestamp},
};

use super::{
    audit_bridge::{metric_audit_metadata, stage_requirement_label},
    stage_guard::{
        metric_required_effects, MetricsStageGuard, METRIC_CAPABILITY_ID, METRIC_STAGE_REQUIREMENT,
    },
};

const METRIC_DOMAIN: &str = "runtime";
const METRIC_EMIT_DIAGNOSTIC_CODE: &str = "core.diagnostics.metric_emit_failed";

/// Core.Diagnostics で共有するメトリクスポイント。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MetricPoint {
    pub name: String,
    pub value: MetricValue,
    pub timestamp: Timestamp,
    #[serde(default)]
    pub tags: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audit_id: Option<String>,
}

impl MetricPoint {
    pub fn new(name: impl Into<String>, value: MetricValue) -> Self {
        let timestamp = time::now().unwrap_or_else(|_| Timestamp::unix_epoch());
        Self {
            name: name.into(),
            value,
            timestamp,
            tags: BTreeMap::new(),
            audit_id: None,
        }
    }

    pub fn with_timestamp(mut self, timestamp: Timestamp) -> Self {
        self.timestamp = timestamp;
        self
    }

    pub fn with_tag(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.tags.insert(key.into(), value.into());
        self
    }

    pub fn with_tags<I, K, V>(mut self, tags: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        for (key, value) in tags {
            self.tags.insert(key.into(), value.into());
        }
        self
    }

    pub fn with_audit_id(mut self, audit_id: impl Into<String>) -> Self {
        self.audit_id = Some(audit_id.into());
        self
    }

    fn into_record(self, guard: &MetricsStageGuard) -> MetricAuditRecord {
        let metadata = metric_audit_metadata(
            &self,
            guard.requirement(),
            guard.actual_stage(),
            guard.required_effects(),
        );
        MetricAuditRecord {
            metric: self,
            metadata,
        }
    }
}

/// メトリクス値。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum MetricValue {
    Float(f64),
    Int(i64),
    Duration(Duration),
    Timestamp(Timestamp),
}

impl MetricValue {
    pub(crate) fn kind_label(&self) -> &'static str {
        match self {
            MetricValue::Float(_) => "float",
            MetricValue::Int(_) => "int",
            MetricValue::Duration(_) => "duration",
            MetricValue::Timestamp(_) => "timestamp",
        }
    }

    pub(crate) fn metadata_value(&self) -> Value {
        match self {
            MetricValue::Float(value) => match Number::from_f64(*value) {
                Some(number) => Value::Number(number),
                None => Value::String(value.to_string()),
            },
            MetricValue::Int(value) => Value::Number(Number::from(*value)),
            MetricValue::Duration(duration) => json!({
                "seconds": duration.seconds(),
                "nanos": duration.nanos(),
            }),
            MetricValue::Timestamp(timestamp) => json!({
                "seconds": timestamp.seconds(),
                "nanos": timestamp.nanos(),
            }),
        }
    }
}

/// `MetricPoint` へ変換可能な型。
pub trait IntoMetricValue {
    fn into_metric_value(self) -> MetricValue;
}

impl IntoMetricValue for f64 {
    fn into_metric_value(self) -> MetricValue {
        MetricValue::Float(self)
    }
}

impl IntoMetricValue for f32 {
    fn into_metric_value(self) -> MetricValue {
        MetricValue::Float(self as f64)
    }
}

impl IntoMetricValue for i64 {
    fn into_metric_value(self) -> MetricValue {
        MetricValue::Int(self)
    }
}

impl IntoMetricValue for i32 {
    fn into_metric_value(self) -> MetricValue {
        MetricValue::Int(self as i64)
    }
}

impl IntoMetricValue for Duration {
    fn into_metric_value(self) -> MetricValue {
        MetricValue::Duration(self)
    }
}

impl IntoMetricValue for Timestamp {
    fn into_metric_value(self) -> MetricValue {
        MetricValue::Timestamp(self)
    }
}

impl IntoMetricValue for &Duration {
    fn into_metric_value(self) -> MetricValue {
        MetricValue::Duration(*self)
    }
}

impl IntoMetricValue for &Timestamp {
    fn into_metric_value(self) -> MetricValue {
        MetricValue::Timestamp(*self)
    }
}

/// 監査シンクへ送出するレコード。
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct MetricAuditRecord {
    pub metric: MetricPoint,
    #[serde(rename = "audit_metadata")]
    pub metadata: Map<String, Value>,
}

impl MetricAuditRecord {
    pub fn metadata(&self) -> &Map<String, Value> {
        &self.metadata
    }
}

/// `AuditSink` へ `MetricPoint` を送出する。
pub fn emit_metric<S>(metric: MetricPoint, sink: &mut S) -> Result<(), GuardDiagnostic>
where
    S: MetricAuditSink,
{
    let required_effects = metric_required_effects();
    let guard = match MetricsStageGuard::verify(METRIC_STAGE_REQUIREMENT, &required_effects) {
        Ok(guard) => guard,
        Err(err) => {
            return Err(stage_mismatch_diagnostic(
                &metric,
                METRIC_STAGE_REQUIREMENT,
                &required_effects,
                err,
            ))
        }
    };
    let record = metric.into_record(&guard);
    sink.emit_metric(&record)
}

/// `MetricPoint` を構築する。
pub fn metric_point(name: impl Into<String>, value: impl IntoMetricValue) -> MetricPoint {
    MetricPoint::new(name, value.into_metric_value())
}

/// `MetricAuditRecord` を受け取る監査シンク。
pub trait MetricAuditSink {
    fn emit_metric(&mut self, record: &MetricAuditRecord) -> Result<(), GuardDiagnostic>;
}

impl<F> MetricAuditSink for F
where
    F: FnMut(&MetricAuditRecord) -> Result<(), GuardDiagnostic>,
{
    fn emit_metric(&mut self, record: &MetricAuditRecord) -> Result<(), GuardDiagnostic> {
        self(record)
    }
}

/// `AuditSink` 側のデフォルト実装。
pub fn default_emit_sink(record: &MetricAuditRecord) -> Result<(), GuardDiagnostic> {
    let mut extensions = Map::new();
    extensions.insert(
        "metrics.emit".into(),
        Value::Object(record.metadata().clone()),
    );
    Err(GuardDiagnostic {
        code: METRIC_EMIT_DIAGNOSTIC_CODE,
        domain: METRIC_DOMAIN,
        severity: DiagnosticSeverity::Warning,
        message: format!("no AuditSink registered for metric {}", record.metric.name),
        extensions,
        audit_metadata: record.metadata().clone(),
    })
}

fn stage_mismatch_diagnostic(
    metric: &MetricPoint,
    requirement: StageRequirement,
    required_effects: &[String],
    err: CapabilityError,
) -> GuardDiagnostic {
    let actual_stage = err.actual_stage().unwrap_or(StageId::Experimental);
    let mut extensions = Map::new();
    extensions.insert(
        "effects.contract.capability".into(),
        Value::String(METRIC_CAPABILITY_ID.into()),
    );
    extensions.insert(
        "effects.contract.stage.required".into(),
        Value::String(stage_requirement_label(requirement)),
    );
    extensions.insert(
        "effects.contract.stage.actual".into(),
        Value::String(actual_stage.as_str().into()),
    );
    if !required_effects.is_empty() {
        extensions.insert(
            "effects.contract.required_effects".into(),
            Value::Array(
                required_effects
                    .iter()
                    .cloned()
                    .map(Value::String)
                    .collect(),
            ),
        );
    }
    extensions.insert(
        "effects.contract.detail".into(),
        Value::String(err.detail().to_string()),
    );

    let mut audit_metadata =
        metric_audit_metadata(metric, requirement, actual_stage, required_effects);
    audit_metadata.insert(
        "effects.contract.detail".into(),
        Value::String(err.detail().to_string()),
    );

    GuardDiagnostic {
        code: "effects.contract.stage_mismatch",
        domain: METRIC_DOMAIN,
        severity: DiagnosticSeverity::Error,
        message: format!(
            "metrics capability '{}' denied: {}",
            METRIC_CAPABILITY_ID,
            err.detail()
        ),
        extensions,
        audit_metadata,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::{MetricsStageGuard, METRIC_CAPABILITY_ID};

    fn sample_timestamp() -> Timestamp {
        Timestamp::from_parts(1_700_000_000, 123_000_000)
    }

    #[test]
    fn metric_point_builder_sets_tags() {
        let metric = metric_point("latency.mean", 12.5_f64)
            .with_timestamp(sample_timestamp())
            .with_tag("unit", "ms")
            .with_tag("component", "frontend");
        assert_eq!(metric.tags.len(), 2);
        assert_eq!(metric.tags.get("unit").unwrap(), "ms");
    }

    #[test]
    fn with_audit_id_sets_identifier() {
        let metric = metric_point("latency.p95", 99_i64).with_audit_id("audit-123");
        assert_eq!(metric.audit_id.as_deref(), Some("audit-123"));
    }

    #[test]
    fn emit_metric_populates_metadata() {
        let metric = metric_point("latency.mean", 14.0_f32)
            .with_timestamp(sample_timestamp())
            .with_tag("unit", "ms")
            .with_audit_id("audit-xyz");
        let mut captured = None;
        let mut sink = |record: &MetricAuditRecord| {
            captured = Some(record.clone());
            Ok(())
        };
        emit_metric(metric, &mut sink).expect("emit");
        let record = captured.expect("metric captured");
        assert_eq!(record.metric.name, "latency.mean");
        let required_stage = stage_requirement_label(METRIC_STAGE_REQUIREMENT);
        assert_eq!(
            record
                .metadata()
                .get("effect.capability")
                .and_then(Value::as_str),
            Some(METRIC_CAPABILITY_ID)
        );
        assert_eq!(
            record
                .metadata()
                .get("effect.stage.required")
                .and_then(Value::as_str),
            Some(required_stage.as_str())
        );
        assert_eq!(
            record
                .metadata()
                .get("effect.stage.actual")
                .and_then(Value::as_str),
            Some("stable")
        );
        assert!(
            record
                .metadata()
                .get("effect.required_effects")
                .and_then(Value::as_array)
                .map(|array| array.iter().any(|value| value == "audit"))
                .unwrap_or(false),
            "required effects should include audit"
        );
        assert_eq!(
            record
                .metadata()
                .get("metric_point.tag.unit")
                .and_then(Value::as_str),
            Some("ms")
        );
        assert_eq!(
            record
                .metadata()
                .get("metric_point.audit_id")
                .and_then(Value::as_str),
            Some("audit-xyz")
        );
    }

    #[test]
    fn stage_mismatch_produces_guard_diagnostic() {
        let metric = metric_point("latency.mean", 21.0_f64);
        let required_effects = metric_required_effects();
        let error =
            MetricsStageGuard::verify(StageRequirement::Exact(StageId::Beta), &required_effects)
                .expect_err("beta requirement should fail");
        let diagnostic = stage_mismatch_diagnostic(
            &metric,
            StageRequirement::Exact(StageId::Beta),
            &required_effects,
            error,
        );
        assert_eq!(diagnostic.code, "effects.contract.stage_mismatch");
        assert!(
            diagnostic
                .audit_metadata
                .get("metric_point.name")
                .and_then(Value::as_str)
                .is_some(),
            "metric metadata should be present"
        );
        assert_eq!(
            diagnostic
                .extensions
                .get("effects.contract.capability")
                .and_then(Value::as_str),
            Some(METRIC_CAPABILITY_ID)
        );
    }
}
