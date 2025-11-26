use serde_json::{json, Map as JsonMap, Value};

use super::{UnicodeError, UnicodeErrorKind};
use crate::prelude::ensure::{DiagnosticSeverity, GuardDiagnostic, PreludeGuardMetadata};

/// Unicode エラーを Diagnostics へ変換する補助。
pub struct UnicodeDiagnosticBuilder<'a> {
  span_label: &'a str,
}

impl<'a> UnicodeDiagnosticBuilder<'a> {
  pub fn new(span_label: &'a str) -> Self {
    Self { span_label }
  }

  pub fn to_guard_diagnostic(&self, error: &UnicodeError) -> GuardDiagnostic {
    let metadata = PreludeGuardMetadata::new(super::prelude_guard_kind(), error.phase());
    GuardDiagnostic {
      code: diagnostic_code(error.kind()),
      domain: diagnostic_domain(error.kind()),
      severity: DiagnosticSeverity::Error,
      message: error.message().into(),
      extensions: self.extensions(error),
      audit_metadata: self.audit_metadata(error),
    }
    .with_metadata(metadata)
  }

  fn extensions(&self, error: &UnicodeError) -> JsonMap<String, Value> {
    let mut map = JsonMap::new();
    map.insert("phase".into(), Value::String(error.phase().into()));
    map.insert("span_label".into(), Value::String(self.span_label.into()));
    if let Some(offset) = error.offset() {
      map.insert("offset".into(), Value::Number(offset.into()));
    }
    map
  }

  fn audit_metadata(&self, error: &UnicodeError) -> JsonMap<String, Value> {
    let mut metadata = JsonMap::new();
    metadata.insert(
      "text.unicode.kind".into(),
      Value::String(format!("{:?}", error.kind())),
    );
    metadata.insert(
      "text.unicode.phase".into(),
      Value::String(error.phase().into()),
    );
    metadata
  }
}

fn diagnostic_code(kind: UnicodeErrorKind) -> &'static str {
  match kind {
    UnicodeErrorKind::InvalidUtf8 => "U1001",
    UnicodeErrorKind::UnsupportedScalar => "U1002",
    UnicodeErrorKind::UnsupportedLocale => "U1003",
    UnicodeErrorKind::InvalidIdentifier => "U1004",
    UnicodeErrorKind::InvalidRange => "U1005",
    UnicodeErrorKind::DecodeFailure => "U1006",
    UnicodeErrorKind::EncodeFailure => "U1007",
  }
}

fn diagnostic_domain(kind: UnicodeErrorKind) -> &'static str {
  match kind {
    UnicodeErrorKind::InvalidUtf8 => "parser",
    UnicodeErrorKind::UnsupportedLocale => "text",
    UnicodeErrorKind::InvalidIdentifier => "parser",
    _ => "unicode",
  }
}

pub fn unicode_error_to_parse_error(error: UnicodeError) -> UnicodeError {
  error
}
