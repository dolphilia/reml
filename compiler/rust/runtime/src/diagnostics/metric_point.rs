use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Number, Value};

use crate::{
    prelude::ensure::{DiagnosticSeverity, GuardDiagnostic},
    time::{self, Duration, Timestamp},
};

use super::audit_bridge::metric_audit_metadata;

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

    fn into_record(self) -> MetricAuditRecord {
        let metadata = metric_audit_metadata(&self);
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
    let record = metric.into_record();
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::audit_bridge::METRIC_CAPABILITY_ID;

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
}
