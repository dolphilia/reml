use serde_json::{Map, Number, Value};

use crate::prelude::ensure::{DiagnosticSeverity, GuardDiagnostic, IntoDiagnostic};

const NUMERIC_DIAGNOSTIC_DOMAIN: &str = "runtime";
const NUMERIC_DIAGNOSTIC_CODE_BASE: &str = "core.numeric.statistics";
const NUMERIC_PRECISION_CODE_BASE: &str = "core.numeric.precision";

/// Core.Numeric 統計 API で利用するエラー。
#[derive(Debug, Clone)]
pub struct StatisticsError {
    pub kind: StatisticsErrorKind,
    pub message: String,
    pub bucket_index: Option<usize>,
    pub bucket_label: Option<String>,
    pub violated_rule: Option<String>,
    pub value: Option<f64>,
    pub context_code: Option<&'static str>,
    pub column: Option<String>,
    pub aggregation: Option<String>,
    pub audit_id: Option<String>,
}

/// `data.stats.*` メタデータをまとめて管理するタグ。
#[derive(Debug, Clone, Default)]
pub struct StatisticsTags {
    pub column: Option<String>,
    pub aggregation: Option<String>,
    pub audit_id: Option<String>,
}

impl StatisticsTags {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn column(mut self, value: impl Into<String>) -> Self {
        self.column = Some(value.into());
        self
    }

    pub fn aggregation(mut self, value: impl Into<String>) -> Self {
        self.aggregation = Some(value.into());
        self
    }

    pub fn audit_id(mut self, value: impl Into<String>) -> Self {
        self.audit_id = Some(value.into());
        self
    }
}

impl StatisticsError {
    pub fn insufficient_data(message: impl Into<String>) -> Self {
        Self::new(StatisticsErrorKind::InsufficientData, message)
    }

    pub fn invalid_parameter(message: impl Into<String>) -> Self {
        Self::new(StatisticsErrorKind::InvalidParameter, message)
    }

    pub fn numerical_instability(message: impl Into<String>) -> Self {
        Self::new(StatisticsErrorKind::NumericalInstability, message)
    }

    fn new(kind: StatisticsErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            bucket_index: None,
            bucket_label: None,
            violated_rule: None,
            value: None,
            context_code: None,
            column: None,
            aggregation: None,
            audit_id: None,
        }
    }

    pub fn with_bucket_context(mut self, index: usize, label: impl Into<String>) -> Self {
        self.bucket_index = Some(index);
        self.bucket_label = Some(label.into());
        self
    }

    pub fn with_rule(mut self, rule: impl Into<String>) -> Self {
        self.violated_rule = Some(rule.into());
        self
    }

    pub fn with_value(mut self, value: f64) -> Self {
        self.value = Some(value);
        self
    }

    pub fn with_context_code(mut self, code: &'static str) -> Self {
        self.context_code = Some(code);
        self
    }

    pub fn with_column(mut self, column: impl Into<String>) -> Self {
        self.column = Some(column.into());
        self
    }

    pub fn with_aggregation(mut self, aggregation: impl Into<String>) -> Self {
        self.aggregation = Some(aggregation.into());
        self
    }

    pub fn with_audit_id(mut self, audit_id: impl Into<String>) -> Self {
        self.audit_id = Some(audit_id.into());
        self
    }

    pub fn with_tags(mut self, tags: StatisticsTags) -> Self {
        if let Some(column) = tags.column {
            self.column = Some(column);
        }
        if let Some(aggregation) = tags.aggregation {
            self.aggregation = Some(aggregation);
        }
        if let Some(audit_id) = tags.audit_id {
            self.audit_id = Some(audit_id);
        }
        self
    }
}

impl IntoDiagnostic for StatisticsError {
    fn into_diagnostic(self) -> GuardDiagnostic {
        let StatisticsError {
            kind,
            message,
            bucket_index,
            bucket_label,
            violated_rule,
            value,
            context_code,
            column,
            aggregation,
            audit_id,
        } = self;

        let code = context_code.unwrap_or_else(|| kind.default_code());

        let mut numeric_extensions = Map::new();
        numeric_extensions.insert("kind".into(), Value::String(kind.as_str().into()));
        if let Some(index) = bucket_index {
            numeric_extensions.insert("bucket_index".into(), Value::Number(Number::from(index)));
        }
        if let Some(label) = bucket_label.as_ref() {
            numeric_extensions.insert("bucket_label".into(), Value::String(label.clone()));
        }
        if let Some(rule) = violated_rule.as_ref() {
            numeric_extensions.insert("violated_rule".into(), Value::String(rule.clone()));
        }
        if let Some(value) = value {
            let encoded_value = encode_sample_value(value);
            numeric_extensions.insert("sample_value".into(), encoded_value.clone());
        }
        if let Some(column) = column.as_ref() {
            numeric_extensions.insert("column".into(), Value::String(column.clone()));
        }
        if let Some(aggregation) = aggregation.as_ref() {
            numeric_extensions.insert("aggregation".into(), Value::String(aggregation.clone()));
        }
        if let Some(audit_id) = audit_id.as_ref() {
            numeric_extensions.insert("audit_id".into(), Value::String(audit_id.clone()));
        }

        let mut data_stats_extensions = Map::new();
        if let Some(column) = column.as_ref() {
            data_stats_extensions.insert("column".into(), Value::String(column.clone()));
        }
        if let Some(aggregation) = aggregation.as_ref() {
            data_stats_extensions.insert("aggregation".into(), Value::String(aggregation.clone()));
        }
        if let Some(audit_id) = audit_id.as_ref() {
            data_stats_extensions.insert("audit_id".into(), Value::String(audit_id.clone()));
        }

        let mut extensions = Map::new();
        extensions.insert(
            "numeric.statistics".into(),
            Value::Object(numeric_extensions.clone()),
        );
        extensions.insert("message".into(), Value::String(message.clone()));
        if !data_stats_extensions.is_empty() {
            extensions.insert("data.stats".into(), Value::Object(data_stats_extensions));
        }

        let mut audit_metadata = Map::new();
        audit_metadata.insert(
            "numeric.statistics.kind".into(),
            Value::String(kind.as_str().into()),
        );
        if let Some(index) = bucket_index {
            audit_metadata.insert(
                "numeric.statistics.bucket_index".into(),
                Value::Number(Number::from(index)),
            );
        }
        if let Some(label) = bucket_label.as_ref() {
            audit_metadata.insert(
                "numeric.statistics.bucket_label".into(),
                Value::String(label.clone()),
            );
        }
        if let Some(rule) = violated_rule.as_ref() {
            audit_metadata.insert(
                "numeric.statistics.rule".into(),
                Value::String(rule.clone()),
            );
        }
        if let Some(value) = value {
            let encoded_value = encode_sample_value(value);
            audit_metadata.insert("numeric.statistics.sample_value".into(), encoded_value);
        }
        if let Some(column) = column.as_ref() {
            let column_value = Value::String(column.clone());
            audit_metadata.insert("numeric.statistics.column".into(), column_value.clone());
            audit_metadata.insert("data.stats.column".into(), column_value);
        }
        if let Some(aggregation) = aggregation.as_ref() {
            let aggregation_value = Value::String(aggregation.clone());
            audit_metadata.insert(
                "numeric.statistics.aggregation".into(),
                aggregation_value.clone(),
            );
            audit_metadata.insert("data.stats.aggregation".into(), aggregation_value);
        }
        if let Some(audit_id) = audit_id.as_ref() {
            let audit_value = Value::String(audit_id.clone());
            audit_metadata.insert("numeric.statistics.audit_id".into(), audit_value.clone());
            audit_metadata.insert("data.stats.audit_id".into(), audit_value);
        }

        GuardDiagnostic {
            code,
            domain: NUMERIC_DIAGNOSTIC_DOMAIN,
            severity: DiagnosticSeverity::Error,
            message: format!("{NUMERIC_DIAGNOSTIC_CODE_BASE}: {message}"),
            extensions,
            audit_metadata,
        }
    }
}

/// Core.Numeric 精度 API で利用するエラー。
#[derive(Debug, Clone)]
pub struct NumericError {
    pub kind: NumericErrorKind,
    pub message: String,
    pub precision_kind: Option<String>,
    pub precision_scale: Option<u8>,
    pub precision_digits: Option<u8>,
    pub value_repr: Option<String>,
    pub context_code: Option<&'static str>,
    pub currency_code: Option<String>,
    pub currency_scale: Option<u8>,
}

impl NumericError {
    pub fn invalid_precision(message: impl Into<String>) -> Self {
        Self::new(NumericErrorKind::InvalidPrecision, message)
    }

    pub fn unsupported_precision(message: impl Into<String>) -> Self {
        Self::new(NumericErrorKind::UnsupportedPrecision, message)
    }

    pub fn conversion_failed(message: impl Into<String>) -> Self {
        Self::new(NumericErrorKind::ConversionFailed, message)
    }

    pub fn precision_overflow(message: impl Into<String>) -> Self {
        Self::new(NumericErrorKind::PrecisionOverflow, message)
    }

    pub fn unsupported_currency(message: impl Into<String>) -> Self {
        Self::new(NumericErrorKind::UnsupportedCurrency, message)
    }

    fn new(kind: NumericErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            precision_kind: None,
            precision_scale: None,
            precision_digits: None,
            value_repr: None,
            context_code: None,
            currency_code: None,
            currency_scale: None,
        }
    }

    pub fn with_precision_kind(mut self, kind: impl Into<String>) -> Self {
        self.precision_kind = Some(kind.into());
        self
    }

    pub fn with_precision_scale(mut self, scale: u8) -> Self {
        self.precision_scale = Some(scale);
        self
    }

    pub fn with_precision_digits(mut self, digits: u8) -> Self {
        self.precision_digits = Some(digits);
        self
    }

    pub fn with_value_repr(mut self, value: impl Into<String>) -> Self {
        self.value_repr = Some(value.into());
        self
    }

    pub fn with_context_code(mut self, code: &'static str) -> Self {
        self.context_code = Some(code);
        self
    }

    pub fn with_currency_code(mut self, code: impl Into<String>) -> Self {
        self.currency_code = Some(code.into());
        self
    }

    pub fn with_currency_scale(mut self, scale: u8) -> Self {
        self.currency_scale = Some(scale);
        self
    }
}

impl IntoDiagnostic for NumericError {
    fn into_diagnostic(self) -> GuardDiagnostic {
        let NumericError {
            kind,
            message,
            precision_kind,
            precision_scale,
            precision_digits,
            value_repr,
            context_code,
            currency_code,
            currency_scale,
        } = self;

        let code = context_code.unwrap_or_else(|| kind.default_code());

        let mut precision_extensions = Map::new();
        precision_extensions.insert("error_kind".into(), Value::String(kind.as_str().into()));
        if let Some(kind_label) = precision_kind.as_ref() {
            precision_extensions.insert("precision_kind".into(), Value::String(kind_label.clone()));
        }
        if let Some(scale) = precision_scale {
            precision_extensions.insert("scale".into(), Value::Number(Number::from(scale)));
        }
        if let Some(digits) = precision_digits {
            precision_extensions.insert("precision".into(), Value::Number(Number::from(digits)));
        }
        if let Some(value) = value_repr.as_ref() {
            precision_extensions.insert("value".into(), Value::String(value.clone()));
        }

        let mut extensions = Map::new();
        let mut audit_metadata = Map::new();
        extensions.insert(
            "numeric.precision".into(),
            Value::Object(precision_extensions.clone()),
        );
        extensions.insert("message".into(), Value::String(message.clone()));

        if currency_code.is_some() || currency_scale.is_some() {
            let mut finance_extensions = Map::new();
            if let Some(code) = currency_code.as_ref() {
                finance_extensions.insert("currency_code".into(), Value::String(code.clone()));
            }
            if let Some(scale) = currency_scale {
                finance_extensions.insert("scale".into(), Value::Number(Number::from(scale)));
            }
            extensions.insert(
                "numeric.finance".into(),
                Value::Object(finance_extensions.clone()),
            );
            for (key, value) in finance_extensions.iter() {
                audit_metadata.insert(format!("numeric.finance.{key}"), value.clone());
            }
        }

        for (key, value) in precision_extensions.iter() {
            audit_metadata.insert(format!("numeric.precision.{key}"), value.clone());
        }

        GuardDiagnostic {
            code,
            domain: NUMERIC_DIAGNOSTIC_DOMAIN,
            severity: DiagnosticSeverity::Error,
            message: format!("{NUMERIC_PRECISION_CODE_BASE}: {message}"),
            extensions,
            audit_metadata,
        }
    }
}

fn encode_sample_value(value: f64) -> Value {
    if let Some(number) = Number::from_f64(value) {
        Value::Number(number)
    } else if value.is_nan() {
        Value::String("NaN".into())
    } else if value.is_infinite() {
        if value.is_sign_positive() {
            Value::String("Infinity".into())
        } else {
            Value::String("-Infinity".into())
        }
    } else {
        Value::String(value.to_string())
    }
}

/// 統計エラー種別。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatisticsErrorKind {
    InsufficientData,
    InvalidParameter,
    NumericalInstability,
}

impl StatisticsErrorKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            StatisticsErrorKind::InsufficientData => "insufficient_data",
            StatisticsErrorKind::InvalidParameter => "invalid_parameter",
            StatisticsErrorKind::NumericalInstability => "numerical_instability",
        }
    }

    fn default_code(&self) -> &'static str {
        match self {
            StatisticsErrorKind::InsufficientData => "core.numeric.statistics.insufficient_data",
            StatisticsErrorKind::InvalidParameter => "core.numeric.statistics.invalid_parameter",
            StatisticsErrorKind::NumericalInstability => {
                "core.numeric.statistics.numerical_instability"
            }
        }
    }
}

/// 精度エラー種別。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumericErrorKind {
    InvalidPrecision,
    UnsupportedPrecision,
    ConversionFailed,
    PrecisionOverflow,
    UnsupportedCurrency,
}

impl NumericErrorKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            NumericErrorKind::InvalidPrecision => "invalid_precision",
            NumericErrorKind::UnsupportedPrecision => "unsupported_precision",
            NumericErrorKind::ConversionFailed => "conversion_failed",
            NumericErrorKind::PrecisionOverflow => "precision_overflow",
            NumericErrorKind::UnsupportedCurrency => "unsupported_currency",
        }
    }

    fn default_code(&self) -> &'static str {
        match self {
            NumericErrorKind::InvalidPrecision => "core.numeric.precision.invalid_precision",
            NumericErrorKind::UnsupportedPrecision => {
                "core.numeric.precision.unsupported_precision"
            }
            NumericErrorKind::ConversionFailed => "core.numeric.precision.conversion_failed",
            NumericErrorKind::PrecisionOverflow => "core.numeric.precision.precision_overflow",
            NumericErrorKind::UnsupportedCurrency => "core.numeric.precision.unsupported_currency",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn statistics_error_carries_data_quality_context() {
        let tags = StatisticsTags::new()
            .column("latency_ms")
            .aggregation("histogram")
            .audit_id("audit-1234");
        let diag = StatisticsError::invalid_parameter("invalid bucket")
            .with_rule("H-04")
            .with_context_code("data.stats.invalid_bucket")
            .with_tags(tags)
            .into_diagnostic();

        let numeric_meta = diag
            .extensions
            .get("numeric.statistics")
            .and_then(Value::as_object)
            .expect("numeric statistics metadata");
        assert_eq!(
            numeric_meta.get("column").and_then(Value::as_str),
            Some("latency_ms")
        );
        assert_eq!(
            numeric_meta.get("aggregation").and_then(Value::as_str),
            Some("histogram")
        );
        assert_eq!(
            numeric_meta.get("audit_id").and_then(Value::as_str),
            Some("audit-1234")
        );

        let data_stats = diag
            .extensions
            .get("data.stats")
            .and_then(Value::as_object)
            .expect("data.stats extension");
        assert_eq!(
            data_stats.get("column").and_then(Value::as_str),
            Some("latency_ms")
        );
        assert_eq!(
            data_stats.get("aggregation").and_then(Value::as_str),
            Some("histogram")
        );
        assert_eq!(
            data_stats.get("audit_id").and_then(Value::as_str),
            Some("audit-1234")
        );

        assert_eq!(
            diag.audit_metadata
                .get("data.stats.column")
                .and_then(Value::as_str),
            Some("latency_ms")
        );
        assert_eq!(
            diag.audit_metadata
                .get("data.stats.aggregation")
                .and_then(Value::as_str),
            Some("histogram")
        );
        assert_eq!(
            diag.audit_metadata
                .get("data.stats.audit_id")
                .and_then(Value::as_str),
            Some("audit-1234")
        );
        assert_eq!(diag.code, "data.stats.invalid_bucket");
    }

    #[test]
    fn statistics_error_preserves_non_finite_samples() {
        let diag = StatisticsError::numerical_instability("non finite value")
            .with_value(f64::NAN)
            .into_diagnostic();

        let numeric_meta = diag
            .extensions
            .get("numeric.statistics")
            .and_then(Value::as_object)
            .expect("numeric statistics metadata");
        assert_eq!(
            numeric_meta.get("sample_value").and_then(Value::as_str),
            Some("NaN")
        );
        assert_eq!(
            diag.audit_metadata
                .get("numeric.statistics.sample_value")
                .and_then(Value::as_str),
            Some("NaN")
        );
    }

    #[test]
    fn numeric_error_includes_precision_metadata() {
        let diag = NumericError::invalid_precision("scale cannot exceed precision")
            .with_precision_kind("decimal")
            .with_precision_scale(12)
            .with_precision_digits(8)
            .with_value_repr("1234.56789")
            .into_diagnostic();

        let precision_meta = diag
            .extensions
            .get("numeric.precision")
            .and_then(Value::as_object)
            .expect("precision metadata");
        assert_eq!(
            precision_meta.get("precision_kind").and_then(Value::as_str),
            Some("decimal")
        );
        assert_eq!(
            precision_meta.get("scale").and_then(Value::as_u64),
            Some(12)
        );
        assert_eq!(
            precision_meta.get("precision").and_then(Value::as_u64),
            Some(8)
        );
        assert_eq!(
            precision_meta.get("value").and_then(Value::as_str),
            Some("1234.56789")
        );
        assert_eq!(diag.code, "core.numeric.precision.invalid_precision");
    }

    #[test]
    fn numeric_error_records_currency_metadata() {
        let diag = NumericError::unsupported_currency("currency not supported")
            .with_currency_code("USD")
            .with_currency_scale(2)
            .into_diagnostic();

        let finance_meta = diag
            .extensions
            .get("numeric.finance")
            .and_then(Value::as_object)
            .expect("finance metadata");
        assert_eq!(
            finance_meta.get("currency_code").and_then(Value::as_str),
            Some("USD")
        );
        assert_eq!(finance_meta.get("scale").and_then(Value::as_u64), Some(2));
    }
}
