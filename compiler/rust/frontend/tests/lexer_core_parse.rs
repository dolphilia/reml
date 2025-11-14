use std::fs;
use std::path::Path;
use std::str::FromStr;

use serde::Deserialize;
use serde_json::Value;

use reml_frontend::lexer::{lex_source_with_options, IdentifierProfile, LexerOptions};

#[derive(Debug, Deserialize)]
struct CoreParseCase {
    name: String,
    profile: String,
    source: String,
    tokens: Value,
}

const EXPECTED_ERRORS: &[&str] = &["core_parse_shebang_comment"];

fn load_golden_cases() -> Result<Vec<CoreParseCase>, Box<dyn std::error::Error>> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let golden_path =
        manifest_dir.join("../../ocaml/tests/golden/core_parse_lex_tests.tokens.json");
    let text = fs::read_to_string(&golden_path)?;
    let cases = serde_json::from_str::<Vec<CoreParseCase>>(&text)?;
    Ok(cases)
}

fn run_case(case: &CoreParseCase) -> Result<(), Box<dyn std::error::Error>> {
    let profile = IdentifierProfile::from_str(&case.profile).map_err(|_| {
        format!(
            "不正な profile `{}` が指定されました (case: {})",
            case.profile, case.name
        )
    })?;
    let options = LexerOptions {
        identifier_profile: profile,
    };
    let output = lex_source_with_options(&case.source, options);
    if !output.errors.is_empty() {
        if !EXPECTED_ERRORS.contains(&case.name.as_str()) {
            panic!(
                "case `{}` で字句解析エラーが発生しました: {:?}",
                case.name,
                output
                    .errors
                    .iter()
                    .map(|error| error.message())
                    .collect::<Vec<_>>()
            );
        }
    }
    let actual_tokens = serde_json::to_value(&output.tokens)?;
    assert_eq!(
        actual_tokens, case.tokens,
        "case `{}` のトークン列が期待値と一致しません",
        case.name
    );
    Ok(())
}

#[test]
fn core_parse_lex_tokens_match_ocaml_golden() -> Result<(), Box<dyn std::error::Error>> {
    let cases = load_golden_cases()?;
    for case in &cases {
        run_case(case)?;
    }
    Ok(())
}
