use super::{Duration, TimeError, TimeResult, Timestamp, NANOS_PER_SECOND_I128};
use crate::{
    io::time_env_snapshot,
    runtime::api::guard_time_capability,
    stage::{StageId, StageRequirement},
};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

const LOOKUP_CAPABILITY: &str = "core.time.timezone.lookup";
const LOCAL_CAPABILITY: &str = "core.time.timezone.local";
const MAX_OFFSET_SECONDS: i64 = 18 * 60 * 60;
const TIME_EFFECT_SCOPE: &[&str] = &["time"];
const IANA_TIMEZONES: &[(&str, i64)] = &[
    ("Asia/Tokyo", 9 * 3600),
    ("Europe/London", 0),
    ("America/New_York", -5 * 3600),
];

/// Core.Time タイムゾーン表現。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Timezone {
    name: String,
    offset: Duration,
}

impl Timezone {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn offset(&self) -> Duration {
        self.offset
    }
}

pub fn utc() -> Timezone {
    Timezone {
        name: "UTC".into(),
        offset: Duration::zero(),
    }
}

pub fn timezone(name: impl AsRef<str>) -> TimeResult<Timezone> {
    verify_capability(LOOKUP_CAPABILITY)?;
    let raw = name.as_ref().trim();
    if raw.is_empty() {
        let snapshot = time_env_snapshot();
        return Err(TimeError::invalid_timezone("timezone name cannot be empty")
            .with_env_snapshot(&snapshot));
    }
    if let Some(tz) = timezone_from_iana(raw) {
        return Ok(tz);
    }
    let offset_seconds = parse_timezone_offset(raw).ok_or_else(|| {
        let snapshot = time_env_snapshot();
        TimeError::invalid_timezone(format!("unsupported timezone '{raw}'"))
            .with_timezone(raw)
            .with_env_snapshot(&snapshot)
    })?;
    build_timezone(offset_seconds)
}

pub fn local() -> TimeResult<Timezone> {
    verify_capability(LOCAL_CAPABILITY)?;
    let now = OffsetDateTime::now_local().map_err(|err| {
        let snapshot = time_env_snapshot();
        TimeError::system_clock_unavailable(format!("failed to resolve local timezone: {err}"))
            .with_env_snapshot(&snapshot)
    })?;
    let offset_seconds = now.offset().whole_seconds();
    build_timezone(i64::from(offset_seconds))
}

pub fn convert_timezone(ts: Timestamp, from: Timezone, to: Timezone) -> TimeResult<Timestamp> {
    let delta = to.offset().total_nanoseconds() - from.offset().total_nanoseconds();
    let duration = Duration::from_total_nanoseconds(delta)?;
    ts.checked_add_duration(duration)
}

fn build_timezone(offset_seconds: i64) -> TimeResult<Timezone> {
    let offset = Duration::from_parts(offset_seconds, 0);
    let name = canonical_name(offset);
    build_timezone_with_label(name, offset_seconds)
}

fn build_timezone_with_label(name: String, offset_seconds: i64) -> TimeResult<Timezone> {
    ensure_offset_range(offset_seconds)?;
    let offset = Duration::from_parts(offset_seconds, 0);
    Ok(Timezone { name, offset })
}

fn ensure_offset_range(offset_seconds: i64) -> TimeResult<()> {
    if offset_seconds > MAX_OFFSET_SECONDS || offset_seconds < -MAX_OFFSET_SECONDS {
        let snapshot = time_env_snapshot();
        return Err(TimeError::invalid_timezone(format!(
            "offset {offset_seconds} is out of range"
        ))
        .with_timezone(format!("offset:{offset_seconds}"))
        .with_env_snapshot(&snapshot));
    }
    Ok(())
}

fn canonical_name(offset: Duration) -> String {
    if offset.is_zero() {
        return "UTC".into();
    }
    let mut seconds = offset.total_nanoseconds() / NANOS_PER_SECOND_I128;
    let sign = if seconds >= 0 { '+' } else { '-' };
    if seconds < 0 {
        seconds = -seconds;
    }
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    format!("UTC{sign}{hours:02}:{minutes:02}")
}

fn parse_timezone_offset(input: &str) -> Option<i64> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }
    let upper = trimmed.to_ascii_uppercase();
    if matches!(upper.as_str(), "UTC" | "GMT" | "Z" | "ETC/UTC") {
        return Some(0);
    }
    let spec = if let Some(rest) = upper.strip_prefix("UTC") {
        rest
    } else if let Some(rest) = upper.strip_prefix("GMT") {
        rest
    } else if trimmed.starts_with('+') || trimmed.starts_with('-') {
        trimmed
    } else {
        return None;
    };
    parse_offset_components(spec.trim())
}

fn parse_offset_components(spec: &str) -> Option<i64> {
    if spec.is_empty() {
        return Some(0);
    }
    let sign = match spec.chars().next()? {
        '+' => 1_i64,
        '-' => -1_i64,
        _ => return None,
    };
    let body = spec[1..].trim();
    if body.is_empty() {
        return Some(0);
    }
    let (hours_part, minutes_part) = if let Some((h, m)) = body.split_once(':') {
        (h.trim(), m.trim())
    } else if body.len() == 4 {
        (&body[..2], &body[2..])
    } else if body.len() == 3 {
        (&body[..1], &body[1..])
    } else if body.len() <= 2 {
        (body, "0")
    } else {
        return None;
    };
    let hours: i64 = hours_part.parse().ok()?;
    let minutes: i64 = minutes_part.parse().ok()?;
    if hours.abs() > 18 || minutes.abs() >= 60 {
        return None;
    }
    let total_minutes = hours * 60 + minutes;
    Some(sign * total_minutes * 60)
}

fn timezone_from_iana(raw: &str) -> Option<Timezone> {
    IANA_TIMEZONES
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(raw))
        .and_then(|(name, offset)| build_timezone_with_label((*name).to_string(), *offset).ok())
}

fn verify_capability(capability: &str) -> TimeResult<()> {
    let requirement = StageRequirement::AtLeast(StageId::Beta);
    let snapshot = time_env_snapshot();
    guard_time_capability(capability, requirement, TIME_EFFECT_SCOPE)
        .map(|_| ())
        .map_err(|err| {
            TimeError::system_clock_unavailable(err.detail().to_string())
                .with_capability_context(capability.to_string(), Some(requirement), None)
                .with_env_snapshot(&snapshot)
        })
}
