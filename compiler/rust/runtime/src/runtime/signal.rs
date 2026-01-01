use serde::{Deserialize, Serialize};

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

/// Core.Runtime の SignalError 種別。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignalErrorKind {
    Unsupported,
    PermissionDenied,
    TimedOut,
    InvalidSignal,
    RuntimeFailure,
}
