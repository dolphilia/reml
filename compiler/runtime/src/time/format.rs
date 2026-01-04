mod icu;

use super::{
    timestamp_from_total_nanos, TimeError, TimeFormat, TimeResult, Timestamp, NANOS_PER_SECOND_I128,
};
use crate::text::{self, LocaleId, Str, String as TextString};
use icu::resolve_custom_pattern;
use time::error::InvalidFormatDescription;
use time::format_description::{self, well_known::Rfc3339, FormatItem};
use time::{Date, OffsetDateTime, PrimitiveDateTime, Time};

const RFC3339_LABEL: &str = "rfc3339";
const UNIX_LABEL: &str = "unix";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LocaleStatus {
    Supported,
    Planned,
    #[allow(dead_code)]
    Deprecated,
}

impl LocaleStatus {
    fn is_supported(&self) -> bool {
        matches!(self, LocaleStatus::Supported)
    }

    fn as_str(&self) -> &'static str {
        match self {
            LocaleStatus::Supported => "supported",
            LocaleStatus::Planned => "planned",
            LocaleStatus::Deprecated => "deprecated",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct TimeLocaleEntry {
    pub id: &'static str,
    pub supports_rfc3339: bool,
    pub supports_unix: bool,
    pub supports_custom: bool,
    pub status: LocaleStatus,
    pub fallback: Option<&'static str>,
    #[allow(dead_code)]
    pub notes: &'static str,
}

impl TimeLocaleEntry {
    fn supports_format(&self, fmt: &TimeFormat) -> bool {
        match fmt {
            TimeFormat::Rfc3339 => self.supports_rfc3339,
            TimeFormat::Unix => self.supports_unix,
            TimeFormat::Custom(_) => self.supports_custom,
        }
    }
}

include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/src/time/locale_table_data.rs"
));

/// Timestamp を指定フォーマットで文字列化する。
pub fn format(ts: Timestamp, fmt: &TimeFormat) -> TimeResult<TextString> {
    format_with_locale(ts, fmt, None)
}

/// Timestamp をロケール付きで文字列化する。
pub fn format_with_locale(
    ts: Timestamp,
    fmt: &TimeFormat,
    locale: Option<&LocaleId>,
) -> TimeResult<TextString> {
    let locale = validate_locale(locale.cloned().unwrap_or_else(LocaleId::und), fmt)?;
    match fmt {
        TimeFormat::Rfc3339 => format_rfc3339(ts, &locale),
        TimeFormat::Unix => format_unix(ts, &locale),
        TimeFormat::Custom(pattern) => format_custom(ts, pattern, &locale),
    }
}

/// 文字列を Timestamp へパースする。
pub fn parse(input: &Str<'_>, fmt: &TimeFormat) -> TimeResult<Timestamp> {
    parse_with_locale(input, fmt, None)
}

/// 文字列をロケール指定で Timestamp へパースする。
pub fn parse_with_locale(
    input: &Str<'_>,
    fmt: &TimeFormat,
    locale: Option<&LocaleId>,
) -> TimeResult<Timestamp> {
    let locale = validate_locale(locale.cloned().unwrap_or_else(LocaleId::und), fmt)?;
    text::record_text_unicode_event(input.len_bytes());
    match fmt {
        TimeFormat::Rfc3339 => parse_rfc3339(input, &locale),
        TimeFormat::Unix => parse_unix(input, &locale),
        TimeFormat::Custom(pattern) => parse_custom(input, pattern, &locale),
    }
}

fn format_rfc3339(ts: Timestamp, locale: &LocaleId) -> TimeResult<TextString> {
    let datetime = timestamp_to_offset_datetime(ts)?;
    let formatted = datetime
        .format(&Rfc3339)
        .map_err(|err| invalid_format_error(err, RFC3339_LABEL, locale))?;
    text::record_text_unicode_event(formatted.len());
    Ok(TextString::from_std(formatted))
}

fn format_unix(ts: Timestamp, _locale: &LocaleId) -> TimeResult<TextString> {
    let total = ts.total_nanoseconds();
    let sign = if total < 0 { "-" } else { "" };
    let abs_total = total.abs();
    let seconds = abs_total / NANOS_PER_SECOND_I128;
    let nanos = (abs_total % NANOS_PER_SECOND_I128) as i64;
    let mut output = format!("{sign}{seconds}");
    if nanos != 0 {
        let mut frac = format!("{nanos:09}");
        while frac.ends_with('0') {
            frac.pop();
        }
        output.push('.');
        output.push_str(&frac);
    }
    text::record_text_unicode_event(output.len());
    Ok(TextString::from_std(output))
}

fn format_custom(ts: Timestamp, pattern: &str, locale: &LocaleId) -> TimeResult<TextString> {
    ensure_custom_pattern(pattern, locale)?;
    let resolved = resolve_custom_pattern(pattern, locale)?;
    let description =
        parse_description(&resolved).map_err(|err| invalid_format_error(err, pattern, locale))?;
    let datetime = timestamp_to_offset_datetime(ts)?;
    let formatted = datetime
        .format(&description)
        .map_err(|err| invalid_format_error(err, pattern, locale))?;
    text::record_text_unicode_event(formatted.len());
    Ok(TextString::from_std(formatted))
}

fn parse_rfc3339(input: &Str<'_>, locale: &LocaleId) -> TimeResult<Timestamp> {
    OffsetDateTime::parse(input.as_str(), &Rfc3339)
        .map_err(|err| invalid_format_error(err, RFC3339_LABEL, locale))
        .and_then(timestamp_from_datetime)
}

fn parse_unix(input: &Str<'_>, locale: &LocaleId) -> TimeResult<Timestamp> {
    let text = input.as_str().trim();
    if text.is_empty() {
        return Err(
            TimeError::invalid_format("unix timestamp string cannot be empty")
                .with_format_pattern(UNIX_LABEL.to_string())
                .with_locale(locale.canonical().to_string()),
        );
    }
    let (sign, digits) = parse_sign(text)?;
    let (seconds_part, frac_part) = split_fraction(digits);
    let seconds = if seconds_part.is_empty() {
        0
    } else {
        seconds_part
            .parse::<i128>()
            .map_err(|_| invalid_unix_number(text, locale))?
    };
    let nanos = frac_part
        .map(|part| parse_fractional_nanos(part, locale))
        .transpose()?
        .unwrap_or(0);
    let total = sign * (seconds * NANOS_PER_SECOND_I128 + nanos);
    timestamp_from_total_nanos(total)
}

fn parse_custom(input: &Str<'_>, pattern: &str, locale: &LocaleId) -> TimeResult<Timestamp> {
    ensure_custom_pattern(pattern, locale)?;
    let resolved = resolve_custom_pattern(pattern, locale)?;
    let description =
        parse_description(&resolved).map_err(|err| invalid_format_error(err, pattern, locale))?;
    match OffsetDateTime::parse(input.as_str(), &description) {
        Ok(datetime) => timestamp_from_datetime(datetime),
        Err(offset_err) => {
            if let Ok(naive) = PrimitiveDateTime::parse(input.as_str(), &description) {
                return timestamp_from_datetime(naive.assume_utc());
            }
            let date = Date::parse(input.as_str(), &description)
                .map_err(|_| invalid_format_error(offset_err, pattern, locale))?;
            let midnight = date.with_time(Time::MIDNIGHT);
            timestamp_from_datetime(midnight.assume_utc())
        }
    }
}

fn timestamp_to_offset_datetime(ts: Timestamp) -> TimeResult<OffsetDateTime> {
    let total = ts.total_nanoseconds();
    OffsetDateTime::from_unix_timestamp_nanos(total).map_err(|err| {
        TimeError::time_overflow(format!(
            "timestamp could not be represented as OffsetDateTime: {err}"
        ))
    })
}

fn timestamp_from_datetime(datetime: OffsetDateTime) -> TimeResult<Timestamp> {
    timestamp_from_total_nanos(datetime.unix_timestamp_nanos())
}

fn ensure_custom_pattern(pattern: &str, locale: &LocaleId) -> TimeResult<()> {
    if pattern.trim().is_empty() {
        return Err(
            TimeError::invalid_format("custom time format pattern cannot be empty")
                .with_locale(locale.canonical().to_string())
                .with_format_pattern(pattern.to_string()),
        );
    }
    Ok(())
}

fn parse_description<'a>(
    pattern: &'a str,
) -> Result<Vec<FormatItem<'a>>, InvalidFormatDescription> {
    format_description::parse(pattern)
}

fn validate_locale(locale: LocaleId, fmt: &TimeFormat) -> TimeResult<LocaleId> {
    let canonical = locale.canonical().to_string();
    let pattern = pattern_label(fmt);
    let Some(entry) = TIME_LOCALE_TABLE
        .iter()
        .find(|entry| entry.id.eq_ignore_ascii_case(&canonical))
    else {
        return Err(unknown_locale_error(&canonical, &pattern));
    };
    if !entry.status.is_supported() {
        return Err(locale_status_error(&canonical, entry, &pattern));
    }
    if !entry.supports_format(fmt) {
        return Err(locale_format_error(&canonical, fmt, entry, &pattern));
    }
    Ok(locale)
}

fn pattern_label(fmt: &TimeFormat) -> String {
    match fmt {
        TimeFormat::Rfc3339 => RFC3339_LABEL.to_string(),
        TimeFormat::Unix => UNIX_LABEL.to_string(),
        TimeFormat::Custom(pattern) => pattern.clone(),
    }
}

fn parse_sign(input: &str) -> TimeResult<(i128, &str)> {
    if let Some(rest) = input.strip_prefix('-') {
        Ok((-1, rest))
    } else if let Some(rest) = input.strip_prefix('+') {
        Ok((1, rest))
    } else {
        Ok((1, input))
    }
}

fn split_fraction(input: &str) -> (&str, Option<&str>) {
    if let Some((int_part, frac_part)) = input.split_once('.') {
        (int_part, Some(frac_part))
    } else {
        (input, None)
    }
}

fn parse_fractional_nanos(frac: &str, locale: &LocaleId) -> TimeResult<i128> {
    if frac.is_empty() {
        return Ok(0);
    }
    let mut digits = String::with_capacity(9);
    for ch in frac.chars() {
        if !ch.is_ascii_digit() {
            return Err(invalid_unix_number(frac, locale));
        }
        if digits.len() < 9 {
            digits.push(ch);
        }
    }
    while digits.len() < 9 {
        digits.push('0');
    }
    digits
        .parse::<i128>()
        .map_err(|_| invalid_unix_number(frac, locale))
}

fn invalid_unix_number(input: &str, locale: &LocaleId) -> TimeError {
    TimeError::invalid_format(format!("`{input}` is not a valid unix timestamp"))
        .with_format_pattern(UNIX_LABEL.to_string())
        .with_locale(locale.canonical().to_string())
}

fn invalid_format_error(
    err: impl std::fmt::Display,
    pattern: &str,
    locale: &LocaleId,
) -> TimeError {
    TimeError::invalid_format(format!("{err}"))
        .with_format_pattern(pattern.to_string())
        .with_locale(locale.canonical().to_string())
}

fn unknown_locale_error(canonical: &str, pattern: &str) -> TimeError {
    TimeError::invalid_format(format!(
        "locale `{canonical}` is not registered for Core.Time formatting"
    ))
    .with_locale(canonical.to_string())
    .with_format_pattern(pattern.to_string())
}

fn locale_status_error(canonical: &str, entry: &TimeLocaleEntry, pattern: &str) -> TimeError {
    let mut message = format!(
        "locale `{canonical}` is not available (status: {})",
        entry.status.as_str()
    );
    if let Some(fallback) = entry.fallback {
        message.push_str(&format!(" (try `{fallback}`)"));
    }
    TimeError::invalid_format(message)
        .with_locale(canonical.to_string())
        .with_format_pattern(pattern.to_string())
}

fn locale_format_error(
    canonical: &str,
    fmt: &TimeFormat,
    entry: &TimeLocaleEntry,
    pattern: &str,
) -> TimeError {
    let mut message = format!(
        "locale `{canonical}` does not support {} formats",
        format_kind(fmt)
    );
    if let Some(fallback) = entry.fallback {
        message.push_str(&format!(" (try `{fallback}`)"));
    }
    TimeError::invalid_format(message)
        .with_locale(canonical.to_string())
        .with_format_pattern(pattern.to_string())
}

fn format_kind(fmt: &TimeFormat) -> &'static str {
    match fmt {
        TimeFormat::Rfc3339 => "RFC3339",
        TimeFormat::Unix => "Unix",
        TimeFormat::Custom(_) => "custom",
    }
}
