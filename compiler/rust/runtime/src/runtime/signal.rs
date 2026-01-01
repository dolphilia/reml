use serde::{Deserialize, Serialize};
use serde_json::{Map as JsonMap, Value};

use crate::prelude::ensure::{DiagnosticSeverity, GuardDiagnostic, IntoDiagnostic};

/// Core.Runtime の Signal 型。
pub type Signal = i64;

/// Core.Runtime の SignalInfo。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignalInfo {
    pub signal: Signal,
    pub sender: i64,
}

impl SignalInfo {
    pub fn new(signal: Signal, sender: i64) -> Self {
        Self { signal, sender }
    }
}

/// Core.Runtime の SignalError。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalError {
    pub kind: SignalErrorKind,
    pub message: String,
}

impl SignalError {
    pub fn new(kind: SignalErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

impl IntoDiagnostic for SignalError {
    fn into_diagnostic(self) -> GuardDiagnostic {
        let SignalError { kind, message } = self;

        let mut signal_extensions = JsonMap::new();
        signal_extensions.insert("error_kind".into(), Value::String(kind.as_str().into()));
        signal_extensions.insert("message".into(), Value::String(message.clone()));

        let mut extensions = JsonMap::new();
        extensions.insert("signal".into(), Value::Object(signal_extensions));

        let mut audit_metadata = JsonMap::new();
        audit_metadata.insert(
            "signal.error.kind".into(),
            Value::String(kind.as_str().into()),
        );

        GuardDiagnostic {
            code: diagnostic_code_for_signal_error(kind, &message),
            domain: "runtime",
            severity: DiagnosticSeverity::Error,
            message,
            notes: Vec::new(),
            extensions,
            audit_metadata,
        }
    }
}

/// Core.Runtime の SignalError 種別。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignalErrorKind {
    Unsupported,
    PermissionDenied,
    TimedOut,
    InvalidSignal,
    RuntimeFailure,
}

impl SignalErrorKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            SignalErrorKind::Unsupported => "unsupported",
            SignalErrorKind::PermissionDenied => "permission_denied",
            SignalErrorKind::TimedOut => "timed_out",
            SignalErrorKind::InvalidSignal => "invalid_signal",
            SignalErrorKind::RuntimeFailure => "runtime_failure",
        }
    }

    fn default_code(&self) -> &'static str {
        match self {
            SignalErrorKind::Unsupported => "core.system.signal.unsupported",
            SignalErrorKind::PermissionDenied => "core.system.signal.permission_denied",
            SignalErrorKind::TimedOut => "core.system.signal.timed_out",
            SignalErrorKind::InvalidSignal => "core.system.signal.invalid_signal",
            SignalErrorKind::RuntimeFailure => "core.system.signal.runtime_failure",
        }
    }
}

fn diagnostic_code_for_signal_error(kind: SignalErrorKind, message: &str) -> &'static str {
    if kind == SignalErrorKind::Unsupported && is_missing_capability_message(message) {
        return "system.capability.missing";
    }
    kind.default_code()
}

fn is_missing_capability_message(message: &str) -> bool {
    message.contains("capability") && message.contains("not registered")
}
