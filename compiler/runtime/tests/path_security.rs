use std::fs;
use std::path::{Path, PathBuf as StdPathBuf};

use reml_runtime::{
    path::{
        is_safe_symlink, sandbox_path, validate_path, PathBuf, PathSecurityErrorKind,
        SecurityPolicy,
    },
    prelude::ensure::IntoDiagnostic,
};
use serde_json::Value;

const RELATIVE_DENIED_JSON: &str = "tests/data/core_path/security/relative_denied.json";
const SANDBOX_ESCAPE_JSON: &str = "tests/data/core_path/security/sandbox_escape.json";
const SYMLINK_ABSOLUTE_JSON: &str = "tests/data/core_path/security/symlink_absolute.json";

#[test]
fn validate_path_rejects_relative_input() {
    let policy = SecurityPolicy::new().add_allowed_root(sample_root());
    let path = PathBuf::try_from("config.yaml").expect("valid path literal");
    let err = validate_path(&path, &policy).expect_err("policy should reject relative paths");
    assert_eq!(err.kind(), PathSecurityErrorKind::InvalidInput);
    let actual = err.into_diagnostic().into_json();
    let expected = load_expected(RELATIVE_DENIED_JSON);
    assert_contains(&actual, &expected);
}

#[test]
fn sandbox_path_detects_escape() {
    let root = sample_root();
    let forbidden = PathBuf::try_from("../secrets/ssh.key").expect("relative path");
    let err = sandbox_path(&forbidden, &root).expect_err("sandbox must block traversal");
    assert_eq!(err.kind(), PathSecurityErrorKind::SandboxViolation);
    let actual = err.into_diagnostic().into_json();
    let expected = load_expected(SANDBOX_ESCAPE_JSON);
    assert_contains(&actual, &expected);
}

#[cfg(unix)]
#[test]
fn is_safe_symlink_rejects_absolute_target() {
    use std::os::unix::fs::symlink;
    let dir = tempfile::tempdir().expect("tempdir");
    let link_path = dir.path().join("config.yaml");
    symlink("/etc/passwd", &link_path).expect("symlink creation");

    let path_buf = PathBuf::from_std(link_path.clone());
    let err = is_safe_symlink(&path_buf).expect_err("absolute targets should be blocked");
    assert_eq!(err.kind(), PathSecurityErrorKind::SymlinkViolation);

    let actual = err.into_diagnostic().into_json();
    let expected = load_expected(SYMLINK_ABSOLUTE_JSON);
    assert_contains(&actual, &expected);
}

fn load_expected(relative: &str) -> Value {
    let path = repo_root().join(relative);
    let raw = fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
    serde_json::from_str(&raw)
        .unwrap_or_else(|err| panic!("invalid JSON in {}: {err}", path.display()))
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
            let actual_array = actual.as_array().expect("actual JSON should be an array");
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

fn sample_root() -> PathBuf {
    #[cfg(target_family = "windows")]
    {
        PathBuf::try_from(r"C:\reml\sandbox").expect("windows root literal")
    }
    #[cfg(not(target_family = "windows"))]
    {
        PathBuf::try_from("/tmp/reml/sandbox").expect("posix root literal")
    }
}

fn repo_root() -> StdPathBuf {
    let manifest_dir = StdPathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .ancestors()
        .nth(2)
        .map(Path::to_path_buf)
        .expect("CARGO_MANIFEST_DIR should have at least 3 ancestors")
}
