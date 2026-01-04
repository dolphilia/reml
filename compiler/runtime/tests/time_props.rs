#![cfg(feature = "core-time")]

use proptest::prelude::*;
use reml_runtime::time::{
    convert_timezone, duration_between, timezone, Duration, Timestamp, Timezone,
};

fn timestamp_strategy() -> impl Strategy<Value = Timestamp> {
    (
        -1_000_000_000i64..=1_000_000_000,
        -750_000_000i32..=750_000_000,
    )
        .prop_map(|(seconds, nanos)| {
            Timestamp::try_from_parts(seconds, nanos).expect("timestamp parts must normalize")
        })
}

fn duration_strategy() -> impl Strategy<Value = Duration> {
    (-1_000_000i64..=1_000_000, -750_000_000i32..=750_000_000).prop_map(|(seconds, nanos)| {
        Duration::try_from_parts(seconds, nanos).expect("duration parts must normalize")
    })
}

fn timezone_offset_minutes_strategy() -> impl Strategy<Value = i32> {
    (
        -12i32..=12,
        prop_oneof![Just(0i32), Just(15), Just(30), Just(45)],
    )
        .prop_map(|(hours, minutes)| {
            let mut total = hours * 60;
            if hours >= 0 {
                total += minutes;
            } else {
                total -= minutes;
            }
            total
        })
}

fn timezone_from_minutes(minutes: i32) -> Timezone {
    if minutes == 0 {
        return reml_runtime::time::utc();
    }
    let sign = if minutes >= 0 { '+' } else { '-' };
    let abs_minutes = minutes.abs();
    let hours = abs_minutes / 60;
    let mins = abs_minutes % 60;
    let spec = format!("UTC{sign}{hours:02}:{mins:02}");
    timezone(spec).expect("timezone offset string must parse")
}

proptest! {
    #[test]
    fn duration_between_matches_checked_addition(
        start in timestamp_strategy(),
        delta in duration_strategy(),
    ) {
        prop_assume!(start.checked_add_duration(delta).is_ok());
        let end = start.checked_add_duration(delta).expect("checked above");
        let measured = duration_between(start, end);
        prop_assert_eq!(measured, delta);

        let reverse = duration_between(end, start);
        prop_assert_eq!(reverse.total_nanoseconds(), -delta.total_nanoseconds());
    }
}

proptest! {
    #[test]
    fn convert_timezone_is_reversible(
        ts in timestamp_strategy(),
        from_minutes in timezone_offset_minutes_strategy(),
        to_minutes in timezone_offset_minutes_strategy(),
    ) {
        let from = timezone_from_minutes(from_minutes);
        let to = timezone_from_minutes(to_minutes);
        let converted = convert_timezone(ts, from.clone(), to.clone());
        prop_assume!(converted.is_ok());
        let converted = converted.expect("assumed success");
        let reverted = convert_timezone(converted, to.clone(), from.clone());
        prop_assume!(reverted.is_ok());
        let reverted = reverted.expect("assumed success");

        prop_assert_eq!(reverted, ts);
        let delta = duration_between(ts, converted);
        let expected =
            to.offset().total_nanoseconds() - from.offset().total_nanoseconds();
        prop_assert_eq!(delta.total_nanoseconds(), expected);
    }
}
