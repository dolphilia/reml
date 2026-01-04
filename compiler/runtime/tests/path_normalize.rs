use std::fs;
use std::path::{Path, PathBuf as StdPathBuf};

use reml_runtime::path::PathBuf;
use serde::Deserialize;

#[cfg(target_family = "unix")]
const NORMALIZE_CASES: &str = "tests/data/core_path/normalize_posix.json";
#[cfg(target_family = "windows")]
const NORMALIZE_CASES: &str = "tests/data/core_path/normalize_windows.json";

#[derive(Debug, Deserialize)]
struct JoinCase {
    segment: String,
    result: String,
}

#[derive(Debug, Deserialize)]
struct PathCase {
    input: String,
    normalized: String,
    is_absolute: bool,
    parent: Option<String>,
    components: Vec<String>,
    #[serde(default)]
    join: Option<JoinCase>,
}

#[test]
fn normalize_and_join_follow_golden_cases() {
    let data_path = repo_root().join(NORMALIZE_CASES);
    let raw = fs::read_to_string(&data_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", data_path.display()));
    let cases: Vec<PathCase> = serde_json::from_str(&raw)
        .unwrap_or_else(|err| panic!("failed to parse {NORMALIZE_CASES}: {err}"));

    for case in cases {
        let parsed = PathBuf::try_from(case.input.as_str())
            .unwrap_or_else(|err| panic!("failed to parse `{}`: {err}", case.input));
        let normalized = parsed.normalize();
        assert_eq!(
            normalized.to_string_lossy(),
            case.normalized,
            "normalized mismatch for input `{}`",
            case.input
        );
        assert_eq!(
            normalized.is_absolute(),
            case.is_absolute,
            "absolute flag mismatch for input `{}`",
            case.input
        );

        let parent = normalized
            .parent()
            .map(|p| p.to_string_lossy().into_owned());
        assert_eq!(
            parent, case.parent,
            "parent mismatch for input `{}`",
            case.input
        );

        assert_eq!(
            normalized.components_as_strings(),
            case.components,
            "components mismatch for input `{}`",
            case.input
        );

        if let Some(join) = case.join {
            let joined = normalized
                .join(&join.segment)
                .unwrap_or_else(|err| panic!("join failed for `{}`: {err}", case.input));
            assert_eq!(
                joined.to_string_lossy(),
                join.result,
                "join result mismatch for input `{}`",
                case.input
            );
        }
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
