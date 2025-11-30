use std::time::UNIX_EPOCH;

use reml_runtime::{
    io::{IoContext, IoError, IoErrorKind},
    prelude::{
        ensure::IntoDiagnostic,
        iter::EffectLabels,
    },
};
use serde_json::Value;

const EXPECTED_JSON: &str = include_str!("expected/io_error_open.json");
const SAMPLE_PATH: &str = "/tmp/config.toml";

#[test]
fn io_error_into_diagnostic_matches_expected_subset() {
    let mut context = IoContext::new("with_reader")
        .with_path(SAMPLE_PATH)
        .with_capability("io.fs.read")
        .with_bytes_processed(128)
        .with_timestamp(UNIX_EPOCH);
    context.set_effects(sample_effects());

    let actual = IoError::new(IoErrorKind::NotFound, "failed to open file")
        .with_path(SAMPLE_PATH)
        .with_context(context)
        .into_diagnostic()
        .into_json();

    let expected: Value =
        serde_json::from_str(EXPECTED_JSON).expect("expected diagnostic JSON should parse");

    assert_contains(&actual, &expected);
}

#[test]
fn unsupported_platform_error_includes_platform_metadata() {
    let context = IoContext::new("watch").with_capability("watcher.fschange");
    let diagnostic = IoError::new(
        IoErrorKind::UnsupportedPlatform,
        "watcher feature disabled",
    )
    .with_context(context)
    .with_platform("test-os")
    .with_feature("watcher.fschange")
    .into_diagnostic()
    .into_json();

    let io_extensions = diagnostic
        .get("extensions")
        .and_then(|value| value.get("io"))
        .expect("diagnostic should have io extensions");
    assert_eq!(
        io_extensions
            .get("platform")
            .and_then(Value::as_str)
            .expect("platform key missing"),
        "test-os"
    );
    assert_eq!(
        io_extensions
            .get("feature")
            .and_then(Value::as_str)
            .expect("feature key missing"),
        "watcher.fschange"
    );

    let audit_metadata = diagnostic
        .get("audit")
        .expect("diagnostic should contain audit metadata");
    assert_eq!(
        audit_metadata
            .get("io.platform")
            .and_then(Value::as_str)
            .expect("audit io.platform missing"),
        "test-os"
    );
    assert_eq!(
        audit_metadata
            .get("io.feature")
            .and_then(Value::as_str)
            .expect("audit io.feature missing"),
        "watcher.fschange"
    );
}

fn sample_effects() -> EffectLabels {
    EffectLabels {
        mem: false,
        mutating: false,
        debug: false,
        async_pending: false,
        audit: false,
        cell: false,
        rc: false,
        unicode: false,
        io: true,
        io_blocking: true,
        io_async: false,
        security: false,
        transfer: false,
        fs_sync: false,
        mem_bytes: 0,
        predicate_calls: 0,
        rc_ops: 0,
        time: false,
        time_calls: 0,
        io_blocking_calls: 1,
        io_async_calls: 0,
        fs_sync_calls: 0,
        security_events: 0,
    }
}

fn assert_contains(actual: &Value, expected: &Value) {
    match expected {
        Value::Object(expected_map) => {
            let actual_map = actual
                .as_object()
                .expect("actual JSON should contain an object");
            for (key, expected_value) in expected_map {
                let actual_value = actual_map
                    .get(key)
                    .unwrap_or_else(|| panic!("missing key `{key}` in diagnostic JSON"));
                assert_contains(actual_value, expected_value);
            }
        }
        Value::Array(expected_array) => {
            let actual_array = actual
                .as_array()
                .expect("actual JSON should contain an array");
            for (index, expected_value) in expected_array.iter().enumerate() {
                let actual_value = actual_array.get(index).unwrap_or_else(|| {
                    panic!("missing index {index} in diagnostic JSON array")
                });
                assert_contains(actual_value, expected_value);
            }
        }
        _ => assert_eq!(
            actual, expected,
            "diagnostic value mismatch (expected {expected:?}, got {actual:?})"
        ),
    }
}
