use super::Timestamp;
use std::error::Error;
use std::fmt;

/// Core.Time 共通の結果型。
pub type TimeResult<T> = Result<T, TimeError>;

/// 時刻 API のエラー。
#[derive(Debug, Clone)]
pub struct TimeError {
    kind: TimeErrorKind,
    message: String,
    timestamp: Option<Timestamp>,
}

impl TimeError {
    pub fn new(kind: TimeErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            timestamp: None,
        }
    }

    pub fn kind(&self) -> TimeErrorKind {
        self.kind
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn timestamp(&self) -> Option<&Timestamp> {
        self.timestamp.as_ref()
    }

    pub fn with_timestamp(mut self, timestamp: Timestamp) -> Self {
        self.timestamp = Some(timestamp);
        self
    }

    pub fn system_clock_unavailable(message: impl Into<String>) -> Self {
        Self::new(TimeErrorKind::SystemClockUnavailable, message)
    }

    pub fn time_overflow(message: impl Into<String>) -> Self {
        Self::new(TimeErrorKind::TimeOverflow, message)
    }
}

impl fmt::Display for TimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.message)
    }
}

impl Error for TimeError {}

/// 時刻エラー種別。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeErrorKind {
    SystemClockUnavailable,
    InvalidTimezone,
    TimeOverflow,
}
