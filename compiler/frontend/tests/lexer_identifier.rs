use reml_frontend::{
    error::FrontendErrorKind,
    lexer::{lex_source_with_options, IdentifierProfile, LexOutput, LexerOptions},
    token::TokenKind,
    Token,
};

fn collect_tokens(source: &str, profile: IdentifierProfile) -> LexOutput {
    let options = LexerOptions {
        identifier_profile: profile,
        identifier_locale: None,
    };
    lex_source_with_options(source, options)
}

fn first_non_eof_token<'a>(output: &'a LexOutput) -> &'a Token {
    output
        .tokens
        .iter()
        .find(|token| token.kind != TokenKind::EndOfFile)
        .expect("字句列に EndOfFile 以外のトークンが含まれていません")
}

fn find_token<'a>(output: &'a LexOutput, kind: TokenKind, lexeme: &str) -> Option<&'a Token> {
    output
        .tokens
        .iter()
        .find(|token| token.kind == kind && token.lexeme.as_deref() == Some(lexeme))
}

#[test]
fn ascii_profile_accepts_basic_identifiers() {
    let profile = IdentifierProfile::AsciiCompat;
    let cases = [
        ("foo", TokenKind::Identifier, "foo"),
        ("_aux", TokenKind::Identifier, "_aux"),
        ("parseExpr", TokenKind::Identifier, "parseExpr"),
        ("parse_expr", TokenKind::Identifier, "parse_expr"),
        ("var123", TokenKind::Identifier, "var123"),
    ];

    for (source, expected_kind, expected_lexeme) in cases {
        let output = collect_tokens(source, profile);
        assert!(
            output.errors.is_empty(),
            "ASCII プロファイルでテスト `{source}` がエラーを出力しました: {:?}",
            output
                .errors
                .iter()
                .map(|err| err.message())
                .collect::<Vec<_>>()
        );
        let token = first_non_eof_token(&output);
        assert_eq!(
            token.kind, expected_kind,
            "トークン種別が期待値と異なります ({source})"
        );
        assert_eq!(
            token.lexeme.as_deref(),
            Some(expected_lexeme),
            "トークン文字列が期待値と異なります ({source})"
        );
    }
}

#[test]
fn unicode_profile_accepts_non_ascii_identifiers() {
    let profile = IdentifierProfile::Unicode;
    let cases = [
        ("let れむる = 1", TokenKind::Identifier, "れむる"),
        (
            "let ユーザー_識別子 = 1",
            TokenKind::Identifier,
            "ユーザー_識別子",
        ),
        ("let Δοκιμή = 1", TokenKind::UpperIdentifier, "Δοκιμή"),
        (
            "let пользователь = 1",
            TokenKind::Identifier,
            "пользователь",
        ),
        ("let 데이터 = 1", TokenKind::Identifier, "데이터"),
    ];

    for (source, expected_kind, expected_lexeme) in cases {
        let output = collect_tokens(source, profile);
        assert!(
            output.errors.is_empty(),
            "Unicode プロファイルで `{source}` がエラー: {:?}",
            output
                .errors
                .iter()
                .map(|err| err.message())
                .collect::<Vec<_>>()
        );
        assert!(
            find_token(&output, expected_kind, expected_lexeme).is_some(),
            "トークン `{expected_kind:?}` が存在せず、lexeme `{expected_lexeme}` も見つかりませんでした ({source})"
        );
    }
}

#[test]
fn unicode_profile_preserves_normalized_identifiers() {
    let profile = IdentifierProfile::Unicode;
    let combining_source = "let cafe\u{0301} = 1";
    let combining_output = collect_tokens(combining_source, profile);
    assert!(
        combining_output
            .errors
            .iter()
            .any(|err| matches!(err.kind, FrontendErrorKind::UnexpectedStructure { .. })),
        "NFC ではない識別子がエラーになりませんでした"
    );
    let combining_token = find_token(&combining_output, TokenKind::Unknown, "cafe\u{0301}")
        .expect("NFC でない識別子が Unknown トークンになりませんでした");
    assert!(
        combining_token.lexeme.as_deref() == Some("cafe\u{0301}"),
        "NFC でない識別子の lexeme が期待値ではありません"
    );

    let joiner_identifier = "مثال\u{200D}اختبار";
    let joiner_source = format!("let {joiner_identifier} = 1");
    let joiner_output = collect_tokens(&joiner_source, profile);
    let joiner_token = find_token(&joiner_output, TokenKind::Identifier, joiner_identifier)
        .expect("ゼロ幅結合子付き識別子が残っていません");
    let lexeme = joiner_token.lexeme.as_deref().unwrap_or("");
    assert!(
        lexeme.contains('\u{200D}'),
        "識別子がゼロ幅結合子を含みませんでした: {lexeme}"
    );
}

#[test]
fn ascii_profile_reports_unicode_rejection() {
    let output = collect_tokens("let 解析器 = 1", IdentifierProfile::AsciiCompat);
    assert!(
        output
            .errors
            .iter()
            .any(|err| matches!(err.kind, FrontendErrorKind::UnexpectedStructure { .. })),
        "ASCII プロファイルのエラーが発生しませんでした"
    );
    let error = output
        .errors
        .iter()
        .find(|err| matches!(err.kind, FrontendErrorKind::UnexpectedStructure { .. }))
        .expect("UnexpectedStructure の診断がありません");
    if let FrontendErrorKind::UnexpectedStructure {
        message,
        span: Some(span),
        ..
    } = &error.kind
    {
        assert!(
            message.contains("U+89E3"),
            "拒否メッセージにコードポイントが含まれていません: {message}"
        );
        assert!(
            message.contains("profile=ascii-compat"),
            "拒否メッセージに profile=ascii-compat が含まれていません: {message}"
        );
        assert_eq!(span.start, 4, "span.start が期待値 (4) と異なります");
        assert_eq!(span.end, 13, "span.end が期待値 (13) と異なります");
    } else {
        panic!("UnexpectedStructure 型で span 情報がありません");
    }
    assert!(
        output
            .tokens
            .iter()
            .any(|token| token.kind == TokenKind::Unknown
                && token.lexeme.as_deref() == Some("解析器")),
        "Unknown トークンに拒否された識別子が含まれていません"
    );
}

#[test]
fn context_keywords_are_tokenized_as_identifiers() {
    let output = collect_tokens(
        "let operation = 1\nlet pattern = 2",
        IdentifierProfile::Unicode,
    );
    assert!(
        output.errors.is_empty(),
        "字句解析エラーが発生しました: {:?}",
        output
            .errors
            .iter()
            .map(|err| err.message())
            .collect::<Vec<_>>()
    );
    assert!(
        find_token(&output, TokenKind::Identifier, "operation").is_some(),
        "operation が Identifier として認識されませんでした"
    );
    assert!(
        find_token(&output, TokenKind::Identifier, "pattern").is_some(),
        "pattern が Identifier として認識されませんでした"
    );
    assert!(
        !output.tokens.iter().any(|token| matches!(
            token.kind,
            TokenKind::KeywordOperation | TokenKind::KeywordPattern
        )),
        "context keyword がキーワードトークンとして残っています"
    );
}
