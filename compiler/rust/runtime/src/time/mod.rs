//! Core.Time 仕様（`docs/spec/3-4-core-numeric-time.md`）の Timestamp / Duration 基本実装。

mod effects;
pub mod error;
mod timezone;

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::thread;
use std::time::{Duration as StdDuration, Instant, SystemTime, UNIX_EPOCH};

use crate::prelude::iter::EffectLabels;

pub use effects::TimeSyscallMetrics;
pub use error::{TimeError, TimeErrorKind, TimeResult};
pub use timezone::Timezone;

const NANOS_PER_SECOND_I128: i128 = 1_000_000_000;
const MAX_TOTAL_NANOS: i128 =
    (i64::MAX as i128) * NANOS_PER_SECOND_I128 + (NANOS_PER_SECOND_I128 - 1);
const MIN_TOTAL_NANOS: i128 = (i64::MIN as i128) * NANOS_PER_SECOND_I128;

static CLOCK: Lazy<SystemClockAdapter> = Lazy::new(SystemClockAdapter::new);

/// UNIX エポック基準の時刻。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Timestamp {
    seconds: i64,
    nanos: i32,
}

impl Timestamp {
    /// UNIX エポックを示す Timestamp。
    pub const fn unix_epoch() -> Self {
        Self {
            seconds: 0,
            nanos: 0,
        }
    }

    /// 秒成分。
    pub fn seconds(&self) -> i64 {
        self.seconds
    }

    /// ナノ秒成分。
    pub fn nanos(&self) -> i32 {
        self.nanos
    }

    /// `seconds`/`nanos` から Timestamp を生成する（範囲外の場合は panic）。
    pub fn from_parts(seconds: i64, nanos: i32) -> Self {
        Self::try_from_parts(seconds, nanos).expect("timestamp components exceeded supported range")
    }

    /// `seconds`/`nanos` から Timestamp を生成する（範囲チェック付き）。
    pub fn try_from_parts(seconds: i64, nanos: i32) -> TimeResult<Self> {
        let (seconds, nanos) = normalize_parts(seconds, nanos)?;
        Ok(Self { seconds, nanos })
    }

    /// `std::time::SystemTime` を Timestamp へ変換する。
    pub fn from_system_time(time: SystemTime) -> TimeResult<Self> {
        match time.duration_since(UNIX_EPOCH) {
            Ok(duration) => timestamp_from_total_nanos(total_nanos_from_std_duration(duration)?),
            Err(error) => {
                let duration = error.duration();
                let total = total_nanos_from_std_duration(duration)?;
                timestamp_from_total_nanos(-total)
            }
        }
    }

    /// `Duration` を加算した Timestamp を返す（範囲外は panic）。
    pub fn add_duration(self, delta: Duration) -> Self {
        self.checked_add_duration(delta)
            .expect("timestamp addition exceeded supported range")
    }

    /// `Duration` を加算し、範囲外の場合は `TimeError` を返す。
    pub fn checked_add_duration(self, delta: Duration) -> TimeResult<Self> {
        let total = self
            .total_nanoseconds()
            .checked_add(delta.total_nanoseconds())
            .ok_or_else(|| {
                TimeError::time_overflow("timestamp addition overflowed supported range")
            })?;
        timestamp_from_total_nanos(total)
    }

    fn total_nanoseconds(&self) -> i128 {
        (self.seconds as i128) * NANOS_PER_SECOND_I128 + self.nanos as i128
    }
}

/// Core.Time の Duration。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Duration {
    seconds: i64,
    nanos: i32,
}

/// Core.Time のフォーマット指定。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "pattern", rename_all = "snake_case")]
pub enum TimeFormat {
    Rfc3339,
    Unix,
    Custom(String),
}

impl TimeFormat {
    pub fn custom(pattern: impl Into<String>) -> Self {
        TimeFormat::Custom(pattern.into())
    }

    pub fn is_custom(&self) -> bool {
        matches!(self, TimeFormat::Custom(_))
    }
}

impl Duration {
    pub const fn zero() -> Self {
        Self {
            seconds: 0,
            nanos: 0,
        }
    }

    pub fn seconds(&self) -> i64 {
        self.seconds
    }

    pub fn nanos(&self) -> i32 {
        self.nanos
    }

    pub fn is_negative(&self) -> bool {
        self.seconds < 0
    }

    pub fn is_zero(&self) -> bool {
        self.seconds == 0 && self.nanos == 0
    }

    pub fn from_parts(seconds: i64, nanos: i32) -> Self {
        Self::try_from_parts(seconds, nanos).expect("duration components exceeded supported range")
    }

    pub fn try_from_parts(seconds: i64, nanos: i32) -> TimeResult<Self> {
        let (seconds, nanos) = normalize_parts(seconds, nanos)?;
        Ok(Self { seconds, nanos })
    }

    pub fn from_seconds(seconds: i64) -> Self {
        Self { seconds, nanos: 0 }
    }

    pub fn from_millis(milliseconds: i64) -> Self {
        duration_from_total_nanos((milliseconds as i128) * 1_000_000)
            .expect("duration milliseconds exceeded supported range")
    }

    pub fn from_std(duration: StdDuration) -> TimeResult<Self> {
        duration_from_total_nanos(total_nanos_from_std_duration(duration)?)
    }

    pub fn to_std(&self) -> TimeResult<StdDuration> {
        if self.is_negative() {
            return Err(TimeError::time_overflow(
                "negative Duration cannot be converted into std::time::Duration",
            ));
        }
        Ok(StdDuration::new(self.seconds as u64, self.nanos as u32))
    }

    pub fn total_nanoseconds(&self) -> i128 {
        (self.seconds as i128) * NANOS_PER_SECOND_I128 + self.nanos as i128
    }

    pub(crate) fn from_total_nanoseconds(total: i128) -> TimeResult<Self> {
        duration_from_total_nanos(total)
    }
}

/// 現在のシステム時刻を返す。
pub fn now() -> TimeResult<Timestamp> {
    CLOCK.now()
}

/// 単調増加クロック由来の Timestamp を返す。
pub fn monotonic_now() -> TimeResult<Timestamp> {
    CLOCK.monotonic_now()
}

/// 2 つの Timestamp の差分 Duration。
pub fn duration_between(start: Timestamp, end: Timestamp) -> Duration {
    duration_from_total_nanos(end.total_nanoseconds() - start.total_nanoseconds())
        .expect("timestamp difference exceeded Duration range")
}

/// Timestamp へ Duration を加算する。
pub fn add_duration(ts: Timestamp, delta: Duration) -> Timestamp {
    ts.add_duration(delta)
}

/// 指定時間スリープする。
pub fn sleep(duration: Duration) -> TimeResult<()> {
    CLOCK.sleep(duration)
}

/// effect / KPI 計測の Snapshot。
pub fn take_time_effects_snapshot() -> EffectLabels {
    effects::take_recorded_effects().to_labels()
}

/// システムコールの遅延統計 Snapshot。
pub fn take_time_syscall_metrics() -> TimeSyscallMetrics {
    effects::take_syscall_metrics()
}

/// UTC タイムゾーンを返す。
pub fn utc() -> Timezone {
    timezone::utc()
}

/// 指定名のタイムゾーンを解決する。
pub fn timezone(name: impl AsRef<str>) -> TimeResult<Timezone> {
    timezone::timezone(name)
}

/// ローカルタイムゾーンを返す。
pub fn local() -> TimeResult<Timezone> {
    timezone::local()
}

/// タイムゾーン間で Timestamp を変換する。
pub fn convert_timezone(ts: Timestamp, from: Timezone, to: Timezone) -> TimeResult<Timestamp> {
    timezone::convert_timezone(ts, from, to)
}

struct SystemClockAdapter {
    base_instant: Instant,
    base_timestamp: Timestamp,
}

impl SystemClockAdapter {
    fn new() -> Self {
        let system_now = SystemTime::now();
        let base_timestamp =
            Timestamp::from_system_time(system_now).unwrap_or_else(|_| Timestamp::unix_epoch());
        Self {
            base_instant: Instant::now(),
            base_timestamp,
        }
    }

    fn now(&self) -> TimeResult<Timestamp> {
        let call_started = Instant::now();
        let result = Timestamp::from_system_time(SystemTime::now());
        effects::record_time_call(call_started.elapsed());
        result
    }

    fn monotonic_now(&self) -> TimeResult<Timestamp> {
        let call_started = Instant::now();
        let elapsed = call_started.duration_since(self.base_instant);
        let result = Duration::from_std(elapsed).and_then(|delta| {
            self.base_timestamp
                .checked_add_duration(delta)
                .map_err(|_| TimeError::time_overflow("monotonic clock overflowed timestamp range"))
        });
        effects::record_time_call(call_started.elapsed());
        result
    }

    fn sleep(&self, duration: Duration) -> TimeResult<()> {
        let std_duration = duration.to_std()?;
        let call_started = Instant::now();
        thread::sleep(std_duration);
        effects::record_time_call(call_started.elapsed());
        Ok(())
    }
}

fn normalize_parts(seconds: i64, nanos: i32) -> TimeResult<(i64, i32)> {
    let total = (seconds as i128) * NANOS_PER_SECOND_I128 + nanos as i128;
    split_total_nanos(total)
}

fn timestamp_from_total_nanos(total: i128) -> TimeResult<Timestamp> {
    let (seconds, nanos) = split_total_nanos(total)?;
    Ok(Timestamp { seconds, nanos })
}

fn duration_from_total_nanos(total: i128) -> TimeResult<Duration> {
    let (seconds, nanos) = split_total_nanos(total)?;
    Ok(Duration { seconds, nanos })
}

fn split_total_nanos(total: i128) -> TimeResult<(i64, i32)> {
    if total > MAX_TOTAL_NANOS || total < MIN_TOTAL_NANOS {
        return Err(TimeError::time_overflow(
            "value exceeded Core.Time representable range",
        ));
    }
    let seconds = total.div_euclid(NANOS_PER_SECOND_I128);
    let nanos = total.rem_euclid(NANOS_PER_SECOND_I128);
    Ok((seconds as i64, nanos as i32))
}

fn total_nanos_from_std_duration(duration: StdDuration) -> TimeResult<i128> {
    if duration.as_secs() > i64::MAX as u64 {
        return Err(TimeError::time_overflow(
            "std::time::Duration is larger than Core.Time range",
        ));
    }
    let secs = duration.as_secs() as i128;
    let nanos = duration.subsec_nanos() as i128;
    let total = secs * NANOS_PER_SECOND_I128 + nanos;
    if total > MAX_TOTAL_NANOS {
        return Err(TimeError::time_overflow(
            "std::time::Duration is larger than Core.Time range",
        ));
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::ensure::IntoDiagnostic;
    use serde_json::Value;
    use std::time::Duration as StdDuration;

    const TIMEZONE_CASES_JSON: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../..",
        "/tests/data/time/timezone_cases.json"
    ));

    #[test]
    fn timestamp_from_parts_normalizes() {
        let ts = Timestamp::from_parts(10, 1_500_000_000);
        assert_eq!(ts.seconds(), 11);
        assert_eq!(ts.nanos(), 500_000_000);
    }

    #[test]
    fn duration_between_matches_manual_diff() {
        let start = Timestamp::from_parts(5, 250_000_000);
        let end = Timestamp::from_parts(8, 750_000_000);
        let diff = duration_between(start, end);
        assert_eq!(diff.seconds(), 3);
        assert_eq!(diff.nanos(), 500_000_000);
    }

    #[test]
    fn add_duration_respects_sign() {
        let ts = Timestamp::from_parts(10, 0);
        let delta = Duration::from_parts(-2, 500_000_000);
        let result = add_duration(ts, delta);
        assert_eq!(result.seconds(), 8);
        assert_eq!(result.nanos(), 500_000_000);
    }

    #[test]
    fn monotonic_now_is_non_decreasing() {
        let first = monotonic_now().expect("monotonic clock should be available");
        thread::sleep(StdDuration::from_millis(1));
        let second = monotonic_now().expect("monotonic clock should be available");
        assert!(second >= first, "monotonic clock regressed");
    }

    #[test]
    fn sleep_rejects_negative_duration() {
        let err = sleep(Duration::from_parts(-1, 0)).expect_err("negative sleep should fail");
        assert_eq!(err.kind(), TimeErrorKind::TimeOverflow);
    }

    #[test]
    fn time_effects_and_metrics_are_recorded() {
        let _ = take_time_effects_snapshot();
        let _ = take_time_syscall_metrics();
        now().expect("now");
        let zero = Duration::from_millis(0);
        sleep(zero).expect("sleep");
        let labels = take_time_effects_snapshot();
        assert!(
            labels.time,
            "time effect bit should be set after calling now/sleep"
        );
        assert!(
            labels.time_calls >= 2,
            "time_calls should record per-call count"
        );
        let metrics = take_time_syscall_metrics();
        assert!(metrics.calls >= 2);
    }

    #[test]
    fn timezone_cases_from_dataset() {
        let dataset: Value =
            serde_json::from_str(TIMEZONE_CASES_JSON).expect("timezone dataset json");
        let cases = dataset["cases"]
            .as_array()
            .expect("cases array must exist");
        for case in cases {
            let expected = case["offset_seconds"]
                .as_i64()
                .expect("offset seconds as i64");
            let aliases = case["aliases"]
                .as_array()
                .expect("aliases array must exist");
            for alias in aliases {
                let alias = alias.as_str().expect("alias as string");
                let tz = timezone(alias).expect("timezone lookup should succeed");
                assert_eq!(tz.offset().seconds(), expected);
            }
        }
        let convert_cases = dataset["convert_cases"]
            .as_array()
            .expect("convert cases");
        for case in convert_cases {
            let ts_obj = case["timestamp"].as_object().expect("timestamp object");
            let seconds = ts_obj["seconds"].as_i64().expect("seconds field");
            let nanos = ts_obj["nanos"].as_i64().unwrap_or(0) as i32;
            let ts = Timestamp::from_parts(seconds, nanos);
            let from = case["from"].as_str().expect("from timezone");
            let to = case["to"].as_str().expect("to timezone");
            let converted = convert_timezone(
                ts,
                timezone(from).expect("source tz"),
                timezone(to).expect("target tz"),
            )
            .expect("conversion");
            assert_eq!(
                converted.seconds(),
                case["expected_seconds"].as_i64().expect("expected seconds")
            );
            assert_eq!(
                converted.nanos(),
                case["expected_nanos"].as_i64().unwrap_or(0) as i32
            );
        }
        let bounds = dataset["local_timezone_expectations"]
            .as_object()
            .expect("local bounds");
        if let Ok(local_zone) = local() {
            let min = bounds["allowed_min_offset_seconds"].as_i64().unwrap_or(-50400);
            let max = bounds["allowed_max_offset_seconds"].as_i64().unwrap_or(50400);
            let actual = local_zone.offset().seconds();
            assert!(
                actual >= min && actual <= max,
                "local offset {actual} is outside {min}..={max}"
            );
        } else {
            // `time::OffsetDateTime::now_local()` may fail inside hermetic CI environments.
        }
    }

    #[test]
    fn time_error_into_diagnostic_includes_metadata() {
        let ts = Timestamp::from_parts(42, 100);
        let diag = TimeError::invalid_timezone("invalid tz")
            .with_timezone("UTC+99:99")
            .with_timestamp(ts)
            .into_diagnostic();
        assert_eq!(diag.code, "core.time.invalid_timezone");
        let time_extension = diag
            .extensions
            .get("time")
            .and_then(|value| value.as_object())
            .expect("time extension");
        assert_eq!(
            time_extension.get("timezone"),
            Some(&Value::String("UTC+99:99".into()))
        );
        assert_eq!(
            diag.audit_metadata.get("time.platform"),
            Some(&Value::String(std::env::consts::OS.into()))
        );
    }
}
