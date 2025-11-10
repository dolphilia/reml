//! OCaml 版 `parser_driver` と同等の責務を担う Rust フロントエンド PoC。

use chumsky::error::{Simple, SimpleReason};
use chumsky::prelude::*;
use chumsky::stream::Stream;
use smallvec::SmallVec;
use smol_str::SmolStr;
use std::ops::Range;

pub mod ast;

use crate::diagnostic::{
    recover::ExpectedTokensSummary, DiagnosticBuilder, DiagnosticNote, ExpectedToken,
    ExpectedTokenCollector, FrontendDiagnostic,
};
use crate::error::{FrontendError, Recoverability};
use crate::lexer::{lex_source, LexOutput};
use crate::span::Span;
use crate::streaming::{
    Expectation as StreamingExpectation, ExpectationSummary, PackratEntry, PackratStats,
    StreamMetrics, StreamingState, StreamingStateConfig, TokenSample, TraceFrame,
};
use crate::token::{Token, TokenKind};
use ast::{Expr, Function, Module, Param};

/// パース結果の簡易表現。
#[derive(Debug, Default)]
pub struct ParsedModule {
    pub tokens: Vec<Token>,
    pub diagnostics: Vec<FrontendDiagnostic>,
    pub ast: Option<Module>,
    pub packrat_stats: PackratStats,
    pub stream_metrics: StreamMetrics,
    pub span_trace: Vec<TraceFrame>,
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
        Self::parse_with_config(source, StreamingStateConfig::default())
    }

    pub fn parse_with_config(source: &str, config: StreamingStateConfig) -> ParsedModule {
        let LexOutput { tokens, errors } = lex_source(source);
        let streaming_state = StreamingState::new(config);

        let mut diagnostics = DiagnosticBuilder::with_capacity(errors.len());
        diagnostics.extend(errors.into_iter().map(Self::error_to_diagnostic));

        let (ast, parse_errors) = parse_tokens(&tokens, source, &streaming_state);
        diagnostics.extend(parse_errors.into_iter().map(|err| {
            let formatted = err.1;
            let mut diagnostic = FrontendDiagnostic::new(formatted.message.clone());
            if let Some(span) = err.0 {
                diagnostic = diagnostic.with_span(span);
            }
            diagnostic = diagnostic.apply_expected_summary(&formatted.summary);
            if formatted.summary.has_alternatives() {
                if let Some(text) = formatted.summary.humanized.clone() {
                    diagnostic.add_note(DiagnosticNote::new("recover.expected_tokens", text));
                }
            }
            diagnostic.with_recoverability(Recoverability::Recoverable)
        }));

        let span_trace = streaming_state.drain_span_trace();
        let stream_metrics = streaming_state.metrics_snapshot();

        let diagnostics = diagnostics.into_vec();

        ParsedModule {
            tokens,
            diagnostics,
            ast,
            packrat_stats: stream_metrics.packrat,
            stream_metrics,
            span_trace,
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

struct FormattedSimpleError {
    message: String,
    summary: ExpectedTokensSummary,
}

fn parse_tokens(
    tokens: &[Token],
    source: &str,
    streaming_state: &StreamingState,
) -> (Option<Module>, Vec<(Option<Span>, FormattedSimpleError)>) {
    let token_pairs: Vec<_> = tokens
        .iter()
        .filter(|token| token.kind != TokenKind::Whitespace)
        .map(|token| {
            let span = token.span;
            (token.kind, (span.start as usize)..(span.end as usize))
        })
        .collect();

    let end = source.len();
    let parser = module_parser(source, streaming_state);
    let (ast, errors) = parser.parse_recovery(Stream::from_iter(end..end, token_pairs.into_iter()));

    let mapped_errors = errors
        .into_iter()
        .map(|err| {
            let span = Some(convert_range(err.span()));
            let formatted = format_simple_error(&err);
            record_streaming_error(streaming_state, &err, tokens, &formatted);
            (span, formatted)
        })
        .collect();

    (ast, mapped_errors)
}

fn convert_range(range: Range<usize>) -> Span {
    Span::new(range.start as u32, range.end as u32)
}

fn format_simple_error(err: &Simple<TokenKind>) -> FormattedSimpleError {
    let summary = build_expected_summary(err);
    let message = match err.reason() {
        SimpleReason::Unexpected | SimpleReason::Unclosed { .. } => {
            "構文エラー: 入力を解釈できません".to_string()
        }
        SimpleReason::Custom(msg) => msg.clone(),
    };

    FormattedSimpleError { message, summary }
}

fn build_expected_summary(err: &Simple<TokenKind>) -> ExpectedTokensSummary {
    let mut collector = ExpectedTokenCollector::new();
    for expectation in err.expected() {
        match expectation {
            Some(kind) => collector.extend(token_kind_expectations(&kind)),
            None => collector.push(ExpectedToken::eof()),
        }
    }
    collector.summarize()
}

fn token_kind_expectations(kind: &TokenKind) -> Vec<ExpectedToken> {
    use ExpectedToken as ET;

    match kind {
        TokenKind::KeywordFn => vec![ET::keyword("fn")],
        TokenKind::KeywordLet => vec![ET::keyword("let")],
        TokenKind::KeywordModule => vec![ET::keyword("module")],
        TokenKind::KeywordEffect => vec![ET::keyword("effect")],
        TokenKind::Identifier => vec![ET::class("識別子")],
        TokenKind::IntLiteral => vec![ET::class("整数リテラル")],
        TokenKind::FloatLiteral => vec![ET::class("浮動小数リテラル")],
        TokenKind::StringLiteral => vec![ET::class("文字列リテラル")],
        TokenKind::LParen => vec![ET::token("(")],
        TokenKind::RParen => vec![ET::token(")")],
        TokenKind::LBrace => vec![ET::token("{")],
        TokenKind::RBrace => vec![ET::token("}")],
        TokenKind::LBracket => vec![ET::token("[")],
        TokenKind::RBracket => vec![ET::token("]")],
        TokenKind::Comma => vec![ET::token(",")],
        TokenKind::Colon => vec![ET::token(":")],
        TokenKind::Semi => vec![ET::token(";")],
        TokenKind::Arrow => vec![ET::token("->")],
        TokenKind::Assign => vec![ET::token("=")],
        TokenKind::Operator => vec![ET::token("+")],
        TokenKind::Comment => vec![ET::class("コメント")],
        TokenKind::Whitespace => vec![ET::class("空白")],
        TokenKind::EndOfFile => vec![ET::eof()],
        TokenKind::Unknown => vec![ET::custom("未知のトークン")],
    }
}

fn module_parser<'src>(
    source: &'src str,
    streaming_state: &StreamingState,
) -> impl Parser<TokenKind, Module, Error = Simple<TokenKind>> + Clone + 'src {
    let span_to_span = |span: Range<usize>| Span::new(span.start as u32, span.end as u32);
    let streaming_state_success = streaming_state.clone();

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
        .map(move |(((fn_span, (name, name_span)), params), body)| {
            let function_span = Span::new(fn_span.start.min(name_span.start), body.span().end);
            record_streaming_success(&streaming_state_success, function_span);
            Function {
                name,
                params,
                span: function_span,
                body,
            }
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

fn record_streaming_error(
    streaming_state: &StreamingState,
    err: &Simple<TokenKind>,
    tokens: &[Token],
    formatted: &FormattedSimpleError,
) {
    let span = convert_range(err.span());
    streaming_state.push_span_trace(Some(SmolStr::new_inline("parser.simple_error")), span);

    let mut sample_tokens: SmallVec<[TokenSample; 8]> = SmallVec::new();
    let context_start = span.start.saturating_sub(16);
    let context_end = span.end.saturating_add(16);
    for token in tokens
        .iter()
        .filter(|token| !matches!(token.kind, TokenKind::Whitespace | TokenKind::Comment))
    {
        if token.span.end < context_start || token.span.start > context_end {
            continue;
        }
        let kind_label = format!("{:?}", token.kind);
        let lexeme = token.lexeme.as_deref().unwrap_or_default();
        sample_tokens.push(TokenSample {
            kind: SmolStr::from(kind_label),
            lexeme: SmolStr::from(lexeme),
        });
        if sample_tokens.len() >= 8 {
            break;
        }
    }
    if sample_tokens.is_empty() {
        sample_tokens.push(TokenSample {
            kind: SmolStr::new_inline("None"),
            lexeme: SmolStr::new_inline(""),
        });
    }

    let expectation_labels = formatted.summary.tokens();
    let expectations: Vec<StreamingExpectation> = expectation_labels
        .iter()
        .map(|label| StreamingExpectation {
            description: SmolStr::from(label.as_str()),
        })
        .collect();
    let summary_humanized = if let Some(text) = formatted.summary.humanized.as_ref() {
        Some(SmolStr::from(text.as_str()))
    } else if !formatted.message.is_empty() {
        Some(SmolStr::from(formatted.message.as_str()))
    } else {
        None
    };
    let summary = if summary_humanized.is_some() || !expectation_labels.is_empty() {
        Some(ExpectationSummary {
            humanized: summary_humanized,
            alternatives: expectation_labels.into_iter().map(SmolStr::from).collect(),
        })
    } else {
        None
    };

    let entry = PackratEntry::new(sample_tokens, expectations, summary);
    let range_start = span.start;
    let mut range_end = span.end;
    if range_end <= range_start {
        range_end = range_start.saturating_add(1);
    }
    let parser_id = 1;
    let range = range_start..range_end;
    const PACKRAT_WARM_CONSUMERS: usize = 6;
    // packrat miss
    let _ = streaming_state.lookup_packrat(parser_id, range.clone());
    streaming_state.store_packrat(parser_id, range.clone(), entry);
    // warm consumers (LSP/CLI/監査 etc.)
    for _ in 0..PACKRAT_WARM_CONSUMERS {
        let _ = streaming_state.lookup_packrat(parser_id, range.clone());
    }
}

fn record_streaming_success(streaming_state: &StreamingState, span: Span) {
    streaming_state.push_span_trace(Some(SmolStr::new_inline("parser.success")), span);

    let mut sample_tokens: SmallVec<[TokenSample; 8]> = SmallVec::new();
    sample_tokens.push(TokenSample {
        kind: SmolStr::new_inline("Success"),
        lexeme: SmolStr::new_inline("function"),
    });

    let summary = Some(ExpectationSummary {
        humanized: Some(SmolStr::new_inline("success")),
        alternatives: Vec::new(),
    });

    let entry = PackratEntry::new(sample_tokens, Vec::new(), summary);
    let parser_id = 2;
    let range_start = span.start;
    let mut range_end = span.end;
    if range_end <= range_start {
        range_end = range_start.saturating_add(1);
    }
    let range = range_start..range_end;
    const PACKRAT_WARM_CONSUMERS: usize = 2;
    let _ = streaming_state.lookup_packrat(parser_id, range.clone());
    streaming_state.store_packrat(parser_id, range.clone(), entry);
    for _ in 0..PACKRAT_WARM_CONSUMERS {
        let _ = streaming_state.lookup_packrat(parser_id, range.clone());
    }
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
        assert_eq!(diag.notes[0].message, "ここで`)` または `,`のいずれかが必要です");
    }
}
