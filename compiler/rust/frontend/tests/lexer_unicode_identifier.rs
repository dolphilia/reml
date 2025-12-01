use reml_frontend::{
    error::FrontendErrorKind,
    lexer::{lex_source_with_options, IdentifierProfile, LexOutput, LexerOptions},
    token::{Token, TokenKind},
};
use reml_runtime::text::LocaleId;

#[derive(Debug)]
struct SuccessCase<'a> {
    name: &'a str,
    identifier: &'a str,
    expected_lexeme: &'a str,
    expected_kind: TokenKind,
    locale: Option<&'a str>,
}

#[derive(Debug)]
struct FailureCase<'a> {
    name: &'a str,
    identifier: &'a str,
    expected_unknown: &'a str,
    expected_message_fragments: &'a [&'a str],
    locale: Option<&'a str>,
}

fn lex_unicode(source: &str, locale: Option<&str>) -> LexOutput {
    let locale = locale.map(|tag| LocaleId::parse(tag).expect("ロケール文字列が正しくありません"));
    let options = LexerOptions {
        identifier_profile: IdentifierProfile::Unicode,
        identifier_locale: locale,
    };
    lex_source_with_options(source, options)
}

fn find_token<'a>(output: &'a LexOutput, kind: TokenKind, lexeme: &str) -> Option<&'a Token> {
    output
        .tokens
        .iter()
        .find(|token| token.kind == kind && token.lexeme.as_deref() == Some(lexeme))
}

const SUCCESS_CASES: &[SuccessCase<'_>] = &[
    SuccessCase {
        name: "normalized_latin",
        identifier: "café",
        expected_lexeme: "café",
        expected_kind: TokenKind::Identifier,
        locale: None,
    },
    SuccessCase {
        name: "cjk_fullwidth",
        identifier: "解析器入力",
        expected_lexeme: "解析器入力",
        expected_kind: TokenKind::Identifier,
        locale: None,
    },
    SuccessCase {
        name: "emoji_joiner_cluster",
        identifier: "foo👨‍👩‍👧‍👦bar",
        expected_lexeme: "foo👨‍👩‍👧‍👦bar",
        expected_kind: TokenKind::Identifier,
        locale: None,
    },
    SuccessCase {
        name: "zwj_arabic",
        identifier: "مثال\u{200D}اختبار",
        expected_lexeme: "مثال\u{200D}اختبار",
        expected_kind: TokenKind::Identifier,
        locale: None,
    },
    SuccessCase {
        name: "uppercase_greek",
        identifier: "Δοκιμή",
        expected_lexeme: "Δοκιμή",
        expected_kind: TokenKind::UpperIdentifier,
        locale: None,
    },
    SuccessCase {
        name: "locale_tr_supported",
        identifier: "k\u{0131}demli",
        expected_lexeme: "k\u{0131}demli",
        expected_kind: TokenKind::Identifier,
        locale: Some("tr-TR"),
    },
];

const FAILURE_CASES: &[FailureCase<'_>] = &[
    FailureCase {
        name: "non_nfc_combining",
        identifier: "cafe\u{0301}",
        expected_unknown: "cafe\u{0301}",
        expected_message_fragments: &["NFC"],
        locale: None,
    },
    FailureCase {
        name: "bidi_rlo",
        identifier: "foo\u{202E}bar",
        expected_unknown: "foo\u{202E}bar",
        expected_message_fragments: &["U+202E"],
        locale: None,
    },
    FailureCase {
        name: "bidi_lri",
        identifier: "foo\u{2066}bar",
        expected_unknown: "foo\u{2066}bar",
        expected_message_fragments: &["U+2066"],
        locale: None,
    },
    FailureCase {
        name: "bidi_rli",
        identifier: "foo\u{2067}bar",
        expected_unknown: "foo\u{2067}bar",
        expected_message_fragments: &["U+2067"],
        locale: None,
    },
    FailureCase {
        name: "bidi_lre",
        identifier: "foo\u{202A}bar",
        expected_unknown: "foo\u{202A}bar",
        expected_message_fragments: &["U+202A"],
        locale: None,
    },
    FailureCase {
        name: "unsupported_locale",
        identifier: "foo",
        expected_unknown: "foo",
        expected_message_fragments: &["lex.identifier_locale", "az-Latn"],
        locale: Some("az-Latn"),
    },
];

#[test]
fn unicode_identifier_success_matrix() {
    for case in SUCCESS_CASES {
        let source = format!("let {} = 1", case.identifier);
        let output = lex_unicode(&source, case.locale);
        assert!(
            output.errors.is_empty(),
            "ケース `{}` で診断が発生しました: {:?}",
            case.name,
            output
                .errors
                .iter()
                .map(|err| err.message())
                .collect::<Vec<_>>()
        );
        let token = find_token(&output, case.expected_kind, case.expected_lexeme).unwrap_or_else(
            || {
                panic!(
                    "ケース `{}` の識別子 `{}` が TokenKind::{:?} として出現しませんでした。tokens={:?}",
                    case.name, case.expected_lexeme, case.expected_kind, output.tokens
                )
            },
        );
        assert_eq!(
            token.lexeme.as_deref(),
            Some(case.expected_lexeme),
            "ケース `{}` の lexeme が期待値と異なります",
            case.name
        );
    }
}

#[test]
fn unicode_identifier_error_matrix() {
    for case in FAILURE_CASES {
        let source = format!("let {} = 1", case.identifier);
        let output = lex_unicode(&source, case.locale);
        assert!(
            !output.errors.is_empty(),
            "ケース `{}` でエラーが発生せず、`prepare_identifier` の検証ができません",
            case.name
        );
        assert_eq!(
            output.errors.len(),
            1,
            "ケース `{}` は単一エラーを想定しています: {:?}",
            case.name,
            output
                .errors
                .iter()
                .map(|err| err.message())
                .collect::<Vec<_>>()
        );
        let error = &output.errors[0];
        let (message, span) = match &error.kind {
            FrontendErrorKind::UnexpectedStructure {
                message,
                span: Some(span),
                ..
            } => (message.clone(), *span),
            other => panic!(
                "ケース `{}` が UnexpectedStructure 以外のエラーを返しました: {other:?}",
                case.name
            ),
        };
        for fragment in case.expected_message_fragments {
            assert!(
                message.contains(fragment),
                "ケース `{}` のエラーメッセージに `{}` が含まれていません: {}",
                case.name,
                fragment,
                message
            );
        }
        let unknown = find_token(&output, TokenKind::Unknown, case.expected_unknown)
            .unwrap_or_else(|| {
                panic!(
                    "ケース `{}` で Unknown トークン `{}` が検出されませんでした。tokens={:?}",
                    case.name, case.expected_unknown, output.tokens
                )
            });
        assert_eq!(
            unknown.span, span,
            "ケース `{}` の Unknown トークンと診断のスパンが一致しません（token={:?}, diag={:?}）",
            case.name, unknown.span, span
        );
    }
}
