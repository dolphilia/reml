//! OCaml 版 `parser_driver` と同等の責務を担う Rust フロントエンド PoC。

use chumsky::error::{Simple, SimpleReason};
use chumsky::prelude::*;
use chumsky::stream::Stream;
use std::ops::Range;

pub mod ast;

use crate::diagnostic::{DiagnosticNote, FrontendDiagnostic};
use crate::error::{FrontendError, Recoverability};
use crate::lexer::{lex_source, LexOutput};
use crate::span::Span;
use crate::token::{Token, TokenKind};
use ast::{Expr, Function, Module, Param};

/// パース結果の簡易表現。
#[derive(Debug, Default)]
pub struct ParsedModule {
    pub tokens: Vec<Token>,
    pub diagnostics: Vec<FrontendDiagnostic>,
    pub ast: Option<Module>,
}

impl ParsedModule {
    pub fn ast_render(&self) -> Option<String> {
        self.ast.as_ref().map(Module::render)
    }
}

/// Rust フロントエンドのパーサドライバ。
pub struct ParserDriver;

impl ParserDriver {
    pub fn parse(source: &str) -> ParsedModule {
        let LexOutput { tokens, errors } = lex_source(source);
        let mut diagnostics: Vec<FrontendDiagnostic> =
            errors.into_iter().map(Self::error_to_diagnostic).collect();

        let (ast, parse_errors) = parse_tokens(&tokens, source);
        diagnostics.extend(parse_errors.into_iter().map(|err| {
            let (message, expected_tokens) = err.1;
            let mut diagnostic = FrontendDiagnostic::new(message);
            if let Some(span) = err.0 {
                diagnostic = diagnostic.with_span(span);
            }
            if !expected_tokens.is_empty() {
                diagnostic.add_note(
                    DiagnosticNote::new("recover.expected_tokens", expected_tokens.join(", ")),
                );
            }
            diagnostic.with_recoverability(Recoverability::Recoverable)
        }));

        ParsedModule {
            tokens,
            diagnostics,
            ast,
        }
    }

    fn error_to_diagnostic(error: FrontendError) -> FrontendDiagnostic {
        let mut diagnostic = FrontendDiagnostic::new(error.message());

        match error.recoverability {
            Recoverability::Recoverable => {
                diagnostic = diagnostic.with_recoverability(Recoverability::Recoverable);
            }
            Recoverability::Fatal => {}
        }

        match error.kind {
            crate::error::FrontendErrorKind::UnknownToken { span } => {
                diagnostic = diagnostic.with_span(span);
                diagnostic.add_note(
                    DiagnosticNote::new("lexer", "未定義のトークンをスキップします")
                        .with_span(span),
                );
            }
            crate::error::FrontendErrorKind::MissingToken { span, .. } => {
                diagnostic = diagnostic.with_span(span);
            }
            crate::error::FrontendErrorKind::UnexpectedStructure { span, .. } => {
                if let Some(span) = span {
                    diagnostic = diagnostic.with_span(span);
                }
            }
            crate::error::FrontendErrorKind::InternalState { .. } => {}
        }

        diagnostic
    }
}

fn parse_tokens(
    tokens: &[Token],
    source: &str,
) -> (Option<Module>, Vec<(Option<Span>, (String, Vec<String>))>) {
    let token_pairs: Vec<_> = tokens
        .iter()
        .filter(|token| token.kind != TokenKind::Whitespace)
        .map(|token| {
            let span = token.span;
            (token.kind, (span.start as usize)..(span.end as usize))
        })
        .collect();

    let end = source.len();
    let parser = module_parser(source);
    let (ast, errors) = parser.parse_recovery(Stream::from_iter(end..end, token_pairs.into_iter()));

    let mapped_errors = errors
        .into_iter()
        .map(|err| {
            let span = Some(convert_range(err.span()));
            let message = format_simple_error(&err);
            (span, message)
        })
        .collect();

    (ast, mapped_errors)
}

fn convert_range(range: Range<usize>) -> Span {
    Span::new(range.start as u32, range.end as u32)
}

fn format_simple_error(err: &Simple<TokenKind>) -> (String, Vec<String>) {
    let expected_tokens: Vec<String> = err
        .expected()
        .filter_map(|opt| opt.map(|kind| format!("{kind:?}")))
        .collect();

    let message = match err.reason() {
        SimpleReason::Unexpected | SimpleReason::Unclosed { .. } => {
            "構文エラー: 入力を解釈できません".to_string()
        }
        SimpleReason::Custom(msg) => msg.clone(),
    };

    (message, expected_tokens)
}

fn module_parser<'src>(
    source: &'src str,
) -> impl Parser<TokenKind, Module, Error = Simple<TokenKind>> + Clone + 'src {
    let span_to_span = |span: Range<usize>| Span::new(span.start as u32, span.end as u32);

    let identifier = just(TokenKind::Identifier).map_with_span(move |_, span: Range<usize>| {
        let slice = &source[span.start..span.end];
        (slice.to_string(), span_to_span(span))
    });

    let int_literal = just(TokenKind::IntLiteral).map_with_span(move |_, span: Range<usize>| {
        let slice = &source[span.start..span.end];
        let value = slice.parse::<i64>().unwrap_or_default();
        Expr::int(value, span_to_span(span))
    });

    let expr = recursive(|expr| {
        let ident_expr = identifier
            .clone()
            .map(|(name, span)| Expr::identifier(name, span));

        let paren_expr = expr
            .clone()
            .delimited_by(just(TokenKind::LParen), just(TokenKind::RParen));

        let atom = choice((int_literal.clone(), ident_expr, paren_expr)).boxed();

        let args = expr
            .clone()
            .separated_by(just(TokenKind::Comma))
            .allow_trailing()
            .delimited_by(just(TokenKind::LParen), just(TokenKind::RParen));

        let call = atom
            .clone()
            .then(args.repeated())
            .map(|(callee, arg_lists)| {
                arg_lists.into_iter().fold(callee, |acc, args| {
                    let span = args
                        .iter()
                        .fold(acc.span(), |span, arg| span_union(span, arg.span()));
                    Expr::call(acc, args, span)
                })
            })
            .boxed();

        let plus = filter_map(move |span: Range<usize>, kind| {
            if kind == TokenKind::Operator {
                let text = &source[span.start..span.end];
                if text == "+" {
                    Ok(span_to_span(span))
                } else {
                    Err(Simple::custom(
                        span.clone(),
                        format!("未対応の演算子 `{text}`"),
                    ))
                }
            } else {
                Err(Simple::custom(
                    span.clone(),
                    format!("`+` 演算子を期待しましたが `{kind:?}` でした"),
                ))
            }
        });

        call.clone()
            .then(plus.then(call).map(|(_, rhs)| rhs).repeated())
            .map(|(first, rest)| {
                rest.into_iter().fold(first, |lhs, rhs| {
                    let span = span_union(lhs.span(), rhs.span());
                    Expr::binary("+", lhs, rhs, span)
                })
            })
    });

    let params = identifier
        .clone()
        .map(|(name, span)| Param { name, span })
        .separated_by(just(TokenKind::Comma))
        .allow_trailing()
        .delimited_by(just(TokenKind::LParen), just(TokenKind::RParen));

    let function = just(TokenKind::KeywordFn)
        .map_with_span(move |_, span: Range<usize>| span_to_span(span))
        .then(identifier.clone())
        .then(params)
        .then_ignore(just(TokenKind::Assign))
        .then(expr)
        .map(|(((fn_span, (name, name_span)), params), body)| Function {
            name,
            params,
            span: Span::new(fn_span.start.min(name_span.start), body.span().end),
            body,
        });

    function
        .repeated()
        .at_least(1)
        .then_ignore(just(TokenKind::EndOfFile).or_not())
        .map(|functions| Module { functions })
}

fn span_union(left: Span, right: Span) -> Span {
    Span::new(left.start.min(right.start), left.end.max(right.end))
}

#[cfg(test)]
mod driver {
    use super::*;

    struct Case {
        source: &'static str,
        expected_ast: Option<&'static str>,
        expected_messages: &'static [&'static str],
    }

    fn run_case(case: &Case) {
        let result = ParserDriver::parse(case.source);

        match (result.ast_render(), case.expected_ast) {
            (Some(actual), Some(expected)) => assert_eq!(actual, expected),
            (None, None) => {}
            (actual, expected) => panic!("AST mismatch: actual={actual:?}, expected={expected:?}"),
        }

        assert_eq!(
            result.diagnostics.len(),
            case.expected_messages.len(),
            "diagnostic count mismatch"
        );

        for (diag, expected_message) in result.diagnostics.iter().zip(case.expected_messages.iter())
        {
            assert_eq!(&diag.message, expected_message);
        }
    }

    #[test]
    fn basic_roundtrip() {
        let cases = [
            Case {
                source: "fn answer() = 42",
                expected_ast: Some("fn answer() = int(42:base10)"),
                expected_messages: &[],
            },
            Case {
                source: "fn log(x) = x\nfn log_twice(x) = log(log(x))",
                expected_ast: Some(
                    "fn log(x) = var(x)\nfn log_twice(x) = call(var(log))[call(var(log))[var(x)]]",
                ),
                expected_messages: &[],
            },
            Case {
                source: "fn add(x, y) = x + y",
                expected_ast: Some("fn add(x, y) = binary(var(x) + var(y))"),
                expected_messages: &[],
            },
            Case {
                source: "fn missing(x = x",
                expected_ast: None,
                expected_messages: &["構文エラー: 入力を解釈できません"],
            },
        ];

        for case in cases.iter() {
            run_case(case);
        }

        let error_case = ParserDriver::parse("fn missing(x = x");
        assert_eq!(error_case.diagnostics.len(), 1);
        let diag = &error_case.diagnostics[0];
        assert_eq!(diag.message, "構文エラー: 入力を解釈できません");
        assert!(!diag.notes.is_empty());
        assert_eq!(diag.notes[0].label, "recover.expected_tokens");
        assert!(
            diag.notes[0].message.contains("RParen"),
            "expected token list to contain RParen"
        );
    }
}
