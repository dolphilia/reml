use std::fs;
use std::path::{Path, PathBuf as StdPathBuf};

use reml_runtime::{path, text::Str};
use serde::Deserialize;

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
        .nth(3)
        .map(Path::to_path_buf)
        .expect("CARGO_MANIFEST_DIR should have at least 3 ancestors")
}
