use super::Timestamp;
use crate::io::TimeEnvSnapshot;
use crate::prelude::ensure::{DiagnosticSeverity, GuardDiagnostic, IntoDiagnostic};
use crate::stage::{StageId, StageRequirement};
use serde_json::{Map, Value};
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
    timezone: Option<String>,
    format_pattern: Option<String>,
    locale: Option<String>,
    platform: &'static str,
    capability: Option<String>,
    required_stage: Option<String>,
    actual_stage: Option<String>,
    env_timezone: Option<String>,
    env_locale: Option<String>,
}

impl TimeError {
    pub fn new(kind: TimeErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            timestamp: None,
            timezone: None,
            format_pattern: None,
            locale: None,
            platform: std::env::consts::OS,
            capability: None,
            required_stage: None,
            actual_stage: None,
            env_timezone: None,
            env_locale: None,
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

    pub fn with_timezone(mut self, timezone: impl Into<String>) -> Self {
        self.timezone = Some(timezone.into());
        self
    }

    pub fn with_format_pattern(mut self, pattern: impl Into<String>) -> Self {
        self.format_pattern = Some(pattern.into());
        self
    }

    pub fn with_locale(mut self, locale: impl Into<String>) -> Self {
        self.locale = Some(locale.into());
        self
    }

    pub fn with_capability_context(
        mut self,
        capability: impl Into<String>,
        required: Option<StageRequirement>,
        actual: Option<StageId>,
    ) -> Self {
        self.capability = Some(capability.into());
        self.required_stage = required.map(stage_requirement_label);
        self.actual_stage = actual.map(|stage| stage.as_str().into());
        self
    }

    pub fn with_env_snapshot(mut self, snapshot: &TimeEnvSnapshot) -> Self {
        if let Some(tz) = snapshot.timezone_env() {
            self.env_timezone = Some(tz.to_string());
        }
        if let Some(locale) = snapshot.locale_env() {
            self.env_locale = Some(locale.to_string());
        }
        self
    }

    pub fn system_clock_unavailable(message: impl Into<String>) -> Self {
        Self::new(TimeErrorKind::SystemClockUnavailable, message)
    }

    pub fn time_overflow(message: impl Into<String>) -> Self {
        Self::new(TimeErrorKind::TimeOverflow, message)
    }

    pub fn invalid_timezone(message: impl Into<String>) -> Self {
        Self::new(TimeErrorKind::InvalidTimezone, message)
    }

    pub fn invalid_format(message: impl Into<String>) -> Self {
        Self::new(TimeErrorKind::InvalidFormat, message)
    }
}

impl fmt::Display for TimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.message)
    }
}

impl Error for TimeError {}

impl IntoDiagnostic for TimeError {
    fn into_diagnostic(self) -> GuardDiagnostic {
        let TimeError {
            kind,
            message,
            timestamp,
            timezone,
            format_pattern,
            locale,
            platform,
            capability,
            required_stage,
            actual_stage,
            env_timezone,
            env_locale,
        } = self;

        let mut time_extensions = Map::new();
        time_extensions.insert("platform".into(), Value::String(platform.to_string()));
        if let Some(tz) = timezone.as_ref() {
            time_extensions.insert("timezone".into(), Value::String(tz.clone()));
        }
        if let Some(pattern) = format_pattern.as_ref() {
            time_extensions.insert("format_pattern".into(), Value::String(pattern.clone()));
        }
        if let Some(locale) = locale.as_ref() {
            time_extensions.insert("locale".into(), Value::String(locale.clone()));
        }
        if let Some(ts) = timestamp {
            if let Ok(value) = serde_json::to_value(ts) {
                time_extensions.insert("timestamp".into(), value);
            }
        }
        if let Some(capability) = capability.as_ref() {
            time_extensions.insert("capability".into(), Value::String(capability.clone()));
        }
        if let Some(stage) = required_stage.as_ref() {
            time_extensions.insert("required_stage".into(), Value::String(stage.clone()));
        }
        if let Some(actual) = actual_stage.as_ref() {
            time_extensions.insert("actual_stage".into(), Value::String(actual.clone()));
        }
        if let Some(tz) = env_timezone.as_ref() {
            time_extensions.insert("env_timezone".into(), Value::String(tz.clone()));
        }
        if let Some(locale) = env_locale.as_ref() {
            time_extensions.insert("env_locale".into(), Value::String(locale.clone()));
        }

        let mut extensions = Map::new();
        extensions.insert("time".into(), Value::Object(time_extensions.clone()));
        extensions.insert("message".into(), Value::String(message.clone()));

        let mut audit_metadata = Map::new();
        audit_metadata.insert("time.platform".into(), Value::String(platform.to_string()));
        if let Some(tz) = timezone.as_ref() {
            audit_metadata.insert("time.timezone".into(), Value::String(tz.clone()));
        }
        if let Some(pattern) = format_pattern.as_ref() {
            audit_metadata.insert("time.format.pattern".into(), Value::String(pattern.clone()));
        }
        if let Some(locale) = locale.as_ref() {
            audit_metadata.insert("time.locale".into(), Value::String(locale.clone()));
        }
        if let Some(capability) = capability.as_ref() {
            audit_metadata.insert("time.capability".into(), Value::String(capability.clone()));
        }
        if let Some(stage) = required_stage.as_ref() {
            audit_metadata.insert("time.required_stage".into(), Value::String(stage.clone()));
        }
        if let Some(actual) = actual_stage.as_ref() {
            audit_metadata.insert("time.actual_stage".into(), Value::String(actual.clone()));
        }
        if let Some(ts) = timestamp {
            if let Ok(value) = serde_json::to_value(ts) {
                audit_metadata.insert("time.timestamp".into(), value);
            }
        }
        if let Some(tz) = env_timezone.as_ref() {
            audit_metadata.insert("time.env.timezone".into(), Value::String(tz.clone()));
        }
        if let Some(locale) = env_locale.as_ref() {
            audit_metadata.insert("time.env.locale".into(), Value::String(locale.clone()));
        }

        GuardDiagnostic {
            code: kind.default_code(),
            domain: "runtime",
            severity: DiagnosticSeverity::Error,
            message,
            extensions,
            audit_metadata,
        }
    }
}

/// 時刻エラー種別。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeErrorKind {
    SystemClockUnavailable,
    InvalidTimezone,
    TimeOverflow,
    InvalidFormat,
}

impl TimeErrorKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            TimeErrorKind::SystemClockUnavailable => "system_clock_unavailable",
            TimeErrorKind::InvalidTimezone => "invalid_timezone",
            TimeErrorKind::TimeOverflow => "time_overflow",
            TimeErrorKind::InvalidFormat => "invalid_format",
        }
    }

    fn default_code(&self) -> &'static str {
        match self {
            TimeErrorKind::SystemClockUnavailable => "core.time.system_clock_unavailable",
            TimeErrorKind::InvalidTimezone => "core.time.invalid_timezone",
            TimeErrorKind::TimeOverflow => "core.time.overflow",
            TimeErrorKind::InvalidFormat => "core.time.invalid_format",
        }
    }
}

fn stage_requirement_label(requirement: StageRequirement) -> String {
    match requirement {
        StageRequirement::Exact(stage) => format!("exact:{}", stage.as_str()),
        StageRequirement::AtLeast(stage) => format!("at_least:{}", stage.as_str()),
    }
}
