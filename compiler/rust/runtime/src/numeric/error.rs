use serde_json::{Map, Number, Value};

use crate::prelude::ensure::{DiagnosticSeverity, GuardDiagnostic, IntoDiagnostic};

const NUMERIC_DIAGNOSTIC_DOMAIN: &str = "runtime";
const NUMERIC_DIAGNOSTIC_CODE_BASE: &str = "core.numeric.statistics";

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
        }
    }

    pub fn with_bucket_context(
        mut self,
        index: usize,
        label: impl Into<String>,
    ) -> Self {
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
        } = self;

        let code = context_code.unwrap_or_else(|| kind.default_code());

        let mut numeric_extensions = Map::new();
        numeric_extensions.insert(
            "kind".into(),
            Value::String(kind.as_str().into()),
        );
        if let Some(index) = bucket_index {
            numeric_extensions.insert(
                "bucket_index".into(),
                Value::Number(Number::from(index)),
            );
        }
        if let Some(label) = bucket_label.as_ref() {
            numeric_extensions
                .insert("bucket_label".into(), Value::String(label.clone()));
        }
        if let Some(rule) = violated_rule.as_ref() {
            numeric_extensions.insert(
                "violated_rule".into(),
                Value::String(rule.clone()),
            );
        }
        if let Some(value) = value {
            numeric_extensions.insert(
                "sample_value".into(),
                Value::Number(Number::from_f64(value).unwrap_or(Number::from(0))),
            );
        }

        let mut extensions = Map::new();
        extensions.insert(
            "numeric.statistics".into(),
            Value::Object(numeric_extensions.clone()),
        );
        extensions.insert("message".into(), Value::String(message.clone()));

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
            StatisticsErrorKind::InsufficientData => {
                "core.numeric.statistics.insufficient_data"
            }
            StatisticsErrorKind::InvalidParameter => {
                "core.numeric.statistics.invalid_parameter"
            }
            StatisticsErrorKind::NumericalInstability => {
                "core.numeric.statistics.numerical_instability"
            }
        }
    }
}
