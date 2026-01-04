use std::fs;
use std::path::{Path, PathBuf};

use reml_runtime::path::{
    is_absolute_str, join_paths_str, normalize_path_str, relative_to, PathStyle,
};
use reml_runtime::text::Str;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct JoinCase {
    parts: Vec<String>,
    result: String,
}

#[derive(Debug, Deserialize)]
struct RelativeCase {
    base: String,
    target: String,
    result: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum CaseStyle {
    Native,
    Posix,
    Windows,
}

impl From<CaseStyle> for PathStyle {
    fn from(value: CaseStyle) -> Self {
        match value {
            CaseStyle::Native => PathStyle::Native,
            CaseStyle::Posix => PathStyle::Posix,
            CaseStyle::Windows => PathStyle::Windows,
        }
    }
}

#[derive(Debug, Deserialize)]
struct UnicodeCase {
    description: String,
    style: CaseStyle,
    input: String,
    normalized: String,
    absolute: bool,
    #[serde(default)]
    join: Option<JoinCase>,
    #[serde(default)]
    relative: Option<RelativeCase>,
}

#[test]
fn unicode_string_cases_follow_golden() {
    let cases_path = repo_root().join("tests/data/core_path/unicode_cases.json");
    let raw = fs::read_to_string(&cases_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", cases_path.display()));
    let cases: Vec<UnicodeCase> = serde_json::from_str(&raw)
        .unwrap_or_else(|err| panic!("failed to parse unicode cases: {err}"));

    for case in cases {
        let style = PathStyle::from(case.style);
        let normalized = normalize_path_str(Str::from(case.input.as_str()), style)
            .unwrap_or_else(|err| panic!("normalize failed for {}: {err}", case.description));
        assert_eq!(
            normalized.as_str(),
            case.normalized,
            "normalized mismatch for {}",
            case.description
        );
        assert_eq!(
            is_absolute_str(Str::from(case.input.as_str()), style),
            case.absolute,
            "absolute flag mismatch for {}",
            case.description
        );

        if let Some(join_case) = case.join {
            let segments: Vec<Str<'_>> = join_case
                .parts
                .iter()
                .map(|segment| Str::from(segment.as_str()))
                .collect();
            let joined = join_paths_str(&segments, style)
                .unwrap_or_else(|err| panic!("join failed for {}: {err}", case.description));
            assert_eq!(
                joined.as_str(),
                join_case.result,
                "join result mismatch for {}",
                case.description
            );
        }

        if let Some(relative_case) = case.relative {
            let rel = relative_to(
                Str::from(relative_case.base.as_str()),
                Str::from(relative_case.target.as_str()),
                style,
            )
            .unwrap_or_else(|err| panic!("relative_to failed for {}: {err}", case.description));
            assert_eq!(
                rel.as_str(),
                relative_case.result,
                "relative path mismatch for {}",
                case.description
            );
        }
    }
}

#[test]
fn native_style_matches_effective_style() {
    #[cfg(target_family = "unix")]
    {
        let sample = "/var//log/../tmp";
        let native =
            normalize_path_str(Str::from(sample), PathStyle::Native).expect("native normalize");
        let posix =
            normalize_path_str(Str::from(sample), PathStyle::Posix).expect("posix normalize");
        assert_eq!(native.as_str(), posix.as_str());
        assert!(is_absolute_str(Str::from("/etc"), PathStyle::Native));
    }

    #[cfg(target_family = "windows")]
    {
        let sample = r"C:\Work\.\Logs\..\Current";
        let native =
            normalize_path_str(Str::from(sample), PathStyle::Native).expect("native normalize");
        let win =
            normalize_path_str(Str::from(sample), PathStyle::Windows).expect("windows normalize");
        assert_eq!(native.as_str(), win.as_str());
        assert!(is_absolute_str(Str::from(r"C:\Windows"), PathStyle::Native));
    }
}

fn repo_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .ancestors()
        .nth(2)
        .map(Path::to_path_buf)
        .expect("CARGO_MANIFEST_DIR should have at least 3 ancestors")
}
