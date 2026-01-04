use std::fs;
use std::path::{Path, PathBuf};

use reml_runtime::text::{NormalizationForm, String as TextString};

const NORMALIZATION_TEST_PATH: &str = "tests/data/unicode/UAX15/NormalizationTest-15.1.0.txt";

#[derive(Debug)]
struct NormalizationRow {
    c1: String,
    c2: String,
    c3: String,
    c4: String,
    c5: String,
}

#[test]
#[cfg_attr(not(feature = "unicode_full"), ignore)]
fn unicode_conformance_normalization() {
    let data_path = repo_root().join(NORMALIZATION_TEST_PATH);
    assert!(
        data_path.is_file(),
        "Unicode NormalizationTest data not found at {}",
        data_path.display()
    );
    let contents = fs::read_to_string(&data_path).expect("read normalization test data");
    let mut executed = 0usize;
    for (line_idx, line) in contents.lines().enumerate() {
        let trimmed = line.split('#').next().unwrap_or("").trim();
        if trimmed.is_empty() || trimmed.starts_with('@') {
            continue;
        }
        let row = parse_row(trimmed).unwrap_or_else(|err| {
            panic!("{} (line {})", err, line_idx + 1);
        });
        verify_row(&row, line_idx + 1);
        executed += 1;
    }
    assert!(executed > 0, "normalization conformance data was empty");
}

fn verify_row(row: &NormalizationRow, line_no: usize) {
    // NFC invariants
    assert_normalizes_to(&row.c1, &row.c2, NormalizationForm::Nfc, "NFC(c1)", line_no);
    assert_normalizes_to(&row.c2, &row.c2, NormalizationForm::Nfc, "NFC(c2)", line_no);
    assert_normalizes_to(&row.c3, &row.c2, NormalizationForm::Nfc, "NFC(c3)", line_no);
    assert_normalizes_to(&row.c4, &row.c4, NormalizationForm::Nfc, "NFC(c4)", line_no);
    assert_normalizes_to(&row.c5, &row.c4, NormalizationForm::Nfc, "NFC(c5)", line_no);

    // NFD invariants
    assert_normalizes_to(&row.c1, &row.c3, NormalizationForm::Nfd, "NFD(c1)", line_no);
    assert_normalizes_to(&row.c2, &row.c3, NormalizationForm::Nfd, "NFD(c2)", line_no);
    assert_normalizes_to(&row.c3, &row.c3, NormalizationForm::Nfd, "NFD(c3)", line_no);
    assert_normalizes_to(&row.c4, &row.c5, NormalizationForm::Nfd, "NFD(c4)", line_no);
    assert_normalizes_to(&row.c5, &row.c5, NormalizationForm::Nfd, "NFD(c5)", line_no);

    // NFKC invariants
    assert_normalizes_to(
        &row.c1,
        &row.c4,
        NormalizationForm::Nfkc,
        "NFKC(c1)",
        line_no,
    );
    assert_normalizes_to(
        &row.c2,
        &row.c4,
        NormalizationForm::Nfkc,
        "NFKC(c2)",
        line_no,
    );
    assert_normalizes_to(
        &row.c3,
        &row.c4,
        NormalizationForm::Nfkc,
        "NFKC(c3)",
        line_no,
    );
    assert_normalizes_to(
        &row.c4,
        &row.c4,
        NormalizationForm::Nfkc,
        "NFKC(c4)",
        line_no,
    );
    assert_normalizes_to(
        &row.c5,
        &row.c4,
        NormalizationForm::Nfkc,
        "NFKC(c5)",
        line_no,
    );

    // NFKD invariants
    assert_normalizes_to(
        &row.c1,
        &row.c5,
        NormalizationForm::Nfkd,
        "NFKD(c1)",
        line_no,
    );
    assert_normalizes_to(
        &row.c2,
        &row.c5,
        NormalizationForm::Nfkd,
        "NFKD(c2)",
        line_no,
    );
    assert_normalizes_to(
        &row.c3,
        &row.c5,
        NormalizationForm::Nfkd,
        "NFKD(c3)",
        line_no,
    );
    assert_normalizes_to(
        &row.c4,
        &row.c5,
        NormalizationForm::Nfkd,
        "NFKD(c4)",
        line_no,
    );
    assert_normalizes_to(
        &row.c5,
        &row.c5,
        NormalizationForm::Nfkd,
        "NFKD(c5)",
        line_no,
    );
}

fn assert_normalizes_to(
    input: &str,
    expected: &str,
    form: NormalizationForm,
    label: &str,
    line_no: usize,
) {
    let actual = normalize_text(input, form);
    assert_eq!(actual, expected, "{} mismatch at line {}", label, line_no);
}

fn normalize_text(input: &str, form: NormalizationForm) -> String {
    TextString::from_str(input)
        .normalize(form)
        .expect("normalize")
        .into_std()
}

fn parse_row(line: &str) -> Result<NormalizationRow, String> {
    let mut sequences: Vec<String> = Vec::with_capacity(5);
    for column in line.split(';') {
        let trimmed = column.trim();
        if trimmed.is_empty() {
            continue;
        }
        sequences.push(parse_sequence(trimmed)?);
    }
    if sequences.len() != 5 {
        return Err(format!(
            "expected 5 columns, found {} entries",
            sequences.len()
        ));
    }
    let mut iter = sequences.into_iter();
    Ok(NormalizationRow {
        c1: iter.next().unwrap(),
        c2: iter.next().unwrap(),
        c3: iter.next().unwrap(),
        c4: iter.next().unwrap(),
        c5: iter.next().unwrap(),
    })
}

fn parse_sequence(column: &str) -> Result<String, String> {
    let mut result = String::new();
    for value in column.split_whitespace() {
        if value.is_empty() {
            continue;
        }
        let scalar = u32::from_str_radix(value, 16)
            .map_err(|err| format!("invalid scalar {value:?}: {err}"))?;
        let ch = char::from_u32(scalar).ok_or_else(|| format!("invalid code point {value:?}"))?;
        result.push(ch);
    }
    Ok(result)
}

fn repo_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .ancestors()
        .nth(3)
        .map(Path::to_path_buf)
        .expect("CARGO_MANIFEST_DIR should have at least 3 ancestors")
}
