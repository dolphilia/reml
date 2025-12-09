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

const CODE_UNKNOWN_TOKEN: &str = "parser.lexer.unknown_token";
const CODE_MISSING_TOKEN: &str = "parser.syntax.missing_token";
const CODE_UNEXPECTED_STRUCTURE: &str = "parser.syntax.unexpected_structure";
const CODE_INTERNAL_STATE: &str = "parser.internal.state";
const CODE_EXPECTED_TOKENS: &str = "parser.syntax.expected_tokens";

use crate::diagnostic::{
    recover::ExpectedTokensSummary, DiagnosticBuilder, DiagnosticDomain, DiagnosticNote,
    DiagnosticSeverity, ExpectedToken, ExpectedTokenCollector, FrontendDiagnostic,
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
    Attribute, ConductorArg, ConductorChannelRoute, ConductorDecl, ConductorDslDef,
    ConductorDslTail, ConductorEndpoint, ConductorExecutionBlock, ConductorMonitorTarget,
    ConductorMonitoringBlock, ConductorPipelineSpec, Decl, DeclKind, EffectAnnotation, EffectDecl,
    Expr, ExprKind, Function, FunctionSignature, HandleExpr, HandlerDecl, HandlerEntry, Ident,
    ImplDecl, ImplItem, Literal, LiteralKind, MatchArm, Module, ModuleHeader, ModulePath,
    OperationDecl, Param, Pattern, PatternKind, RecordField, RelativeHead, Stmt, StmtKind,
    TraitDecl, TraitItem, TraitRef, TypeAnnot, TypeKind, UseDecl, UseItem, UseTree, Visibility,
    WherePredicate,
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
        diagnostics
            .extend(errors.into_iter().map(Self::error_to_diagnostic))
            .expect("lexer diagnostics must include severity/domain/code");
        diagnostics
            .extend(detect_handle_missing_with_tokens(&tokens).into_iter())
            .expect("token diagnostics must include severity/domain/code");

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

        let mut diagnostics = diagnostics.into_vec();
        if !span_trace.is_empty() {
            for diagnostic in &mut diagnostics {
                if diagnostic.span_trace.is_empty() {
                    diagnostic.set_span_trace(span_trace.clone());
                }
            }
        }
        if let Some(module) = ast.as_ref() {
            collect_effect_handler_diagnostics(module, &mut diagnostics);
        }

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
        let mut diagnostic = FrontendDiagnostic::new(error.message())
            .with_severity(DiagnosticSeverity::Error)
            .with_domain(DiagnosticDomain::Parser);

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
                diagnostic.push_code(CODE_UNKNOWN_TOKEN);
            }
            crate::error::FrontendErrorKind::MissingToken { span, .. } => {
                diagnostic = diagnostic.with_span(span);
                diagnostic.push_code(CODE_MISSING_TOKEN);
            }
            crate::error::FrontendErrorKind::UnexpectedStructure { span, unicode, .. } => {
                if let Some(span) = span {
                    diagnostic = diagnostic.with_span(span);
                }
                if let Some(detail) = unicode.clone() {
                    diagnostic = diagnostic.with_unicode_detail(detail.clone());
                    diagnostic.push_code(unicode_diagnostic_code(detail.kind()));
                } else {
                    diagnostic.push_code(CODE_UNEXPECTED_STRUCTURE);
                }
            }
            crate::error::FrontendErrorKind::InternalState { .. } => {
                diagnostic.push_code(CODE_INTERNAL_STATE);
            }
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
        .filter_map(|diag| diag.primary_span().map(|span| span.end))
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
    let mut diagnostic = FrontendDiagnostic::new(message)
        .with_severity(DiagnosticSeverity::Error)
        .with_domain(DiagnosticDomain::Parser)
        .with_code(CODE_EXPECTED_TOKENS);
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

fn collect_effect_handler_diagnostics(module: &Module, diagnostics: &mut Vec<FrontendDiagnostic>) {
    fn record(expr: &Expr, diagnostics: &mut Vec<FrontendDiagnostic>) {
        if let ExprKind::Handle { handle } = &expr.kind {
            if !handle.with_keyword {
                let mut diagnostic = FrontendDiagnostic::new(
                    "`handle expr with handler` 構文で `with` キーワードが欠落しています。",
                )
                .with_severity(DiagnosticSeverity::Error)
                .with_domain(DiagnosticDomain::Parser)
                .with_code("effects.handler.missing_with")
                .with_recoverability(Recoverability::Recoverable)
                .with_span(handle.handler.span);
                diagnostic.add_note(DiagnosticNote::new(
                    "effects.handler.fix",
                    "`handle emit() with handler Console { ... }` の形式へ修正してください。",
                ));
                diagnostics.push(diagnostic);
            }
            record(&handle.target, diagnostics);
        }
        match &expr.kind {
            ExprKind::Literal(_) | ExprKind::Identifier(_) | ExprKind::ModulePath(_) => {}
            ExprKind::Call { callee, args } => {
                record(callee, diagnostics);
                for arg in args {
                    record(arg, diagnostics);
                }
            }
            ExprKind::PerformCall { call } => record(&call.argument, diagnostics),
            ExprKind::Lambda { body, .. }
            | ExprKind::Loop { body }
            | ExprKind::Unsafe { body }
            | ExprKind::Defer { body } => record(body, diagnostics),
            ExprKind::Pipe { left, right } | ExprKind::Binary { left, right, .. } => {
                record(left, diagnostics);
                record(right, diagnostics);
            }
            ExprKind::Unary { expr: inner, .. }
            | ExprKind::Propagate { expr: inner }
            | ExprKind::Return { value: Some(inner) } => record(inner, diagnostics),
            ExprKind::Return { value: None } | ExprKind::Continue => {}
            ExprKind::FieldAccess { target, .. }
            | ExprKind::TupleAccess { target, .. }
            | ExprKind::Index { target, .. } => record(target, diagnostics),
            ExprKind::IfElse {
                condition,
                then_branch,
                else_branch,
            } => {
                record(condition, diagnostics);
                record(then_branch, diagnostics);
                if let Some(branch) = else_branch {
                    record(branch, diagnostics);
                }
            }
            ExprKind::Match { target, arms } => {
                record(target, diagnostics);
                for arm in arms {
                    if let Some(guard) = &arm.guard {
                        record(guard, diagnostics);
                    }
                    record(&arm.body, diagnostics);
                }
            }
            ExprKind::While { condition, body } => {
                record(condition, diagnostics);
                record(body, diagnostics);
            }
            ExprKind::For { start, end, .. } => {
                record(start, diagnostics);
                record(end, diagnostics);
            }
            ExprKind::Block { statements, .. } => {
                for stmt in statements {
                    match &stmt.kind {
                        StmtKind::Decl { decl } => match &decl.kind {
                            DeclKind::Let { value, .. } | DeclKind::Var { value, .. } => {
                                record(value, diagnostics)
                            }
                            _ => {}
                        },
                        StmtKind::Expr { expr } | StmtKind::Defer { expr } => {
                            record(expr, diagnostics)
                        }
                        StmtKind::Assign { target, value } => {
                            record(target, diagnostics);
                            record(value, diagnostics);
                        }
                    }
                }
            }
            ExprKind::Assign { target, value } => {
                record(target, diagnostics);
                record(value, diagnostics);
            }
            ExprKind::Handle { .. } => {}
        }
    }

    for function in &module.functions {
        record(&function.body, diagnostics);
    }
    for decl in &module.decls {
        match &decl.kind {
            DeclKind::Let { value, .. } | DeclKind::Var { value, .. } => record(value, diagnostics),
            _ => {}
        }
    }
}

fn detect_handle_missing_with_tokens(tokens: &[Token]) -> Vec<FrontendDiagnostic> {
    let mut diags = Vec::new();
    let mut pending_handle: Option<Span> = None;
    for token in tokens {
        match token.kind {
            TokenKind::KeywordHandle => pending_handle = Some(token.span),
            TokenKind::KeywordWith => pending_handle = None,
            TokenKind::KeywordHandler => {
                if pending_handle.take().is_some() {
                    let mut diagnostic = FrontendDiagnostic::new(
                        "`handle expr with handler` 構文で `with` キーワードが欠落しています。",
                    )
                    .with_severity(DiagnosticSeverity::Error)
                    .with_domain(DiagnosticDomain::Parser)
                    .with_code("effects.handler.missing_with")
                    .with_recoverability(Recoverability::Recoverable)
                    .with_span(token.span);
                    diagnostic.add_note(DiagnosticNote::new(
                        "effects.handler.fix",
                        "`handle emit() with handler Console { ... }` の形式へ修正してください。",
                    ));
                    diags.push(diagnostic);
                }
            }
            TokenKind::Semicolon
            | TokenKind::RBrace
            | TokenKind::KeywordFn
            | TokenKind::KeywordLet
            | TokenKind::KeywordVar => pending_handle = None,
            _ => {}
        }
    }
    diags
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
    error.unicode = Some(UnicodeDetail::from_error(unicode_error).with_phase(phase.to_string()));
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
        ET::keyword("fn"),
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
        just(TokenKind::KeywordSelf),
    ))
    .map_with_span(move |_, span: Range<usize>| {
        let slice = &source[span.start..span.end];
        (slice.to_string(), range_to_span(span))
    });

    let ident = identifier.clone().map(|(name, span)| Ident { name, span });

    let lower_ident = just(TokenKind::Identifier).map_with_span(move |_, span: Range<usize>| {
        let slice = &source[span.start..span.end];
        Ident {
            name: slice.to_string(),
            span: range_to_span(span),
        }
    });

    let int_literal = just(TokenKind::IntLiteral).map_with_span(move |_, span: Range<usize>| {
        let slice = &source[span.start..span.end];
        let value = slice.parse::<i64>().unwrap_or_default();
        Expr::int(value, slice.to_string(), range_to_span(span))
    });

    let separator = choice((
        just(TokenKind::Dot).to(()),
        just(TokenKind::Colon)
            .ignore_then(just(TokenKind::Colon))
            .to(()),
    ));

    let qualified_ident = ident
        .clone()
        .then(separator.clone().ignore_then(ident.clone()).repeated())
        .map(|(first, rest)| merge_qualified_ident(first, rest));

    let dotted_ident = ident
        .clone()
        .then(just(TokenKind::Dot).ignore_then(ident.clone()).repeated())
        .map(|(first, rest)| merge_dotted_ident(first, rest));

    let type_parser = recursive(|ty| {
        let args = ty
            .clone()
            .separated_by(just(TokenKind::Comma))
            .allow_trailing()
            .delimited_by(just(TokenKind::Lt), just(TokenKind::Gt));

        let tuple_type = ty
            .clone()
            .separated_by(just(TokenKind::Comma))
            .allow_trailing()
            .delimited_by(just(TokenKind::LParen), just(TokenKind::RParen))
            .map_with_span(|elements, span: Range<usize>| TypeAnnot {
                span: range_to_span(span),
                kind: TypeKind::Tuple { elements },
                annotation_kind: None,
            });

        let simple = qualified_ident.clone().map(|name| TypeAnnot {
            span: name.span,
            kind: TypeKind::Ident { name },
            annotation_kind: None,
        });

        let app = qualified_ident.clone().then(args.clone()).map_with_span(
            |(callee, args), span: Range<usize>| TypeAnnot {
                span: range_to_span(span),
                kind: TypeKind::App { callee, args },
                annotation_kind: None,
            },
        );

        let record_field = ident
            .clone()
            .then_ignore(just(TokenKind::Colon))
            .then(ty.clone());

        let record_type = record_field
            .separated_by(just(TokenKind::Comma))
            .allow_trailing()
            .delimited_by(just(TokenKind::LBrace), just(TokenKind::RBrace))
            .map_with_span(|fields, span: Range<usize>| TypeAnnot {
                span: range_to_span(span),
                kind: TypeKind::Record { fields },
                annotation_kind: None,
            });

        let atom = choice((tuple_type, record_type, app, simple));

        atom.clone()
            .then(just(TokenKind::Arrow).ignore_then(ty.clone()).or_not())
            .map(|(left, ret_opt)| {
                if let Some(ret_ty) = ret_opt {
                    let span = Span::new(left.span.start, ret_ty.span.end);
                    let left_span = left.span;
                    let left_annotation_kind = left.annotation_kind;
                    match left.kind {
                        TypeKind::Tuple { elements } => TypeAnnot {
                            span,
                            kind: TypeKind::Fn {
                                params: elements,
                                ret: Box::new(ret_ty),
                            },
                            annotation_kind: None,
                        },
                        other_kind => {
                            let param = TypeAnnot {
                                span: left_span,
                                kind: other_kind,
                                annotation_kind: left_annotation_kind,
                            };
                            TypeAnnot {
                                span,
                                kind: TypeKind::Fn {
                                    params: vec![param],
                                    ret: Box::new(ret_ty),
                                },
                                annotation_kind: None,
                            }
                        }
                    }
                } else {
                    left
                }
            })
    });

    let type_parser_for_expr = type_parser.clone();
    let pattern_var = lower_ident.clone().map(|ident| Pattern {
        span: ident.span,
        kind: PatternKind::Var(ident),
    });

    let pattern = recursive(|pat| {
        let tuple_pattern = pat
            .clone()
            .separated_by(just(TokenKind::Comma))
            .at_least(2)
            .allow_trailing()
            .delimited_by(just(TokenKind::LParen), just(TokenKind::RParen))
            .map_with_span(|elements, span: Range<usize>| Pattern {
                span: range_to_span(span),
                kind: PatternKind::Tuple { elements },
            });

        let pattern_ctor = qualified_ident
            .clone()
            .then(
                pat.clone()
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

        choice((
            tuple_pattern,
            pattern_var.clone(),
            pattern_ctor,
            wildcard_pattern,
        ))
    });

    let pattern_for_expr = pattern.clone();
    let pattern_for_block = pattern.clone();
    let ident_for_expr = ident.clone();
    let expr = recursive(move |expr| {
        let attribute = build_attribute_parser(expr.clone(), ident_for_expr.clone());
        let attr_list = attribute.clone().repeated();
        let ident_expr = ident_for_expr.clone().map(Expr::identifier);

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

        let tuple_literal = expr
            .clone()
            .separated_by(just(TokenKind::Comma))
            .at_least(2)
            .allow_trailing()
            .delimited_by(just(TokenKind::LParen), just(TokenKind::RParen))
            .map_with_span(|elements, span: Range<usize>| {
                Expr::literal(
                    Literal {
                        value: LiteralKind::Tuple { elements },
                    },
                    range_to_span(span),
                )
            });

        let paren_expr = expr
            .clone()
            .delimited_by(just(TokenKind::LParen), just(TokenKind::RParen));

        let array_literal = expr
            .clone()
            .separated_by(just(TokenKind::Comma))
            .allow_trailing()
            .delimited_by(just(TokenKind::LBracket), just(TokenKind::RBracket))
            .map_with_span(|elements, span: Range<usize>| {
                Expr::literal(
                    Literal {
                        value: LiteralKind::Array { elements },
                    },
                    range_to_span(span),
                )
            });

        let record_literal_field = ident_for_expr
            .clone()
            .then_ignore(just(TokenKind::Assign))
            .then(expr.clone());

        let record_literal = record_literal_field
            .separated_by(just(TokenKind::Comma))
            .allow_trailing()
            .delimited_by(just(TokenKind::LBrace), just(TokenKind::RBrace))
            .map_with_span(|fields, span: Range<usize>| {
                let mapped_fields = fields
                    .into_iter()
                    .map(|(key, value)| RecordField { key, value })
                    .collect::<Vec<_>>();
                Expr::literal(
                    Literal {
                        value: LiteralKind::Record {
                            fields: mapped_fields,
                        },
                    },
                    range_to_span(span),
                )
            });

        let stmt = build_stmt_parser(
            expr.clone(),
            pattern_for_expr.clone(),
            type_parser_for_expr.clone(),
            ident_expr.clone(),
        );

        let raw_block = just(TokenKind::LBrace)
            .ignore_then(stmt.repeated())
            .then_ignore(just(TokenKind::RBrace))
            .map_with_span(|stmts, span: Range<usize>| (stmts, range_to_span(span)));

        let block_expr = attr_list
            .clone()
            .then(raw_block.clone())
            .map(|(attrs, (stmts, span))| {
                if attrs.is_empty() {
                    Expr::block(stmts, span)
                } else {
                    Expr::block_with_attrs(stmts, attrs, span)
                }
            });

        let match_guard = just(TokenKind::KeywordWhen)
            .ignore_then(expr.clone())
            .then(
                just(TokenKind::KeywordAs)
                    .ignore_then(lower_ident.clone())
                    .map(Some)
                    .or_not(),
            )
            .map(|(guard_expr, alias)| (guard_expr, alias.unwrap_or(None)))
            .boxed();

        let match_arm = just(TokenKind::Bar)
            .or_not()
            .ignore_then(pattern_for_expr.clone())
            .then(match_guard.or_not())
            .then(
                just(TokenKind::KeywordAs)
                    .ignore_then(lower_ident.clone())
                    .map(Some)
                    .or_not(),
            )
            .then_ignore(just(TokenKind::Arrow))
            .then(expr.clone())
            .try_map(
                |(((pattern, guard_info), alias_after_guard), body), span: Range<usize>| {
                    let (guard, alias_from_guard) = guard_info
                        .map(|(guard_expr, alias)| (Some(guard_expr), alias))
                        .unwrap_or((None, None));
                    let alias_tail = alias_after_guard.flatten();
                    if alias_from_guard.is_some() && alias_tail.is_some() {
                        return Err(Simple::custom(
                            span.clone(),
                            "match arm に複数の `as` は指定できません",
                        ));
                    }
                    let alias = alias_from_guard.or(alias_tail);
                    Ok(MatchArm {
                        pattern,
                        guard,
                        alias,
                        body,
                        span: range_to_span(span),
                    })
                },
            );

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

        let handler_param = ident_for_expr
            .clone()
            .then(
                just(TokenKind::Colon)
                    .ignore_then(type_parser_for_expr.clone())
                    .or_not(),
            )
            .then(just(TokenKind::Assign).ignore_then(expr.clone()).or_not())
            .map(|((name, ty), default)| Param {
                span: name.span,
                name,
                type_annotation: ty,
                default,
            });

        let handler_param_list = handler_param
            .separated_by(just(TokenKind::Comma))
            .allow_trailing()
            .or_not()
            .map(|params| params.unwrap_or_default())
            .delimited_by(just(TokenKind::LParen), just(TokenKind::RParen));

        let handler_operation_entry = attr_list
            .clone()
            .then_ignore(just(TokenKind::KeywordOperation))
            .then(ident_for_expr.clone())
            .then(handler_param_list.clone())
            .then(block_expr.clone())
            .map_with_span(|(((attrs, name), params), body), span: Range<usize>| {
                HandlerEntry::Operation {
                    attrs,
                    name,
                    params,
                    body,
                    span: range_to_span(span),
                }
            });

        let handler_return_entry = just(TokenKind::KeywordReturn)
            .ignore_then(ident_for_expr.clone())
            .then(block_expr.clone())
            .map_with_span(
                |(value_ident, body), span: Range<usize>| HandlerEntry::Return {
                    value_ident,
                    body,
                    span: range_to_span(span),
                },
            );

        let handler_literal = just(TokenKind::KeywordHandler)
            .ignore_then(ident_for_expr.clone())
            .then(
                just(TokenKind::LBrace)
                    .ignore_then(
                        choice((handler_operation_entry, handler_return_entry))
                            .repeated()
                            .at_least(1),
                    )
                    .then_ignore(just(TokenKind::RBrace)),
            )
            .map_with_span(|(name, entries), span: Range<usize>| HandlerDecl {
                name,
                entries,
                span: range_to_span(span),
            });

        let handle_expr = just(TokenKind::KeywordHandle)
            .ignore_then(expr.clone())
            .then(just(TokenKind::KeywordWith).to(true).or_not())
            .then(handler_literal.clone())
            .map_with_span(
                |((target, with_present), handler), span: Range<usize>| Expr {
                    span: range_to_span(span),
                    kind: ExprKind::Handle {
                        handle: HandleExpr {
                            target: Box::new(target),
                            handler,
                            with_keyword: with_present.unwrap_or(false),
                        },
                    },
                },
            );

        let lambda_body_expr = choice((block_expr.clone(), expr.clone())).boxed();
        let lambda_param = ident_for_expr
            .clone()
            .then(
                just(TokenKind::Colon)
                    .ignore_then(type_parser_for_expr.clone())
                    .or_not(),
            )
            .then(just(TokenKind::Assign).ignore_then(expr.clone()).or_not());

        let lambda_params = lambda_param
            .map(|((name, ty), default)| Param {
                span: name.span,
                name,
                type_annotation: ty,
                default,
            })
            .separated_by(just(TokenKind::Comma))
            .allow_trailing()
            .delimited_by(just(TokenKind::LParen), just(TokenKind::RParen));

        let fn_lambda_expr = just(TokenKind::KeywordFn)
            .ignore_then(lambda_params.clone())
            .then(
                just(TokenKind::Arrow)
                    .ignore_then(type_parser_for_expr.clone())
                    .or_not(),
            )
            .then_ignore(just(TokenKind::DoubleArrow))
            .then(lambda_body_expr.clone())
            .map_with_span(|((params, ret_type), body), span: Range<usize>| Expr {
                span: range_to_span(span),
                kind: ExprKind::Lambda {
                    params,
                    ret_type,
                    body: Box::new(body),
                },
            });

        let atom = choice((
            block_expr.clone(),
            match_expr,
            handle_expr,
            fn_lambda_expr,
            int_literal.clone(),
            bool_literal,
            string_literal,
            array_literal,
            record_literal,
            tuple_literal,
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
            ident_for_expr.clone(),
            just(TokenKind::KeywordNew).map_with_span(|_, span: Range<usize>| Ident {
                name: "new".to_string(),
                span: range_to_span(span),
            }),
            just(TokenKind::KeywordThen).map_with_span(|_, span: Range<usize>| Ident {
                name: "then".to_string(),
                span: range_to_span(span),
            }),
        ));

        let postfix = choice((
            call_args.map(|(args, span)| Postfix::Call(args, span)),
            separator.clone().ignore_then(field_ident).map(|field| {
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

        let multiplicative = call
            .clone()
            .then(
                choice((
                    just(TokenKind::Star).to("*"),
                    just(TokenKind::Slash).to("/"),
                    just(TokenKind::Percent).to("%"),
                ))
                .then(call.clone())
                .repeated(),
            )
            .map(|(first, rest)| {
                rest.into_iter().fold(first, |lhs, (op, rhs)| {
                    let span = span_union(lhs.span(), rhs.span());
                    Expr::binary(op, lhs, rhs, span)
                })
            });

        let additive = multiplicative
            .clone()
            .then(
                choice((
                    just(TokenKind::Plus).to("+"),
                    just(TokenKind::Minus).to("-"),
                ))
                .then(multiplicative.clone())
                .repeated(),
            )
            .map(|(first, rest)| {
                rest.into_iter().fold(first, |lhs, (op, rhs)| {
                    let span = span_union(lhs.span(), rhs.span());
                    Expr::binary(op, lhs, rhs, span)
                })
            });

        let comparison = additive
            .clone()
            .then(
                choice((
                    just(TokenKind::Gt).to(">"),
                    just(TokenKind::Ge).to(">="),
                    just(TokenKind::Lt).to("<"),
                    just(TokenKind::Le).to("<="),
                    just(TokenKind::EqEq).to("=="),
                    just(TokenKind::NotEqual).to("!="),
                ))
                .then(additive.clone())
                .repeated(),
            )
            .map(|(first, rest)| {
                rest.into_iter().fold(first, |lhs, (op, rhs)| {
                    let span = span_union(lhs.span(), rhs.span());
                    Expr::binary(op, lhs, rhs, span)
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

        let effect_args = expr
            .clone()
            .separated_by(just(TokenKind::Comma))
            .allow_trailing()
            .delimited_by(just(TokenKind::LParen), just(TokenKind::RParen))
            .map_with_span(|args, span: Range<usize>| (args, range_to_span(span)));

        let perform_expr = just(TokenKind::KeywordPerform)
            .map_with_span(move |_, span: Range<usize>| range_to_span(span))
            .then(qualified_ident.clone())
            .then(effect_args.clone())
            .map(|((perform_span, effect), (args, args_span))| {
                let argument = build_effect_argument_expr(args, args_span);
                let effect_span = effect.span;
                let span = span_union(perform_span, span_union(effect_span, args_span));
                Expr::perform(effect, argument, span)
            });

        let do_expr = just(TokenKind::KeywordDo)
            .map_with_span(move |_, span: Range<usize>| range_to_span(span))
            .then(qualified_ident.clone())
            .then(effect_args.clone())
            .map(|((do_span, effect), (args, args_span))| {
                let argument = build_effect_argument_expr(args, args_span);
                let effect_span = effect.span;
                let span = span_union(do_span, span_union(effect_span, args_span));
                Expr::perform(effect, argument, span)
            });

        let assignment_expr = ident_expr
            .clone()
            .then_ignore(just(TokenKind::Assign))
            .then(expr.clone())
            .map_with_span(|(target, value), span: Range<usize>| {
                Expr::assign(target, value, range_to_span(span))
            });

        choice((
            if_expr,
            perform_expr,
            do_expr,
            assignment_expr,
            block_expr,
            comparison,
        ))
        .boxed()
    });

    let attribute = build_attribute_parser(expr.clone(), ident.clone());
    let attr_list = attribute.clone().repeated();

    let param = ident
        .clone()
        .then(
            just(TokenKind::Colon)
                .ignore_then(type_parser.clone())
                .or_not(),
        )
        .then(just(TokenKind::Assign).ignore_then(expr.clone()).or_not());

    let params = param
        .map(|((name, ty), default)| Param {
            span: name.span,
            name,
            type_annotation: ty,
            default,
        })
        .separated_by(just(TokenKind::Comma))
        .allow_trailing()
        .delimited_by(just(TokenKind::LParen), just(TokenKind::RParen));

    let generic_params = ident
        .clone()
        .separated_by(just(TokenKind::Comma))
        .allow_trailing()
        .delimited_by(just(TokenKind::Lt), just(TokenKind::Gt));

    let parse_generics = generic_params
        .clone()
        .or_not()
        .map(|params| params.unwrap_or_default());

    let trait_reference = type_parser.clone().try_map(|ty, span: Range<usize>| {
        TraitRef::from_type_annotation(&ty).ok_or(Simple::custom(span, "トレイト参照が必要です"))
    });

    let type_bound = type_parser
        .clone()
        .then_ignore(just(TokenKind::Colon))
        .then(
            trait_reference
                .clone()
                .separated_by(just(TokenKind::Comma))
                .at_least(1),
        )
        .map_with_span(
            |(target, bounds), span: Range<usize>| WherePredicate::TypeBound {
                target,
                bounds,
                span: range_to_span(span),
            },
        );

    let where_predicate = choice((
        type_bound,
        trait_reference.map(|trait_ref| WherePredicate::Trait { trait_ref }),
    ));

    let where_clause = just(TokenKind::KeywordWhere)
        .ignore_then(
            where_predicate
                .clone()
                .separated_by(just(TokenKind::Comma))
                .allow_trailing(),
        )
        .or_not()
        .map(|predicates| predicates.unwrap_or_default());

    let effect_annotation = just(TokenKind::Not)
        .ignore_then(
            ident
                .clone()
                .separated_by(just(TokenKind::Comma))
                .allow_trailing()
                .or_not()
                .map(|tags| tags.unwrap_or_default())
                .delimited_by(just(TokenKind::LBrace), just(TokenKind::RBrace)),
        )
        .map_with_span(|tags, span: Range<usize>| EffectAnnotation {
            tags,
            span: range_to_span(span),
        });

    let fn_signature = just(TokenKind::KeywordFn)
        .map_with_span(move |_, span: Range<usize>| range_to_span(span))
        .then(ident.clone())
        .then(parse_generics.clone())
        .then(params.clone())
        .then(
            just(TokenKind::Arrow)
                .ignore_then(type_parser.clone())
                .or_not(),
        )
        .then(effect_annotation.clone().or_not())
        .then(where_clause.clone())
        .then(effect_annotation.clone().or_not())
        .map_with_span(
            |(
                (
                    (((((fn_span, name), generics), params), ret_type), effect_before_where),
                    where_clause,
                ),
                effect_after_where,
            ),
             span: Range<usize>| {
                let effect = effect_after_where.or(effect_before_where);
                let signature_span = range_to_span(span);
                FunctionSignature {
                    name,
                    generics,
                    params,
                    ret_type,
                    where_clause,
                    effect,
                    span: Span::new(fn_span.start.min(signature_span.start), signature_span.end),
                }
            },
        );

    let let_decl_raw = build_let_decl_parser(pattern.clone(), type_parser.clone(), expr.clone());
    let let_decl = attr_list
        .clone()
        .then(let_decl_raw.clone())
        .map(|(attrs, mut decl)| {
            if !attrs.is_empty() {
                decl.attrs = attrs;
            }
            decl
        });

    let var_decl_raw = build_var_decl_parser(pattern.clone(), type_parser.clone(), expr.clone());
    let var_decl = attr_list
        .clone()
        .then(var_decl_raw.clone())
        .map(|(attrs, mut decl)| {
            if !attrs.is_empty() {
                decl.attrs = attrs;
            }
            decl
        });

    let type_decl_raw = just(TokenKind::KeywordType)
        .ignore_then(ident.clone())
        .then(
            just(TokenKind::Assign)
                .ignore_then(type_parser.clone())
                .or_not(),
        )
        .map_with_span(|(name, _body), span: Range<usize>| Decl {
            attrs: Vec::new(),
            visibility: Visibility::Private,
            span: range_to_span(span.clone()),
            kind: DeclKind::Type {
                name,
                span: range_to_span(span),
            },
        });

    let type_decl = attr_list
        .clone()
        .then(type_decl_raw.clone())
        .map(|(attrs, mut decl)| {
            if !attrs.is_empty() {
                decl.attrs = attrs;
            }
            decl
        });

    let block_body_parser = {
        let stmt = build_stmt_parser(
            expr.clone(),
            pattern_for_block.clone(),
            type_parser.clone(),
            ident.clone().map(Expr::identifier),
        );
        just(TokenKind::LBrace)
            .ignore_then(stmt.repeated())
            .then_ignore(just(TokenKind::RBrace))
            .map_with_span(|stmts, span: Range<usize>| Expr::block(stmts, range_to_span(span)))
    };

    let fn_body = choice((
        just(TokenKind::Assign).ignore_then(expr.clone()),
        block_body_parser.clone(),
    ));

    let fn_core = fn_signature
        .clone()
        .then(fn_body)
        .map(move |(signature, body)| {
            let function_span = Span::new(signature.span.start, body.span().end);
            record_streaming_success(&streaming_state_success, function_span);
            Function {
                name: signature.name.clone(),
                generics: signature.generics.clone(),
                params: signature.params.clone(),
                body,
                ret_type: signature.ret_type.clone(),
                where_clause: signature.where_clause.clone(),
                effect: signature.effect.clone(),
                span: function_span,
                attrs: Vec::new(),
            }
        });

    let function = attr_list
        .clone()
        .then(fn_core.clone())
        .map(|(attrs, mut function)| {
            if !attrs.is_empty() {
                function.attrs = attrs;
            }
            function
        });

    let trait_item_body = choice((
        choice((
            just(TokenKind::Assign).ignore_then(expr.clone()),
            block_body_parser.clone(),
        ))
        .map(Some),
        just(TokenKind::Semicolon).to(None),
    ))
    .or_not()
    .map(|result| result.unwrap_or(None));

    let trait_item = attr_list
        .clone()
        .then(fn_signature.clone())
        .then(trait_item_body.clone())
        .map_with_span(|((attrs, signature), body), span: Range<usize>| TraitItem {
            attrs,
            signature,
            default_body: body,
            span: range_to_span(span),
        });

    let trait_decl_raw = just(TokenKind::KeywordTrait)
        .ignore_then(ident.clone())
        .then(parse_generics.clone())
        .then(where_clause.clone())
        .then(
            just(TokenKind::LBrace)
                .ignore_then(trait_item.repeated())
                .then_ignore(just(TokenKind::RBrace)),
        )
        .map_with_span(
            |(((name, generics), where_clause), items), span: Range<usize>| {
                let trait_decl = TraitDecl {
                    name,
                    generics,
                    where_clause,
                    items,
                    span: range_to_span(span.clone()),
                };
                Decl {
                    attrs: Vec::new(),
                    visibility: Visibility::Private,
                    span: range_to_span(span),
                    kind: DeclKind::Trait(trait_decl),
                }
            },
        );

    let trait_decl = attr_list
        .clone()
        .then(trait_decl_raw.clone())
        .map(|(attrs, mut decl)| {
            if !attrs.is_empty() {
                decl.attrs = attrs;
            }
            decl
        });

    let impl_item = choice((
        function.clone().map(ImplItem::Function),
        let_decl.clone().map(ImplItem::Decl),
        var_decl.clone().map(ImplItem::Decl),
    ));

    let impl_decl_raw = just(TokenKind::KeywordImpl)
        .ignore_then(parse_generics.clone())
        .then(
            type_parser.clone().then(
                just(TokenKind::KeywordFor)
                    .ignore_then(type_parser.clone())
                    .or_not(),
            ),
        )
        .then(where_clause.clone())
        .then(
            just(TokenKind::LBrace)
                .ignore_then(impl_item.repeated())
                .then_ignore(just(TokenKind::RBrace)),
        )
        .try_map(
            |(((generics, (head, target_opt)), where_clause), items), span: Range<usize>| {
                let (trait_ref, target) = match target_opt {
                    Some(target) => {
                        let trait_ref = TraitRef::from_type_annotation(&head).ok_or_else(|| {
                            Simple::custom(span_to_range(head.span), "トレイト参照が必要です")
                        })?;
                        (Some(trait_ref), target)
                    }
                    None => (None, head),
                };
                let impl_decl = ImplDecl {
                    generics,
                    trait_ref,
                    target,
                    where_clause,
                    items,
                    span: range_to_span(span.clone()),
                };
                Ok(Decl {
                    attrs: Vec::new(),
                    visibility: Visibility::Private,
                    span: range_to_span(span),
                    kind: DeclKind::Impl(impl_decl),
                })
            },
        );

    let impl_decl = attr_list
        .clone()
        .then(impl_decl_raw.clone())
        .map(|(attrs, mut decl)| {
            if !attrs.is_empty() {
                decl.attrs = attrs;
            }
            decl
        });

    #[derive(Clone)]
    enum ConductorSection {
        Dsl(ConductorDslDef),
        Channels(Vec<ConductorChannelRoute>),
        Execution(ConductorExecutionBlock),
        Monitoring(ConductorMonitoringBlock),
    }

    let conductor_endpoint = dotted_ident.clone().map(|path| ConductorEndpoint {
        span: path.span,
        path,
    });

    let conductor_arg = choice((
        ident
            .clone()
            .then_ignore(just(TokenKind::Colon))
            .then(expr.clone())
            .map_with_span(|(name, value), span: Range<usize>| ConductorArg {
                name: Some(name),
                value,
                span: range_to_span(span),
            }),
        expr.clone()
            .map_with_span(|value, span: Range<usize>| ConductorArg {
                name: None,
                value,
                span: range_to_span(span),
            }),
    ));

    let conductor_tail = just(TokenKind::PipeForward)
        .ignore_then(ident.clone())
        .then(
            conductor_arg
                .clone()
                .separated_by(just(TokenKind::Comma))
                .allow_trailing()
                .delimited_by(just(TokenKind::LParen), just(TokenKind::RParen))
                .or_not(),
        )
        .map_with_span(|(stage, args), span: Range<usize>| ConductorDslTail {
            stage,
            args: args.unwrap_or_default(),
            span: range_to_span(span),
        });

    let conductor_dsl_def = ident
        .clone()
        .then_ignore(just(TokenKind::Colon))
        .then(ident.clone())
        .then(
            just(TokenKind::Assign)
                .ignore_then(expr.clone())
                .map(|expr| {
                    let span = expr.span();
                    ConductorPipelineSpec { expr, span }
                })
                .or_not(),
        )
        .then(conductor_tail.repeated())
        .map_with_span(|(((alias, target), pipeline), tails), span: Range<usize>| {
            ConductorSection::Dsl(ConductorDslDef {
                alias,
                target,
                pipeline,
                tails,
                span: range_to_span(span),
            })
        });

    let conductor_channel_route = conductor_endpoint
        .clone()
        .then_ignore(just(TokenKind::ChannelPipe))
        .then(conductor_endpoint.clone())
        .then_ignore(just(TokenKind::Colon))
        .then(type_parser.clone())
        .map_with_span(
            |((source, target), payload), span: Range<usize>| ConductorChannelRoute {
                source,
                target,
                payload,
                span: range_to_span(span),
            },
        );

    let conductor_channels = just(TokenKind::KeywordChannels)
        .ignore_then(just(TokenKind::LBrace))
        .ignore_then(
            conductor_channel_route
                .clone()
                .then_ignore(
                    just(TokenKind::Comma)
                        .or(just(TokenKind::Semicolon))
                        .repeated(),
                )
                .repeated(),
        )
        .then_ignore(just(TokenKind::RBrace))
        .map(ConductorSection::Channels);

    let block_only_expr = expr
        .clone()
        .try_map(|body, span: Range<usize>| match body.kind {
            ExprKind::Block { .. } => Ok((body, range_to_span(span))),
            _ => Err(Simple::custom(span, "ブロック式が必要です")),
        });

    let conductor_execution = just(TokenKind::KeywordExecution)
        .ignore_then(block_only_expr.clone())
        .map(|(body, span)| ConductorSection::Execution(ConductorExecutionBlock { body, span }));

    let monitor_target = choice((
        just(TokenKind::KeywordWith)
            .ignore_then(qualified_ident.clone())
            .map(ConductorMonitorTarget::Module),
        dotted_ident.clone().map(|path| {
            ConductorMonitorTarget::Endpoint(ConductorEndpoint {
                span: path.span,
                path,
            })
        }),
    ))
    .or_not();

    let conductor_monitoring = just(TokenKind::KeywordMonitoring)
        .ignore_then(monitor_target)
        .then(block_only_expr.clone().or_not())
        .map_with_span(|(target, body), span: Range<usize>| {
            let fallback_span = range_to_span(span);
            let (body, block_span) = body
                .map(|(expr, expr_span)| (expr, expr_span))
                .unwrap_or_else(|| {
                    let empty_block = Expr::block(Vec::new(), fallback_span);
                    (empty_block, fallback_span)
                });
            ConductorSection::Monitoring(ConductorMonitoringBlock {
                target,
                body,
                span: block_span,
            })
        });

    let conductor_section = choice((
        conductor_dsl_def,
        conductor_channels,
        conductor_execution,
        conductor_monitoring,
    ));

    let conductor_decl = just(TokenKind::KeywordConductor)
        .ignore_then(ident.clone())
        .then(
            just(TokenKind::LBrace)
                .ignore_then(conductor_section.repeated())
                .then_ignore(just(TokenKind::RBrace)),
        )
        .map_with_span(|(name, sections), span: Range<usize>| {
            let mut dsl_defs = Vec::new();
            let mut channels = Vec::new();
            let mut execution = None;
            let mut monitoring = None;
            for section in sections {
                match section {
                    ConductorSection::Dsl(def) => dsl_defs.push(def),
                    ConductorSection::Channels(mut routes) => channels.append(&mut routes),
                    ConductorSection::Execution(block) => {
                        if execution.is_none() {
                            execution = Some(block);
                        }
                    }
                    ConductorSection::Monitoring(block) => {
                        if monitoring.is_none() {
                            monitoring = Some(block);
                        }
                    }
                }
            }
            Decl {
                attrs: Vec::new(),
                visibility: Visibility::Private,
                span: range_to_span(span.clone()),
                kind: DeclKind::Conductor(ConductorDecl {
                    name,
                    dsl_defs,
                    channels,
                    execution,
                    monitoring,
                    span: range_to_span(span),
                }),
            }
        });

    let effect_operation = attr_list
        .clone()
        .then_ignore(just(TokenKind::KeywordOperation))
        .then(ident.clone())
        .then(
            just(TokenKind::Colon)
                .ignore_then(type_parser.clone())
                .or_not(),
        )
        .map_with_span(
            |((attrs, name), signature), span: Range<usize>| OperationDecl {
                attrs,
                name,
                signature,
                span: range_to_span(span),
            },
        );

    let effect_body = effect_operation
        .repeated()
        .at_least(1)
        .delimited_by(just(TokenKind::LBrace), just(TokenKind::RBrace));

    let effect_decl = just(TokenKind::KeywordEffect)
        .ignore_then(ident.clone())
        .then_ignore(just(TokenKind::Colon))
        .then(ident.clone())
        .then(effect_body.clone())
        .map_with_span(|((name, tag), operations), span: Range<usize>| EffectDecl {
            span: range_to_span(span.clone()),
            name,
            tag: Some(tag),
            operations,
        });

    #[derive(Clone)]
    enum ModuleItem {
        Effect(EffectDecl),
        Function(Function),
        Decl(Decl),
    }

    let module_item = choice((
        effect_decl.clone().map(ModuleItem::Effect),
        trait_decl.clone().map(ModuleItem::Decl),
        impl_decl.clone().map(ModuleItem::Decl),
        type_decl.clone().map(ModuleItem::Decl),
        let_decl.clone().map(ModuleItem::Decl),
        var_decl.clone().map(ModuleItem::Decl),
        conductor_decl.clone().map(ModuleItem::Decl),
        function.clone().map(ModuleItem::Function),
    ));

    module_item
        .repeated()
        .then_ignore(just(TokenKind::EndOfFile))
        .map(|items| {
            let mut effects_vec = Vec::new();
            let mut functions_vec = Vec::new();
            let mut decls_vec = Vec::new();
            for item in items {
                match item {
                    ModuleItem::Effect(effect) => effects_vec.push(effect),
                    ModuleItem::Function(function) => functions_vec.push(function),
                    ModuleItem::Decl(decl) => decls_vec.push(decl),
                }
            }
            Module {
                header: None,
                uses: Vec::new(),
                effects: effects_vec,
                functions: functions_vec,
                decls: decls_vec,
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
        DeclKind::Conductor(_)
        | DeclKind::Fn { .. }
        | DeclKind::Type { .. }
        | DeclKind::Trait(_)
        | DeclKind::Impl(_)
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
            events.push(ParserTraceEvent::handler(&handle.handler));
            record_expr_trace_events(&handle.target, events);
        }
        ExprKind::Continue => {}
        ExprKind::Block { statements, .. } => {
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
        let (first_segment, consumed_idx) = parse_module_path_segment(tokens, idx)?;
        idx = consumed_idx;
        let mut segments = vec![first_segment];
        let mut span_end = segments.last().map(|ident| ident.span).unwrap();
        while let Some(token) = tokens.get(idx) {
            if token.kind != TokenKind::Dot {
                break;
            }
            if let Some((segment, consumed_idx)) = parse_module_path_segment(tokens, idx + 1) {
                span_end = span_union(span_end, segment.span);
                segments.push(segment);
                idx = consumed_idx;
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
        if let Some((ident, consumed_idx)) = parse_module_path_segment(tokens, idx + 1) {
            span_end = span_union(span_end, ident.span);
            segments.push(ident);
            idx = consumed_idx;
        } else {
            break;
        }
    }
    let span = Span::new(head_span.start, span_end.end);
    Some((ModulePath::Relative { head, segments }, span, idx))
}

fn parse_module_path_segment(tokens: &[Token], start: usize) -> Option<(Ident, usize)> {
    let token = tokens.get(start)?;
    match token.kind {
        TokenKind::Identifier | TokenKind::UpperIdentifier => parse_ident_with_index(tokens, start),
        TokenKind::KeywordThen => {
            let name = token
                .lexeme
                .clone()
                .or_else(|| token.kind.keyword_literal().map(|text| text.to_string()))
                .unwrap_or_else(String::new);
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

fn merge_dotted_ident(first: Ident, rest: Vec<Ident>) -> Ident {
    rest.into_iter().fold(first, |mut acc, segment| {
        acc.name.push('.');
        acc.name.push_str(&segment.name);
        acc.span = span_union(acc.span, segment.span);
        acc
    })
}

fn range_to_span(span: Range<usize>) -> Span {
    Span::new(span.start as u32, span.end as u32)
}

fn span_to_range(span: Span) -> Range<usize> {
    (span.start as usize)..(span.end as usize)
}

fn build_let_decl_parser<P, Q, R>(
    pattern_var: Q,
    type_parser: R,
    expr: P,
) -> impl ChumskyParser<TokenKind, Decl, Error = Simple<TokenKind>> + Clone
where
    P: ChumskyParser<TokenKind, Expr, Error = Simple<TokenKind>> + Clone,
    Q: ChumskyParser<TokenKind, Pattern, Error = Simple<TokenKind>> + Clone,
    R: ChumskyParser<TokenKind, TypeAnnot, Error = Simple<TokenKind>> + Clone,
{
    just(TokenKind::KeywordLet)
        .ignore_then(pattern_var)
        .then(just(TokenKind::Colon).ignore_then(type_parser).or_not())
        .then_ignore(just(TokenKind::Assign))
        .then(expr)
        .map_with_span(|((pattern, ty), value), span: Range<usize>| Decl {
            attrs: Vec::new(),
            visibility: Visibility::Private,
            kind: DeclKind::Let {
                pattern,
                value,
                type_annotation: ty,
            },
            span: range_to_span(span),
        })
}

fn build_var_decl_parser<P, Q, R>(
    pattern_var: Q,
    type_parser: R,
    expr: P,
) -> impl ChumskyParser<TokenKind, Decl, Error = Simple<TokenKind>> + Clone
where
    P: ChumskyParser<TokenKind, Expr, Error = Simple<TokenKind>> + Clone,
    Q: ChumskyParser<TokenKind, Pattern, Error = Simple<TokenKind>> + Clone,
    R: ChumskyParser<TokenKind, TypeAnnot, Error = Simple<TokenKind>> + Clone,
{
    just(TokenKind::KeywordVar)
        .ignore_then(pattern_var)
        .then(just(TokenKind::Colon).ignore_then(type_parser).or_not())
        .then_ignore(just(TokenKind::Assign))
        .then(expr)
        .map_with_span(|((pattern, ty), value), span: Range<usize>| Decl {
            attrs: Vec::new(),
            visibility: Visibility::Private,
            kind: DeclKind::Var {
                pattern,
                value,
                type_annotation: ty,
            },
            span: range_to_span(span),
        })
}

fn build_stmt_parser<P, Q, R, S>(
    expr: P,
    pattern_var: Q,
    type_parser: R,
    ident_expr: S,
) -> impl ChumskyParser<TokenKind, Stmt, Error = Simple<TokenKind>> + Clone
where
    P: ChumskyParser<TokenKind, Expr, Error = Simple<TokenKind>> + Clone,
    Q: ChumskyParser<TokenKind, Pattern, Error = Simple<TokenKind>> + Clone,
    R: ChumskyParser<TokenKind, TypeAnnot, Error = Simple<TokenKind>> + Clone,
    S: ChumskyParser<TokenKind, Expr, Error = Simple<TokenKind>> + Clone,
{
    let let_stmt_parser =
        build_let_decl_parser(pattern_var.clone(), type_parser.clone(), expr.clone());

    let var_stmt_parser =
        build_var_decl_parser(pattern_var.clone(), type_parser.clone(), expr.clone());

    let decl_stmt = choice((
        let_stmt_parser.map(|decl| {
            let span = decl.span;
            Stmt {
                kind: StmtKind::Decl { decl },
                span,
            }
        }),
        var_stmt_parser.map(|decl| {
            let span = decl.span;
            Stmt {
                kind: StmtKind::Decl { decl },
                span,
            }
        }),
    ));

    let assign_stmt = ident_expr
        .clone()
        .then_ignore(just(TokenKind::ColonAssign))
        .then(expr.clone())
        .map_with_span(|(target, value), span: Range<usize>| Stmt {
            kind: StmtKind::Assign {
                target: Box::new(target),
                value: Box::new(value),
            },
            span: range_to_span(span),
        });

    let expr_stmt = expr.map_with_span(|expression, span: Range<usize>| Stmt {
        kind: StmtKind::Expr {
            expr: Box::new(expression),
        },
        span: range_to_span(span),
    });

    choice((decl_stmt, assign_stmt, expr_stmt))
}

fn build_attribute_parser<P, Q>(
    expr: P,
    ident: Q,
) -> impl ChumskyParser<TokenKind, Attribute, Error = Simple<TokenKind>> + Clone
where
    P: ChumskyParser<TokenKind, Expr, Error = Simple<TokenKind>> + Clone,
    Q: ChumskyParser<TokenKind, Ident, Error = Simple<TokenKind>> + Clone,
{
    let args = expr
        .clone()
        .separated_by(just(TokenKind::Comma))
        .allow_trailing()
        .delimited_by(just(TokenKind::LParen), just(TokenKind::RParen))
        .map_with_span(|values, span: Range<usize>| (values, Some(range_to_span(span))))
        .or_not();

    just(TokenKind::At)
        .map_with_span(|_, span: Range<usize>| range_to_span(span))
        .then(ident)
        .then(args)
        .map(|((at_span, name), args)| {
            let (args, args_span) = args.unwrap_or_else(|| (Vec::new(), None));
            let span_start = at_span.start.min(name.span.start);
            let span_end = args_span
                .as_ref()
                .map(|span| span.end)
                .unwrap_or(name.span.end);
            Attribute {
                name,
                args,
                span: Span::new(span_start, span_end),
            }
        })
}

fn build_effect_argument_expr(args: Vec<Expr>, span: Span) -> Expr {
    match args.len() {
        0 => Expr::literal(
            Literal {
                value: LiteralKind::Unit,
            },
            span,
        ),
        1 => args.into_iter().next().unwrap(),
        _ => Expr::literal(
            Literal {
                value: LiteralKind::Tuple { elements: args },
            },
            span,
        ),
    }
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
            diagnostics
                .push(build_diagnostic_from_error(span, error))
                .expect("parser diagnostics must include required fields");
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
                    let index = diagnostics
                        .push_with_index(pending.into_diagnostic())
                        .expect("streaming diagnostics must include required fields");
                    limiter.record_emission(index);
                } else if let Some(index) = limiter.last_emitted_index() {
                    diagnostics.merge_expected_summary_at(index, &summary);
                }
            } else {
                diagnostics
                    .push(pending.into_diagnostic())
                    .expect("streaming diagnostics must include required fields");
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
fn emit(msg: String) = perform ConsoleLog(msg)
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
