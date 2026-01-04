use reml_frontend::lexer::{lex_source_with_options, IdentifierProfile, LexerOptions};
use reml_frontend::{IntBase, LiteralMetadata, StringKind, TokenKind};

fn collect_tokens(source: &str, profile: IdentifierProfile) -> reml_frontend::lexer::LexOutput {
    let options = LexerOptions {
        identifier_profile: profile,
        identifier_locale: None,
    };
    lex_source_with_options(source, options)
}

#[test]
fn unicode_identifier_is_classified_correctly() {
    let output = collect_tokens("let ユーザー = 1", IdentifierProfile::Unicode);
    assert!(
        output.errors.is_empty(),
        "expected no lexer errors, got {:?}",
        output
            .errors
            .iter()
            .map(|err| err.message())
            .collect::<Vec<_>>()
    );
    let mut seen_unicode_ident = false;
    for token in output.tokens {
        if token.kind == TokenKind::Identifier && token.lexeme.as_deref() == Some("ユーザー") {
            seen_unicode_ident = true;
        }
    }
    assert!(seen_unicode_ident, "Unicode 識別子が認識されていません");
}

#[test]
fn upper_identifier_is_separated_from_lowercase() {
    let output = collect_tokens("perform EffectOp value", IdentifierProfile::Unicode);
    let mut has_upper = false;
    for token in output.tokens {
        if token.kind == TokenKind::UpperIdentifier && token.lexeme.as_deref() == Some("EffectOp") {
            has_upper = true;
        }
    }
    assert!(has_upper, "UpperIdentifier が生成されていません");
}

#[test]
fn ascii_profile_rejects_unicode_identifiers() {
    let output = collect_tokens("let ユーザー = 1", IdentifierProfile::AsciiCompat);
    assert!(
        !output.errors.is_empty(),
        "ASCII プロファイルではエラーを検出するはずです"
    );
    assert!(
        output
            .tokens
            .iter()
            .any(|token| token.kind == TokenKind::Unknown),
        "ASCII プロファイルでは Unknown トークンが生成されるはずです"
    );
}

#[test]
fn int_literal_bases_are_preserved() {
    let output = collect_tokens("0b1010 0o755 42 0xFF", IdentifierProfile::Unicode);
    let bases: Vec<_> = output
        .tokens
        .iter()
        .filter_map(|token| match token.literal {
            Some(LiteralMetadata::Int { base }) => Some(base),
            _ => None,
        })
        .collect();
    assert_eq!(
        bases,
        vec![
            IntBase::Binary,
            IntBase::Octal,
            IntBase::Decimal,
            IntBase::Hexadecimal
        ]
    );
}

#[test]
fn string_kinds_are_distinguished() {
    let source = "\"regular\" r\"raw\" \"\"\"multi\nline\"\"\" 'c'";
    let output = collect_tokens(source, IdentifierProfile::Unicode);
    let mut has_normal = false;
    let mut has_raw = false;
    let mut has_multi = false;
    let mut has_char = false;
    for token in output.tokens {
        match token.literal {
            Some(LiteralMetadata::String {
                kind: StringKind::Normal,
            }) => has_normal = true,
            Some(LiteralMetadata::String {
                kind: StringKind::Raw,
            }) => has_raw = true,
            Some(LiteralMetadata::String {
                kind: StringKind::Multiline,
            }) => has_multi = true,
            Some(LiteralMetadata::Char) => has_char = true,
            _ => {}
        }
    }
    assert!(
        has_normal && has_raw && has_multi && has_char,
        "文字列/文字リテラルの種別が欠けています (normal={has_normal}, raw={has_raw}, multi={has_multi}, char={has_char})"
    );
}

#[test]
fn pipe_and_channel_tokens_exist() {
    let output = collect_tokens("value |> handler ~> monitor", IdentifierProfile::Unicode);
    assert!(output
        .tokens
        .iter()
        .any(|token| token.kind == TokenKind::PipeForward));
    assert!(output
        .tokens
        .iter()
        .any(|token| token.kind == TokenKind::ChannelPipe));
}
