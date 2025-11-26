use reml_frontend::lexer::{lex_source_with_options, IdentifierProfile, LexerOptions};
use reml_runtime::text::LocaleId;

fn lexer_options(locale: Option<LocaleId>) -> LexerOptions {
    LexerOptions {
        identifier_profile: IdentifierProfile::Unicode,
        identifier_locale: locale,
    }
}

#[test]
fn rejects_non_nfc_identifier() {
    let source = "let cafe\u{0301} = 1";
    let output = lex_source_with_options(source, lexer_options(None));
    assert!(
        output
            .errors
            .iter()
            .any(|err| err.message().contains("NFC")),
        "非正規化識別子エラーが発生しませんでした: {:?}",
        output
            .errors
            .iter()
            .map(|err| err.message())
            .collect::<Vec<_>>()
    );
}

#[test]
fn rejects_bidi_control_in_identifier() {
    let source = "let foo\u{202E}bar = 1";
    let output = lex_source_with_options(source, lexer_options(None));
    assert!(
        output
            .errors
            .iter()
            .any(|err| err.message().contains("bidi")),
        "Bidi 制御文字の拒否メッセージがありません: {:?}",
        output
            .errors
            .iter()
            .map(|err| err.message())
            .collect::<Vec<_>>()
    );
}

#[test]
fn unsupported_locale_returns_error() {
    let locale = LocaleId::parse("az-Latn").expect("locale");
    let output = lex_source_with_options("let foo = 1", lexer_options(Some(locale)));
    assert!(
        output
            .errors
            .iter()
            .any(|err| err.message().contains("identifier_locale")),
        "UnsupportedLocale エラーが発生しませんでした: {:?}",
        output
            .errors
            .iter()
            .map(|err| err.message())
            .collect::<Vec<_>>()
    );
}
