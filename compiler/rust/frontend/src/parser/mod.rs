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
use crate::lexer::{lex_source_with_options, IdentifierProfile, LexOutput, LexerOptions};
use crate::span::Span;
use crate::streaming::{
    Expectation as StreamingExpectation, ExpectationSummary, PackratEntry, PackratStats,
    StreamFlowState, StreamMetrics, StreamingState, StreamingStateConfig, TokenSample, TraceFrame,
};
use crate::token::{Token, TokenKind};
use ast::{EffectDecl, Expr, Function, Module, Param};

/// パース結果の簡易表現。
#[derive(Debug, Default)]
pub struct ParsedModule {
    pub tokens: Vec<Token>,
    pub diagnostics: Vec<FrontendDiagnostic>,
    pub ast: Option<Module>,
    pub packrat_stats: PackratStats,
    pub stream_metrics: StreamMetrics,
    pub span_trace: Vec<TraceFrame>,
    pub stream_flow_state: Option<StreamFlowState>,
}

impl ParsedModule {
    pub fn ast_render(&self) -> Option<String> {
        self.ast.as_ref().map(Module::render)
    }
}

#[derive(Clone)]
pub struct ParserOptions {
    pub streaming: StreamingStateConfig,
    pub merge_parse_expected: bool,
    pub streaming_enabled: bool,
    pub stream_flow: Option<StreamFlowState>,
    pub lex_identifier_profile: IdentifierProfile,
}

impl Default for ParserOptions {
    fn default() -> Self {
        Self {
            streaming: StreamingStateConfig::default(),
            merge_parse_expected: true,
            streaming_enabled: false,
            stream_flow: None,
            lex_identifier_profile: IdentifierProfile::Unicode,
        }
    }
}

/// Rust フロントエンドのパーサドライバ。
pub struct ParserDriver;

impl ParserDriver {
    pub fn parse(source: &str) -> ParsedModule {
        Self::parse_with_config(source, StreamingStateConfig::default())
    }

    pub fn parse_with_config(source: &str, config: StreamingStateConfig) -> ParsedModule {
        let mut options = ParserOptions::default();
        options.streaming = config;
        options.merge_parse_expected = true;
        options.streaming_enabled = false;
        Self::parse_with_options(source, options)
    }

    pub fn parse_with_options(source: &str, options: ParserOptions) -> ParsedModule {
        let lexer_options = LexerOptions {
            identifier_profile: options.lex_identifier_profile,
        };
        let LexOutput { tokens, errors } = lex_source_with_options(source, lexer_options);
        let streaming_state = StreamingState::new(options.streaming.clone());
        let stream_flow_state = options.stream_flow.clone();
        let streaming_enabled = options.streaming_enabled
            || stream_flow_state
                .as_ref()
                .map(|state| state.enabled())
                .unwrap_or(false);

        let mut diagnostics = if options.merge_parse_expected {
            DiagnosticBuilder::with_capacity(errors.len())
        } else {
            DiagnosticBuilder::with_merge_parse_expected(false)
        };
        diagnostics.extend(errors.into_iter().map(Self::error_to_diagnostic));

        let (ast, parse_errors) = parse_tokens(&tokens, source, &streaming_state);
        let mut streaming_recover = StreamingRecoverController::new(streaming_enabled);
        streaming_recover.start_checkpoint();
        for (span, formatted) in parse_errors.into_iter() {
            streaming_recover.record(span, formatted, &mut diagnostics);
        }
        if let Some(flow_state) = stream_flow_state.as_ref() {
            flow_state.checkpoint_end(&mut streaming_recover, &mut diagnostics);
        } else {
            streaming_recover.checkpoint_end(&mut diagnostics);
        }

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
            stream_flow_state,
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

#[derive(Clone)]
struct FormattedSimpleError {
    message: String,
    summary: ExpectedTokensSummary,
}

impl FormattedSimpleError {
    fn absorb(&mut self, other: FormattedSimpleError) {
        self.summary.merge_with(&other.summary);
        if self.message.is_empty() {
            self.message = other.message;
        }
    }
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

fn build_diagnostic_from_error(
    span: Option<Span>,
    error: FormattedSimpleError,
) -> FrontendDiagnostic {
    let FormattedSimpleError { message, summary } = error;
    let mut diagnostic = FrontendDiagnostic::new(message);
    if let Some(span) = span {
        diagnostic = diagnostic.with_span(span);
    }
    diagnostic = diagnostic.apply_expected_summary(&summary);
    if summary.has_alternatives() {
        if let Some(text) = summary.humanized.clone() {
            diagnostic.add_note(DiagnosticNote::new("recover.expected_tokens", text));
        }
    }
    diagnostic.with_recoverability(Recoverability::Recoverable)
}

fn build_expected_summary(err: &Simple<TokenKind>) -> ExpectedTokensSummary {
    let mut collector = ExpectedTokenCollector::new();
    let expectations: Vec<Option<TokenKind>> = err.expected().cloned().collect();
    if is_expression_recover_context(&expectations) {
        collector.extend(expression_expected_tokens());
    } else {
        for expectation in expectations {
            match expectation {
                Some(kind) => collector.extend(token_kind_expectations(&kind)),
                None => collector.push(ExpectedToken::eof()),
            }
        }
    }
    collector.summarize()
}

fn is_expression_recover_context(expectations: &[Option<TokenKind>]) -> bool {
    let mut has_identifier = false;
    let mut has_int_literal = false;
    let mut has_lparen = false;
    for expectation in expectations {
        match expectation {
            Some(TokenKind::Identifier) | Some(TokenKind::UpperIdentifier) => {
                has_identifier = true
            }
            Some(TokenKind::IntLiteral) => has_int_literal = true,
            Some(TokenKind::LParen) => has_lparen = true,
            _ => {}
        }
    }
    has_identifier && has_int_literal && has_lparen
}

fn expression_expected_tokens() -> Vec<ExpectedToken> {
    use ExpectedToken as ET;
    vec![
        ET::keyword("continue"),
        ET::keyword("defer"),
        ET::keyword("do"),
        ET::keyword("false"),
        ET::keyword("for"),
        ET::keyword("handle"),
        ET::keyword("if"),
        ET::keyword("loop"),
        ET::keyword("match"),
        ET::keyword("perform"),
        ET::keyword("return"),
        ET::keyword("self"),
        ET::keyword("true"),
        ET::keyword("unsafe"),
        ET::keyword("while"),
        ET::token("!"),
        ET::token("("),
        ET::token("-"),
        ET::token("["),
        ET::token("{"),
        ET::token("|"),
        ET::class("char-literal"),
        ET::class("float-literal"),
        ET::class("identifier"),
        ET::class("integer-literal"),
        ET::class("string-literal"),
        ET::class("upper-identifier"),
    ]
}

fn token_kind_expectations(kind: &TokenKind) -> Vec<ExpectedToken> {
    use ExpectedToken as ET;

    if let Some(keyword) = kind.keyword_literal() {
        return vec![ET::keyword(keyword)];
    }

    match kind {
        TokenKind::Identifier => vec![ET::class("識別子")],
        TokenKind::UpperIdentifier => vec![ET::class("大文字識別子")],
        TokenKind::IntLiteral => vec![ET::class("整数リテラル")],
        TokenKind::FloatLiteral => vec![ET::class("浮動小数リテラル")],
        TokenKind::CharLiteral => vec![ET::class("文字リテラル")],
        TokenKind::StringLiteral => vec![ET::class("文字列リテラル")],
        TokenKind::LParen => vec![ET::token("(")],
        TokenKind::RParen => vec![ET::token(")")],
        TokenKind::LBrace => vec![ET::token("{")],
        TokenKind::RBrace => vec![ET::token("}")],
        TokenKind::LBracket => vec![ET::token("[")],
        TokenKind::RBracket => vec![ET::token("]")],
        TokenKind::Comma => vec![ET::token(",")],
        TokenKind::Colon => vec![ET::token(":")],
        TokenKind::Semicolon => vec![ET::token(";")],
        TokenKind::Arrow => vec![ET::token("->")],
        TokenKind::DoubleArrow => vec![ET::token("=>")],
        TokenKind::Assign => vec![ET::token("=")],
        TokenKind::ColonAssign => vec![ET::token(":=")],
        TokenKind::PipeForward => vec![ET::token("|>")],
        TokenKind::ChannelPipe => vec![ET::token("~>")],
        TokenKind::Bar => vec![ET::token("|")],
        TokenKind::At => vec![ET::token("@")],
        TokenKind::Plus => vec![ET::token("+")],
        TokenKind::Minus => vec![ET::token("-")],
        TokenKind::Star => vec![ET::token("*")],
        TokenKind::Slash => vec![ET::token("/")],
        TokenKind::Percent => vec![ET::token("%")],
        TokenKind::Caret => vec![ET::token("^")],
        TokenKind::Lt => vec![ET::token("<")],
        TokenKind::Le => vec![ET::token("<=")],
        TokenKind::Gt => vec![ET::token(">")],
        TokenKind::Ge => vec![ET::token(">=")],
        TokenKind::EqEq => vec![ET::token("==")],
        TokenKind::NotEqual => vec![ET::token("!=")],
        TokenKind::LogicalAnd => vec![ET::token("&&")],
        TokenKind::LogicalOr => vec![ET::token("||")],
        TokenKind::Not => vec![ET::token("!")],
        TokenKind::Question => vec![ET::token("?")],
        TokenKind::Dot => vec![ET::token(".")],
        TokenKind::DotDot => vec![ET::token("..")],
        TokenKind::Underscore => vec![ET::token("_")],
        TokenKind::Comment => vec![ET::class("コメント")],
        TokenKind::Whitespace => vec![ET::class("空白")],
        TokenKind::EndOfFile => vec![ET::eof()],
        TokenKind::Unknown => vec![ET::custom("未知のトークン")],
        _ => Vec::new(),
    }
}

fn module_parser<'src>(
    source: &'src str,
    streaming_state: &StreamingState,
) -> impl Parser<TokenKind, Module, Error = Simple<TokenKind>> + Clone + 'src {
    let span_to_span = |span: Range<usize>| Span::new(span.start as u32, span.end as u32);
    let streaming_state_success = streaming_state.clone();

    let identifier = choice((just(TokenKind::Identifier), just(TokenKind::UpperIdentifier)))
        .map_with_span(move |_, span: Range<usize>| {
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

        let bool_literal = choice((
            just(TokenKind::KeywordTrue)
                .map_with_span(move |_, span: Range<usize>| Expr::bool(true, span_to_span(span))),
            just(TokenKind::KeywordFalse)
                .map_with_span(move |_, span: Range<usize>| Expr::bool(false, span_to_span(span))),
        ));

        let string_literal =
            just(TokenKind::StringLiteral).map_with_span(move |_, span: Range<usize>| {
                let slice = &source[span.start..span.end];
                let value =
                    if slice.starts_with("\\\"") && slice.ends_with("\\\"") && slice.len() >= 4 {
                        &slice[2..slice.len() - 2]
                    } else if slice.starts_with('"') && slice.ends_with('"') && slice.len() >= 2 {
                        &slice[1..slice.len() - 1]
                    } else {
                        slice
                    };
                let unescaped = value.replace("\\\"", "\"");
                Expr::string(unescaped, span_to_span(span))
            });

        let paren_expr = expr
            .clone()
            .delimited_by(just(TokenKind::LParen), just(TokenKind::RParen));

        let atom = choice((
            int_literal.clone(),
            bool_literal,
            string_literal,
            ident_expr.clone(),
            paren_expr,
        ))
        .boxed();

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

        let additive = call
            .clone()
            .then(
                just(TokenKind::Plus)
                    .ignore_then(call.clone())
                    .repeated(),
            )
            .map(|(first, rest)| {
                rest.into_iter().fold(first, |lhs, rhs| {
                    let span = span_union(lhs.span(), rhs.span());
                    Expr::binary("+", lhs, rhs, span)
                })
            });

        let if_expr = just(TokenKind::KeywordIf)
            .map_with_span(move |_, span: Range<usize>| span_to_span(span))
            .then(expr.clone())
            .then_ignore(just(TokenKind::KeywordThen))
            .then(expr.clone())
            .then_ignore(just(TokenKind::KeywordElse))
            .then(expr.clone())
            .map(|((if_pair, then_branch), else_branch)| {
                let (if_span, condition) = if_pair;
                let if_span_start = if_span.start;
                let else_span = else_branch.span();
                let full_span = Span::new(if_span_start, else_span.end);
                Expr::IfElse {
                    condition: Box::new(condition),
                    then_branch: Box::new(then_branch),
                    else_branch: Box::new(else_branch),
                    span: full_span,
                }
            });

        let perform_expr = just(TokenKind::KeywordPerform)
            .map_with_span(move |_, span: Range<usize>| span_to_span(span))
            .then(identifier.clone())
            .then(expr.clone())
            .map(|((perform_span, (effect, effect_span)), argument)| {
                let arg_span = argument.span();
                let full_span = Span::new(perform_span.start.min(effect_span.start), arg_span.end);
                Expr::Perform {
                    effect,
                    argument: Box::new(argument),
                    span: full_span,
                }
            });

        choice((if_expr, perform_expr, additive)).boxed()
    });

    let param = identifier.clone().then(
        just(TokenKind::Colon)
            .ignore_then(identifier.clone())
            .or_not(),
    );

    let params = param
        .map(|((name, span), _)| Param { name, span })
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

    let effect_decl = just(TokenKind::KeywordEffect)
        .map_with_span(move |_, span: Range<usize>| span_to_span(span))
        .then(identifier.clone())
        .map(|(effect_span, (name, name_span))| EffectDecl {
            name,
            span: Span::new(effect_span.start.min(name_span.start), name_span.end),
        });

    #[derive(Clone)]
    enum ModuleItem {
        Effect(EffectDecl),
        Function(Function),
    }

    let module_item = choice((
        effect_decl
            .clone()
            .map(ModuleItem::Effect),
        function.clone().map(ModuleItem::Function),
    ));

    effect_decl
        .repeated()
        .then(function.clone())
        .then(module_item.repeated())
        .then_ignore(just(TokenKind::EndOfFile).or_not())
        .map(|((effects, first_function), rest)| {
            let mut effects_vec = effects;
            let mut functions_vec = vec![first_function];
            for item in rest {
                match item {
                    ModuleItem::Effect(effect) => effects_vec.push(effect),
                    ModuleItem::Function(function) => functions_vec.push(function),
                }
            }
            Module {
                effects: effects_vec,
                functions: functions_vec,
            }
        })
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

pub struct StreamingRecoverController {
    enabled: bool,
    pending: Option<PendingRecover>,
    limiter: Option<StreamingRecoverLimiter>,
}

impl StreamingRecoverController {
    fn new(enabled: bool) -> Self {
        Self {
            enabled,
            pending: None,
            limiter: if enabled {
                Some(StreamingRecoverLimiter::new(1))
            } else {
                None
            },
        }
    }

    fn start_checkpoint(&mut self) {
        if let Some(limiter) = &mut self.limiter {
            limiter.reset();
        }
    }

    fn record(
        &mut self,
        span: Option<Span>,
        error: FormattedSimpleError,
        diagnostics: &mut DiagnosticBuilder,
    ) {
        if !self.enabled {
            diagnostics.push(build_diagnostic_from_error(span, error));
            return;
        }

        if let Some(pending) = &mut self.pending {
            pending.merge(error);
        } else {
            self.pending = Some(PendingRecover::new(span, error));
        }
    }

    pub(crate) fn checkpoint_end(&mut self, diagnostics: &mut DiagnosticBuilder) {
        if !self.enabled {
            return;
        }
        if let Some(pending) = self.pending.take() {
            if let Some(limiter) = &mut self.limiter {
                let summary = pending.error.summary.clone();
                if limiter.can_emit() {
                    let index = diagnostics.push_with_index(pending.into_diagnostic());
                    limiter.record_emission(index);
                } else if let Some(index) = limiter.last_emitted_index() {
                    diagnostics.merge_expected_summary_at(index, &summary);
                }
            } else {
                diagnostics.push(pending.into_diagnostic());
            }
        }
    }
}

struct StreamingRecoverLimiter {
    max_per_checkpoint: usize,
    emitted_in_checkpoint: usize,
    last_emitted_index: Option<usize>,
}

impl StreamingRecoverLimiter {
    fn new(max_per_checkpoint: usize) -> Self {
        Self {
            max_per_checkpoint,
            emitted_in_checkpoint: 0,
            last_emitted_index: None,
        }
    }

    fn reset(&mut self) {
        self.emitted_in_checkpoint = 0;
        self.last_emitted_index = None;
    }

    fn can_emit(&self) -> bool {
        self.emitted_in_checkpoint < self.max_per_checkpoint
    }

    fn record_emission(&mut self, index: usize) {
        self.emitted_in_checkpoint = self.emitted_in_checkpoint.saturating_add(1);
        self.last_emitted_index = Some(index);
    }

    fn last_emitted_index(&self) -> Option<usize> {
        self.last_emitted_index
    }
}

pub(crate) struct PendingRecover {
    span: Option<Span>,
    error: FormattedSimpleError,
}

impl PendingRecover {
    fn new(span: Option<Span>, error: FormattedSimpleError) -> Self {
        Self { span, error }
    }

    fn merge(&mut self, next: FormattedSimpleError) {
        self.error.absorb(next);
    }

    fn into_diagnostic(self) -> FrontendDiagnostic {
        build_diagnostic_from_error(self.span, self.error)
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
                source: r#"effect ConsoleLog
fn emit(msg: String) = perform ConsoleLog msg
fn main() = emit(\"leak\")"#,
                expected_ast: Some(
                    "effect ConsoleLog\nfn emit(msg) = perform ConsoleLog var(msg)\nfn main() = call(var(emit))[str(\"leak\")]",
                ),
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
        assert_eq!(
            diag.notes[0].message,
            "ここで`)` または `,`のいずれかが必要です"
        );
    }

    fn sample_error(tokens: &[&str]) -> FormattedSimpleError {
        let mut collector = ExpectedTokenCollector::new();
        for token in tokens {
            collector.push_keyword(*token);
        }
        FormattedSimpleError {
            message: "構文エラー: 入力を解釈できません".to_string(),
            summary: collector.summarize(),
        }
    }

    #[test]
    fn streaming_recover_coalesces_errors() {
        let mut builder = DiagnosticBuilder::with_capacity(2);
        let mut controller = StreamingRecoverController::new(true);

        controller.record(Some(Span::new(0, 1)), sample_error(&["fn"]), &mut builder);
        controller.record(Some(Span::new(2, 3)), sample_error(&["if"]), &mut builder);
        controller.checkpoint_end(&mut builder);

        let diagnostics = builder.into_vec();
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(
            diagnostics[0].expected_tokens,
            vec!["fn".to_string(), "if".to_string()]
        );
    }

    #[test]
    fn non_streaming_emits_all_errors() {
        let mut builder = DiagnosticBuilder::with_capacity(2);
        let mut controller = StreamingRecoverController::new(false);

        controller.record(Some(Span::new(0, 1)), sample_error(&["fn"]), &mut builder);
        controller.record(Some(Span::new(2, 3)), sample_error(&["if"]), &mut builder);
        controller.checkpoint_end(&mut builder);

        let diagnostics = builder.into_vec();
        assert_eq!(diagnostics.len(), 2);
    }
}
