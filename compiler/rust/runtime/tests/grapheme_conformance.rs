use std::fs;
use std::path::{Path, PathBuf};

use reml_runtime::text::{segment_graphemes, Str};

const GRAPHEME_TEST_PATH: &str = "tests/data/unicode/UAX29/GraphemeBreakTest-15.1.0.txt";

#[derive(Debug)]
struct GraphemeBreakCase {
    scalars: Vec<char>,
    boundaries: Vec<usize>,
}

#[test]
#[cfg_attr(not(feature = "unicode_full"), ignore)]
fn unicode_conformance_grapheme() {
    let data_path = repo_root().join(GRAPHEME_TEST_PATH);
    assert!(
        data_path.is_file(),
        "Unicode GraphemeBreakTest data not found at {}",
        data_path.display()
    );
    let contents = fs::read_to_string(&data_path).expect("read grapheme test data");
    let mut executed = 0usize;
    for (line_idx, line) in contents.lines().enumerate() {
        let trimmed = line.split('#').next().unwrap_or(" ").trim();
        if trimmed.is_empty() {
            continue;
        }
        let case = parse_case(trimmed).unwrap_or_else(|err| {
            panic!("{} (line {})", err, line_idx + 1);
        });
        verify_case(&case, line_idx + 1);
        executed += 1;
    }
    assert!(executed > 0, "grapheme conformance data was empty");
}

fn verify_case(case: &GraphemeBreakCase, line_no: usize) {
    let text: String = case.scalars.iter().collect();
    let str_ref = Str::from(text.as_str());
    let seq = segment_graphemes(&str_ref).expect("segment");
    let actual: Vec<String> = seq.iter().map(|g| g.as_str().to_string()).collect();
    let expected = expected_clusters(case);
    assert_eq!(
        actual, expected,
        "grapheme segmentation mismatch at line {}",
        line_no
    );
}

fn parse_case(data: &str) -> Result<GraphemeBreakCase, String> {
    let mut scalars = Vec::new();
    let mut boundaries = Vec::new();
    for token in data.split_whitespace() {
        match token {
            "÷" => boundaries.push(scalars.len()),
            "×" => {}
            value => {
                let code = u32::from_str_radix(value, 16)
                    .map_err(|err| format!("invalid scalar {value}: {err}"))?;
                let ch =
                    char::from_u32(code).ok_or_else(|| format!("invalid code point {value}"))?;
                scalars.push(ch);
            }
        }
    }
    if *boundaries.last().unwrap_or(&usize::MAX) != scalars.len() {
        boundaries.push(scalars.len());
    }
    Ok(GraphemeBreakCase {
        scalars,
        boundaries,
    })
}

fn expected_clusters(case: &GraphemeBreakCase) -> Vec<String> {
    case.boundaries
        .windows(2)
        .map(|window| {
            let start = window[0];
            let end = window[1];
            case.scalars[start..end].iter().collect()
        })
        .collect()
}

fn repo_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .ancestors()
        .nth(3)
        .map(Path::to_path_buf)
        .expect("CARGO_MANIFEST_DIR should have at least 3 ancestors")
}
