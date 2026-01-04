use std::fs;
use std::path::{Path, PathBuf as StdPathBuf};

use reml_runtime::{path, prelude::ensure::IntoDiagnostic, text::Str};
use serde::Deserialize;
use serde_json::{json, Value};

#[cfg(target_family = "unix")]
const CASES_FILE: &str = "tests/data/core_path/glob_posix.json";
#[cfg(target_family = "windows")]
const CASES_FILE: &str = "tests/data/core_path/glob_windows.json";

#[derive(Debug, Deserialize)]
struct GlobCase {
    pattern: String,
    expected: Vec<String>,
}

#[test]
fn glob_matches_expected_paths() {
    let cases = load_cases();
    for case in cases {
        let pattern_path = repo_root().join(&case.pattern);
        let pattern_str = pattern_path
            .to_str()
            .unwrap_or_else(|| panic!("pattern contains invalid UTF-8: {}", case.pattern));
        let matches = path::glob(Str::from(pattern_str))
            .unwrap_or_else(|err| panic!("glob failed for `{}`: {err}", case.pattern));

        let mut actual = normalize_matches(matches);
        let mut expected = normalize_expected(&case.expected);
        actual.sort();
        expected.sort();

        assert_eq!(
            actual, expected,
            "glob results mismatch for pattern `{}`",
            case.pattern
        );
    }
}

#[test]
fn glob_invalid_pattern_reports_diagnostic_metadata() {
    let invalid_pattern = repo_root().join("[").to_string_lossy().into_owned();
    let error = path::glob(Str::from(invalid_pattern.as_str()))
        .expect_err("invalid patterns should produce an error");
    let diagnostic = error.into_diagnostic().into_json();
    let expected = json!({
        "code": "core.path.glob.invalid_pattern",
        "extensions": {
            "io": {
                "glob": {
                    "pattern": invalid_pattern
                }
            }
        },
        "audit": {
            "io.glob.pattern": invalid_pattern
        }
    });
    assert_contains(&diagnostic, &expected);
}

fn load_cases() -> Vec<GlobCase> {
    let path = repo_root().join(CASES_FILE);
    let raw = fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
    serde_json::from_str(&raw)
        .unwrap_or_else(|err| panic!("invalid JSON in {}: {err}", path.display()))
}

fn normalize_matches(matches: Vec<path::PathBuf>) -> Vec<String> {
    matches
        .into_iter()
        .map(|path_buf| path_buf.to_string_lossy().into_owned())
        .collect()
}

fn normalize_expected(entries: &[String]) -> Vec<String> {
    let root = repo_root();
    entries
        .iter()
        .map(|entry| root.join(entry).to_string_lossy().into_owned())
        .collect()
}

fn repo_root() -> StdPathBuf {
    let manifest_dir = StdPathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .ancestors()
        .nth(2)
        .map(Path::to_path_buf)
        .expect("CARGO_MANIFEST_DIR should have at least 2 ancestors")
}

fn assert_contains(actual: &Value, expected: &Value) {
    match expected {
        Value::Object(map) => {
            let actual_map = actual.as_object().expect("actual JSON should be an object");
            for (key, expected_value) in map {
                let actual_value = actual_map
                    .get(key)
                    .unwrap_or_else(|| panic!("missing key `{key}` in diagnostic JSON"));
                assert_contains(actual_value, expected_value);
            }
        }
        Value::Array(expected_array) => {
            let actual_array = actual.as_array().expect("actual JSON must be an array");
            for (index, expected_value) in expected_array.iter().enumerate() {
                let actual_value = actual_array
                    .get(index)
                    .unwrap_or_else(|| panic!("missing index {index} in diagnostic JSON array"));
                assert_contains(actual_value, expected_value);
            }
        }
        _ => assert_eq!(
            actual, expected,
            "diagnostic value mismatch (expected {expected:?}, got {actual:?})"
        ),
    }
}
