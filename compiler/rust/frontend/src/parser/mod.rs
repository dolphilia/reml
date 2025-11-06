//! OCaml 版 `parser_driver` と同等の責務を担う Rust フロントエンド PoC。

use chumsky::error::{Rich, RichReason};
use chumsky::prelude::*;
use chumsky::span::SimpleSpan;

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
            let mut diagnostic = FrontendDiagnostic::new(err.1);
            if let Some(span) = err.0 {
                diagnostic = diagnostic.with_span(span);
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

fn parse_tokens(tokens: &[Token], source: &str) -> (Option<Module>, Vec<(Option<Span>, String)>) {
    let token_stream = tokens
        .iter()
        .filter(|token| token.kind != TokenKind::Whitespace)
        .map(|token| {
            (
                token.kind,
                SimpleSpan::new(token.span.start as usize, token.span.end as usize),
            )
        })
        .collect::<Vec<_>>();

    let end = source.len();
    let parser = module_parser(source);
    match parser.parse(Stream::from_iter(
        SimpleSpan::new(end, end),
        token_stream.into_iter(),
    )) {
        Ok(ast) => (Some(ast), Vec::new()),
        Err(errors) => {
            let mapped = errors
                .into_iter()
                .map(|err| {
                    let span = err.span().map(|s| Span::new(s.start as u32, s.end as u32));
                    let message = format_simple_error(&err, source);
                    (span, message)
                })
                .collect();
            (None, mapped)
        }
    }
}

fn format_simple_error(err: &Rich<'_, TokenKind>, source: &str) -> String {
    match err.reason() {
        RichReason::ExpectedFound { expected, found } => {
            let expected_tokens = expected
                .iter()
                .filter_map(|token| match token {
                    Some(kind) => Some(format!("{kind:?}")),
                    None => None,
                })
                .collect::<Vec<_>>();
            let expected_text = if expected_tokens.is_empty() {
                "トークン".to_string()
            } else {
                expected_tokens.join(", ")
            };
            let found_text = found
                .map(|kind| format!("{kind:?}"))
                .unwrap_or_else(|| "EOF".to_string());
            format!("`{expected_text}` のいずれかが必要でしたが `{found_text}` が見つかりました")
        }
        RichReason::Custom(msg) => msg.clone(),
        RichReason::Many(reasons) => reasons
            .iter()
            .map(|reason| format_simple_error(reason, source))
            .collect::<Vec<_>>()
            .join("; "),
    }
}

fn module_parser<'src>(
    source: &'src str,
) -> impl Parser<TokenKind, Module, Error = Rich<'src, TokenKind>> + Clone {
    let span_to_span = |span: SimpleSpan<usize>| Span::new(span.start as u32, span.end as u32);

    let identifier = just(TokenKind::Identifier).map_with_span(move |_, span| {
        let slice = &source[span.start..span.end];
        (slice.to_string(), span_to_span(span))
    });

    let int_literal = just(TokenKind::IntLiteral).map_with_span(move |_, span| {
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

        let plus = filter_map(move |span: SimpleSpan<usize>, kind| {
            if kind == TokenKind::Operator {
                let text = &source[span.start..span.end];
                if text == "+" {
                    Ok(span_to_span(span))
                } else {
                    Err(Rich::custom(span, format!("未対応の演算子 `{text}`")))
                }
            } else {
                Err(Rich::custom(
                    span,
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
        .map_with_span(move |_, span| span_to_span(span))
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
