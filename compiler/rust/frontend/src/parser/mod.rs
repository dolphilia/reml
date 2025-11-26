//! OCaml 版 `parser_driver` と同等の責務を担う Rust フロントエンド PoC。

use chumsky::error::{Simple, SimpleReason};
use chumsky::prelude::*;
use chumsky::stream::Stream;
use chumsky::Parser as ChumskyParser;
use reml_runtime::text::{LocaleId, UnicodeError};
use serde::Serialize;
use smallvec::SmallVec;
use smol_str::SmolStr;
use std::ops::Range;

pub mod api;
pub mod ast;
pub mod streaming_runner;

pub use self::api::{
    LeftRecursionMode, ParseError, ParseResult, ParseResultWithRest, Parser, Reply, RunConfig,
    State,
};
pub use self::streaming_runner::{
    Continuation, DemandHint, StreamMeta, StreamOutcome, StreamingRunner,
};

use crate::diagnostic::{
    recover::ExpectedTokensSummary, DiagnosticBuilder, DiagnosticNote, ExpectedToken,
    ExpectedTokenCollector, FrontendDiagnostic,
};
use crate::error::{FrontendError, Recoverability};
use crate::lexer::{lex_source_with_options, IdentifierProfile, LexOutput, LexerOptions};
use crate::span::Span;
use crate::streaming::{
    Expectation as StreamingExpectation, ExpectationSummary, PackratCacheEntry, PackratEntry,
    PackratSnapshot, PackratStats, StreamFlowState, StreamMetrics, StreamingState,
    StreamingStateConfig, TokenSample, TraceFrame,
};
use crate::token::{Token, TokenKind};
use crate::unicode::{unicode_diagnostic_code, UnicodeDetail};
use ast::{
    Decl, DeclKind, EffectDecl, Expr, ExprKind, Function, HandlerDecl, Ident, Literal, LiteralKind,
    MatchArm, Module, ModuleHeader, ModulePath, Param, Pattern, PatternKind, RelativeHead, Stmt,
    StmtKind, TypeAnnot, TypeKind, UseDecl, UseItem, UseTree, Visibility,
};

/// パース結果の簡易表現。
#[derive(Debug, Default)]
pub struct ParsedModule {
    pub tokens: Vec<Token>,
    pub diagnostics: Vec<FrontendDiagnostic>,
    pub recovered: bool,
    pub ast: Option<Module>,
    pub packrat_stats: PackratStats,
    pub packrat_snapshot: PackratSnapshot,
    pub stream_metrics: StreamMetrics,
    pub span_trace: Vec<TraceFrame>,
    pub packrat_cache: Option<Vec<PackratCacheEntry>>,
    pub stream_flow_state: Option<StreamFlowState>,
    pub trace_events: Vec<ParserTraceEvent>,
}

impl ParsedModule {
    pub fn ast_render(&self) -> Option<String> {
        self.ast.as_ref().map(Module::render)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ParserTraceEvent {
    pub trace_id: SmolStr,
    #[serde(rename = "event_kind")]
    pub kind: ParserTraceEventKind,
    pub span: Span,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

fn trace_id(prefix: &str, kind: &str) -> SmolStr {
    SmolStr::from(format!("{prefix}::{kind}"))
}

fn expr_trace_id(kind: &str) -> SmolStr {
    trace_id("syntax:expr", kind)
}

fn effect_trace_id(kind: &str) -> SmolStr {
    trace_id("syntax:effect", kind)
}

fn handler_trace_id(kind: &str) -> SmolStr {
    trace_id("syntax:handler", kind)
}

fn operation_trace_id(kind: &str) -> SmolStr {
    trace_id("syntax:operation", kind)
}

impl ParserTraceEvent {
    fn module_header(header: &ModuleHeader) -> Self {
        Self {
            trace_id: SmolStr::new_inline("syntax:module-header"),
            kind: ParserTraceEventKind::ModuleHeaderAccepted,
            span: header.span,
            label: Some(header.path.render()),
        }
    }

    fn use_decl(decl: &UseDecl) -> Self {
        Self {
            trace_id: SmolStr::new_inline("syntax:use"),
            kind: ParserTraceEventKind::UseDeclAccepted,
            span: decl.span,
            label: Some(decl.tree.render()),
        }
    }

    fn expr_enter(kind: &str, span: Span) -> Self {
        Self {
            trace_id: expr_trace_id(kind),
            kind: ParserTraceEventKind::ExprEnter,
            span,
            label: Some(kind.to_string()),
        }
    }

    fn expr_leave(kind: &str, span: Span) -> Self {
        Self {
            trace_id: expr_trace_id(kind),
            kind: ParserTraceEventKind::ExprLeave,
            span,
            label: Some(kind.to_string()),
        }
    }

    fn effect_enter(kind: &str, span: Span, label: Option<String>) -> Self {
        Self {
            trace_id: effect_trace_id(kind),
            kind: ParserTraceEventKind::EffectEnter,
            span,
            label,
        }
    }

    fn effect_exit(kind: &str, span: Span, label: Option<String>) -> Self {
        Self {
            trace_id: effect_trace_id(kind),
            kind: ParserTraceEventKind::EffectExit,
            span,
            label,
        }
    }

    fn handler(handler: &HandlerDecl) -> Self {
        Self {
            trace_id: handler_trace_id(handler.name.name.as_str()),
            kind: ParserTraceEventKind::HandlerAccepted,
            span: handler.span,
            label: Some(handler.name.name.clone()),
        }
    }

    fn operation_resume(label: impl Into<String>, span: Span) -> Self {
        Self {
            trace_id: operation_trace_id("resume"),
            kind: ParserTraceEventKind::OperationResume,
            span,
            label: Some(label.into()),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ParserTraceEventKind {
    ModuleHeaderAccepted,
    UseDeclAccepted,
    ExprEnter,
    ExprLeave,
    EffectEnter,
    EffectExit,
    HandlerAccepted,
    OperationResume,
}

impl ParserTraceEventKind {
    pub fn label(&self) -> &'static str {
        match self {
            ParserTraceEventKind::ModuleHeaderAccepted => "module_header_accepted",
            ParserTraceEventKind::UseDeclAccepted => "use_decl_accepted",
            ParserTraceEventKind::ExprEnter => "expr_enter",
            ParserTraceEventKind::ExprLeave => "expr_leave",
            ParserTraceEventKind::EffectEnter => "effect_enter",
            ParserTraceEventKind::EffectExit => "effect_exit",
            ParserTraceEventKind::HandlerAccepted => "handler_accepted",
            ParserTraceEventKind::OperationResume => "operation_resume",
        }
    }
}

#[derive(Clone, Debug)]
pub struct ParserOptions {
    pub streaming: StreamingStateConfig,
    pub merge_parse_expected: bool,
    pub streaming_enabled: bool,
    pub stream_flow: Option<StreamFlowState>,
    pub lex_identifier_profile: IdentifierProfile,
    pub lex_identifier_locale: Option<LocaleId>,
}

impl Default for ParserOptions {
    fn default() -> Self {
        Self {
            streaming: StreamingStateConfig::default(),
            merge_parse_expected: true,
            streaming_enabled: false,
            stream_flow: None,
            lex_identifier_profile: IdentifierProfile::Unicode,
            lex_identifier_locale: None,
        }
    }
}

impl ParserOptions {
    pub fn from_run_config(run_config: &RunConfig) -> Self {
        let mut streaming = StreamingStateConfig::default();
        streaming.packrat_enabled = run_config.packrat;
        streaming.trace_enabled = run_config.trace;
        Self {
            streaming,
            merge_parse_expected: run_config.merge_warnings,
            streaming_enabled: run_config.trace,
            stream_flow: None,
            lex_identifier_profile: lex_identifier_profile_from_run_config(run_config),
            lex_identifier_locale: lex_identifier_locale_from_run_config(run_config),
        }
    }

    pub fn with_stream_flow(mut self, flow: Option<StreamFlowState>) -> Self {
        self.stream_flow = flow;
        self
    }

    pub fn with_streaming_enabled(mut self, enabled: bool) -> Self {
        self.streaming_enabled = enabled;
        self
    }

    pub fn with_lex_identifier_profile(mut self, profile: IdentifierProfile) -> Self {
        self.lex_identifier_profile = profile;
        self
    }

    pub fn with_lex_identifier_locale(mut self, locale: Option<LocaleId>) -> Self {
        self.lex_identifier_locale = locale;
        self
    }
}

fn lex_identifier_profile_from_run_config(run_config: &RunConfig) -> IdentifierProfile {
    run_config
        .extension("lex")
        .and_then(|value| value.as_object())
        .and_then(|map| map.get("identifier_profile"))
        .and_then(|value| value.as_str())
        .and_then(|text| text.parse::<IdentifierProfile>().ok())
        .unwrap_or_default()
}

fn lex_identifier_locale_from_run_config(run_config: &RunConfig) -> Option<LocaleId> {
    run_config
        .extension("lex")
        .and_then(|value| value.as_object())
        .and_then(|map| map.get("identifier_locale"))
        .and_then(|value| value.as_str())
        .and_then(|text| LocaleId::parse(text).ok())
}

/// Rust フロントエンドのパーサドライバ。
pub struct ParserDriver;

impl ParserDriver {
    pub fn parse(source: &str) -> ParseResult<Module> {
        let run_config = RunConfig::default();
        let options = ParserOptions::from_run_config(&run_config);
        Self::parse_with_options_and_run_config(source, options, run_config)
    }

    pub fn parse_with_config(source: &str, config: StreamingStateConfig) -> ParsedModule {
        let mut options = ParserOptions::default();
        options.streaming = config;
        options.merge_parse_expected = true;
        options.streaming_enabled = false;
        let (parsed, _) = Self::parse_with_options(source, options);
        parsed
    }

    pub fn parse_with_options_and_run_config(
        source: &str,
        options: ParserOptions,
        run_config: RunConfig,
    ) -> ParseResult<Module> {
        let (parsed, legacy_error) = Self::parse_with_options(source, options);
        let _reply = build_parser_reply(source, &parsed, legacy_error.as_ref());
        parse_result_from_module(parsed, run_config, legacy_error)
    }

    pub fn parse_with_options(
        source: &str,
        options: ParserOptions,
    ) -> (ParsedModule, Option<ParseError>) {
        let lexer_options = LexerOptions {
            identifier_profile: options.lex_identifier_profile,
            identifier_locale: options.lex_identifier_locale.clone(),
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

        let (ast, parse_errors, legacy_error, trace_events) =
            parse_tokens(&tokens, source, &streaming_state);
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

        let packrat_cache = streaming_state.packrat_cache_entries();
        let span_trace = streaming_state.drain_span_trace();
        let stream_metrics = streaming_state.metrics_snapshot();
        let packrat_snapshot = streaming_state.packrat_snapshot();
        let recovered = streaming_recover.recovered();

        let diagnostics = diagnostics.into_vec();

        (
            ParsedModule {
                tokens,
                diagnostics,
                recovered,
                ast,
                packrat_stats: stream_metrics.packrat,
                packrat_snapshot,
                stream_metrics,
                span_trace,
                packrat_cache,
                stream_flow_state,
                trace_events,
            },
            legacy_error,
        )
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
            crate::error::FrontendErrorKind::UnexpectedStructure { span, unicode, .. } => {
                if let Some(span) = span {
                    diagnostic = diagnostic.with_span(span);
                }
                if let Some(detail) = unicode.clone() {
                    diagnostic = diagnostic.with_unicode_detail(detail.clone());
                    diagnostic.push_code(unicode_diagnostic_code(detail.kind()));
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
) -> (
    Option<Module>,
    Vec<(Option<Span>, FormattedSimpleError)>,
    Option<ParseError>,
    Vec<ParserTraceEvent>,
) {
    let mut prefix = parse_top_level_prefix(tokens);
    let token_pairs: Vec<_> = tokens
        .iter()
        .enumerate()
        .filter(|(index, token)| *index >= prefix.consumed && token.kind != TokenKind::Whitespace)
        .map(|(_, token)| {
            let span = token.span;
            (token.kind, (span.start as usize)..(span.end as usize))
        })
        .collect();

    let end = source.len();
    let parser = module_parser(source, streaming_state);
    let (mut ast, errors) =
        parser.parse_recovery(Stream::from_iter(end..end, token_pairs.into_iter()));

    let mut legacy_error = None;
    let mapped_errors = errors
        .into_iter()
        .map(|err| {
            let span = Some(convert_range(err.span()));
            let formatted = format_simple_error(&err);
            record_streaming_error(streaming_state, &err, tokens, &formatted);
            if legacy_error.is_none() {
                legacy_error = Some(build_parse_error(
                    span.unwrap_or_else(|| Span::new(0, 0)),
                    &formatted.summary,
                ));
            }
            (span, formatted)
        })
        .collect();

    if let Some(module) = ast.as_mut() {
        module.header = prefix.header.clone();
        module.uses = prefix.uses.clone();
    }

    let mut trace_events = prefix.events;
    if let Some(module) = ast.as_ref() {
        append_module_trace_events(module, &mut trace_events);
    }

    (ast, mapped_errors, legacy_error, trace_events)
}

fn convert_range(range: Range<usize>) -> Span {
    Span::new(range.start as u32, range.end as u32)
}

fn parse_result_from_module(
    parsed: ParsedModule,
    run_config: RunConfig,
    legacy_error: Option<ParseError>,
) -> ParseResult<Module> {
    let ParsedModule {
        tokens,
        diagnostics,
        recovered,
        ast,
        packrat_stats,
        packrat_snapshot,
        stream_metrics,
        span_trace,
        packrat_cache,
        stream_flow_state,
        trace_events,
    } = parsed;

    let farthest_error_offset = diagnostics
        .iter()
        .filter_map(|diag| diag.span.map(|span| span.end))
        .max();

    ParseResult::new(
        ast,
        None,
        diagnostics,
        recovered,
        legacy_error,
        farthest_error_offset,
        packrat_cache,
        tokens,
        packrat_stats,
        packrat_snapshot,
        stream_metrics,
        span_trace,
        stream_flow_state,
        run_config,
        trace_events,
    )
}

fn build_parser_reply(
    source: &str,
    parsed: &ParsedModule,
    legacy_error: Option<&ParseError>,
) -> Reply<Module> {
    if let Some(module) = &parsed.ast {
        let span_end = source.len().min(u32::MAX as usize) as u32;
        let span = Span::new(0, span_end);
        Reply::Ok {
            value: module.clone(),
            span,
            consumed: true,
        }
    } else if let Some(error) = legacy_error {
        Reply::Err {
            error: error.clone(),
            consumed: true,
            committed: error.committed,
        }
    } else {
        Reply::Err {
            error: ParseError::new(Span::new(0, 0), Vec::new()),
            consumed: false,
            committed: false,
        }
    }
}

impl ParseResult<Module> {
    pub(crate) fn ast_render(&self) -> Option<String> {
        self.value.as_ref().map(Module::render)
    }
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

fn build_parse_error(span: Span, summary: &ExpectedTokensSummary) -> ParseError {
    let mut context = Vec::new();
    if let Some(text) = summary.context_note.as_ref() {
        if !text.trim().is_empty() {
            context.push(text.clone());
        }
    }
    if let Some(text) = summary.humanized.as_ref() {
        if !text.trim().is_empty() {
            context.push(text.clone());
        }
    }
    let mut error = ParseError::new(span, summary.alternatives.clone());
    error.context = context;
    error
}

/// UnicodeError を ParseError へ正規化する。
pub fn unicode_error_to_parse_error(
    span: Span,
    unicode_error: &UnicodeError,
    phase: &str,
) -> ParseError {
    let mut error = ParseError::new(span, Vec::new());
    error.notes.push(unicode_error.message().to_string());
    error
        .context
        .push(format!("unicode.{:?}", unicode_error.kind()).to_lowercase());
    error.unicode = Some(
        UnicodeDetail::from_error(unicode_error).with_phase(phase.to_string()),
    );
    error
}

fn is_expression_recover_context(expectations: &[Option<TokenKind>]) -> bool {
    let mut has_identifier = false;
    let mut has_int_literal = false;
    let mut has_lparen = false;
    for expectation in expectations {
        match expectation {
            Some(TokenKind::Identifier) | Some(TokenKind::UpperIdentifier) => has_identifier = true,
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
        TokenKind::Identifier => vec![ET::class("identifier")],
        TokenKind::UpperIdentifier => vec![ET::class("upper-identifier")],
        TokenKind::IntLiteral => vec![ET::class("integer-literal")],
        TokenKind::FloatLiteral => vec![ET::class("float-literal")],
        TokenKind::CharLiteral => vec![ET::class("char-literal")],
        TokenKind::StringLiteral => vec![ET::class("string-literal")],
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
        TokenKind::Comment => vec![ET::custom("comment")],
        TokenKind::Whitespace => vec![ET::custom("whitespace")],
        TokenKind::EndOfFile => vec![ET::eof()],
        TokenKind::Unknown => vec![ET::custom("unknown token")],
        _ => Vec::new(),
    }
}

fn module_parser<'src>(
    source: &'src str,
    streaming_state: &StreamingState,
) -> impl chumsky::Parser<TokenKind, Module, Error = Simple<TokenKind>> + Clone + 'src {
    let streaming_state_success = streaming_state.clone();

    let identifier = choice((
        just(TokenKind::Identifier),
        just(TokenKind::UpperIdentifier),
    ))
    .map_with_span(move |_, span: Range<usize>| {
        let slice = &source[span.start..span.end];
        (slice.to_string(), range_to_span(span))
    });

    let ident = identifier.clone().map(|(name, span)| Ident { name, span });

    let int_literal = just(TokenKind::IntLiteral).map_with_span(move |_, span: Range<usize>| {
        let slice = &source[span.start..span.end];
        let value = slice.parse::<i64>().unwrap_or_default();
        Expr::int(value, slice.to_string(), range_to_span(span))
    });

    let type_name = ident.clone().map(|ident| TypeAnnot {
        span: ident.span,
        kind: TypeKind::Ident { name: ident },
        annotation_kind: None,
    });
    let pattern_var = ident.clone().map(|ident| Pattern {
        span: ident.span,
        kind: PatternKind::Var(ident),
    });

    let expr = recursive(|expr| {
        let ident_expr = ident.clone().map(Expr::identifier);

        let bool_literal = choice((
            just(TokenKind::KeywordTrue)
                .map_with_span(move |_, span: Range<usize>| Expr::bool(true, range_to_span(span))),
            just(TokenKind::KeywordFalse)
                .map_with_span(move |_, span: Range<usize>| Expr::bool(false, range_to_span(span))),
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
                Expr::string(unescaped, range_to_span(span))
            });

        let paren_expr = expr
            .clone()
            .delimited_by(just(TokenKind::LParen), just(TokenKind::RParen));

        let stmt = build_stmt_parser(expr.clone(), pattern_var.clone(), type_name.clone());

        let block_expr = just(TokenKind::LBrace)
            .ignore_then(stmt.repeated())
            .then_ignore(just(TokenKind::RBrace))
            .map_with_span(|stmts, span: Range<usize>| Expr::block(stmts, range_to_span(span)));

        let separator = choice((
            just(TokenKind::Dot).to(()),
            just(TokenKind::Colon)
                .ignore_then(just(TokenKind::Colon))
                .to(()),
        ));

        let qualified_ident = ident
            .clone()
            .then(separator.ignore_then(ident.clone()).repeated())
            .map(|(first, rest)| merge_qualified_ident(first, rest));

        let pattern_ctor = qualified_ident
            .clone()
            .then(
                pattern_var
                    .clone()
                    .separated_by(just(TokenKind::Comma))
                    .allow_trailing()
                    .delimited_by(just(TokenKind::LParen), just(TokenKind::RParen))
                    .or_not(),
            )
            .map(|(name, args)| Pattern {
                span: name.span,
                kind: PatternKind::Constructor {
                    name,
                    args: args.unwrap_or_default(),
                },
            });

        let wildcard_pattern =
            just(TokenKind::Underscore).map_with_span(|_, span: Range<usize>| Pattern {
                span: range_to_span(span),
                kind: PatternKind::Wildcard,
            });

        let pattern = choice((pattern_ctor, wildcard_pattern, pattern_var.clone()));

        let match_arm = just(TokenKind::Bar)
            .or_not()
            .ignore_then(pattern.clone())
            .then_ignore(just(TokenKind::Arrow))
            .then(expr.clone())
            .map_with_span(|(pattern, body), span: Range<usize>| MatchArm {
                pattern,
                guard: None,
                body,
                span: range_to_span(span),
            });

        let match_expr = just(TokenKind::KeywordMatch)
            .ignore_then(expr.clone())
            .then_ignore(just(TokenKind::KeywordWith))
            .then(match_arm.repeated().at_least(1))
            .map_with_span(|(target, arms), span: Range<usize>| Expr {
                span: range_to_span(span),
                kind: ExprKind::Match {
                    target: Box::new(target),
                    arms,
                },
            });

        let atom = choice((
            block_expr.clone(),
            match_expr,
            int_literal.clone(),
            bool_literal,
            string_literal,
            ident_expr.clone(),
            paren_expr,
        ))
        .boxed();

        #[derive(Clone)]
        enum Postfix {
            Call(Vec<Expr>, Span),
            Field(Ident, Span),
        }

        let call_args = expr
            .clone()
            .separated_by(just(TokenKind::Comma))
            .allow_trailing()
            .delimited_by(just(TokenKind::LParen), just(TokenKind::RParen))
            .map_with_span(|args, span: Range<usize>| (args, range_to_span(span)));

        let field_ident = choice((
            ident.clone(),
            just(TokenKind::KeywordNew).map_with_span(|_, span: Range<usize>| Ident {
                name: "new".to_string(),
                span: range_to_span(span),
            }),
        ));

        let postfix = choice((
            call_args.map(|(args, span)| Postfix::Call(args, span)),
            separator.ignore_then(field_ident).map(|field| {
                let span = field.span;
                Postfix::Field(field, span)
            }),
        ));

        let call = atom
            .clone()
            .then(postfix.repeated())
            .map(|(base, postfixes)| {
                postfixes
                    .into_iter()
                    .fold(base, |acc, postfix| match postfix {
                        Postfix::Call(args, span) => {
                            let call_span =
                                args.iter().fold(span_union(acc.span(), span), |s, arg| {
                                    span_union(s, arg.span())
                                });
                            Expr::call(acc, args, call_span)
                        }
                        Postfix::Field(field, span) => {
                            let combined = span_union(acc.span(), span);
                            Expr::field_access(acc, field, combined)
                        }
                    })
            })
            .boxed();

        let additive = call
            .clone()
            .then(just(TokenKind::Plus).ignore_then(call.clone()).repeated())
            .map(|(first, rest)| {
                rest.into_iter().fold(first, |lhs, rhs| {
                    let span = span_union(lhs.span(), rhs.span());
                    Expr::binary("+", lhs, rhs, span)
                })
            });

        let if_expr = just(TokenKind::KeywordIf)
            .map_with_span(move |_, span: Range<usize>| range_to_span(span))
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
                Expr::if_else(condition, then_branch, else_branch, full_span)
            });

        let perform_expr = just(TokenKind::KeywordPerform)
            .map_with_span(move |_, span: Range<usize>| range_to_span(span))
            .then(ident.clone())
            .then(expr.clone())
            .map(|((perform_span, effect), argument)| {
                let arg_span = argument.span();
                let full_span = Span::new(perform_span.start.min(effect.span.start), arg_span.end);
                Expr::perform(effect, argument, full_span)
            });

        choice((if_expr, perform_expr, block_expr, additive)).boxed()
    });

    let param = ident.clone().then(
        just(TokenKind::Colon)
            .ignore_then(type_name.clone())
            .or_not(),
    );

    let params = param
        .map(|(name, _)| Param {
            span: name.span,
            name,
            type_annotation: None,
            default: None,
        })
        .separated_by(just(TokenKind::Comma))
        .allow_trailing()
        .delimited_by(just(TokenKind::LParen), just(TokenKind::RParen));

    let block_body_parser = {
        let stmt = build_stmt_parser(expr.clone(), pattern_var.clone(), type_name.clone());
        just(TokenKind::LBrace)
            .ignore_then(stmt.repeated())
            .then_ignore(just(TokenKind::RBrace))
            .map_with_span(|stmts, span: Range<usize>| Expr::block(stmts, range_to_span(span)))
    };

    let function = just(TokenKind::KeywordFn)
        .map_with_span(move |_, span: Range<usize>| range_to_span(span))
        .then(ident.clone())
        .then(params)
        .then(
            just(TokenKind::Arrow)
                .ignore_then(type_name.clone())
                .or_not(),
        )
        .then(choice((
            just(TokenKind::Assign).ignore_then(expr.clone()),
            block_body_parser.clone(),
        )))
        .map(move |((((fn_span, name), params), _ret_type), body)| {
            let function_span = Span::new(fn_span.start.min(name.span.start), body.span().end);
            record_streaming_success(&streaming_state_success, function_span);
            Function {
                name,
                params,
                span: function_span,
                body,
                ret_type: None,
            }
        });

    let effect_decl = just(TokenKind::KeywordEffect)
        .map_with_span(move |_, span: Range<usize>| range_to_span(span))
        .then(ident.clone())
        .map(|(effect_span, name)| EffectDecl {
            span: Span::new(effect_span.start.min(name.span.start), name.span.end),
            name,
            tag: None,
            operations: Vec::new(),
        });

    #[derive(Clone)]
    enum ModuleItem {
        Effect(EffectDecl),
        Function(Function),
    }

    let module_item = choice((
        effect_decl.clone().map(ModuleItem::Effect),
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
                header: None,
                uses: Vec::new(),
                effects: effects_vec,
                functions: functions_vec,
                decls: Vec::new(),
            }
        })
}

#[derive(Default)]
struct TopLevelPrefix {
    header: Option<ModuleHeader>,
    uses: Vec<UseDecl>,
    consumed: usize,
    events: Vec<ParserTraceEvent>,
}

fn parse_top_level_prefix(tokens: &[Token]) -> TopLevelPrefix {
    let mut prefix = TopLevelPrefix::default();
    let mut index = 0usize;
    if tokens.is_empty() {
        return prefix;
    }
    if let Some((header, consumed)) = parse_module_header_tokens(tokens, index) {
        prefix.events.push(ParserTraceEvent::module_header(&header));
        prefix.header = Some(header);
        index = consumed;
    }
    loop {
        if index >= tokens.len() {
            break;
        }
        if matches!(tokens[index].kind, TokenKind::EndOfFile) {
            break;
        }
        if let Some((use_decl, event, consumed)) = parse_use_decl_tokens(tokens, index) {
            prefix.events.push(event);
            prefix.uses.push(use_decl);
            index = consumed;
        } else {
            break;
        }
    }
    prefix.consumed = index;
    prefix
}

fn append_module_trace_events(module: &Module, events: &mut Vec<ParserTraceEvent>) {
    for effect in &module.effects {
        record_effect_decl_trace_events(effect, events);
    }
    for decl in &module.decls {
        record_decl_trace_events(decl, events);
    }
    for function in &module.functions {
        record_function_trace_events(function, events);
    }
}

fn record_function_trace_events(function: &Function, events: &mut Vec<ParserTraceEvent>) {
    for param in &function.params {
        if let Some(default) = &param.default {
            record_expr_trace_events(default, events);
        }
    }
    record_expr_trace_events(&function.body, events);
}

fn record_decl_trace_events(decl: &Decl, events: &mut Vec<ParserTraceEvent>) {
    match &decl.kind {
        DeclKind::Let { value, .. } => {
            events.push(ParserTraceEvent::expr_enter("let", decl.span));
            record_expr_trace_events(value, events);
            events.push(ParserTraceEvent::expr_leave("let", decl.span));
        }
        DeclKind::Var { value, .. } => {
            events.push(ParserTraceEvent::expr_enter("var", decl.span));
            record_expr_trace_events(value, events);
            events.push(ParserTraceEvent::expr_leave("var", decl.span));
        }
        DeclKind::Effect(effect) => record_effect_decl_trace_events(effect, events),
        DeclKind::Handler(handler) => {
            events.push(ParserTraceEvent::handler(handler));
        }
        DeclKind::Conductor { .. }
        | DeclKind::Fn { .. }
        | DeclKind::Type { .. }
        | DeclKind::Trait { .. }
        | DeclKind::Impl { .. }
        | DeclKind::Extern { .. } => {}
    }
}

fn record_effect_decl_trace_events(effect: &EffectDecl, events: &mut Vec<ParserTraceEvent>) {
    let effect_label = Some(effect.name.name.clone());
    events.push(ParserTraceEvent::effect_enter(
        "decl",
        effect.span,
        effect_label.clone(),
    ));
    for operation in &effect.operations {
        let op_label = format!("{}::{}", effect.name.name, operation.name.name);
        events.push(ParserTraceEvent::effect_enter(
            "operation",
            operation.span,
            Some(op_label.clone()),
        ));
        events.push(ParserTraceEvent::effect_exit(
            "operation",
            operation.span,
            Some(op_label),
        ));
    }
    events.push(ParserTraceEvent::effect_exit(
        "decl",
        effect.span,
        effect_label,
    ));
}

fn record_stmt_trace_events(stmt: &Stmt, events: &mut Vec<ParserTraceEvent>) {
    match &stmt.kind {
        StmtKind::Decl { decl } => record_decl_trace_events(decl, events),
        StmtKind::Expr { expr } => record_expr_trace_events(expr, events),
        StmtKind::Assign { target, value } => {
            record_expr_trace_events(target, events);
            record_expr_trace_events(value, events);
        }
        StmtKind::Defer { expr } => record_expr_trace_events(expr, events),
    }
}

fn record_expr_trace_events(expr: &Expr, events: &mut Vec<ParserTraceEvent>) {
    let kind = expr_trace_kind(expr);
    events.push(ParserTraceEvent::expr_enter(kind, expr.span));
    if let Some(label) = resume_call_label(expr) {
        events.push(ParserTraceEvent::operation_resume(label, expr.span));
    }
    match &expr.kind {
        ExprKind::Literal(literal) => record_literal_trace_events(literal, events),
        ExprKind::Identifier(_) | ExprKind::ModulePath(_) => {}
        ExprKind::Call { callee, args } => {
            record_expr_trace_events(callee, events);
            for arg in args {
                record_expr_trace_events(arg, events);
            }
        }
        ExprKind::PerformCall { call } => {
            let label = Some(call.effect.name.clone());
            events.push(ParserTraceEvent::effect_enter(
                "perform",
                expr.span,
                label.clone(),
            ));
            record_expr_trace_events(&call.argument, events);
            events.push(ParserTraceEvent::effect_exit("perform", expr.span, label));
        }
        ExprKind::Lambda { body, .. } => {
            record_expr_trace_events(body, events);
        }
        ExprKind::Pipe { left, right } => {
            record_expr_trace_events(left, events);
            record_expr_trace_events(right, events);
        }
        ExprKind::Binary { left, right, .. } => {
            record_expr_trace_events(left, events);
            record_expr_trace_events(right, events);
        }
        ExprKind::Unary { expr: inner, .. } => record_expr_trace_events(inner, events),
        ExprKind::FieldAccess { target, .. }
        | ExprKind::TupleAccess { target, .. }
        | ExprKind::Propagate { expr: target }
        | ExprKind::Loop { body: target }
        | ExprKind::Unsafe { body: target }
        | ExprKind::Defer { body: target } => {
            record_expr_trace_events(target, events);
        }
        ExprKind::Index { target, index } => {
            record_expr_trace_events(target, events);
            record_expr_trace_events(index, events);
        }
        ExprKind::IfElse {
            condition,
            then_branch,
            else_branch,
        } => {
            record_expr_trace_events(condition, events);
            record_expr_trace_events(then_branch, events);
            if let Some(else_branch) = else_branch {
                record_expr_trace_events(else_branch, events);
            }
        }
        ExprKind::Match { target, arms } => {
            record_expr_trace_events(target, events);
            for arm in arms {
                if let Some(guard) = &arm.guard {
                    record_expr_trace_events(guard, events);
                }
                record_expr_trace_events(&arm.body, events);
            }
        }
        ExprKind::While { condition, body } => {
            record_expr_trace_events(condition, events);
            record_expr_trace_events(body, events);
        }
        ExprKind::For { start, end, .. } => {
            record_expr_trace_events(start, events);
            record_expr_trace_events(end, events);
        }
        ExprKind::Handle { handle } => {
            let handler = HandlerDecl {
                name: handle.handler.clone(),
                span: expr.span,
            };
            events.push(ParserTraceEvent::handler(&handler));
            record_expr_trace_events(&handle.target, events);
        }
        ExprKind::Continue => {}
        ExprKind::Block { statements } => {
            for stmt in statements {
                record_stmt_trace_events(stmt, events);
            }
        }
        ExprKind::Return { value } => {
            if let Some(expr) = value {
                record_expr_trace_events(expr, events);
            }
        }
        ExprKind::Assign { target, value } => {
            record_expr_trace_events(target, events);
            record_expr_trace_events(value, events);
        }
    }
    events.push(ParserTraceEvent::expr_leave(kind, expr.span));
}

fn record_literal_trace_events(literal: &Literal, events: &mut Vec<ParserTraceEvent>) {
    match &literal.value {
        LiteralKind::Tuple { elements } | LiteralKind::Array { elements } => {
            for element in elements {
                record_expr_trace_events(element, events);
            }
        }
        LiteralKind::Record { fields } => {
            for field in fields {
                record_expr_trace_events(&field.value, events);
            }
        }
        _ => {}
    }
}

fn expr_trace_kind(expr: &Expr) -> &'static str {
    match &expr.kind {
        ExprKind::Literal(_) => "literal",
        ExprKind::Identifier(_) => "identifier",
        ExprKind::ModulePath(_) => "module-path",
        ExprKind::Call { .. } => "call",
        ExprKind::PerformCall { .. } => "perform",
        ExprKind::Lambda { .. } => "lambda",
        ExprKind::Pipe { .. } => "pipe",
        ExprKind::Binary { .. } => "binary",
        ExprKind::Unary { .. } => "unary",
        ExprKind::FieldAccess { .. } => "field-access",
        ExprKind::TupleAccess { .. } => "tuple-access",
        ExprKind::Index { .. } => "index",
        ExprKind::Propagate { .. } => "propagate",
        ExprKind::IfElse { .. } => "if",
        ExprKind::Match { .. } => "match",
        ExprKind::While { .. } => "while",
        ExprKind::For { .. } => "for",
        ExprKind::Loop { .. } => "loop",
        ExprKind::Handle { .. } => "handle",
        ExprKind::Continue => "continue",
        ExprKind::Block { .. } => "block",
        ExprKind::Unsafe { .. } => "unsafe",
        ExprKind::Return { .. } => "return",
        ExprKind::Defer { .. } => "defer",
        ExprKind::Assign { .. } => "assign",
    }
}

fn resume_call_label(expr: &Expr) -> Option<String> {
    if let ExprKind::Call { callee, .. } = &expr.kind {
        if let ExprKind::Identifier(ident) = &callee.kind {
            if ident.name == "resume" {
                return Some(ident.name.clone());
            }
        }
    }
    None
}

fn parse_module_header_tokens(tokens: &[Token], start: usize) -> Option<(ModuleHeader, usize)> {
    let mut idx = start;
    if idx >= tokens.len() {
        return None;
    }
    let mut visibility = Visibility::Private;
    if tokens[idx].kind == TokenKind::KeywordPub {
        visibility = Visibility::Public;
        idx += 1;
    }
    let module_token = tokens.get(idx)?;
    if module_token.kind != TokenKind::KeywordModule {
        return None;
    }
    let span_start = if visibility == Visibility::Public {
        tokens[start].span
    } else {
        module_token.span
    };
    idx += 1;
    let (path, path_span, next_idx) = parse_module_path(tokens, idx)?;
    let header = ModuleHeader {
        path,
        visibility,
        attrs: Vec::new(),
        span: span_union(span_start, path_span),
    };
    Some((header, next_idx))
}

fn parse_use_decl_tokens(
    tokens: &[Token],
    start: usize,
) -> Option<(UseDecl, ParserTraceEvent, usize)> {
    let mut idx = start;
    if idx >= tokens.len() {
        return None;
    }
    let mut is_pub = false;
    if tokens[idx].kind == TokenKind::KeywordPub {
        if matches!(
            tokens.get(idx + 1),
            Some(token) if token.kind == TokenKind::KeywordUse
        ) {
            is_pub = true;
            idx += 1;
        } else {
            return None;
        }
    }
    let use_token = tokens.get(idx)?;
    if use_token.kind != TokenKind::KeywordUse {
        return None;
    }
    let span_start = if is_pub {
        tokens[start].span
    } else {
        use_token.span
    };
    idx += 1;
    let (path, mut span_end, next_idx) = parse_module_path(tokens, idx)?;
    idx = next_idx;
    let mut tree = UseTree::Path {
        path: path.clone(),
        alias: None,
    };
    if let Some(token) = tokens.get(idx) {
        match token.kind {
            TokenKind::KeywordAs => {
                idx += 1;
                let (alias, consumed_idx) = parse_ident_with_index(tokens, idx)?;
                idx = consumed_idx;
                span_end = span_union(span_end, alias.span);
                if let UseTree::Path { alias: slot, .. } = &mut tree {
                    *slot = Some(alias);
                }
            }
            TokenKind::Dot => {
                if let Some(next) = tokens.get(idx + 1) {
                    match next.kind {
                        TokenKind::LBrace => {
                            idx += 1;
                            let (items, brace_span, consumed_idx) = parse_use_items(tokens, idx)?;
                            idx = consumed_idx;
                            span_end = span_union(span_end, brace_span);
                            tree = UseTree::Brace {
                                path: path.clone(),
                                items,
                            };
                        }
                        TokenKind::Star => {
                            idx += 2;
                            let glob_item = UseItem {
                                name: None,
                                alias: None,
                                nested: Vec::new(),
                                glob: true,
                                span: next.span,
                            };
                            span_end = span_union(span_end, next.span);
                            tree = UseTree::Brace {
                                path: path.clone(),
                                items: vec![glob_item],
                            };
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
    if let Some(token) = tokens.get(idx) {
        if token.kind == TokenKind::Semicolon {
            idx += 1;
        }
    }
    let span = span_union(span_start, span_end);
    let decl = UseDecl { is_pub, tree, span };
    let event = ParserTraceEvent::use_decl(&decl);
    Some((decl, event, idx))
}

fn parse_module_path(tokens: &[Token], start: usize) -> Option<(ModulePath, Span, usize)> {
    let mut idx = start;
    if idx >= tokens.len() {
        return None;
    }
    if matches!(
        tokens.get(idx),
        Some(token) if token.kind == TokenKind::Colon
    ) && matches!(
        tokens.get(idx + 1),
        Some(token) if token.kind == TokenKind::Colon
    ) {
        idx += 2;
        let (first_segment, consumed_idx) = parse_ident_with_index(tokens, idx)?;
        idx = consumed_idx;
        let mut segments = vec![first_segment];
        let mut span_end = segments.last().map(|ident| ident.span).unwrap();
        while let Some(token) = tokens.get(idx) {
            if token.kind != TokenKind::Dot {
                break;
            }
            if let Some(next) = tokens.get(idx + 1) {
                match next.kind {
                    TokenKind::Identifier | TokenKind::UpperIdentifier => {
                        let (segment, consumed_idx) = parse_ident_with_index(tokens, idx + 1)?;
                        span_end = span_union(span_end, segment.span);
                        segments.push(segment);
                        idx = consumed_idx;
                    }
                    TokenKind::LBrace | TokenKind::Star | TokenKind::KeywordAs => break,
                    _ => break,
                }
            } else {
                break;
            }
        }
        let span_start = segments.first().map(|ident| ident.span).unwrap_or(span_end);
        let span = Span::new(span_start.start, span_end.end);
        return Some((ModulePath::Root { segments }, span, idx));
    }

    let token = tokens.get(idx)?;
    let mut head_span = token.span;
    let head = match token.kind {
        TokenKind::KeywordSelf => {
            idx += 1;
            RelativeHead::Self_
        }
        TokenKind::KeywordSuper => {
            idx += 1;
            let mut depth = 1u32;
            while let Some(dot) = tokens.get(idx) {
                if dot.kind != TokenKind::Dot {
                    break;
                }
                if let Some(next) = tokens.get(idx + 1) {
                    if next.kind == TokenKind::KeywordSuper {
                        head_span = span_union(head_span, next.span);
                        idx += 2;
                        depth = depth.saturating_add(1);
                        continue;
                    }
                }
                break;
            }
            RelativeHead::Super(depth)
        }
        TokenKind::Identifier | TokenKind::UpperIdentifier => {
            let (ident, consumed_idx) = parse_ident_with_index(tokens, idx)?;
            idx = consumed_idx;
            head_span = ident.span;
            RelativeHead::PlainIdent(ident)
        }
        _ => return None,
    };
    let mut segments = Vec::new();
    let mut span_end = head_span;
    while let Some(token) = tokens.get(idx) {
        if token.kind != TokenKind::Dot {
            break;
        }
        if let Some(next) = tokens.get(idx + 1) {
            match next.kind {
                TokenKind::Identifier | TokenKind::UpperIdentifier => {
                    let (ident, consumed_idx) = parse_ident_with_index(tokens, idx + 1)?;
                    span_end = span_union(span_end, ident.span);
                    segments.push(ident);
                    idx = consumed_idx;
                }
                TokenKind::LBrace | TokenKind::Star | TokenKind::KeywordAs => break,
                _ => break,
            }
        } else {
            break;
        }
    }
    let span = Span::new(head_span.start, span_end.end);
    Some((ModulePath::Relative { head, segments }, span, idx))
}

fn parse_use_items(tokens: &[Token], start: usize) -> Option<(Vec<UseItem>, Span, usize)> {
    let mut idx = start;
    let brace = tokens.get(idx)?;
    if brace.kind != TokenKind::LBrace {
        return None;
    }
    let mut span = brace.span;
    idx += 1;
    let mut items = Vec::new();
    loop {
        if idx >= tokens.len() {
            return None;
        }
        if let Some(token) = tokens.get(idx) {
            if token.kind == TokenKind::RBrace {
                span = span_union(span, token.span);
                idx += 1;
                break;
            }
        }
        let (item, item_span, consumed_idx) = parse_use_item(tokens, idx)?;
        span = span_union(span, item_span);
        idx = consumed_idx;
        items.push(item);
        if let Some(token) = tokens.get(idx) {
            match token.kind {
                TokenKind::Comma => idx += 1,
                TokenKind::RBrace => {}
                _ => {}
            }
        } else {
            return None;
        }
    }
    Some((items, span, idx))
}

fn parse_use_item(tokens: &[Token], start: usize) -> Option<(UseItem, Span, usize)> {
    let mut idx = start;
    let token = tokens.get(idx)?;
    if token.kind == TokenKind::Star {
        idx += 1;
        return Some((
            UseItem {
                name: None,
                alias: None,
                nested: Vec::new(),
                glob: true,
                span: token.span,
            },
            token.span,
            idx,
        ));
    }
    let (ident, consumed_idx) = parse_ident_with_index(tokens, idx)?;
    idx = consumed_idx;
    let mut span = ident.span;
    let mut alias = None;
    if let Some(peek) = tokens.get(idx) {
        if peek.kind == TokenKind::KeywordAs {
            idx += 1;
            let (alias_ident, consumed_idx) = parse_ident_with_index(tokens, idx)?;
            span = span_union(span, alias_ident.span);
            alias = Some(alias_ident.clone());
            idx = consumed_idx;
        }
    }
    let mut nested = Vec::new();
    if let Some(dot) = tokens.get(idx) {
        if dot.kind == TokenKind::Dot {
            if let Some(next) = tokens.get(idx + 1) {
                if next.kind == TokenKind::LBrace {
                    idx += 1;
                    let (items, brace_span, consumed_idx) = parse_use_items(tokens, idx)?;
                    nested = items;
                    span = span_union(span, brace_span);
                    idx = consumed_idx;
                }
            }
        }
    }
    let item = UseItem {
        name: Some(ident),
        alias,
        nested,
        glob: false,
        span,
    };
    Some((item, span, idx))
}

fn parse_ident_with_index(tokens: &[Token], start: usize) -> Option<(Ident, usize)> {
    let token = tokens.get(start)?;
    match token.kind {
        TokenKind::Identifier | TokenKind::UpperIdentifier => {
            let name = token
                .lexeme
                .clone()
                .or_else(|| token.kind.keyword_literal().map(|text| text.to_string()))
                .unwrap_or_default();
            Some((
                Ident {
                    name,
                    span: token.span,
                },
                start + 1,
            ))
        }
        _ => None,
    }
}

fn span_union(left: Span, right: Span) -> Span {
    Span::new(left.start.min(right.start), left.end.max(right.end))
}

fn merge_qualified_ident(first: Ident, rest: Vec<Ident>) -> Ident {
    rest.into_iter().fold(first, |mut acc, segment| {
        acc.name.push_str("::");
        acc.name.push_str(&segment.name);
        acc.span = span_union(acc.span, segment.span);
        acc
    })
}

fn range_to_span(span: Range<usize>) -> Span {
    Span::new(span.start as u32, span.end as u32)
}

fn build_stmt_parser<P, Q, R>(
    expr: P,
    pattern_var: Q,
    type_parser: R,
) -> impl ChumskyParser<TokenKind, Stmt, Error = Simple<TokenKind>> + Clone
where
    P: ChumskyParser<TokenKind, Expr, Error = Simple<TokenKind>> + Clone,
    Q: ChumskyParser<TokenKind, Pattern, Error = Simple<TokenKind>> + Clone,
    R: ChumskyParser<TokenKind, TypeAnnot, Error = Simple<TokenKind>> + Clone,
{
    let let_stmt = just(TokenKind::KeywordLet)
        .ignore_then(pattern_var.clone())
        .then(
            just(TokenKind::Colon)
                .ignore_then(type_parser.clone())
                .or_not(),
        )
        .then_ignore(just(TokenKind::Assign))
        .then(expr.clone())
        .map_with_span(|((pattern, ty), value), span: Range<usize>| {
            let decl = Decl {
                attrs: Vec::new(),
                visibility: Visibility::Private,
                kind: DeclKind::Let {
                    pattern,
                    value,
                    type_annotation: ty,
                },
                span: range_to_span(span.clone()),
            };
            Stmt {
                kind: StmtKind::Decl { decl },
                span: range_to_span(span),
            }
        });

    let expr_stmt = expr.map_with_span(|expression, span: Range<usize>| Stmt {
        kind: StmtKind::Expr {
            expr: Box::new(expression),
        },
        span: range_to_span(span),
    });

    choice((let_stmt, expr_stmt))
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
    recovered: bool,
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
            recovered: false,
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

        self.recovered = true;

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

impl StreamingRecoverController {
    pub fn recovered(&self) -> bool {
        self.recovered
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
fn main() = emit("leak")"#,
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
            "ここで`)`、`,` または `:`のいずれかが必要です"
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

#[cfg(test)]
mod parser_option_tests {
    use super::api::RunConfig;
    use super::*;
    use serde_json::{json, Value};

    #[test]
    fn parser_options_follow_lex_identifier_profile_extension() {
        let run_config = RunConfig::default().with_extension("lex", |existing| {
            let mut payload = existing
                .and_then(|value| value.as_object().cloned())
                .unwrap_or_default();
            payload.insert("identifier_profile".to_string(), json!("ascii-compat"));
            Value::Object(payload)
        });
        let options = ParserOptions::from_run_config(&run_config);
        assert_eq!(
            options.lex_identifier_profile,
            IdentifierProfile::AsciiCompat
        );
    }
}

#[cfg(test)]
mod expectation_tests {
    use super::{token_kind_expectations, TokenKind};
    use crate::diagnostic::{
        ExpectedToken, ExpectedTokenCollector, FrontendDiagnostic, EXPECTED_PLACEHOLDER_TOKEN,
        PARSE_EXPECTED_EMPTY_KEY,
    };

    #[test]
    fn keyword_expectations_cover_spec_keywords() {
        let keywords = [
            (TokenKind::KeywordVar, "var"),
            (TokenKind::KeywordMatch, "match"),
            (TokenKind::KeywordType, "type"),
        ];

        for &(kind, label) in keywords.iter() {
            let expectations = token_kind_expectations(&kind);
            assert_eq!(expectations, vec![ExpectedToken::keyword(label)]);
        }
    }

    #[test]
    fn identifier_expectations_group_by_profile() {
        let upper = token_kind_expectations(&TokenKind::UpperIdentifier);
        assert_eq!(upper, vec![ExpectedToken::class("upper-identifier")]);

        let lower = token_kind_expectations(&TokenKind::Identifier);
        assert_eq!(lower, vec![ExpectedToken::class("identifier")]);
    }

    #[test]
    fn empty_expected_summary_injects_placeholder_tokens() {
        let summary = ExpectedTokenCollector::new().summarize();
        assert!(summary.alternatives.is_empty());

        let diagnostic = FrontendDiagnostic::new("oops").apply_expected_summary(&summary);
        assert_eq!(
            diagnostic.expected_tokens,
            vec![EXPECTED_PLACEHOLDER_TOKEN.to_string()]
        );
        assert_eq!(
            diagnostic.expected_message_key.as_deref(),
            Some(PARSE_EXPECTED_EMPTY_KEY)
        );
        assert!(diagnostic.expected_alternatives.is_empty());
    }
}

#[cfg(test)]
mod parse_result_tests {
    use super::ParserDriver;

    #[test]
    fn parse_failure_records_offset_and_expected_summary() {
        let result = ParserDriver::parse("fn broken( ->");
        assert!(result.value.is_none());
        assert!(!result.diagnostics.is_empty());
        assert!(result.farthest_error_offset.is_some());

        let diag = result.diagnostics.first().expect("diagnostics missing");
        assert!(
            diag.expected_summary
                .as_ref()
                .map(|summary| summary.has_alternatives())
                .unwrap_or(false),
            "expected summary alternatives missing"
        );

        let legacy = result.legacy_error.expect("legacy error missing");
        assert!(
            !legacy.expected.is_empty(),
            "legacy expected tokens should not be empty"
        );
    }

    #[test]
    fn lexer_error_generates_diagnostics() {
        let result = ParserDriver::parse("fn @@@");
        assert!(
            !result.diagnostics.is_empty(),
            "lexer error should produce diagnostics"
        );
    }
}
