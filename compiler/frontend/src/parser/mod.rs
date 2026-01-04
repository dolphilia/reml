//! パーサドライバ相当の責務を担う Rust フロントエンド実装。

use chumsky::error::{Simple, SimpleReason};
use chumsky::prelude::*;
use chumsky::recursive::Recursive;
use chumsky::stream::Stream;
use chumsky::Parser as ChumskyParser;
use reml_runtime::text::{LocaleId, UnicodeError};
use serde::Serialize;
use serde_json::Value;
use smallvec::SmallVec;
use smol_str::SmolStr;
use std::collections::HashSet;
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
    ActivePatternDecl, ActorSpecDecl, Attribute, BinaryOp, ConductorArg, ConductorChannelRoute,
    ConductorDecl, ConductorDslDef, ConductorDslTail, ConductorEndpoint, ConductorExecutionBlock,
    ConductorMonitorTarget, ConductorMonitoringBlock, ConductorPipelineSpec, Decl, DeclKind,
    EffectAnnotation, EffectCall, EffectDecl, EnumDecl, EnumVariant, Expr, ExprKind, ExternItem,
    FixityKind, Function, FunctionSignature, HandleExpr, HandlerDecl, HandlerEntry, Ident,
    ImplDecl, ImplItem, InlineAsmExpr, InlineAsmInput, InlineAsmOutput, IntBase, Literal,
    LiteralKind, LlvmIrExpr, MacroDecl, MatchArm, Module, ModuleBody, ModuleDecl, ModuleHeader,
    ModulePath, OperationDecl, Param, Pattern, PatternKind, PatternRecordField, QualifiedName,
    RecordField, RelativeHead, SlicePatternItem, Stmt, StmtKind, StringKind, StructDecl, TraitDecl,
    TraitItem, TraitRef, TypeAnnot, TypeArrayLength, TypeDecl, TypeDeclBody, TypeDeclVariant,
    TypeDeclVariantPayload, TypeKind, TypeLiteral, TypeRecordField, TypeTupleElement,
    TypeUnionVariant, UnaryOp, UseDecl, UseItem, UseTree, VariantPayload, Visibility,
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

trait CutParserExt<I, O>: chumsky::Parser<I, O, Error = Simple<I>> + Sized
where
    I: Clone + std::hash::Hash + Eq,
{
    fn cut(self) -> Self {
        self
    }
}

impl<I, O, P> CutParserExt<I, O> for P
where
    I: Clone + std::hash::Hash + Eq,
    P: chumsky::Parser<I, O, Error = Simple<I>> + Sized,
{
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
    pub allow_top_level_expr: bool,
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
            allow_top_level_expr: false,
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
            allow_top_level_expr: run_config.allow_top_level_expr,
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
            collect_intrinsic_attribute_diagnostics(module, &mut diagnostics);
            if !options.allow_top_level_expr {
                collect_top_level_expr_diagnostics(module, &mut diagnostics);
            }
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
    let prefix = parse_top_level_prefix(tokens);
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

fn parse_string_literal_value(source: &str, span: Range<usize>) -> String {
    let slice = &source[span.start..span.end];
    let value = if slice.starts_with("r\"") && slice.ends_with('"') && slice.len() >= 3 {
        &slice[2..slice.len() - 1]
    } else if slice.starts_with("\\\"") && slice.ends_with("\\\"") && slice.len() >= 4 {
        &slice[2..slice.len() - 2]
    } else if slice.starts_with('"') && slice.ends_with('"') && slice.len() >= 2 {
        &slice[1..slice.len() - 1]
    } else {
        slice
    };
    value.replace("\\\"", "\"")
}

fn parse_result_from_module(
    parsed: ParsedModule,
    run_config: RunConfig,
    legacy_error: Option<ParseError>,
) -> ParseResult<Module> {
    let ParsedModule {
        tokens,
        mut diagnostics,
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

    if let Some(module) = ast.as_ref() {
        collect_cfg_diagnostics(module, &run_config, &mut diagnostics);
        collect_use_diagnostics(module, &mut diagnostics);
        collect_match_guard_diagnostics(module, &mut diagnostics);
        collect_rec_lambda_diagnostics(module, &mut diagnostics);
    }

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
    #[allow(dead_code)]
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
    let label = err.label().and_then(|text| {
        let trimmed = text.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    });
    let expectations: Vec<Option<TokenKind>> = err.expected().cloned().collect();
    let is_expr_context = is_expression_recover_context(&expectations);

    if let Some(label) = &label {
        collector.push(ExpectedToken::rule(label.clone()));
    }
    if is_expr_context {
        collector.extend(expression_expected_tokens());
    } else {
        for expectation in &expectations {
            match expectation {
                Some(kind) => collector.extend(token_kind_expectations(kind)),
                None => collector.push(ExpectedToken::eof()),
            }
        }
    }
    let mut summary = collector.summarize();
    if summary.context_note.is_none() {
        if expectations
            .iter()
            .any(|expectation| matches!(expectation, Some(TokenKind::RParen)))
        {
            summary.context_note = Some("`(` に対応する `)` が必要です".to_string());
        } else if is_expr_context {
            summary.context_note = Some(
                label
                    .as_ref()
                    .map(|name| format!("演算子の後に {name} が必要です"))
                    .unwrap_or_else(|| "演算子の後に式が必要です".to_string()),
            );
        } else if let Some(label) = label {
            summary.context_note = Some(format!("{label} が必要です"));
        }
    }
    summary
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
    let already_reported = diagnostics.iter().any(|diag| {
        diag.code
            .as_deref()
            .map(|code| code == "effects.handler.missing_with")
            .unwrap_or(false)
            || diag
                .codes
                .iter()
                .any(|code| code == "effects.handler.missing_with")
    });
    if already_reported {
        return;
    }

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
            ExprKind::Literal(_)
            | ExprKind::FixityLiteral(_)
            | ExprKind::Identifier(_)
            | ExprKind::ModulePath(_) => {}
            ExprKind::Call { callee, args } => {
                record(callee, diagnostics);
                for arg in args {
                    record(arg, diagnostics);
                }
            }
            ExprKind::PerformCall { call } => record(&call.argument, diagnostics),
            ExprKind::InlineAsm(asm) => {
                for output in &asm.outputs {
                    record(&output.target, diagnostics);
                }
                for input in &asm.inputs {
                    record(&input.expr, diagnostics);
                }
            }
            ExprKind::LlvmIr(ir) => {
                for input in &ir.inputs {
                    record(input, diagnostics);
                }
            }
            ExprKind::Lambda { body, .. }
            | ExprKind::Loop { body }
            | ExprKind::Unsafe { body }
            | ExprKind::Defer { body }
            | ExprKind::EffectBlock { body }
            | ExprKind::Async { body, .. } => record(body, diagnostics),
            ExprKind::Pipe { left, right } | ExprKind::Binary { left, right, .. } => {
                record(left, diagnostics);
                record(right, diagnostics);
            }
            ExprKind::Unary { expr: inner, .. }
            | ExprKind::Rec { expr: inner }
            | ExprKind::Propagate { expr: inner }
            | ExprKind::Return { value: Some(inner) } => record(inner, diagnostics),
            ExprKind::Await { expr: inner } => record(inner, diagnostics),
            ExprKind::Break { value } => {
                if let Some(inner) = value {
                    record(inner, diagnostics);
                }
            }
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
                            DeclKind::Let { value, .. }
                            | DeclKind::Var { value, .. }
                            | DeclKind::Const { value, .. } => record(value, diagnostics),
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

    fn record_module_body(body: &ModuleBody, diagnostics: &mut Vec<FrontendDiagnostic>) {
        for function in &body.functions {
            record(&function.body, diagnostics);
        }
        for active_pattern in &body.active_patterns {
            for param in &active_pattern.params {
                if let Some(default) = &param.default {
                    record(default, diagnostics);
                }
            }
            record(&active_pattern.body, diagnostics);
        }
        for decl in &body.decls {
            match &decl.kind {
                DeclKind::Let { value, .. }
                | DeclKind::Var { value, .. }
                | DeclKind::Const { value, .. } => record(value, diagnostics),
                DeclKind::Module(module_decl) => record_module_body(&module_decl.body, diagnostics),
                DeclKind::Macro(macro_decl) => {
                    for param in &macro_decl.params {
                        if let Some(default) = &param.default {
                            record(default, diagnostics);
                        }
                    }
                    record(&macro_decl.body, diagnostics);
                }
                DeclKind::ActorSpec(actor_spec) => {
                    for param in &actor_spec.params {
                        if let Some(default) = &param.default {
                            record(default, diagnostics);
                        }
                    }
                    record(&actor_spec.body, diagnostics);
                }
                _ => {}
            }
        }
        for expr in &body.exprs {
            record(expr, diagnostics);
        }
    }

    for function in &module.functions {
        record(&function.body, diagnostics);
    }
    for active_pattern in &module.active_patterns {
        for param in &active_pattern.params {
            if let Some(default) = &param.default {
                record(default, diagnostics);
            }
        }
        record(&active_pattern.body, diagnostics);
    }
    for decl in &module.decls {
        match &decl.kind {
            DeclKind::Let { value, .. }
            | DeclKind::Var { value, .. }
            | DeclKind::Const { value, .. } => record(value, diagnostics),
            DeclKind::Module(module_decl) => {
                record_module_body(&module_decl.body, diagnostics);
            }
            DeclKind::Macro(macro_decl) => {
                for param in &macro_decl.params {
                    if let Some(default) = &param.default {
                        record(default, diagnostics);
                    }
                }
                record(&macro_decl.body, diagnostics);
            }
            DeclKind::ActorSpec(actor_spec) => {
                for param in &actor_spec.params {
                    if let Some(default) = &param.default {
                        record(default, diagnostics);
                    }
                }
                record(&actor_spec.body, diagnostics);
            }
            _ => {}
        }
    }
    for expr in &module.exprs {
        record(expr, diagnostics);
    }
}

fn collect_intrinsic_attribute_diagnostics(
    module: &Module,
    diagnostics: &mut Vec<FrontendDiagnostic>,
) {
    fn report_intrinsic_error(
        attr: &Attribute,
        message: impl Into<String>,
        diagnostics: &mut Vec<FrontendDiagnostic>,
    ) {
        let diagnostic = FrontendDiagnostic::new(message)
            .with_severity(DiagnosticSeverity::Error)
            .with_domain(DiagnosticDomain::Parser)
            .with_code("native.intrinsic.invalid_syntax")
            .with_recoverability(Recoverability::Recoverable)
            .with_span(attr.span);
        diagnostics.push(diagnostic);
    }

    fn intrinsic_literal_arg(attr: &Attribute) -> Option<&str> {
        if attr.args.len() != 1 {
            return None;
        }
        match &attr.args[0].kind {
            ExprKind::Literal(Literal {
                value: LiteralKind::String { value, .. },
            }) => Some(value.as_str()),
            _ => None,
        }
    }

    fn validate_intrinsic_attr(
        attr: &Attribute,
        allow_on_target: bool,
        target_label: &str,
        diagnostics: &mut Vec<FrontendDiagnostic>,
    ) {
        if attr.name.name != "intrinsic" {
            return;
        }
        if !allow_on_target {
            report_intrinsic_error(
                attr,
                format!("`@intrinsic` は関数宣言にのみ付与できます（対象: {target_label}）。"),
                diagnostics,
            );
            return;
        }
        if intrinsic_literal_arg(attr).is_none() {
            report_intrinsic_error(
                attr,
                "`@intrinsic(\"llvm.sqrt.f64\")` の形式で intrinsic 名を指定してください。",
                diagnostics,
            );
        }
    }

    fn validate_attrs(
        attrs: &[Attribute],
        allow_on_target: bool,
        target_label: &str,
        diagnostics: &mut Vec<FrontendDiagnostic>,
    ) {
        for attr in attrs {
            validate_intrinsic_attr(attr, allow_on_target, target_label, diagnostics);
        }
    }

    fn inspect_expr(expr: &Expr, diagnostics: &mut Vec<FrontendDiagnostic>) {
        match &expr.kind {
            ExprKind::Block {
                attrs, statements, ..
            } => {
                validate_attrs(attrs, false, "ブロック", diagnostics);
                for stmt in statements {
                    inspect_stmt(stmt, diagnostics);
                }
            }
            ExprKind::Call { callee, args } => {
                inspect_expr(callee, diagnostics);
                for arg in args {
                    inspect_expr(arg, diagnostics);
                }
            }
            ExprKind::PerformCall { call } => inspect_expr(&call.argument, diagnostics),
            ExprKind::InlineAsm(asm) => {
                for output in &asm.outputs {
                    inspect_expr(&output.target, diagnostics);
                }
                for input in &asm.inputs {
                    inspect_expr(&input.expr, diagnostics);
                }
            }
            ExprKind::LlvmIr(ir) => {
                for input in &ir.inputs {
                    inspect_expr(input, diagnostics);
                }
            }
            ExprKind::Lambda { body, .. }
            | ExprKind::Loop { body }
            | ExprKind::Unsafe { body }
            | ExprKind::Defer { body }
            | ExprKind::EffectBlock { body }
            | ExprKind::Async { body, .. } => inspect_expr(body, diagnostics),
            ExprKind::Pipe { left, right } | ExprKind::Binary { left, right, .. } => {
                inspect_expr(left, diagnostics);
                inspect_expr(right, diagnostics);
            }
            ExprKind::Unary { expr: inner, .. }
            | ExprKind::Rec { expr: inner }
            | ExprKind::Propagate { expr: inner }
            | ExprKind::Return { value: Some(inner) } => inspect_expr(inner, diagnostics),
            ExprKind::Await { expr: inner } => inspect_expr(inner, diagnostics),
            ExprKind::Break { value } => {
                if let Some(inner) = value {
                    inspect_expr(inner, diagnostics);
                }
            }
            ExprKind::Return { value: None } | ExprKind::Continue => {}
            ExprKind::FieldAccess { target, .. }
            | ExprKind::TupleAccess { target, .. }
            | ExprKind::Index { target, .. } => inspect_expr(target, diagnostics),
            ExprKind::IfElse {
                condition,
                then_branch,
                else_branch,
            } => {
                inspect_expr(condition, diagnostics);
                inspect_expr(then_branch, diagnostics);
                if let Some(branch) = else_branch {
                    inspect_expr(branch, diagnostics);
                }
            }
            ExprKind::Match { target, arms } => {
                inspect_expr(target, diagnostics);
                for arm in arms {
                    if let Some(guard) = &arm.guard {
                        inspect_expr(guard, diagnostics);
                    }
                    inspect_expr(&arm.body, diagnostics);
                }
            }
            ExprKind::While { condition, body } => {
                inspect_expr(condition, diagnostics);
                inspect_expr(body, diagnostics);
            }
            ExprKind::For { start, end, .. } => {
                inspect_expr(start, diagnostics);
                inspect_expr(end, diagnostics);
            }
            ExprKind::Assign { target, value } => {
                inspect_expr(target, diagnostics);
                inspect_expr(value, diagnostics);
            }
            ExprKind::Literal(_)
            | ExprKind::FixityLiteral(_)
            | ExprKind::Identifier(_)
            | ExprKind::ModulePath(_)
            | ExprKind::Handle { .. } => {}
        }
    }

    fn inspect_stmt(stmt: &Stmt, diagnostics: &mut Vec<FrontendDiagnostic>) {
        match &stmt.kind {
            StmtKind::Decl { decl } => inspect_decl(decl, diagnostics),
            StmtKind::Expr { expr } | StmtKind::Defer { expr } => inspect_expr(expr, diagnostics),
            StmtKind::Assign { target, value } => {
                inspect_expr(target, diagnostics);
                inspect_expr(value, diagnostics);
            }
        }
    }

    fn inspect_decl(decl: &Decl, diagnostics: &mut Vec<FrontendDiagnostic>) {
        validate_attrs(&decl.attrs, false, "宣言", diagnostics);
        match &decl.kind {
            DeclKind::Extern { functions, .. } => {
                for func in functions {
                    validate_attrs(&func.attrs, false, "extern 関数", diagnostics);
                }
            }
            DeclKind::Handler(handler) => {
                for entry in &handler.entries {
                    if let HandlerEntry::Operation { attrs, body, .. } = entry {
                        validate_attrs(attrs, false, "handler operation", diagnostics);
                        inspect_expr(body, diagnostics);
                    }
                }
            }
            DeclKind::Module(module_decl) => {
                inspect_module_body(&module_decl.body, diagnostics);
            }
            DeclKind::Macro(macro_decl) => {
                for param in &macro_decl.params {
                    if let Some(default) = &param.default {
                        inspect_expr(default, diagnostics);
                    }
                }
                inspect_expr(&macro_decl.body, diagnostics);
            }
            DeclKind::ActorSpec(actor_spec) => {
                for param in &actor_spec.params {
                    if let Some(default) = &param.default {
                        inspect_expr(default, diagnostics);
                    }
                }
                inspect_expr(&actor_spec.body, diagnostics);
            }
            DeclKind::Conductor(conductor) => {
                if let Some(exec) = &conductor.execution {
                    inspect_expr(&exec.body, diagnostics);
                }
                if let Some(monitor) = &conductor.monitoring {
                    inspect_expr(&monitor.body, diagnostics);
                }
            }
            DeclKind::Let { value, .. }
            | DeclKind::Var { value, .. }
            | DeclKind::Const { value, .. } => {
                inspect_expr(value, diagnostics);
            }
            DeclKind::Effect(effect) => {
                for op in &effect.operations {
                    validate_attrs(&op.attrs, false, "effect operation", diagnostics);
                }
            }
            _ => {}
        }
    }

    fn inspect_module_body(body: &ModuleBody, diagnostics: &mut Vec<FrontendDiagnostic>) {
        for function in &body.functions {
            validate_attrs(&function.attrs, true, "関数", diagnostics);
            inspect_expr(&function.body, diagnostics);
        }
        for active in &body.active_patterns {
            validate_attrs(&active.attrs, false, "Active Pattern", diagnostics);
            inspect_expr(&active.body, diagnostics);
        }
        for decl in &body.decls {
            inspect_decl(decl, diagnostics);
        }
        for expr in &body.exprs {
            inspect_expr(expr, diagnostics);
        }
    }

    if let Some(header) = &module.header {
        validate_attrs(&header.attrs, false, "モジュールヘッダ", diagnostics);
    }
    for function in &module.functions {
        validate_attrs(&function.attrs, true, "関数", diagnostics);
        inspect_expr(&function.body, diagnostics);
    }
    for active in &module.active_patterns {
        validate_attrs(&active.attrs, false, "Active Pattern", diagnostics);
        inspect_expr(&active.body, diagnostics);
    }
    for decl in &module.decls {
        inspect_decl(decl, diagnostics);
    }
    for expr in &module.exprs {
        inspect_expr(expr, diagnostics);
    }
}

fn collect_top_level_expr_diagnostics(module: &Module, diagnostics: &mut Vec<FrontendDiagnostic>) {
    if module.exprs.is_empty() {
        return;
    }
    let span = module.exprs.first().map(|expr| expr.span);
    let mut diagnostic = FrontendDiagnostic::new("トップレベル式は許可されていません。")
        .with_severity(DiagnosticSeverity::Error)
        .with_domain(DiagnosticDomain::Parser)
        .with_code("parser.top_level_expr.disallowed")
        .with_recoverability(Recoverability::Recoverable);
    if let Some(span) = span {
        diagnostic = diagnostic.with_span(span);
    }
    diagnostic.add_note(DiagnosticNote::new(
        "parser.top_level_expr.hint",
        "`fn` で包むか、`RunConfig.allow_top_level_expr = true` / `--allow-top-level-expr` を利用してください。",
    ));
    diagnostics.push(diagnostic);
}

fn collect_cfg_diagnostics(
    module: &Module,
    run_config: &RunConfig,
    diagnostics: &mut Vec<FrontendDiagnostic>,
) {
    let registry = CfgTargetRegistry::from_run_config(run_config);
    if registry.is_empty() {
        return;
    }
    if let Some(header) = &module.header {
        evaluate_cfg_attributes(&header.attrs, diagnostics, &registry);
    }
    for active_pattern in &module.active_patterns {
        evaluate_cfg_attributes(&active_pattern.attrs, diagnostics, &registry);
    }
    for decl in &module.decls {
        evaluate_cfg_attributes(&decl.attrs, diagnostics, &registry);
    }
    for effect in &module.effects {
        for op in &effect.operations {
            evaluate_cfg_attributes(&op.attrs, diagnostics, &registry);
        }
    }
    for function in &module.functions {
        evaluate_cfg_attributes(&function.attrs, diagnostics, &registry);
        inspect_cfg_expr(&function.body, diagnostics, &registry);
    }
    for active_pattern in &module.active_patterns {
        inspect_cfg_expr(&active_pattern.body, diagnostics, &registry);
    }
    for expr in &module.exprs {
        inspect_cfg_expr(expr, diagnostics, &registry);
    }
}

#[derive(Copy, Clone, Eq, PartialEq)]
enum ModuleScope {
    Root,
    Nested,
}

fn module_scope_kind(header: Option<&ModuleHeader>) -> ModuleScope {
    match header.map(|header| &header.path) {
        Some(ModulePath::Relative { head, .. }) => match head {
            RelativeHead::Self_ | RelativeHead::Super(_) => ModuleScope::Nested,
            RelativeHead::PlainIdent(_) => ModuleScope::Root,
        },
        _ => ModuleScope::Root,
    }
}

fn collect_use_diagnostics(module: &Module, diagnostics: &mut Vec<FrontendDiagnostic>) {
    if module_scope_kind(module.header.as_ref()) == ModuleScope::Nested {
        return;
    }
    for decl in &module.uses {
        if use_tree_has_super(&decl.tree) {
            diagnostics.push(build_invalid_super_diagnostic(decl));
        }
    }
}

fn use_tree_has_super(tree: &UseTree) -> bool {
    match tree {
        UseTree::Path { path, .. } | UseTree::Brace { path, .. } => {
            matches!(
                path,
                ModulePath::Relative {
                    head: RelativeHead::Super(_),
                    ..
                }
            )
        }
    }
}

fn build_invalid_super_diagnostic(decl: &UseDecl) -> FrontendDiagnostic {
    let mut diagnostic = FrontendDiagnostic::new("ルートモジュールでは `super` を利用できません。")
        .with_code("language.use.invalid_super")
        .with_severity(DiagnosticSeverity::Error)
        .with_domain(DiagnosticDomain::Parser)
        .with_span(decl.span)
        .with_recoverability(Recoverability::Fatal);
    diagnostic.add_note(DiagnosticNote::new(
        "language.use.invalid_super.note",
        "`use ::Core.Prelude` など明示的なルート参照に書き換えてください。",
    ));
    diagnostic
}

fn collect_match_guard_diagnostics(module: &Module, diagnostics: &mut Vec<FrontendDiagnostic>) {
    fn walk_expr(expr: &Expr, diagnostics: &mut Vec<FrontendDiagnostic>) {
        match &expr.kind {
            ExprKind::Match { target, arms } => {
                walk_expr(target, diagnostics);
                for arm in arms {
                    if arm.guard_used_if {
                        diagnostics.push(build_if_guard_deprecated_diagnostic(arm.span));
                    }
                    if let Some(guard_expr) = &arm.guard {
                        walk_expr(guard_expr, diagnostics);
                    }
                    walk_expr(&arm.body, diagnostics);
                }
            }
            ExprKind::Call { callee, args } => {
                walk_expr(callee, diagnostics);
                for arg in args {
                    walk_expr(arg, diagnostics);
                }
            }
            ExprKind::PerformCall { call } => walk_expr(&call.argument, diagnostics),
            ExprKind::InlineAsm(asm) => {
                for output in &asm.outputs {
                    walk_expr(&output.target, diagnostics);
                }
                for input in &asm.inputs {
                    walk_expr(&input.expr, diagnostics);
                }
            }
            ExprKind::LlvmIr(ir) => {
                for input in &ir.inputs {
                    walk_expr(input, diagnostics);
                }
            }
            ExprKind::Lambda { body, .. }
            | ExprKind::Loop { body }
            | ExprKind::Unsafe { body }
            | ExprKind::Defer { body }
            | ExprKind::EffectBlock { body }
            | ExprKind::Async { body, .. } => walk_expr(body, diagnostics),
            ExprKind::Pipe { left, right } | ExprKind::Binary { left, right, .. } => {
                walk_expr(left, diagnostics);
                walk_expr(right, diagnostics);
            }
            ExprKind::Unary { expr: inner, .. }
            | ExprKind::Rec { expr: inner }
            | ExprKind::Propagate { expr: inner }
            | ExprKind::Return { value: Some(inner) }
            | ExprKind::Await { expr: inner } => walk_expr(inner, diagnostics),
            ExprKind::Break { value: Some(inner) } => walk_expr(inner, diagnostics),
            ExprKind::FieldAccess { target, .. }
            | ExprKind::TupleAccess { target, .. }
            | ExprKind::Index { target, .. } => walk_expr(target, diagnostics),
            ExprKind::IfElse {
                condition,
                then_branch,
                else_branch,
            } => {
                walk_expr(condition, diagnostics);
                walk_expr(then_branch, diagnostics);
                if let Some(branch) = else_branch {
                    walk_expr(branch, diagnostics);
                }
            }
            ExprKind::While { condition, body } => {
                walk_expr(condition, diagnostics);
                walk_expr(body, diagnostics);
            }
            ExprKind::For { start, end, .. } => {
                walk_expr(start, diagnostics);
                walk_expr(end, diagnostics);
            }
            ExprKind::Block { statements, .. } => {
                for stmt in statements {
                    match &stmt.kind {
                        StmtKind::Decl { decl } => {
                            if let DeclKind::Let { value, .. }
                            | DeclKind::Var { value, .. }
                            | DeclKind::Const { value, .. } = &decl.kind
                            {
                                walk_expr(value, diagnostics);
                            }
                        }
                        StmtKind::Expr { expr } | StmtKind::Defer { expr } => {
                            walk_expr(expr, diagnostics)
                        }
                        StmtKind::Assign { target, value } => {
                            walk_expr(target, diagnostics);
                            walk_expr(value, diagnostics);
                        }
                    }
                }
            }
            ExprKind::Handle { handle } => {
                walk_expr(&handle.target, diagnostics);
                for entry in &handle.handler.entries {
                    if let HandlerEntry::Operation { body, .. } = entry {
                        walk_expr(body, diagnostics);
                    }
                }
            }
            ExprKind::Return { value: None }
            | ExprKind::Break { value: None }
            | ExprKind::Continue
            | ExprKind::Literal(_)
            | ExprKind::FixityLiteral(_)
            | ExprKind::Identifier(_)
            | ExprKind::ModulePath(_)
            | ExprKind::Assign { .. } => {}
        }
    }

    for function in &module.functions {
        walk_expr(&function.body, diagnostics);
    }
    for active_pattern in &module.active_patterns {
        walk_expr(&active_pattern.body, diagnostics);
    }
    for decl in &module.decls {
        if let DeclKind::Let { value, .. }
        | DeclKind::Var { value, .. }
        | DeclKind::Const { value, .. } = &decl.kind
        {
            walk_expr(value, diagnostics);
        }
    }
    for expr in &module.exprs {
        walk_expr(expr, diagnostics);
    }
}

fn collect_rec_lambda_diagnostics(module: &Module, diagnostics: &mut Vec<FrontendDiagnostic>) {
    fn rec_invalid_form(span: Span) -> FrontendDiagnostic {
        FrontendDiagnostic::new("`rec` の後ろは識別子のみ許可されます。")
            .with_severity(DiagnosticSeverity::Error)
            .with_domain(DiagnosticDomain::Parser)
            .with_code("parser.rec.invalid_form")
            .with_recoverability(Recoverability::Recoverable)
            .with_span(span)
    }

    fn rec_unsupported_position(span: Span) -> FrontendDiagnostic {
        FrontendDiagnostic::new("`rec` を代入対象として使用できません。")
            .with_severity(DiagnosticSeverity::Error)
            .with_domain(DiagnosticDomain::Parser)
            .with_code("parser.rec.unsupported_position")
            .with_recoverability(Recoverability::Recoverable)
            .with_span(span)
    }

    fn walk_expr(expr: &Expr, diagnostics: &mut Vec<FrontendDiagnostic>) {
        match &expr.kind {
            ExprKind::Lambda {
                params: _, body, ..
            } => {
                walk_expr(body, diagnostics);
            }
            ExprKind::Rec { expr: inner } => {
                if !matches!(inner.kind, ExprKind::Identifier(_)) {
                    diagnostics.push(rec_invalid_form(inner.span));
                }
                walk_expr(inner, diagnostics);
            }
            ExprKind::Assign { target, value } => {
                if matches!(target.kind, ExprKind::Rec { .. }) {
                    diagnostics.push(rec_unsupported_position(target.span));
                }
                walk_expr(target, diagnostics);
                walk_expr(value, diagnostics);
            }
            ExprKind::Call { callee, args } => {
                walk_expr(callee, diagnostics);
                for arg in args {
                    walk_expr(arg, diagnostics);
                }
            }
            ExprKind::PerformCall { call } => walk_expr(&call.argument, diagnostics),
            ExprKind::InlineAsm(asm) => {
                for output in &asm.outputs {
                    walk_expr(&output.target, diagnostics);
                }
                for input in &asm.inputs {
                    walk_expr(&input.expr, diagnostics);
                }
            }
            ExprKind::LlvmIr(ir) => {
                for input in &ir.inputs {
                    walk_expr(input, diagnostics);
                }
            }
            ExprKind::Pipe { left, right } | ExprKind::Binary { left, right, .. } => {
                walk_expr(left, diagnostics);
                walk_expr(right, diagnostics);
            }
            ExprKind::Unary { expr: inner, .. }
            | ExprKind::Propagate { expr: inner }
            | ExprKind::Return { value: Some(inner) }
            | ExprKind::Await { expr: inner } => walk_expr(inner, diagnostics),
            ExprKind::Break { value: Some(inner) } => walk_expr(inner, diagnostics),
            ExprKind::FieldAccess { target, .. }
            | ExprKind::TupleAccess { target, .. }
            | ExprKind::Index { target, .. } => walk_expr(target, diagnostics),
            ExprKind::IfElse {
                condition,
                then_branch,
                else_branch,
            } => {
                walk_expr(condition, diagnostics);
                walk_expr(then_branch, diagnostics);
                if let Some(branch) = else_branch.as_deref() {
                    walk_expr(branch, diagnostics);
                }
            }
            ExprKind::Match { target, arms } => {
                walk_expr(target, diagnostics);
                for arm in arms {
                    if let Some(guard) = &arm.guard {
                        walk_expr(guard, diagnostics);
                    }
                    walk_expr(&arm.body, diagnostics);
                }
            }
            ExprKind::While { condition, body } => {
                walk_expr(condition, diagnostics);
                walk_expr(body, diagnostics);
            }
            ExprKind::For { start, end, .. } => {
                walk_expr(start, diagnostics);
                walk_expr(end, diagnostics);
            }
            ExprKind::Loop { body }
            | ExprKind::Unsafe { body }
            | ExprKind::Defer { body }
            | ExprKind::EffectBlock { body }
            | ExprKind::Async { body, .. } => walk_expr(body, diagnostics),
            ExprKind::Block {
                statements, defers, ..
            } => {
                for stmt in statements {
                    walk_stmt(stmt, diagnostics);
                }
                for defer in defers {
                    walk_expr(defer, diagnostics);
                }
            }
            ExprKind::Break { value: None }
            | ExprKind::Return { value: None }
            | ExprKind::Continue
            | ExprKind::Literal(_)
            | ExprKind::FixityLiteral(_)
            | ExprKind::Identifier(_)
            | ExprKind::ModulePath(_)
            | ExprKind::Handle { .. } => {}
        }
    }

    fn walk_stmt(stmt: &Stmt, diagnostics: &mut Vec<FrontendDiagnostic>) {
        match &stmt.kind {
            StmtKind::Decl { decl } => match &decl.kind {
                DeclKind::Let { value, .. }
                | DeclKind::Var { value, .. }
                | DeclKind::Const { value, .. } => walk_expr(value, diagnostics),
                _ => {}
            },
            StmtKind::Expr { expr } | StmtKind::Defer { expr } => walk_expr(expr, diagnostics),
            StmtKind::Assign { target, value } => {
                if matches!(target.kind, ExprKind::Rec { .. }) {
                    diagnostics.push(rec_unsupported_position(target.span));
                }
                walk_expr(target, diagnostics);
                walk_expr(value, diagnostics);
            }
        }
    }

    fn walk_decl(decl: &Decl, diagnostics: &mut Vec<FrontendDiagnostic>) {
        match &decl.kind {
            DeclKind::Let { value, .. }
            | DeclKind::Var { value, .. }
            | DeclKind::Const { value, .. } => walk_expr(value, diagnostics),
            DeclKind::Handler(handler) => {
                for entry in &handler.entries {
                    if let HandlerEntry::Operation { body, .. } = entry {
                        walk_expr(body, diagnostics);
                    }
                }
            }
            DeclKind::Conductor(conductor) => {
                if let Some(exec) = &conductor.execution {
                    walk_expr(&exec.body, diagnostics);
                }
                if let Some(monitor) = &conductor.monitoring {
                    walk_expr(&monitor.body, diagnostics);
                }
            }
            _ => {}
        }
    }

    for function in &module.functions {
        walk_expr(&function.body, diagnostics);
    }
    for active in &module.active_patterns {
        walk_expr(&active.body, diagnostics);
    }
    for decl in &module.decls {
        walk_decl(decl, diagnostics);
    }
    for expr in &module.exprs {
        walk_expr(expr, diagnostics);
    }
}

fn build_if_guard_deprecated_diagnostic(span: Span) -> FrontendDiagnostic {
    let mut diagnostic =
        FrontendDiagnostic::new("`if` ガードは非推奨です。正規形の `when` を使用してください。")
            .with_code("pattern.guard.if_deprecated")
            .with_severity(DiagnosticSeverity::Warning)
            .with_domain(DiagnosticDomain::Parser)
            .with_recoverability(Recoverability::Recoverable)
            .with_span(span);
    diagnostic.add_note(DiagnosticNote::new(
        "pattern.guard.if_deprecated.note",
        "`match` のガードは `when` に統一されます。互換目的の `if` は将来削除予定です。",
    ));
    diagnostic
}

fn inspect_cfg_expr(
    expr: &Expr,
    diagnostics: &mut Vec<FrontendDiagnostic>,
    registry: &CfgTargetRegistry,
) {
    match &expr.kind {
        ExprKind::Block {
            attrs, statements, ..
        } => {
            evaluate_cfg_attributes(attrs, diagnostics, registry);
            for stmt in statements {
                inspect_cfg_stmt(stmt, diagnostics, registry);
            }
        }
        ExprKind::Lambda { body, .. }
        | ExprKind::Loop { body }
        | ExprKind::Unsafe { body }
        | ExprKind::Defer { body }
        | ExprKind::EffectBlock { body }
        | ExprKind::Async { body, .. }
        | ExprKind::Return {
            value: Some(body), ..
        } => inspect_cfg_expr(body, diagnostics, registry),
        ExprKind::Pipe { left, right }
        | ExprKind::Binary { left, right, .. }
        | ExprKind::Assign {
            target: left,
            value: right,
        } => {
            inspect_cfg_expr(left, diagnostics, registry);
            inspect_cfg_expr(right, diagnostics, registry);
        }
        ExprKind::Unary { expr: inner, .. }
        | ExprKind::Rec { expr: inner }
        | ExprKind::Propagate { expr: inner }
        | ExprKind::Await { expr: inner }
        | ExprKind::PerformCall {
            call: EffectCall {
                argument: inner, ..
            },
        }
        | ExprKind::FieldAccess { target: inner, .. }
        | ExprKind::TupleAccess { target: inner, .. }
        | ExprKind::Index { target: inner, .. } => {
            inspect_cfg_expr(inner, diagnostics, registry);
        }
        ExprKind::InlineAsm(asm) => {
            for output in &asm.outputs {
                inspect_cfg_expr(&output.target, diagnostics, registry);
            }
            for input in &asm.inputs {
                inspect_cfg_expr(&input.expr, diagnostics, registry);
            }
        }
        ExprKind::LlvmIr(ir) => {
            for input in &ir.inputs {
                inspect_cfg_expr(input, diagnostics, registry);
            }
        }
        ExprKind::Call { callee, args } => {
            inspect_cfg_expr(callee, diagnostics, registry);
            for arg in args {
                inspect_cfg_expr(arg, diagnostics, registry);
            }
        }
        ExprKind::IfElse {
            condition,
            then_branch,
            else_branch,
        } => {
            inspect_cfg_expr(condition, diagnostics, registry);
            inspect_cfg_expr(then_branch, diagnostics, registry);
            if let Some(else_branch) = else_branch {
                inspect_cfg_expr(else_branch, diagnostics, registry);
            }
        }
        ExprKind::Match { target, arms } => {
            inspect_cfg_expr(target, diagnostics, registry);
            for arm in arms {
                inspect_cfg_expr(&arm.body, diagnostics, registry);
                if let Some(guard) = &arm.guard {
                    inspect_cfg_expr(guard, diagnostics, registry);
                }
            }
        }
        ExprKind::While { condition, body } => {
            inspect_cfg_expr(condition, diagnostics, registry);
            inspect_cfg_expr(body, diagnostics, registry);
        }
        ExprKind::For { start, end, .. } => {
            inspect_cfg_expr(start, diagnostics, registry);
            inspect_cfg_expr(end, diagnostics, registry);
        }
        ExprKind::Handle { handle } => {
            inspect_cfg_expr(&handle.target, diagnostics, registry);
            for entry in &handle.handler.entries {
                if let HandlerEntry::Operation { attrs, body, .. } = entry {
                    evaluate_cfg_attributes(attrs, diagnostics, registry);
                    inspect_cfg_expr(body, diagnostics, registry);
                }
            }
        }
        ExprKind::Identifier(_)
        | ExprKind::ModulePath(_)
        | ExprKind::Literal(_)
        | ExprKind::FixityLiteral(_)
        | ExprKind::Break { value: None }
        | ExprKind::Return { value: None }
        | ExprKind::Continue => {}
        ExprKind::Break { value: Some(inner) } => {
            inspect_cfg_expr(inner, diagnostics, registry);
        }
    }
}

fn inspect_cfg_stmt(
    stmt: &Stmt,
    diagnostics: &mut Vec<FrontendDiagnostic>,
    registry: &CfgTargetRegistry,
) {
    match &stmt.kind {
        StmtKind::Decl { decl } => evaluate_cfg_attributes(&decl.attrs, diagnostics, registry),
        StmtKind::Expr { expr } => inspect_cfg_expr(expr, diagnostics, registry),
        StmtKind::Assign { target, value } => {
            inspect_cfg_expr(target, diagnostics, registry);
            inspect_cfg_expr(value, diagnostics, registry);
        }
        StmtKind::Defer { expr } => inspect_cfg_expr(expr, diagnostics, registry),
    }
}

fn evaluate_cfg_attributes(
    attrs: &[Attribute],
    diagnostics: &mut Vec<FrontendDiagnostic>,
    registry: &CfgTargetRegistry,
) {
    for attr in attrs {
        if attr.name.name != "cfg" {
            continue;
        }
        for expr in &attr.args {
            if let Some(target_value) = extract_target_literal(expr) {
                if !registry.is_allowed(&target_value) {
                    diagnostics.push(build_cfg_target_diagnostic(attr, &target_value, registry));
                }
            }
        }
    }
}

fn extract_target_literal(expr: &Expr) -> Option<String> {
    if let ExprKind::Binary {
        operator,
        left,
        right,
    } = &expr.kind
    {
        if matches!(operator, BinaryOp::Eq) && is_target_ident(left) {
            return literal_string(right);
        }
    } else if let ExprKind::Assign { target, value } = &expr.kind {
        if is_target_ident(target) {
            return literal_string(value);
        }
    }
    None
}

fn is_target_ident(expr: &Expr) -> bool {
    matches!(&expr.kind, ExprKind::Identifier(ident) if ident.name == "target")
}

fn literal_string(expr: &Expr) -> Option<String> {
    match &expr.kind {
        ExprKind::Literal(Literal {
            value: LiteralKind::String { value, .. },
        }) => Some(value.clone()),
        _ => None,
    }
}

fn build_cfg_target_diagnostic(
    attr: &Attribute,
    value: &str,
    registry: &CfgTargetRegistry,
) -> FrontendDiagnostic {
    let message = format!("未定義ターゲット `{value}` を参照する `@cfg` は評価できません。");
    let mut diagnostic = FrontendDiagnostic::new(message)
        .with_code("language.cfg.unsatisfied_branch")
        .with_severity(DiagnosticSeverity::Error)
        .with_domain(DiagnosticDomain::Parser)
        .with_span(attr.span)
        .with_recoverability(Recoverability::Fatal);
    if let Some(example) = registry.example_label() {
        let note_message =
            format!("`target = \"{example}\"` などサポート済みのプロファイルを利用してください。");
        diagnostic.add_note(DiagnosticNote::new("cfg.target.example", note_message));
    }
    diagnostic
}

struct CfgTargetRegistry {
    allowed: HashSet<String>,
}

impl CfgTargetRegistry {
    fn from_run_config(run_config: &RunConfig) -> Self {
        let mut allowed = HashSet::new();
        if let Some(target_ext) = run_config.extension("target") {
            if let Some(profile_id) = target_ext.get("profile_id").and_then(Value::as_str) {
                allowed.insert(profile_id.to_string());
            }
            if let Some(detected) = target_ext.get("detected") {
                if let Some(profile_id) = detected.get("profile_id").and_then(Value::as_str) {
                    allowed.insert(profile_id.to_string());
                }
            }
            if let Some(extra) = target_ext.get("extra").and_then(Value::as_object) {
                if let Some(value) = extra.get("target").and_then(Value::as_str) {
                    allowed.insert(value.to_string());
                }
            }
        }
        allowed.insert("cli".to_string());
        Self { allowed }
    }

    fn is_allowed(&self, requested: &str) -> bool {
        let normalized = requested.to_ascii_lowercase();
        self.allowed
            .iter()
            .any(|candidate| candidate.eq_ignore_ascii_case(&normalized))
    }

    fn is_empty(&self) -> bool {
        self.allowed.is_empty()
    }

    fn example_label(&self) -> Option<String> {
        if self.allowed.contains("cli") {
            return Some("cli".to_string());
        }
        self.allowed.iter().next().cloned()
    }
}

fn detect_handle_missing_with_tokens(tokens: &[Token]) -> Vec<FrontendDiagnostic> {
    let mut diags = Vec::new();
    let mut emitted_spans: HashSet<Span> = HashSet::new();
    let mut pending_handle: Option<Span> = None;
    for token in tokens {
        match token.kind {
            TokenKind::KeywordHandle => pending_handle = Some(token.span),
            TokenKind::KeywordWith => pending_handle = None,
            TokenKind::KeywordHandler => {
                if pending_handle.take().is_some() && emitted_spans.insert(token.span) {
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
        ET::keyword("async"),
        ET::keyword("await"),
        ET::keyword("continue"),
        ET::keyword("defer"),
        ET::keyword("do"),
        ET::keyword("effect"),
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
        TokenKind::Hash => vec![ET::token("#")],
        TokenKind::Ampersand => vec![ET::token("&")],
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
        TokenKind::Ellipsis => vec![ET::token("...")],
        TokenKind::DotDot => vec![ET::token("..")],
        TokenKind::Underscore => vec![ET::token("_")],
        TokenKind::Comment => vec![ET::custom("comment")],
        TokenKind::Whitespace => vec![ET::custom("whitespace")],
        TokenKind::EndOfFile => vec![ET::eof()],
        TokenKind::Unknown => vec![ET::custom("unknown token")],
        _ => Vec::new(),
    }
}

fn cases_to_list_expr(cases: Vec<Expr>, span: Span) -> Expr {
    let mut list_expr = Expr::identifier(Ident {
        name: "Nil".to_string(),
        span,
    });
    for case_expr in cases.into_iter().rev() {
        let cons_ident = Ident {
            name: "Cons".to_string(),
            span,
        };
        list_expr = Expr::call(
            Expr::identifier(cons_ident),
            vec![case_expr, list_expr],
            span,
        );
    }
    list_expr
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
        just(TokenKind::KeywordMut),
        just(TokenKind::KeywordNew),
    ))
    .map_with_span(move |_, span: Range<usize>| {
        let slice = &source[span.start..span.end];
        (slice.to_string(), range_to_span(span))
    });

    let ident = identifier.clone().map(|(name, span)| Ident { name, span });

    let lower_ident = choice((
        just(TokenKind::Identifier),
        just(TokenKind::KeywordMut),
        just(TokenKind::KeywordSelf),
    ))
    .map_with_span(move |_, span: Range<usize>| {
        let slice = &source[span.start..span.end];
        Ident {
            name: slice.to_string(),
            span: range_to_span(span),
        }
    });

    let upper_ident =
        just(TokenKind::UpperIdentifier).map_with_span(move |_, span: Range<usize>| {
            let slice = &source[span.start..span.end];
            Ident {
                name: slice.to_string(),
                span: range_to_span(span),
            }
        });

    let context_keyword = |keyword: &'static str, token: TokenKind| {
        choice((
            just(token).to(()),
            lower_ident.clone().try_map(move |ident, span| {
                if ident.name == keyword {
                    Ok(())
                } else {
                    Err(Simple::expected_input_found(span, Vec::new(), None))
                }
            }),
        ))
    };
    let operation_keyword = context_keyword("operation", TokenKind::KeywordOperation);
    let pattern_keyword = context_keyword("pattern", TokenKind::KeywordPattern);

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

    let qualified_name = ident
        .clone()
        .then(separator.clone().ignore_then(ident.clone()).repeated())
        .map_with_span(|(first, rest), span: Range<usize>| {
            let mut segments = Vec::with_capacity(1 + rest.len());
            segments.push(first);
            segments.extend(rest);
            QualifiedName {
                segments,
                span: range_to_span(span),
            }
        });

    let dotted_ident = ident
        .clone()
        .then(just(TokenKind::Dot).ignore_then(ident.clone()).repeated())
        .map(|(first, rest)| merge_dotted_ident(first, rest));

    let module_path_segment = choice((
        just(TokenKind::Identifier),
        just(TokenKind::UpperIdentifier),
        just(TokenKind::KeywordThen),
    ))
    .map_with_span(move |_, span: Range<usize>| {
        let slice = &source[span.start..span.end];
        Ident {
            name: slice.to_string(),
            span: range_to_span(span),
        }
    });

    let module_path_root = just(TokenKind::Colon)
        .ignore_then(just(TokenKind::Colon))
        .ignore_then(
            module_path_segment
                .clone()
                .separated_by(just(TokenKind::Dot))
                .at_least(1),
        )
        .map_with_span(|segments, span: Range<usize>| {
            (ModulePath::Root { segments }, range_to_span(span))
        });

    let module_path_super = just(TokenKind::KeywordSuper)
        .then(
            just(TokenKind::Dot)
                .ignore_then(just(TokenKind::KeywordSuper))
                .repeated(),
        )
        .map_with_span(|(_, supers), span: Range<usize>| {
            (
                RelativeHead::Super(1 + supers.len() as u32),
                range_to_span(span),
            )
        });

    let module_path_self = just(TokenKind::KeywordSelf)
        .map_with_span(|_, span: Range<usize>| (RelativeHead::Self_, range_to_span(span)));

    let module_path_head_ident = choice((
        just(TokenKind::Identifier),
        just(TokenKind::UpperIdentifier),
    ))
    .map_with_span(move |_, span: Range<usize>| {
        let slice = &source[span.start..span.end];
        Ident {
            name: slice.to_string(),
            span: range_to_span(span),
        }
    });

    let module_path_head = choice((
        module_path_self,
        module_path_super,
        module_path_head_ident
            .clone()
            .map(|ident| (RelativeHead::PlainIdent(ident.clone()), ident.span)),
    ));

    let module_path_relative = module_path_head
        .then(
            just(TokenKind::Dot)
                .ignore_then(module_path_segment.clone())
                .repeated(),
        )
        .map_with_span(|((head, _), segments), span: Range<usize>| {
            (ModulePath::Relative { head, segments }, range_to_span(span))
        });

    let module_path = choice((module_path_root, module_path_relative));

    let mut type_parser = Recursive::declare();

    let type_parser_for_expr = type_parser.clone();
    let pattern_var = lower_ident.clone().map(|ident| Pattern {
        span: ident.span,
        kind: PatternKind::Var(ident),
    });

    let pattern = recursive(|pat| {
        #[derive(Clone)]
        enum RecordPatternEntry {
            Field(PatternRecordField),
            Rest,
        }

        let parse_string_literal = |span: Range<usize>| -> (String, StringKind) {
            let slice = &source[span.start..span.end];
            if slice.starts_with("r\"") && slice.ends_with('"') && slice.len() >= 3 {
                (slice[2..slice.len() - 1].to_string(), StringKind::Raw)
            } else if slice.starts_with("\"\"\"") && slice.ends_with("\"\"\"") && slice.len() >= 6 {
                (slice[3..slice.len() - 3].to_string(), StringKind::Multiline)
            } else if slice.starts_with('"') && slice.ends_with('"') && slice.len() >= 2 {
                (
                    slice[1..slice.len() - 1].replace("\\\"", "\""),
                    StringKind::Normal,
                )
            } else {
                (slice.replace("\\\"", "\""), StringKind::Normal)
            }
        };

        let bool_literal_pattern = choice((
            just(TokenKind::KeywordTrue).map_with_span(|_, span: Range<usize>| Pattern {
                span: range_to_span(span),
                kind: PatternKind::Literal(Literal {
                    value: LiteralKind::Bool { value: true },
                }),
            }),
            just(TokenKind::KeywordFalse).map_with_span(|_, span: Range<usize>| Pattern {
                span: range_to_span(span),
                kind: PatternKind::Literal(Literal {
                    value: LiteralKind::Bool { value: false },
                }),
            }),
        ));

        let int_literal_pattern =
            just(TokenKind::IntLiteral).map_with_span(|_, span: Range<usize>| {
                let slice = &source[span.start..span.end];
                let value = slice.parse::<i64>().unwrap_or_default();
                Pattern {
                    span: range_to_span(span),
                    kind: PatternKind::Literal(Literal {
                        value: LiteralKind::Int {
                            value,
                            raw: slice.to_string(),
                            base: IntBase::Base10,
                        },
                    }),
                }
            });

        let float_literal_pattern =
            just(TokenKind::FloatLiteral).map_with_span(|_, span: Range<usize>| {
                let slice = &source[span.start..span.end];
                Pattern {
                    span: range_to_span(span),
                    kind: PatternKind::Literal(Literal {
                        value: LiteralKind::Float {
                            raw: slice.to_string(),
                        },
                    }),
                }
            });

        let string_literal_pattern =
            just(TokenKind::StringLiteral).map_with_span(move |_, span: Range<usize>| {
                let (value, string_kind) = parse_string_literal(span.clone());
                let span = range_to_span(span);
                let kind = if matches!(string_kind, StringKind::Raw) {
                    PatternKind::Regex {
                        pattern: value.clone(),
                        string_kind: string_kind.clone(),
                    }
                } else {
                    PatternKind::Literal(Literal {
                        value: LiteralKind::String {
                            value: value.clone(),
                            string_kind: string_kind.clone(),
                        },
                    })
                };
                Pattern { span, kind }
            });

        let unit_literal_pattern = just(TokenKind::LParen)
            .ignore_then(just(TokenKind::RParen).cut())
            .map_with_span(|_, span: Range<usize>| Pattern {
                span: range_to_span(span),
                kind: PatternKind::Literal(Literal {
                    value: LiteralKind::Unit,
                }),
            });

        let literal_pattern = choice((
            bool_literal_pattern,
            int_literal_pattern,
            float_literal_pattern,
            string_literal_pattern,
            unit_literal_pattern,
        ));

        let tuple_pattern = delimited_with_cut(
            TokenKind::LParen,
            pat.clone()
                .cut()
                .separated_by(just(TokenKind::Comma))
                .at_least(2)
                .allow_trailing(),
            TokenKind::RParen,
        )
        .map_with_span(|elements, span: Range<usize>| Pattern {
            span: range_to_span(span),
            kind: PatternKind::Tuple { elements },
        });

        let slice_rest = just(TokenKind::DotDot)
            .ignore_then(ident.clone().or_not())
            .map(|ident| SlicePatternItem::Rest { ident });
        let slice_element = slice_rest.or(pat.clone().map(SlicePatternItem::Element));

        let slice_pattern = delimited_with_cut(
            TokenKind::LBracket,
            slice_element
                .cut()
                .separated_by(just(TokenKind::Comma))
                .allow_trailing(),
            TokenKind::RBracket,
        )
        .map_with_span(|elements, span: Range<usize>| Pattern {
            span: range_to_span(span),
            kind: PatternKind::Slice { elements },
        });

        let pattern_ctor = qualified_ident
            .clone()
            .then(
                delimited_with_cut(
                    TokenKind::LParen,
                    pat.clone()
                        .cut()
                        .separated_by(just(TokenKind::Comma))
                        .allow_trailing(),
                    TokenKind::RParen,
                )
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

        let active_pattern_suffix = just(TokenKind::Bar).ignore_then(
            just(TokenKind::Underscore)
                .ignore_then(just(TokenKind::Bar))
                .to(true)
                .or_not()
                .map(|is_partial| is_partial.unwrap_or(false)),
        );

        let active_pattern_head = just(TokenKind::LParen)
            .ignore_then(just(TokenKind::Bar))
            .ignore_then(ident.clone())
            .then(active_pattern_suffix)
            .then_ignore(just(TokenKind::RParen).cut());

        let active_pattern = active_pattern_head
            .then(pat.clone().or_not())
            .map_with_span(
                |((name, is_partial), argument), span: Range<usize>| Pattern {
                    span: range_to_span(span),
                    kind: PatternKind::ActivePattern {
                        name,
                        is_partial,
                        argument: argument.map(Box::new),
                    },
                },
            );

        let record_field_alias = ident.clone().map(|field_ident| PatternRecordField {
            key: field_ident.clone(),
            value: None,
        });

        let record_field_with_pattern = ident.clone().then(
            just(TokenKind::Colon)
                .ignore_then(pat.clone().cut())
                .map(|pattern| Box::new(pattern))
                .or_not(),
        );

        let record_field = record_field_with_pattern
            .map(|(key, value)| PatternRecordField { key, value })
            .or(record_field_alias);

        let record_pattern = delimited_with_cut(
            TokenKind::LBrace,
            record_field
                .map(RecordPatternEntry::Field)
                .or(just(TokenKind::DotDot).map(|_| RecordPatternEntry::Rest))
                .cut()
                .separated_by(just(TokenKind::Comma))
                .allow_trailing(),
            TokenKind::RBrace,
        )
        .map_with_span(|entries, span: Range<usize>| {
            let mut fields = Vec::new();
            let mut has_rest = false;
            for entry in entries {
                match entry {
                    RecordPatternEntry::Field(field) => fields.push(field),
                    RecordPatternEntry::Rest => has_rest = true,
                }
            }
            Pattern {
                span: range_to_span(span),
                kind: PatternKind::Record { fields, has_rest },
            }
        });

        let base_pattern = choice((
            active_pattern.clone(),
            tuple_pattern.clone(),
            record_pattern.clone(),
            slice_pattern.clone(),
            pattern_var.clone(),
            pattern_ctor.clone(),
            literal_pattern.clone(),
            wildcard_pattern.clone(),
        ));

        let binding_pattern = choice((
            ident
                .clone()
                .then_ignore(just(TokenKind::At))
                .then(pat.clone())
                .map_with_span(|(name, pattern), span: Range<usize>| Pattern {
                    span: range_to_span(span),
                    kind: PatternKind::Binding {
                        name,
                        pattern: Box::new(pattern),
                        via_at: true,
                    },
                }),
            base_pattern
                .clone()
                .then(just(TokenKind::KeywordAs).ignore_then(ident.clone()))
                .map_with_span(|(pattern, name), span: Range<usize>| Pattern {
                    span: range_to_span(span),
                    kind: PatternKind::Binding {
                        name,
                        pattern: Box::new(pattern),
                        via_at: false,
                    },
                }),
            base_pattern.clone(),
        ));

        let range_with_start = binding_pattern
            .clone()
            .then(
                just(TokenKind::DotDot)
                    .then(just(TokenKind::Assign).or_not())
                    .then(binding_pattern.clone().or_not()),
            )
            .map_with_span(
                |(start, ((_, inclusive_opt), end)), span: Range<usize>| Pattern {
                    span: range_to_span(span),
                    kind: PatternKind::Range {
                        start: Some(Box::new(start)),
                        end: end.map(Box::new),
                        inclusive: inclusive_opt.is_some(),
                    },
                },
            );

        let range_without_start = just(TokenKind::DotDot)
            .then(just(TokenKind::Assign).or_not())
            .then(binding_pattern.clone().or_not())
            .map_with_span(|((_, inclusive_opt), end), span: Range<usize>| Pattern {
                span: range_to_span(span),
                kind: PatternKind::Range {
                    start: None,
                    end: end.map(Box::new),
                    inclusive: inclusive_opt.is_some(),
                },
            });

        let range_pattern = choice((range_with_start, range_without_start, binding_pattern));

        range_pattern
            .clone()
            .separated_by(just(TokenKind::Bar))
            .at_least(1)
            .map_with_span(|variants, span: Range<usize>| {
                if variants.len() == 1 {
                    variants.into_iter().next().unwrap()
                } else {
                    Pattern {
                        span: range_to_span(span),
                        kind: PatternKind::Or { variants },
                    }
                }
            })
    });

    let pattern_for_expr = pattern.clone();
    let pattern_for_block = pattern.clone();
    // ラムダ `|...|` のパラメータでは、`|` がラムダ区切りと衝突するため Or-pattern を受理しない。
    // 例: `|_| (|x| x)` や `|(a, b)| a + b` の区切り `|` を Or と誤解釈するとパースが崩れる。
    let pattern_for_lambda_param = recursive(|pat| {
        let wildcard_pattern =
            just(TokenKind::Underscore).map_with_span(|_, span: Range<usize>| Pattern {
                span: range_to_span(span),
                kind: PatternKind::Wildcard,
            });

        let var_pattern = ident.clone().map(|name| Pattern {
            span: name.span,
            kind: PatternKind::Var(name),
        });

        let tuple_pattern = delimited_with_cut(
            TokenKind::LParen,
            pat.clone()
                .cut()
                .separated_by(just(TokenKind::Comma))
                .at_least(2)
                .allow_trailing(),
            TokenKind::RParen,
        )
        .map_with_span(|elements, span: Range<usize>| Pattern {
            span: range_to_span(span),
            kind: PatternKind::Tuple { elements },
        });

        choice((tuple_pattern, var_pattern, wildcard_pattern))
    });

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
        let float_literal =
            just(TokenKind::FloatLiteral).map_with_span(move |_, span: Range<usize>| {
                let slice = &source[span.start..span.end];
                Expr::float(slice.to_string(), range_to_span(span))
            });

        let string_literal =
            just(TokenKind::StringLiteral).map_with_span(move |_, span: Range<usize>| {
                let unescaped = parse_string_literal_value(source, span.clone());
                Expr::string(unescaped, range_to_span(span))
            });

        let fixity_literal = choice((
            just(TokenKind::FixityPrefix).to(FixityKind::Prefix),
            just(TokenKind::FixityPostfix).to(FixityKind::Postfix),
            just(TokenKind::FixityInfixLeft).to(FixityKind::InfixLeft),
            just(TokenKind::FixityInfixRight).to(FixityKind::InfixRight),
            just(TokenKind::FixityInfixNonassoc).to(FixityKind::InfixNonassoc),
            just(TokenKind::FixityTernary).to(FixityKind::Ternary),
        ))
        .map_with_span(|kind, span: Range<usize>| Expr::fixity(kind, range_to_span(span)));

        let tuple_literal = delimited_with_cut(
            TokenKind::LParen,
            expr.clone()
                .cut()
                .separated_by(just(TokenKind::Comma))
                .at_least(2)
                .allow_trailing(),
            TokenKind::RParen,
        )
        .map_with_span(|elements, span: Range<usize>| {
            Expr::literal(
                Literal {
                    value: LiteralKind::Tuple { elements },
                },
                range_to_span(span),
            )
        });

        let unit_literal = just(TokenKind::LParen)
            .ignore_then(just(TokenKind::RParen).cut())
            .map_with_span(|_, span: Range<usize>| {
                Expr::literal(
                    Literal {
                        value: LiteralKind::Unit,
                    },
                    range_to_span(span),
                )
            });

        let paren_expr = delimited_with_cut(TokenKind::LParen, expr.clone(), TokenKind::RParen);

        let array_literal = delimited_with_cut(
            TokenKind::LBracket,
            expr.clone()
                .cut()
                .separated_by(just(TokenKind::Comma))
                .allow_trailing(),
            TokenKind::RBracket,
        )
        .map_with_span(|elements, span: Range<usize>| {
            Expr::literal(
                Literal {
                    value: LiteralKind::Array { elements },
                },
                range_to_span(span),
            )
        });

        let record_lambda_param = pattern_for_lambda_param
            .clone()
            .then(
                just(TokenKind::Colon)
                    .ignore_then(type_parser_for_expr.clone())
                    .or_not(),
            )
            .then(just(TokenKind::Assign).ignore_then(expr.clone()).or_not())
            .map(|((pattern, ty), default)| Param {
                span: pattern.span,
                pattern,
                type_annotation: ty,
                default,
            });

        let record_lambda_params = just(TokenKind::LParen)
            .ignore_then(
                record_lambda_param
                    .clone()
                    .separated_by(just(TokenKind::Comma))
                    .allow_trailing(),
            )
            .then_ignore(just(TokenKind::RParen));

        let record_field_lambda = record_lambda_params
            .clone()
            .then_ignore(just(TokenKind::Arrow))
            .then(expr.clone())
            .map_with_span(|(params, body), span: Range<usize>| Expr {
                span: range_to_span(span),
                kind: ExprKind::Lambda {
                    params,
                    ret_type: None,
                    body: Box::new(body),
                },
            });

        let record_field_value = choice((record_field_lambda, expr.clone()));

        let record_literal_field = ident
            .clone()
            .then(
                just(TokenKind::Assign)
                    .ignore_then(expr.clone().cut())
                    .or(just(TokenKind::Colon).ignore_then(record_field_value.cut()))
                    .or_not(),
            )
            .map(|(key, value)| {
                let value = value.unwrap_or_else(|| Expr::identifier(key.clone()));
                RecordField { key, value }
            });

        let record_literal_fields = just(TokenKind::LBrace)
            .ignore_then(
                record_literal_field
                    .cut()
                    .separated_by(just(TokenKind::Comma))
                    .allow_trailing(),
            )
            .then_ignore(just(TokenKind::RBrace));

        let record_literal =
            record_literal_fields
                .clone()
                .map_with_span(|fields, span: Range<usize>| {
                    Expr::literal(
                        Literal {
                            value: LiteralKind::Record {
                                type_name: None,
                                fields,
                            },
                        },
                        range_to_span(span),
                    )
                });

        let typed_record_literal = upper_ident
            .clone()
            .then(record_literal_fields.clone())
            .map_with_span(|(type_name, fields), span: Range<usize>| {
                Expr::literal(
                    Literal {
                        value: LiteralKind::Record {
                            type_name: Some(type_name),
                            fields,
                        },
                    },
                    range_to_span(span),
                )
            });

        let set_literal = just(TokenKind::LBrace)
            .ignore_then(
                expr.clone()
                    .separated_by(just(TokenKind::Comma))
                    .allow_trailing()
                    .at_least(1),
            )
            .then_ignore(just(TokenKind::RBrace))
            .map_with_span(|elements, span: Range<usize>| {
                Expr::literal(
                    Literal {
                        value: LiteralKind::Set { elements },
                    },
                    range_to_span(span),
                )
            });

        let lambda_param = pattern_for_lambda_param
            .clone()
            .then(
                just(TokenKind::Colon)
                    .ignore_then(type_parser_for_expr.clone().cut())
                    .or_not(),
            )
            .then(
                just(TokenKind::Assign)
                    .ignore_then(expr.clone().cut())
                    .or_not(),
            )
            .map(|((pattern, ty), default)| Param {
                span: pattern.span,
                pattern,
                type_annotation: ty,
                default,
            });

        let lambda_params = delimited_with_cut(
            TokenKind::LParen,
            lambda_param
                .clone()
                .cut()
                .separated_by(just(TokenKind::Comma))
                .allow_trailing(),
            TokenKind::RParen,
        );

        let assign_field = choice((
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

        let assign_target = ident_expr
            .clone()
            .then(separator.clone().ignore_then(assign_field).repeated())
            .map(|(base, fields)| {
                fields.into_iter().fold(base, |acc, field| {
                    let span = span_union(acc.span(), field.span);
                    Expr::field_access(acc, field, span)
                })
            });

        let string_literal_value = just(TokenKind::StringLiteral)
            .map_with_span(move |_, span: Range<usize>| parse_string_literal_value(source, span));

        let inline_asm_ident = lower_ident.clone().try_map(|ident, span| {
            if ident.name == "inline_asm" {
                Ok(ident)
            } else {
                Err(Simple::expected_input_found(span, Vec::new(), None))
            }
        });

        let inline_asm_output = string_literal_value
            .clone()
            .then_ignore(just(TokenKind::Colon))
            .then(assign_target.clone())
            .map(|(constraint, target)| InlineAsmOutput { constraint, target });

        let inline_asm_input = string_literal_value
            .clone()
            .then_ignore(just(TokenKind::Colon))
            .then(expr.clone())
            .map(|(constraint, expr)| InlineAsmInput { constraint, expr });

        #[derive(Clone)]
        enum InlineAsmArg {
            Outputs(Vec<InlineAsmOutput>),
            Inputs(Vec<InlineAsmInput>),
            Clobbers(Vec<String>),
            Options(Vec<String>),
        }

        let inline_asm_outputs = lower_ident
            .clone()
            .try_map(|ident, span| {
                if ident.name == "outputs" {
                    Ok(())
                } else {
                    Err(Simple::expected_input_found(span, Vec::new(), None))
                }
            })
            .ignore_then(delimited_with_cut(
                TokenKind::LParen,
                inline_asm_output
                    .clone()
                    .separated_by(just(TokenKind::Comma))
                    .allow_trailing()
                    .or_not()
                    .map(|outputs| outputs.unwrap_or_default()),
                TokenKind::RParen,
            ))
            .map(InlineAsmArg::Outputs);

        let inline_asm_inputs = lower_ident
            .clone()
            .try_map(|ident, span| {
                if ident.name == "inputs" {
                    Ok(())
                } else {
                    Err(Simple::expected_input_found(span, Vec::new(), None))
                }
            })
            .ignore_then(delimited_with_cut(
                TokenKind::LParen,
                inline_asm_input
                    .clone()
                    .separated_by(just(TokenKind::Comma))
                    .allow_trailing()
                    .or_not()
                    .map(|inputs| inputs.unwrap_or_default()),
                TokenKind::RParen,
            ))
            .map(InlineAsmArg::Inputs);

        let inline_asm_clobbers = lower_ident
            .clone()
            .try_map(|ident, span| {
                if ident.name == "clobbers" {
                    Ok(())
                } else {
                    Err(Simple::expected_input_found(span, Vec::new(), None))
                }
            })
            .ignore_then(delimited_with_cut(
                TokenKind::LParen,
                string_literal_value
                    .clone()
                    .separated_by(just(TokenKind::Comma))
                    .allow_trailing(),
                TokenKind::RParen,
            ))
            .map(InlineAsmArg::Clobbers);

        let inline_asm_options = lower_ident
            .clone()
            .try_map(|ident, span| {
                if ident.name == "options" {
                    Ok(())
                } else {
                    Err(Simple::expected_input_found(span, Vec::new(), None))
                }
            })
            .ignore_then(delimited_with_cut(
                TokenKind::LParen,
                string_literal_value
                    .clone()
                    .separated_by(just(TokenKind::Comma))
                    .allow_trailing(),
                TokenKind::RParen,
            ))
            .map(InlineAsmArg::Options);

        let inline_asm_arg = choice((
            inline_asm_outputs,
            inline_asm_inputs,
            inline_asm_clobbers,
            inline_asm_options,
        ));

        let inline_asm_expr = inline_asm_ident
            .ignore_then(just(TokenKind::LParen))
            .ignore_then(string_literal_value.clone())
            .then(
                just(TokenKind::Comma)
                    .ignore_then(
                        inline_asm_arg
                            .clone()
                            .separated_by(just(TokenKind::Comma))
                            .allow_trailing(),
                    )
                    .or_not(),
            )
            .then_ignore(just(TokenKind::RParen).cut())
            .map_with_span(|(template, args), span: Range<usize>| {
                let mut outputs = Vec::new();
                let mut inputs = Vec::new();
                let mut clobbers = Vec::new();
                let mut options = Vec::new();
                for arg in args.unwrap_or_default() {
                    match arg {
                        InlineAsmArg::Outputs(items) => outputs.extend(items),
                        InlineAsmArg::Inputs(items) => inputs.extend(items),
                        InlineAsmArg::Clobbers(items) => clobbers.extend(items),
                        InlineAsmArg::Options(items) => options.extend(items),
                    }
                }
                Expr {
                    span: range_to_span(span),
                    kind: ExprKind::InlineAsm(InlineAsmExpr {
                        template,
                        outputs,
                        inputs,
                        clobbers,
                        options,
                    }),
                }
            });

        let llvm_ir_ident = lower_ident.clone().try_map(|ident, span| {
            if ident.name == "llvm_ir" {
                Ok(ident)
            } else {
                Err(Simple::expected_input_found(span, Vec::new(), None))
            }
        });

        let llvm_ir_inputs = lower_ident
            .clone()
            .try_map(|ident, span| {
                if ident.name == "inputs" {
                    Ok(())
                } else {
                    Err(Simple::expected_input_found(span, Vec::new(), None))
                }
            })
            .ignore_then(delimited_with_cut(
                TokenKind::LParen,
                expr.clone()
                    .separated_by(just(TokenKind::Comma))
                    .allow_trailing()
                    .or_not()
                    .map(|inputs| inputs.unwrap_or_default()),
                TokenKind::RParen,
            ));

        let llvm_ir_expr = llvm_ir_ident
            .then_ignore(just(TokenKind::Not))
            .then(delimited_with_cut(
                TokenKind::LParen,
                type_parser_for_expr.clone().cut(),
                TokenKind::RParen,
            ))
            .then(
                just(TokenKind::LBrace)
                    .ignore_then(string_literal_value.clone())
                    .then(just(TokenKind::Comma).ignore_then(llvm_ir_inputs).or_not())
                    .then_ignore(just(TokenKind::RBrace).cut()),
            )
            .map_with_span(
                |((_, result_type), (template, inputs)), span: Range<usize>| Expr {
                    span: range_to_span(span),
                    kind: ExprKind::LlvmIr(LlvmIrExpr {
                        result_type,
                        template,
                        inputs: inputs.unwrap_or_default(),
                    }),
                },
            );

        let stmt = build_stmt_parser(
            expr.clone(),
            pattern_for_expr.clone(),
            lower_ident.clone(),
            lambda_params.clone(),
            type_parser_for_expr.clone(),
            assign_target.clone(),
        );

        let stmt_with_sep = stmt
            .clone()
            .then_ignore(just(TokenKind::Semicolon).repeated());

        let raw_block = just(TokenKind::LBrace)
            .ignore_then(stmt_with_sep.clone().repeated())
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

        let unsafe_expr = just(TokenKind::KeywordUnsafe)
            .ignore_then(block_expr.clone())
            .map_with_span(|body, span: Range<usize>| Expr {
                span: range_to_span(span),
                kind: ExprKind::Unsafe {
                    body: Box::new(body),
                },
            });

        let effect_block_expr = just(TokenKind::KeywordEffect)
            .ignore_then(block_expr.clone())
            .map_with_span(|body, span: Range<usize>| Expr {
                span: range_to_span(span),
                kind: ExprKind::EffectBlock {
                    body: Box::new(body),
                },
            });

        let match_guard = choice((
            just(TokenKind::KeywordWhen).to(false),
            just(TokenKind::KeywordIf).to(true),
        ))
        .then(expr.clone())
        .map(|(used_if, guard_expr)| (guard_expr, used_if));

        let match_alias = just(TokenKind::KeywordAs).ignore_then(lower_ident.clone());

        let match_tail = choice((
            match_guard
                .clone()
                .then(match_alias.clone().or_not())
                .map(|((guard_expr, used_if), alias)| (Some(guard_expr), used_if, alias)),
            match_alias
                .clone()
                .then(match_guard.clone().or_not())
                .map(|(alias, guard_opt)| match guard_opt {
                    Some((guard_expr, used_if)) => (Some(guard_expr), used_if, Some(alias)),
                    None => (None, false, Some(alias)),
                }),
        ))
        .or_not()
        .map(|result| result.unwrap_or((None, false, None)));

        let match_arm = just(TokenKind::Bar)
            .or_not()
            .ignore_then(pattern_for_expr.clone())
            .then(match_tail)
            .then_ignore(just(TokenKind::Arrow))
            .then(expr.clone())
            .map_with_span(
                |((pattern, (guard, used_if, alias)), body), span: Range<usize>| {
                    let guard_used_if = guard.as_ref().is_some() && used_if;
                    MatchArm {
                        pattern,
                        guard,
                        guard_used_if,
                        alias,
                        body,
                        span: range_to_span(span),
                    }
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

        let continue_expr =
            just(TokenKind::KeywordContinue).map_with_span(|_, span: Range<usize>| Expr {
                span: range_to_span(span),
                kind: ExprKind::Continue,
            });

        let break_expr = just(TokenKind::KeywordBreak)
            .ignore_then(expr.clone().or_not())
            .map_with_span(|value, span: Range<usize>| Expr {
                span: range_to_span(span),
                kind: ExprKind::Break {
                    value: value.map(|inner| Box::new(inner)),
                },
            });

        let loop_expr = just(TokenKind::KeywordLoop)
            .ignore_then(expr.clone())
            .map_with_span(|body, span: Range<usize>| Expr {
                span: range_to_span(span),
                kind: ExprKind::Loop {
                    body: Box::new(body),
                },
            });

        let while_expr = just(TokenKind::KeywordWhile)
            .ignore_then(expr.clone())
            .then(expr.clone())
            .map_with_span(|(condition, body), span: Range<usize>| Expr {
                span: range_to_span(span),
                kind: ExprKind::While {
                    condition: Box::new(condition),
                    body: Box::new(body),
                },
            });

        let for_expr = just(TokenKind::KeywordFor)
            .ignore_then(pattern_for_expr.clone())
            .then_ignore(just(TokenKind::KeywordIn))
            .then(expr.clone())
            .then(expr.clone())
            .map_with_span(|((pattern, iterator), body), span: Range<usize>| Expr {
                span: range_to_span(span),
                kind: ExprKind::For {
                    pattern,
                    start: Box::new(iterator),
                    end: Box::new(body),
                },
            });

        let handler_param = pattern_for_expr
            .clone()
            .then(
                just(TokenKind::Colon)
                    .ignore_then(type_parser_for_expr.clone().cut())
                    .or_not(),
            )
            .then(
                just(TokenKind::Assign)
                    .ignore_then(expr.clone().cut())
                    .or_not(),
            )
            .map(|((pattern, ty), default)| Param {
                span: pattern.span,
                pattern,
                type_annotation: ty,
                default,
            });

        let handler_param_list = delimited_with_cut(
            TokenKind::LParen,
            handler_param
                .cut()
                .separated_by(just(TokenKind::Comma))
                .allow_trailing()
                .or_not()
                .map(|params| params.unwrap_or_default()),
            TokenKind::RParen,
        );

        let handler_operation_entry = attr_list
            .clone()
            .then_ignore(operation_keyword.clone())
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

        let fn_lambda_expr = just(TokenKind::KeywordFn)
            .ignore_then(lambda_params.clone())
            .then(
                just(TokenKind::Arrow)
                    .ignore_then(type_parser_for_expr.clone().cut())
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

        let bar_lambda_params = lambda_param
            .separated_by(just(TokenKind::Comma))
            .allow_trailing()
            .or_not()
            .map(|params| params.unwrap_or_default());

        let bar_lambda_empty = just(TokenKind::LogicalOr)
            .ignore_then(
                just(TokenKind::Arrow)
                    .ignore_then(type_parser_for_expr.clone().cut())
                    .or_not(),
            )
            .then(lambda_body_expr.clone())
            .map_with_span(|(ret_type, body), span: Range<usize>| Expr {
                span: range_to_span(span),
                kind: ExprKind::Lambda {
                    params: Vec::new(),
                    ret_type,
                    body: Box::new(body),
                },
            });

        let bar_lambda_expr = just(TokenKind::Bar)
            .ignore_then(bar_lambda_params)
            .then_ignore(just(TokenKind::Bar))
            .then(
                just(TokenKind::Arrow)
                    .ignore_then(type_parser_for_expr.clone().cut())
                    .or_not(),
            )
            .then(lambda_body_expr.clone())
            .map_with_span(|((params, ret_type), body), span: Range<usize>| Expr {
                span: range_to_span(span),
                kind: ExprKind::Lambda {
                    params,
                    ret_type,
                    body: Box::new(body),
                },
            });
        let bar_lambda_expr = choice((bar_lambda_empty, bar_lambda_expr));

        let test_parser_ident = lower_ident.clone().try_map(|ident, span| {
            if ident.name == "test_parser" {
                Ok(ident)
            } else {
                Err(Simple::expected_input_found(span, Vec::new(), None))
            }
        });

        let case_keyword = lower_ident.clone().try_map(|ident, span| {
            if ident.name == "case" {
                Ok(ident)
            } else {
                Err(Simple::expected_input_found(span, Vec::new(), None))
            }
        });

        let case_string =
            just(TokenKind::StringLiteral).map_with_span(move |_, span: Range<usize>| {
                let unescaped = parse_string_literal_value(source, span.clone());
                Expr::string(unescaped, range_to_span(span))
            });

        let case_name_and_source = case_string
            .clone()
            .then(
                just(TokenKind::Colon)
                    .ignore_then(case_string.clone().cut())
                    .or_not(),
            )
            .map(|(first_expr, maybe_second)| match maybe_second {
                Some(second_expr) => (Some(first_expr), second_expr),
                None => (None, first_expr),
            });

        let case_entry = case_keyword
            .ignore_then(case_name_and_source)
            .then_ignore(just(TokenKind::DoubleArrow).cut())
            .then(expr.clone())
            .map_with_span(
                |((name_expr, source_expr), expect_expr), span: Range<usize>| {
                    let span = range_to_span(span);
                    let key_name = Ident {
                        name: "name".to_string(),
                        span,
                    };
                    let key_source = Ident {
                        name: "source".to_string(),
                        span,
                    };
                    let key_expect = Ident {
                        name: "expect".to_string(),
                        span,
                    };
                    let name_value = match name_expr {
                        Some(expr) => {
                            let some_ident = Ident {
                                name: "Some".to_string(),
                                span,
                            };
                            Expr::call(Expr::identifier(some_ident), vec![expr], span)
                        }
                        None => {
                            let none_ident = Ident {
                                name: "None".to_string(),
                                span,
                            };
                            Expr::identifier(none_ident)
                        }
                    };
                    let fields = vec![
                        RecordField {
                            key: key_name,
                            value: name_value,
                        },
                        RecordField {
                            key: key_source,
                            value: source_expr,
                        },
                        RecordField {
                            key: key_expect,
                            value: expect_expr,
                        },
                    ];
                    Expr::literal(
                        Literal {
                            value: LiteralKind::Record {
                                type_name: None,
                                fields,
                            },
                        },
                        span,
                    )
                },
            );

        let case_block = just(TokenKind::LBrace)
            .ignore_then(
                case_entry
                    .then_ignore(just(TokenKind::Semicolon).or_not())
                    .repeated(),
            )
            .then_ignore(just(TokenKind::RBrace))
            .map_with_span(|cases, span: Range<usize>| (cases, range_to_span(span)));

        let test_parser_expr = test_parser_ident
            .then(
                delimited_with_cut(TokenKind::LParen, expr.clone().cut(), TokenKind::RParen)
                    .map_with_span(|parser_expr, span: Range<usize>| {
                        (parser_expr, range_to_span(span))
                    }),
            )
            .then(case_block.clone())
            .map_with_span(
                |((callee_ident, (parser_expr, _parser_span)), (cases, cases_span)),
                 span: Range<usize>| {
                    let callee = Expr::identifier(callee_ident);
                    let cases_expr = cases_to_list_expr(cases, cases_span);
                    Expr::call(callee, vec![parser_expr, cases_expr], range_to_span(span))
                },
            );

        let atom = choice((
            inline_asm_expr,
            llvm_ir_expr,
            test_parser_expr,
            block_expr.clone(),
            effect_block_expr,
            match_expr,
            handle_expr,
            bar_lambda_expr,
            fn_lambda_expr,
            int_literal.clone(),
            float_literal.clone(),
            bool_literal,
            string_literal,
            fixity_literal,
            array_literal,
            typed_record_literal,
            record_literal,
            set_literal,
            tuple_literal,
            unit_literal,
            ident_expr.clone(),
            paren_expr,
        ))
        .boxed();

        #[derive(Clone)]
        enum Postfix {
            Call(Vec<Expr>, Span),
            Field(Ident, Span),
        }

        let call_args = delimited_with_cut(
            TokenKind::LParen,
            expr.clone()
                .cut()
                .separated_by(just(TokenKind::Comma))
                .allow_trailing(),
            TokenKind::RParen,
        )
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

        let call_core = atom
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

        let call = call_core
            .clone()
            .then(
                just(TokenKind::Question)
                    .map_with_span(|_, span: Range<usize>| range_to_span(span))
                    .repeated(),
            )
            .map(|(base, propagations)| {
                propagations.into_iter().fold(base, |acc, span| {
                    let combined = span_union(acc.span(), span);
                    Expr {
                        span: combined,
                        kind: ExprKind::Propagate {
                            expr: Box::new(acc),
                        },
                    }
                })
            });

        let unary: Recursive<'src, TokenKind, Expr, Simple<TokenKind>> = recursive(
            |unary: Recursive<'src, TokenKind, Expr, Simple<TokenKind>>| {
                let prefix_op = choice((
                    just(TokenKind::Not).to(UnaryOp::Not),
                    just(TokenKind::Minus).to(UnaryOp::Neg),
                ))
                .map_with_span(|op, span: Range<usize>| (op, range_to_span(span)));

                let move_keyword = just(TokenKind::KeywordMove);

                let async_expr = just(TokenKind::KeywordAsync)
                    .map_with_span(|_, span: Range<usize>| range_to_span(span))
                    .then(move_keyword.or_not())
                    .then(unary.clone())
                    .map(|((async_span, is_move), inner)| Expr {
                        span: span_union(async_span, inner.span()),
                        kind: ExprKind::Async {
                            body: Box::new(inner),
                            is_move: is_move.is_some(),
                        },
                    });

                let await_expr = just(TokenKind::KeywordAwait)
                    .map_with_span(|_, span: Range<usize>| range_to_span(span))
                    .then(unary.clone())
                    .map(|(await_span, inner)| Expr {
                        span: span_union(await_span, inner.span()),
                        kind: ExprKind::Await {
                            expr: Box::new(inner),
                        },
                    });

                let unary_expr = prefix_op.clone().then(unary.clone()).map(|(op, inner)| {
                    let (operator, op_span) = op;
                    let span = span_union(op_span, inner.span());
                    Expr {
                        span,
                        kind: ExprKind::Unary {
                            operator,
                            expr: Box::new(inner),
                        },
                    }
                });

                let rec_expr = just(TokenKind::KeywordRec)
                    .map_with_span(|_, span: Range<usize>| range_to_span(span))
                    .then(unary.clone())
                    .map(|(rec_span, inner)| Expr {
                        span: span_union(rec_span, inner.span()),
                        kind: ExprKind::Rec {
                            expr: Box::new(inner),
                        },
                    });

                choice((async_expr, await_expr, rec_expr, unary_expr, call.clone()))
            },
        );

        let power = unary
            .clone()
            .then(
                just(TokenKind::Caret)
                    .to("^")
                    .then(unary.clone().cut())
                    .repeated(),
            )
            .map(|(first, rest)| {
                if rest.is_empty() {
                    return first;
                }
                let mut operands = Vec::with_capacity(rest.len() + 1);
                operands.push(first);
                for (_op, rhs) in rest {
                    operands.push(rhs);
                }
                let mut rhs = operands.pop().expect("power operands");
                while let Some(lhs) = operands.pop() {
                    let span = span_union(lhs.span(), rhs.span());
                    rhs = Expr::binary("^", lhs, rhs, span);
                }
                rhs
            });

        let multiplicative = power
            .clone()
            .then(
                choice((
                    just(TokenKind::Star).to("*"),
                    just(TokenKind::Slash).to("/"),
                    just(TokenKind::Percent).to("%"),
                ))
                .then(power.clone().cut())
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
                .then(multiplicative.clone().cut())
                .repeated(),
            )
            .map(|(first, rest)| {
                rest.into_iter().fold(first, |lhs, (op, rhs)| {
                    let span = span_union(lhs.span(), rhs.span());
                    Expr::binary(op, lhs, rhs, span)
                })
            });

        let range_expr = additive
            .clone()
            .then(
                just(TokenKind::DotDot)
                    .to("..")
                    .then(additive.clone().cut())
                    .repeated(),
            )
            .map(|(first, rest)| {
                rest.into_iter().fold(first, |lhs, (_op, rhs)| {
                    let span = span_union(lhs.span(), rhs.span());
                    Expr::binary("..", lhs, rhs, span)
                })
            });

        let comparison = range_expr
            .clone()
            .then(
                choice((
                    just(TokenKind::Gt).to(">"),
                    just(TokenKind::Ge).to(">="),
                    just(TokenKind::Lt).to("<"),
                    just(TokenKind::Le).to("<="),
                ))
                .then(range_expr.clone().cut())
                .or_not(),
            )
            .map(|(first, rest)| match rest {
                Some((op, rhs)) => {
                    let span = span_union(first.span(), rhs.span());
                    Expr::binary(op, first, rhs, span)
                }
                None => first,
            });

        let equality = comparison
            .clone()
            .then(
                choice((
                    just(TokenKind::EqEq).to("=="),
                    just(TokenKind::NotEqual).to("!="),
                ))
                .then(comparison.clone().cut())
                .or_not(),
            )
            .map(|(first, rest)| match rest {
                Some((op, rhs)) => {
                    let span = span_union(first.span(), rhs.span());
                    Expr::binary(op, first, rhs, span)
                }
                None => first,
            });

        let logical_and = equality
            .clone()
            .then(
                just(TokenKind::LogicalAnd)
                    .to("&&")
                    .then(equality.clone().cut())
                    .repeated(),
            )
            .map(|(first, rest)| {
                rest.into_iter().fold(first, |lhs, (op, rhs)| {
                    let span = span_union(lhs.span(), rhs.span());
                    Expr::binary(op, lhs, rhs, span)
                })
            });

        let logical_or = logical_and
            .clone()
            .then(
                just(TokenKind::LogicalOr)
                    .to("||")
                    .then(logical_and.clone().cut())
                    .repeated(),
            )
            .map(|(first, rest)| {
                rest.into_iter().fold(first, |lhs, (op, rhs)| {
                    let span = span_union(lhs.span(), rhs.span());
                    Expr::binary(op, lhs, rhs, span)
                })
            });

        let pipe_expr = logical_or
            .clone()
            .then(
                just(TokenKind::PipeForward)
                    .to("|>")
                    .then(logical_or.clone().cut())
                    .repeated(),
            )
            .map(|(first, rest)| {
                rest.into_iter().fold(first, |lhs, (_op, rhs)| {
                    let span = span_union(lhs.span(), rhs.span());
                    Expr::pipe(lhs, rhs, span)
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

        let effect_args = delimited_with_cut(
            TokenKind::LParen,
            expr.clone()
                .cut()
                .separated_by(just(TokenKind::Comma))
                .allow_trailing(),
            TokenKind::RParen,
        )
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

        let assignment_expr = assign_target
            .clone()
            .then_ignore(just(TokenKind::Assign))
            .then(expr.clone().cut())
            .map_with_span(|(target, value), span: Range<usize>| {
                Expr::assign(target, value, range_to_span(span))
            });

        choice((
            if_expr,
            while_expr,
            for_expr,
            loop_expr,
            break_expr,
            continue_expr,
            perform_expr,
            do_expr,
            assignment_expr,
            block_expr,
            unsafe_expr,
            pipe_expr,
        ))
        .boxed()
    });

    let type_parser_definition = recursive(|ty| {
        let args = ty
            .clone()
            .separated_by(just(TokenKind::Comma))
            .allow_trailing()
            .delimited_by(just(TokenKind::Lt), just(TokenKind::Gt));

        let string_literal_type =
            just(TokenKind::StringLiteral).map_with_span(move |_, span: Range<usize>| TypeAnnot {
                span: range_to_span(span.clone()),
                kind: TypeKind::Literal {
                    value: TypeLiteral::String {
                        value: parse_string_literal_value(source, span),
                    },
                },
                annotation_kind: None,
            });

        let int_literal_type =
            just(TokenKind::IntLiteral).map_with_span(move |_, span: Range<usize>| {
                let slice = &source[span.start..span.end];
                let value = slice.parse::<i64>().unwrap_or_default();
                TypeAnnot {
                    span: range_to_span(span),
                    kind: TypeKind::Literal {
                        value: TypeLiteral::Int {
                            value,
                            raw: slice.to_string(),
                        },
                    },
                    annotation_kind: None,
                }
            });

        let array_length = just(TokenKind::IntLiteral)
            .labelled("array length must be integer literal")
            .map_with_span(move |_, span: Range<usize>| {
                let slice = &source[span.start..span.end];
                let value = slice.parse::<i64>().unwrap_or_default();
                TypeArrayLength {
                    value,
                    raw: slice.to_string(),
                    span: range_to_span(span),
                }
            });

        let tuple_element_labeled = ident
            .clone()
            .then_ignore(just(TokenKind::Colon))
            .then(ty.clone())
            .map(|(label, ty)| TypeTupleElement {
                label: Some(label),
                ty,
            });

        let tuple_element = choice((
            tuple_element_labeled,
            ty.clone().map(|ty| TypeTupleElement { label: None, ty }),
        ));

        let tuple_type = delimited_with_cut(
            TokenKind::LParen,
            tuple_element
                .clone()
                .cut()
                .separated_by(just(TokenKind::Comma))
                .allow_trailing(),
            TokenKind::RParen,
        )
        .map_with_span(|elements, span: Range<usize>| TypeAnnot {
            span: range_to_span(span),
            kind: TypeKind::Tuple { elements },
            annotation_kind: None,
        });

        let slice_type =
            delimited_with_cut(TokenKind::LBracket, ty.clone().cut(), TokenKind::RBracket)
                .map_with_span(|element, span: Range<usize>| TypeAnnot {
                    span: range_to_span(span),
                    kind: TypeKind::Slice {
                        element: Box::new(element),
                    },
                    annotation_kind: None,
                });

        let array_type = just(TokenKind::LBracket)
            .ignore_then(ty.clone())
            .then_ignore(just(TokenKind::Semicolon).cut())
            .then(array_length.clone().cut())
            .then_ignore(just(TokenKind::RBracket).cut())
            .map_with_span(|(element, length), span: Range<usize>| TypeAnnot {
                span: range_to_span(span),
                kind: TypeKind::Array {
                    element: Box::new(element),
                    length,
                },
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
            .then(ty.clone().cut())
            .then(
                just(TokenKind::Assign)
                    .ignore_then(expr.clone().cut())
                    .or_not(),
            )
            .map(|((label, ty), default_expr)| TypeRecordField {
                label,
                ty,
                default_expr,
            });

        let record_type = delimited_with_cut(
            TokenKind::LBrace,
            record_field
                .clone()
                .cut()
                .separated_by(just(TokenKind::Comma))
                .allow_trailing(),
            TokenKind::RBrace,
        )
        .map_with_span(|fields, span: Range<usize>| TypeAnnot {
            span: range_to_span(span),
            kind: TypeKind::Record { fields },
            annotation_kind: None,
        });

        let variant_record_payload = delimited_with_cut(
            TokenKind::LBrace,
            record_field
                .clone()
                .cut()
                .separated_by(just(TokenKind::Comma))
                .allow_trailing(),
            TokenKind::RBrace,
        )
        .map(|fields| VariantPayload::Record { fields });

        let variant_tuple_payload = delimited_with_cut(
            TokenKind::LParen,
            tuple_element
                .clone()
                .cut()
                .separated_by(just(TokenKind::Comma))
                .allow_trailing(),
            TokenKind::RParen,
        )
        .map(|elements| VariantPayload::Tuple { elements });

        let variant_payload = choice((variant_record_payload, variant_tuple_payload));

        let fn_param_labeled = ident
            .clone()
            .then_ignore(just(TokenKind::Colon))
            .then(ty.clone())
            .map(|(label, ty)| (Some(label), ty));

        let fn_param_unlabeled = ty.clone().map(|ty| (None, ty));

        let fn_param = choice((fn_param_labeled, fn_param_unlabeled));

        let fn_type = just(TokenKind::KeywordFn)
            .ignore_then(delimited_with_cut(
                TokenKind::LParen,
                fn_param
                    .clone()
                    .cut()
                    .separated_by(just(TokenKind::Comma))
                    .allow_trailing()
                    .or_not()
                    .map(|params| params.unwrap_or_default()),
                TokenKind::RParen,
            ))
            .then_ignore(just(TokenKind::Arrow))
            .then(ty.clone().cut())
            .map_with_span(|(params, ret_ty), span: Range<usize>| {
                let (param_labels, params) =
                    params
                        .into_iter()
                        .fold((Vec::new(), Vec::new()), |mut acc, entry| {
                            acc.0.push(entry.0);
                            acc.1.push(entry.1);
                            acc
                        });
                TypeAnnot {
                    span: range_to_span(span),
                    kind: TypeKind::Fn {
                        params,
                        param_labels,
                        ret: Box::new(ret_ty),
                    },
                    annotation_kind: None,
                }
            });

        let atom = recursive(|atom| {
            let ref_type = just(TokenKind::Ampersand)
                .then(just(TokenKind::KeywordMut).or_not())
                .then(atom.clone().cut())
                .map_with_span(|((_, mut_kw), target), span: Range<usize>| TypeAnnot {
                    span: range_to_span(span),
                    kind: TypeKind::Ref {
                        target: Box::new(target),
                        mutable: mut_kw.is_some(),
                    },
                    annotation_kind: None,
                });
            let base = choice((
                fn_type,
                tuple_type,
                record_type,
                array_type,
                slice_type,
                app,
                simple,
                string_literal_type,
                int_literal_type,
            ));
            choice((ref_type, base))
        });

        let arrow_type = atom
            .clone()
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
                                params: elements.iter().map(|elem| elem.ty.clone()).collect(),
                                param_labels: elements
                                    .iter()
                                    .map(|elem| elem.label.clone())
                                    .collect(),
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
                                    param_labels: vec![None],
                                    ret: Box::new(ret_ty),
                                },
                                annotation_kind: None,
                            }
                        }
                    }
                } else {
                    left
                }
            });

        let union_variant_payload = ident.clone().then(variant_payload.clone()).map_with_span(
            |(name, payload), span: Range<usize>| TypeUnionVariant::Variant {
                name,
                payload: Some(payload),
                span: range_to_span(span),
            },
        );

        let union_variant = choice((
            union_variant_payload,
            arrow_type.clone().map(|ty| TypeUnionVariant::Type { ty }),
        ));

        union_variant
            .clone()
            .separated_by(just(TokenKind::Bar))
            .at_least(1)
            .map_with_span(|variants, span: Range<usize>| {
                if variants.len() == 1 {
                    match variants.into_iter().next().unwrap() {
                        TypeUnionVariant::Type { ty } => ty,
                        variant => TypeAnnot {
                            span: range_to_span(span),
                            kind: TypeKind::Union {
                                variants: vec![variant],
                            },
                            annotation_kind: None,
                        },
                    }
                } else {
                    TypeAnnot {
                        span: range_to_span(span),
                        kind: TypeKind::Union { variants },
                        annotation_kind: None,
                    }
                }
            })
    });
    type_parser.define(type_parser_definition);

    let attribute = build_attribute_parser(expr.clone(), ident.clone());
    let attr_list = attribute.clone().repeated();
    let visibility = just(TokenKind::KeywordPub)
        .to(Visibility::Public)
        .or_not()
        .map(|vis| vis.unwrap_or(Visibility::Private));

    let receiver_pattern = just(TokenKind::Ampersand)
        .then(just(TokenKind::KeywordMut).or_not())
        .then(just(TokenKind::KeywordSelf))
        .map_with_span(|_, span: Range<usize>| Pattern {
            span: range_to_span(span.clone()),
            kind: PatternKind::Var(Ident {
                name: "self".to_string(),
                span: range_to_span(span),
            }),
        });

    let param_pattern = choice((receiver_pattern, pattern_for_block.clone()));

    let param = param_pattern
        .clone()
        .then(
            just(TokenKind::Colon)
                .ignore_then(type_parser.clone().cut())
                .or_not(),
        )
        .then(
            just(TokenKind::Assign)
                .ignore_then(expr.clone().cut())
                .or_not(),
        );

    let params = delimited_with_cut(
        TokenKind::LParen,
        param
            .clone()
            .map(|((pattern, ty), default)| Param {
                span: pattern.span,
                pattern,
                type_annotation: ty,
                default,
            })
            .cut()
            .separated_by(just(TokenKind::Comma))
            .allow_trailing(),
        TokenKind::RParen,
    );
    let params_with_varargs = params.clone().map(|params| (params, false));

    let extern_param = param.clone().map(|((pattern, ty), default)| Param {
        span: pattern.span,
        pattern,
        type_annotation: ty,
        default,
    });
    let params_no_trailing = extern_param.clone().separated_by(just(TokenKind::Comma));
    let params_with_variadic = params_no_trailing
        .clone()
        .then(
            just(TokenKind::Comma)
                .ignore_then(just(TokenKind::Ellipsis))
                .to(true)
                .or_not(),
        )
        .map(|(params, varargs)| (params, varargs.unwrap_or(false)));
    let params_with_trailing = extern_param
        .clone()
        .separated_by(just(TokenKind::Comma))
        .allow_trailing()
        .map(|params| (params, false));
    let variadic_only = just(TokenKind::Ellipsis).to((Vec::new(), true));
    let extern_params = delimited_with_cut(
        TokenKind::LParen,
        choice((variadic_only, params_with_variadic, params_with_trailing)),
        TokenKind::RParen,
    );

    let generic_param = ident
        .clone()
        .then(
            just(TokenKind::Colon)
                .ignore_then(
                    type_parser
                        .clone()
                        .separated_by(just(TokenKind::Plus))
                        .at_least(1),
                )
                .or_not(),
        )
        .map(|(ident, _bounds)| ident);

    let generic_params = generic_param
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
        trait_reference
            .clone()
            .map(|trait_ref| WherePredicate::Trait { trait_ref }),
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
        .ignore_then(delimited_with_cut(
            TokenKind::LBrace,
            ident
                .clone()
                .cut()
                .separated_by(just(TokenKind::Comma))
                .allow_trailing()
                .or_not()
                .map(|tags| tags.unwrap_or_default()),
            TokenKind::RBrace,
        ))
        .map_with_span(|tags, span: Range<usize>| EffectAnnotation {
            tags,
            span: range_to_span(span),
        });

    let async_flag = just(TokenKind::KeywordAsync)
        .to(true)
        .or_not()
        .map(|flag| flag.unwrap_or(false));

    let unsafe_flag = just(TokenKind::KeywordUnsafe)
        .to(true)
        .or_not()
        .map(|flag| flag.unwrap_or(false));

    let fn_signature = async_flag
        .clone()
        .then(unsafe_flag.clone())
        .then(
            just(TokenKind::KeywordFn)
                .map_with_span(move |_, span: Range<usize>| range_to_span(span)),
        )
        .then(qualified_name.clone())
        .then(parse_generics.clone())
        .then(params_with_varargs.clone())
        .then(
            just(TokenKind::Arrow)
                .ignore_then(type_parser.clone())
                .or_not(),
        )
        .then(effect_annotation.clone().or_not())
        .then(where_clause.clone())
        .then(effect_annotation.clone().or_not())
        .map_with_span(|value, span: Range<usize>| {
            let (value, effect_after_where) = value;
            let (value, where_clause) = value;
            let (value, effect_before_where) = value;
            let (value, ret_type) = value;
            let (value, params_varargs) = value;
            let (value, generics) = value;
            let (value, qualified_name) = value;
            let ((is_async, is_unsafe), fn_span) = value;
            let (params, varargs) = params_varargs;
            let effect = effect_after_where.or(effect_before_where);
            let signature_span = range_to_span(span);
            let is_qualified = qualified_name.segments.len() > 1;
            let name = qualified_name.to_ident();
            let qualified_name = if is_qualified {
                Some(qualified_name)
            } else {
                None
            };
            FunctionSignature {
                name,
                qualified_name,
                generics,
                params,
                varargs,
                ret_type,
                where_clause,
                effect,
                is_async,
                is_unsafe,
                span: Span::new(fn_span.start.min(signature_span.start), signature_span.end),
            }
        });

    let receiver_type = dotted_ident
        .clone()
        .then(
            type_parser
                .clone()
                .separated_by(just(TokenKind::Comma))
                .allow_trailing()
                .delimited_by(just(TokenKind::Lt), just(TokenKind::Gt))
                .or_not(),
        )
        .map_with_span(|(callee, args), span: Range<usize>| TypeAnnot {
            span: range_to_span(span),
            kind: if let Some(args) = args {
                TypeKind::App { callee, args }
            } else {
                TypeKind::Ident { name: callee }
            },
            annotation_kind: None,
        });

    let method_signature = async_flag
        .clone()
        .then(unsafe_flag.clone())
        .then(
            just(TokenKind::KeywordFn)
                .map_with_span(move |_, span: Range<usize>| range_to_span(span)),
        )
        .then(receiver_type.clone())
        .then_ignore(separator.clone())
        .then(ident.clone())
        .then(parse_generics.clone())
        .then(params_with_varargs.clone())
        .then(
            just(TokenKind::Arrow)
                .ignore_then(type_parser.clone())
                .or_not(),
        )
        .then(effect_annotation.clone().or_not())
        .then(where_clause.clone())
        .then(effect_annotation.clone().or_not())
        .map_with_span(|value, span: Range<usize>| {
            let (value, effect_after_where) = value;
            let (value, where_clause) = value;
            let (value, effect_before_where) = value;
            let (value, ret_type) = value;
            let (value, params_varargs) = value;
            let (value, generics) = value;
            let (value, name) = value;
            let (value, receiver) = value;
            let ((is_async, is_unsafe), fn_span) = value;
            let (params, varargs) = params_varargs;
            let effect = effect_after_where.or(effect_before_where);
            let signature_span = range_to_span(span);
            let signature = FunctionSignature {
                name,
                qualified_name: None,
                generics,
                params,
                varargs,
                ret_type,
                where_clause,
                effect,
                is_async,
                is_unsafe,
                span: Span::new(fn_span.start.min(signature_span.start), signature_span.end),
            };
            (receiver, signature)
        });

    let extern_fn_signature = async_flag
        .clone()
        .then(unsafe_flag.clone())
        .then(
            just(TokenKind::KeywordFn)
                .map_with_span(move |_, span: Range<usize>| range_to_span(span)),
        )
        .then(qualified_name.clone())
        .then(parse_generics.clone())
        .then(extern_params.clone())
        .then(
            just(TokenKind::Arrow)
                .ignore_then(type_parser.clone())
                .or_not(),
        )
        .then(effect_annotation.clone().or_not())
        .then(where_clause.clone())
        .then(effect_annotation.clone().or_not())
        .map_with_span(|value, span: Range<usize>| {
            let (value, effect_after_where) = value;
            let (value, where_clause) = value;
            let (value, effect_before_where) = value;
            let (value, ret_type) = value;
            let (value, params_varargs) = value;
            let (value, generics) = value;
            let (value, qualified_name) = value;
            let ((is_async, is_unsafe), fn_span) = value;
            let (params, varargs) = params_varargs;
            let effect = effect_after_where.or(effect_before_where);
            let signature_span = range_to_span(span);
            let is_qualified = qualified_name.segments.len() > 1;
            let name = qualified_name.to_ident();
            let qualified_name = if is_qualified {
                Some(qualified_name)
            } else {
                None
            };
            FunctionSignature {
                name,
                qualified_name,
                generics,
                params,
                varargs,
                ret_type,
                where_clause,
                effect,
                is_async,
                is_unsafe,
                span: Span::new(fn_span.start.min(signature_span.start), signature_span.end),
            }
        });

    let abi_literal = just(TokenKind::StringLiteral)
        .map_with_span(move |_, span: Range<usize>| parse_string_literal_value(source, span));

    let let_decl_raw = build_let_decl_parser(
        pattern.clone(),
        lower_ident.clone(),
        params.clone(),
        type_parser.clone(),
        expr.clone(),
    );
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

    let const_decl_raw = just(TokenKind::KeywordConst)
        .ignore_then(qualified_ident.clone())
        .then_ignore(just(TokenKind::Colon))
        .then(type_parser.clone().cut())
        .then_ignore(just(TokenKind::Assign))
        .then(expr.clone())
        .map_with_span(
            |((name, type_annotation), value), span: Range<usize>| Decl {
                attrs: Vec::new(),
                visibility: Visibility::Private,
                span: range_to_span(span),
                kind: DeclKind::Const {
                    name,
                    value,
                    type_annotation,
                },
            },
        );

    let const_decl = attr_list
        .clone()
        .then(const_decl_raw.clone())
        .map(|(attrs, mut decl)| {
            if !attrs.is_empty() {
                decl.attrs = attrs;
            }
            decl
        });

    let type_decl_name = ident
        .clone()
        .then(parse_generics.clone())
        .map(|(name, generics)| (name, generics));

    enum RecordDeclItem {
        Field(TypeRecordField),
        Rest,
    }

    let record_decl_field = ident
        .clone()
        .then_ignore(just(TokenKind::Colon))
        .then(type_parser.clone().cut())
        .then(
            just(TokenKind::Assign)
                .ignore_then(expr.clone().cut())
                .or_not(),
        )
        .map(|((label, ty), default_expr)| TypeRecordField {
            label,
            ty,
            default_expr,
        });

    let record_decl_rest = just(TokenKind::DotDot).map(|_| RecordDeclItem::Rest);

    let record_decl_body = delimited_with_cut(
        TokenKind::LBrace,
        choice((
            record_decl_field.clone().map(RecordDeclItem::Field),
            record_decl_rest,
        ))
        .cut()
        .separated_by(just(TokenKind::Comma))
        .allow_trailing(),
        TokenKind::RBrace,
    )
    .map(|items| {
        let mut fields = Vec::new();
        let mut has_rest = false;
        for item in items {
            match item {
                RecordDeclItem::Field(field) => fields.push(field),
                RecordDeclItem::Rest => has_rest = true,
            }
        }
        TypeDeclVariantPayload::Record { fields, has_rest }
    });

    let sum_tuple_element_labeled = ident
        .clone()
        .then_ignore(just(TokenKind::Colon))
        .then(type_parser.clone().cut())
        .map(|(label, ty)| TypeTupleElement {
            label: Some(label),
            ty,
        });

    let sum_tuple_element = choice((
        sum_tuple_element_labeled,
        type_parser
            .clone()
            .map(|ty| TypeTupleElement { label: None, ty }),
    ));

    let sum_variant_tuple_payload = delimited_with_cut(
        TokenKind::LParen,
        sum_tuple_element
            .cut()
            .separated_by(just(TokenKind::Comma))
            .allow_trailing(),
        TokenKind::RParen,
    )
    .map(|elements| TypeDeclVariantPayload::Tuple { elements });

    let sum_variant_payload = choice((record_decl_body.clone(), sum_variant_tuple_payload));

    let sum_variant = just(TokenKind::Bar)
        .ignore_then(ident.clone())
        .then(sum_variant_payload.or_not())
        .map_with_span(|(name, payload), span: Range<usize>| TypeDeclVariant {
            name,
            payload,
            span: range_to_span(span),
        });

    let sum_body = sum_variant.repeated().at_least(1);

    let type_decl_body_alias = just(TokenKind::Assign)
        .map_with_span(|_, span: Range<usize>| span)
        .then(type_parser.clone().cut())
        .map_with_span(|(assign_span, ty), span: Range<usize>| {
            (
                TypeDeclBody::Alias { ty },
                range_to_span(assign_span.start..span.end),
            )
        });

    let type_decl_body_default = just(TokenKind::Assign)
        .map_with_span(|_, span: Range<usize>| span)
        .then(choice((
            just(TokenKind::KeywordNew)
                .ignore_then(type_parser.clone().cut())
                .map(|ty| TypeDeclBody::Newtype { ty }),
            sum_body
                .clone()
                .map(|variants| TypeDeclBody::Sum { variants }),
            type_parser.clone().map(|ty| TypeDeclBody::Alias { ty }),
        )))
        .map_with_span(|(assign_span, body), span: Range<usize>| {
            (body, range_to_span(assign_span.start..span.end))
        });

    let type_alias_decl_raw = visibility
        .clone()
        .then(
            just(TokenKind::KeywordType)
                .ignore_then(just(TokenKind::KeywordAlias))
                .ignore_then(type_decl_name.clone())
                .then(type_decl_body_alias),
        )
        .map_with_span(
            |(visibility, ((name, generics), (body, body_span))), span: Range<usize>| Decl {
                attrs: Vec::new(),
                visibility,
                span: range_to_span(span.clone()),
                kind: DeclKind::Type {
                    decl: TypeDecl {
                        name,
                        generics,
                        body: Some(body),
                        span: range_to_span(span),
                        body_span: Some(body_span),
                    },
                },
            },
        );

    let type_decl_raw = visibility
        .clone()
        .then(
            just(TokenKind::KeywordType)
                .ignore_then(type_decl_name.clone())
                .then(type_decl_body_default.or_not()),
        )
        .map_with_span(
            |(visibility, ((name, generics), body)), span: Range<usize>| {
                let (body, body_span) = body
                    .map(|(body, body_span)| (Some(body), Some(body_span)))
                    .unwrap_or((None, None));
                Decl {
                    attrs: Vec::new(),
                    visibility,
                    span: range_to_span(span.clone()),
                    kind: DeclKind::Type {
                        decl: TypeDecl {
                            name,
                            generics,
                            body,
                            span: range_to_span(span),
                            body_span,
                        },
                    },
                }
            },
        );

    let type_decl_raw = choice((type_alias_decl_raw, type_decl_raw))
        .then_ignore(just(TokenKind::Semicolon).or_not());

    let type_decl = attr_list
        .clone()
        .then(type_decl_raw.clone())
        .map(|(attrs, mut decl)| {
            if !attrs.is_empty() {
                decl.attrs = attrs;
            }
            decl
        });

    let struct_field = ident
        .clone()
        .then_ignore(just(TokenKind::Colon))
        .then(type_parser.clone().cut())
        .then(
            just(TokenKind::Assign)
                .ignore_then(expr.clone().cut())
                .or_not(),
        )
        .map(|((label, ty), default_expr)| TypeRecordField {
            label,
            ty,
            default_expr,
        });

    let struct_body = delimited_with_cut(
        TokenKind::LBrace,
        struct_field
            .cut()
            .separated_by(just(TokenKind::Comma))
            .allow_trailing()
            .or_not()
            .map(|fields| fields.unwrap_or_default()),
        TokenKind::RBrace,
    );

    let struct_decl_body = choice((
        struct_body.clone().map(|fields| (fields, false)),
        just(TokenKind::Semicolon).to((Vec::new(), true)),
    ));

    let struct_decl_raw = visibility
        .clone()
        .then(
            just(TokenKind::KeywordStruct)
                .ignore_then(type_decl_name.clone())
                .then(struct_decl_body),
        )
        .then_ignore(just(TokenKind::Semicolon).or_not())
        .map_with_span(
            |(visibility, ((name, generics), (fields, _has_semicolon))), span: Range<usize>| Decl {
                attrs: Vec::new(),
                visibility,
                span: range_to_span(span.clone()),
                kind: DeclKind::Struct(StructDecl {
                    name,
                    generics,
                    fields,
                    span: range_to_span(span),
                }),
            },
        );

    let struct_decl = attr_list
        .clone()
        .then(struct_decl_raw.clone())
        .map(|(attrs, mut decl)| {
            if !attrs.is_empty() {
                decl.attrs = attrs;
            }
            decl
        });

    let enum_tuple_element_labeled = ident
        .clone()
        .then_ignore(just(TokenKind::Colon))
        .then(type_parser.clone())
        .map(|(label, ty)| TypeTupleElement {
            label: Some(label),
            ty,
        });

    let enum_tuple_element = choice((
        enum_tuple_element_labeled,
        type_parser
            .clone()
            .map(|ty| TypeTupleElement { label: None, ty }),
    ));

    let enum_record_field = ident
        .clone()
        .then_ignore(just(TokenKind::Colon))
        .then(type_parser.clone().cut())
        .then(
            just(TokenKind::Assign)
                .ignore_then(expr.clone().cut())
                .or_not(),
        )
        .map(|((label, ty), default_expr)| TypeRecordField {
            label,
            ty,
            default_expr,
        });

    let enum_variant_record_payload = delimited_with_cut(
        TokenKind::LBrace,
        enum_record_field
            .cut()
            .separated_by(just(TokenKind::Comma))
            .allow_trailing(),
        TokenKind::RBrace,
    )
    .map(|fields| VariantPayload::Record { fields });

    let enum_variant_tuple_payload = delimited_with_cut(
        TokenKind::LParen,
        enum_tuple_element
            .cut()
            .separated_by(just(TokenKind::Comma))
            .allow_trailing(),
        TokenKind::RParen,
    )
    .map(|elements| VariantPayload::Tuple { elements });

    let enum_variant_payload = choice((enum_variant_record_payload, enum_variant_tuple_payload));

    let enum_variant = ident
        .clone()
        .then(enum_variant_payload.clone().or_not())
        .map_with_span(|(name, payload), span: Range<usize>| EnumVariant {
            name,
            payload,
            span: range_to_span(span),
        });

    let enum_variant_sep = just(TokenKind::Bar).or(just(TokenKind::Comma));

    let enum_variant_list = enum_variant
        .clone()
        .separated_by(enum_variant_sep)
        .allow_trailing()
        .at_least(1);

    let enum_variants = just(TokenKind::Bar)
        .ignore_then(enum_variant_list.clone())
        .or(enum_variant_list);

    let enum_decl_raw = visibility
        .clone()
        .then(
            just(TokenKind::KeywordEnum)
                .ignore_then(type_decl_name.clone())
                .then_ignore(just(TokenKind::Assign))
                .then(enum_variants),
        )
        .map_with_span(
            |(visibility, ((name, generics), variants)), span: Range<usize>| Decl {
                attrs: Vec::new(),
                visibility,
                span: range_to_span(span.clone()),
                kind: DeclKind::Enum(EnumDecl {
                    name,
                    generics,
                    variants,
                    span: range_to_span(span),
                }),
            },
        );

    let enum_decl = attr_list
        .clone()
        .then(enum_decl_raw.clone())
        .map(|(attrs, mut decl)| {
            if !attrs.is_empty() {
                decl.attrs = attrs;
            }
            decl
        });

    let block_body_parser = {
        let assign_field = choice((
            ident.clone(),
            just(TokenKind::KeywordNew).map_with_span(|_, span: Range<usize>| Ident {
                name: "new".to_string(),
                span: range_to_span(span),
            }),
            just(TokenKind::KeywordThen).map_with_span(|_, span: Range<usize>| Ident {
                name: "then".to_string(),
                span: range_to_span(span),
            }),
        ));
        let assign_target = ident
            .clone()
            .map(Expr::identifier)
            .then(separator.clone().ignore_then(assign_field).repeated())
            .map(|(base, fields)| {
                fields.into_iter().fold(base, |acc, field| {
                    let span = span_union(acc.span(), field.span);
                    Expr::field_access(acc, field, span)
                })
            });
        let stmt = build_stmt_parser(
            expr.clone(),
            pattern_for_block.clone(),
            lower_ident.clone(),
            params.clone(),
            type_parser.clone(),
            assign_target,
        );
        just(TokenKind::LBrace)
            .ignore_then(
                stmt.clone()
                    .then_ignore(just(TokenKind::Semicolon).repeated())
                    .repeated(),
            )
            .then_ignore(just(TokenKind::RBrace))
            .map_with_span(|stmts, span: Range<usize>| Expr::block(stmts, range_to_span(span)))
    };

    let fn_body = choice((
        just(TokenKind::Assign).ignore_then(expr.clone()),
        block_body_parser.clone(),
    ));

    let streaming_state_success_fn = streaming_state_success.clone();
    let streaming_state_success_method = streaming_state_success.clone();

    let fn_core = visibility
        .clone()
        .then(fn_signature.clone())
        .then(fn_body.clone())
        .map(move |((visibility, signature), body)| {
            let function_span = Span::new(signature.span.start, body.span().end);
            record_streaming_success(&streaming_state_success_fn, function_span);
            Function {
                name: signature.name.clone(),
                qualified_name: signature.qualified_name.clone(),
                visibility,
                generics: signature.generics.clone(),
                params: signature.params.clone(),
                body,
                ret_type: signature.ret_type.clone(),
                where_clause: signature.where_clause.clone(),
                effect: signature.effect.clone(),
                is_async: signature.is_async,
                is_unsafe: signature.is_unsafe,
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

    let fn_decl_raw = visibility
        .clone()
        .then(fn_signature.clone())
        .then_ignore(just(TokenKind::Semicolon).or_not())
        .map_with_span(|(visibility, signature), span: Range<usize>| Decl {
            attrs: Vec::new(),
            visibility,
            span: range_to_span(span),
            kind: DeclKind::Fn { signature },
        });

    let fn_decl = attr_list
        .clone()
        .then(fn_decl_raw.clone())
        .map(|(attrs, mut decl)| {
            if !attrs.is_empty() {
                decl.attrs = attrs;
            }
            decl
        });

    let method_core = visibility
        .clone()
        .then(method_signature.clone())
        .then(fn_body)
        .map_with_span(
            move |((visibility, (receiver, signature)), body), span: Range<usize>| {
                let function_span = Span::new(signature.span.start, body.span().end);
                record_streaming_success(&streaming_state_success_method, function_span);
                let function = Function {
                    name: signature.name.clone(),
                    qualified_name: signature.qualified_name.clone(),
                    visibility,
                    generics: signature.generics.clone(),
                    params: signature.params.clone(),
                    body,
                    ret_type: signature.ret_type.clone(),
                    where_clause: signature.where_clause.clone(),
                    effect: signature.effect.clone(),
                    is_async: signature.is_async,
                    is_unsafe: signature.is_unsafe,
                    span: function_span,
                    attrs: Vec::new(),
                };
                Decl {
                    attrs: Vec::new(),
                    visibility: Visibility::Private,
                    span: range_to_span(span.clone()),
                    kind: DeclKind::Impl(ImplDecl {
                        generics: Vec::new(),
                        trait_ref: None,
                        target: receiver,
                        where_clause: Vec::new(),
                        items: vec![ImplItem::Function(function)],
                        span: range_to_span(span),
                    }),
                }
            },
        );

    let method_decl = attr_list
        .clone()
        .then(method_core.clone())
        .map(|(attrs, mut decl)| {
            if !attrs.is_empty() {
                if let DeclKind::Impl(impl_decl) = &mut decl.kind {
                    if let Some(ImplItem::Function(function)) = impl_decl.items.first_mut() {
                        function.attrs = attrs;
                    }
                }
            }
            decl
        });

    let active_pattern_head = pattern_keyword.clone().ignore_then(
        just(TokenKind::LParen)
            .ignore_then(just(TokenKind::Bar))
            .ignore_then(ident.clone())
            .then(
                just(TokenKind::Bar)
                    .ignore_then(just(TokenKind::Underscore))
                    .ignore_then(just(TokenKind::Bar))
                    .to(true)
                    .or(just(TokenKind::Bar).to(false)),
            )
            .then_ignore(just(TokenKind::RParen)),
    );

    let active_pattern_decl = attr_list
        .clone()
        .then(visibility.clone())
        .then(active_pattern_head)
        .then(params.clone())
        .then_ignore(just(TokenKind::Assign))
        .then(expr.clone())
        .map_with_span(
            |((((attrs, visibility), (name, is_partial)), params), body), span: Range<usize>| {
                ActivePatternDecl {
                    name,
                    is_partial,
                    params,
                    body,
                    span: range_to_span(span),
                    attrs,
                    visibility,
                }
            },
        );

    let extern_fn_decl = attr_list
        .clone()
        .then(visibility.clone())
        .then(extern_fn_signature.clone())
        .then_ignore(just(TokenKind::Semicolon))
        .map_with_span(
            |((attrs, visibility), signature), span: Range<usize>| ExternItem {
                attrs,
                visibility,
                signature,
                span: range_to_span(span),
            },
        );

    let extern_block = delimited_with_cut(
        TokenKind::LBrace,
        extern_fn_decl.clone().cut().repeated().at_least(1),
        TokenKind::RBrace,
    );

    let extern_decl_raw = visibility
        .clone()
        .then(
            just(TokenKind::KeywordExtern)
                .ignore_then(abi_literal.clone())
                .then(choice((
                    extern_block.clone(),
                    extern_fn_decl.clone().map(|item| vec![item]),
                ))),
        )
        .map_with_span(|(visibility, (abi, functions)), span: Range<usize>| Decl {
            attrs: Vec::new(),
            visibility,
            span: range_to_span(span.clone()),
            kind: DeclKind::Extern {
                abi,
                functions,
                span: range_to_span(span),
            },
        });

    let extern_decl = attr_list
        .clone()
        .then(extern_decl_raw.clone())
        .map(|(attrs, mut decl)| {
            if !attrs.is_empty() {
                decl.attrs = attrs;
            }
            decl
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

    let trait_function_item = attr_list
        .clone()
        .then(fn_signature.clone())
        .then(trait_item_body.clone())
        .map_with_span(|((attrs, signature), body), span: Range<usize>| TraitItem {
            attrs,
            kind: ast::TraitItemKind::Function {
                signature,
                default_body: body,
            },
            span: range_to_span(span),
        });

    let trait_assoc_bounds = just(TokenKind::Colon)
        .ignore_then(
            trait_reference
                .clone()
                .separated_by(just(TokenKind::Comma))
                .at_least(1),
        )
        .or_not()
        .map(|bounds| bounds.unwrap_or_default());

    let trait_assoc_default = just(TokenKind::Assign)
        .ignore_then(type_parser.clone().cut())
        .or_not();

    let trait_associated_type_raw = just(TokenKind::KeywordType)
        .ignore_then(ident.clone())
        .then(trait_assoc_bounds)
        .then(trait_assoc_default)
        .then_ignore(just(TokenKind::Semicolon).or_not());

    let trait_associated_type = attr_list
        .clone()
        .then(trait_associated_type_raw)
        .map_with_span(
            |(attrs, ((name, bounds), default)), span: Range<usize>| TraitItem {
                attrs,
                kind: ast::TraitItemKind::AssociatedType {
                    name,
                    bounds,
                    default,
                },
                span: range_to_span(span),
            },
        );

    let trait_item = choice((trait_associated_type, trait_function_item));

    let trait_decl_raw = visibility
        .clone()
        .then(
            just(TokenKind::KeywordTrait)
                .ignore_then(ident.clone())
                .then(parse_generics.clone())
                .then(where_clause.clone())
                .then(
                    just(TokenKind::LBrace)
                        .ignore_then(trait_item.repeated())
                        .then_ignore(just(TokenKind::RBrace)),
                ),
        )
        .map_with_span(
            |(visibility, (((name, generics), where_clause), items)), span: Range<usize>| {
                let trait_decl = TraitDecl {
                    name,
                    generics,
                    where_clause,
                    items,
                    span: range_to_span(span.clone()),
                };
                Decl {
                    attrs: Vec::new(),
                    visibility,
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
        type_decl.clone().map(ImplItem::Decl),
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
            .then(expr.clone().cut())
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
            delimited_with_cut(
                TokenKind::LParen,
                conductor_arg
                    .clone()
                    .cut()
                    .separated_by(just(TokenKind::Comma))
                    .allow_trailing(),
                TokenKind::RParen,
            )
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

    let macro_decl_raw = visibility
        .clone()
        .then(
            just(TokenKind::KeywordMacro)
                .ignore_then(ident.clone())
                .then(params.clone())
                .then(block_body_parser.clone()),
        )
        .map_with_span(
            |(visibility, ((name, params), body)), span: Range<usize>| Decl {
                attrs: Vec::new(),
                visibility,
                span: range_to_span(span.clone()),
                kind: DeclKind::Macro(MacroDecl {
                    name,
                    params,
                    body,
                    span: range_to_span(span),
                }),
            },
        );

    let macro_decl = attr_list
        .clone()
        .then(macro_decl_raw.clone())
        .map(|(attrs, mut decl)| {
            if !attrs.is_empty() {
                decl.attrs = attrs;
            }
            decl
        });

    let actor_spec_decl_raw = visibility
        .clone()
        .then(
            just(TokenKind::KeywordActor)
                .ignore_then(just(TokenKind::KeywordSpec))
                .ignore_then(ident.clone())
                .then(
                    params
                        .clone()
                        .or_not()
                        .map(|params| params.unwrap_or_default()),
                )
                .then(block_body_parser.clone()),
        )
        .map_with_span(
            |(visibility, ((name, params), body)), span: Range<usize>| Decl {
                attrs: Vec::new(),
                visibility,
                span: range_to_span(span.clone()),
                kind: DeclKind::ActorSpec(ActorSpecDecl {
                    name,
                    params,
                    body,
                    span: range_to_span(span),
                }),
            },
        );

    let actor_spec_decl =
        attr_list
            .clone()
            .then(actor_spec_decl_raw.clone())
            .map(|(attrs, mut decl)| {
                if !attrs.is_empty() {
                    decl.attrs = attrs;
                }
                decl
            });

    let effect_operation = attr_list
        .clone()
        .then_ignore(operation_keyword.clone())
        .then(ident.clone())
        .then(
            just(TokenKind::Colon)
                .ignore_then(type_parser.clone().cut())
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

    let effect_body = delimited_with_cut(
        TokenKind::LBrace,
        effect_operation.repeated().at_least(1),
        TokenKind::RBrace,
    );

    let effect_decl = just(TokenKind::KeywordEffect)
        .ignore_then(ident.clone())
        .then(
            just(TokenKind::Colon)
                .ignore_then(ident.clone())
                .then(effect_body.clone())
                .or_not(),
        )
        .map_with_span(|(name, detail), span: Range<usize>| {
            let (tag, operations) = match detail {
                Some((tag, operations)) => (Some(tag), operations),
                None => (None, Vec::new()),
            };
            EffectDecl {
                span: range_to_span(span.clone()),
                name,
                tag,
                operations,
            }
        });

    #[derive(Clone)]
    enum ModuleItem {
        Effect(EffectDecl),
        Function(Function),
        ActivePattern(ActivePatternDecl),
        Decl(Decl),
        Expr(Expr),
    }

    let collect_module_body = |items: Vec<ModuleItem>| {
        let mut effects_vec = Vec::new();
        let mut functions_vec = Vec::new();
        let mut active_patterns_vec = Vec::new();
        let mut decls_vec = Vec::new();
        let mut exprs_vec = Vec::new();
        for item in items {
            match item {
                ModuleItem::Effect(effect) => effects_vec.push(effect),
                ModuleItem::Function(function) => functions_vec.push(function),
                ModuleItem::ActivePattern(active) => active_patterns_vec.push(active),
                ModuleItem::Decl(decl) => decls_vec.push(decl),
                ModuleItem::Expr(expr) => exprs_vec.push(expr),
            }
        }
        ModuleBody {
            effects: effects_vec,
            functions: functions_vec,
            active_patterns: active_patterns_vec,
            decls: decls_vec,
            exprs: exprs_vec,
        }
    };

    let top_level_defer = just(TokenKind::KeywordDefer)
        .ignore_then(expr.clone().cut())
        .then_ignore(just(TokenKind::Semicolon).repeated())
        .map_with_span(|body, span: Range<usize>| Expr {
            span: range_to_span(span),
            kind: ExprKind::Defer {
                body: Box::new(body),
            },
        })
        .map(ModuleItem::Expr);

    let top_level_expr = expr
        .clone()
        .then_ignore(just(TokenKind::Semicolon).repeated())
        .map(ModuleItem::Expr);

    let module_item = recursive(move |module_item| {
        let module_block_decl_raw = visibility
            .clone()
            .then(just(TokenKind::KeywordModule).ignore_then(module_path.clone()))
            .then(delimited_with_cut(
                TokenKind::LBrace,
                module_item.clone().repeated(),
                TokenKind::RBrace,
            ))
            .map_with_span(
                move |((visibility, (path, _)), items), span: Range<usize>| Decl {
                    attrs: Vec::new(),
                    visibility,
                    span: range_to_span(span.clone()),
                    kind: DeclKind::Module(ModuleDecl {
                        path,
                        body: collect_module_body(items),
                        span: range_to_span(span),
                    }),
                },
            );

        let module_block_decl = attr_list
            .clone()
            .then(module_block_decl_raw)
            .map(|(attrs, mut decl)| {
                if !attrs.is_empty() {
                    decl.attrs = attrs;
                }
                decl
            })
            .map(ModuleItem::Decl);

        choice((
            effect_decl.clone().map(ModuleItem::Effect),
            module_block_decl,
            trait_decl.clone().map(ModuleItem::Decl),
            impl_decl.clone().map(ModuleItem::Decl),
            type_decl.clone().map(ModuleItem::Decl),
            struct_decl.clone().map(ModuleItem::Decl),
            enum_decl.clone().map(ModuleItem::Decl),
            extern_decl.clone().map(ModuleItem::Decl),
            const_decl.clone().map(ModuleItem::Decl),
            let_decl.clone().map(ModuleItem::Decl),
            var_decl.clone().map(ModuleItem::Decl),
            macro_decl.clone().map(ModuleItem::Decl),
            actor_spec_decl.clone().map(ModuleItem::Decl),
            conductor_decl.clone().map(ModuleItem::Decl),
            active_pattern_decl.clone().map(ModuleItem::ActivePattern),
            method_decl.clone().map(ModuleItem::Decl),
            function.clone().map(ModuleItem::Function),
            fn_decl.clone().map(ModuleItem::Decl),
            top_level_defer,
            top_level_expr,
        ))
    });

    module_item
        .repeated()
        .then_ignore(just(TokenKind::EndOfFile))
        .map(move |items| {
            let body = collect_module_body(items);
            Module {
                header: None,
                uses: Vec::new(),
                effects: body.effects,
                functions: body.functions,
                active_patterns: body.active_patterns,
                decls: body.decls,
                exprs: body.exprs,
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
    for active_pattern in &module.active_patterns {
        record_active_pattern_trace_events(active_pattern, events);
    }
    for function in &module.functions {
        record_function_trace_events(function, events);
    }
    for expr in &module.exprs {
        record_expr_trace_events(expr, events);
    }
}

fn append_module_body_trace_events(body: &ModuleBody, events: &mut Vec<ParserTraceEvent>) {
    for effect in &body.effects {
        record_effect_decl_trace_events(effect, events);
    }
    for decl in &body.decls {
        record_decl_trace_events(decl, events);
    }
    for active_pattern in &body.active_patterns {
        record_active_pattern_trace_events(active_pattern, events);
    }
    for function in &body.functions {
        record_function_trace_events(function, events);
    }
    for expr in &body.exprs {
        record_expr_trace_events(expr, events);
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

fn record_active_pattern_trace_events(
    active_pattern: &ActivePatternDecl,
    events: &mut Vec<ParserTraceEvent>,
) {
    for param in &active_pattern.params {
        if let Some(default) = &param.default {
            record_expr_trace_events(default, events);
        }
    }
    record_expr_trace_events(&active_pattern.body, events);
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
        DeclKind::Const { value, .. } => {
            events.push(ParserTraceEvent::expr_enter("const", decl.span));
            record_expr_trace_events(value, events);
            events.push(ParserTraceEvent::expr_leave("const", decl.span));
        }
        DeclKind::Effect(effect) => record_effect_decl_trace_events(effect, events),
        DeclKind::Handler(handler) => {
            events.push(ParserTraceEvent::handler(handler));
        }
        DeclKind::Module(module_decl) => {
            append_module_body_trace_events(&module_decl.body, events);
        }
        DeclKind::Macro(macro_decl) => {
            for param in &macro_decl.params {
                if let Some(default) = &param.default {
                    record_expr_trace_events(default, events);
                }
            }
            record_expr_trace_events(&macro_decl.body, events);
        }
        DeclKind::ActorSpec(actor_spec) => {
            for param in &actor_spec.params {
                if let Some(default) = &param.default {
                    record_expr_trace_events(default, events);
                }
            }
            record_expr_trace_events(&actor_spec.body, events);
        }
        DeclKind::Conductor(_)
        | DeclKind::Fn { .. }
        | DeclKind::Type { .. }
        | DeclKind::Struct(_)
        | DeclKind::Enum(_)
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
        ExprKind::FixityLiteral(_) => {}
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
        ExprKind::InlineAsm(asm) => {
            for output in &asm.outputs {
                record_expr_trace_events(&output.target, events);
            }
            for input in &asm.inputs {
                record_expr_trace_events(&input.expr, events);
            }
        }
        ExprKind::LlvmIr(ir) => {
            for input in &ir.inputs {
                record_expr_trace_events(input, events);
            }
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
        ExprKind::Unary { expr: inner, .. }
        | ExprKind::Rec { expr: inner }
        | ExprKind::Await { expr: inner } => record_expr_trace_events(inner, events),
        ExprKind::FieldAccess { target, .. }
        | ExprKind::TupleAccess { target, .. }
        | ExprKind::Propagate { expr: target }
        | ExprKind::Loop { body: target }
        | ExprKind::Unsafe { body: target }
        | ExprKind::Defer { body: target }
        | ExprKind::EffectBlock { body: target }
        | ExprKind::Async { body: target, .. } => {
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
        ExprKind::Break { value } => {
            if let Some(inner) = value {
                record_expr_trace_events(inner, events);
            }
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
        LiteralKind::Tuple { elements }
        | LiteralKind::Array { elements }
        | LiteralKind::Set { elements } => {
            for element in elements {
                record_expr_trace_events(element, events);
            }
        }
        LiteralKind::Record { fields, .. } => {
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
        ExprKind::FixityLiteral(_) => "fixity",
        ExprKind::Identifier(_) => "identifier",
        ExprKind::ModulePath(_) => "module-path",
        ExprKind::Call { .. } => "call",
        ExprKind::PerformCall { .. } => "perform",
        ExprKind::Lambda { .. } => "lambda",
        ExprKind::Pipe { .. } => "pipe",
        ExprKind::Binary { .. } => "binary",
        ExprKind::Unary { .. } => "unary",
        ExprKind::Rec { .. } => "rec",
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
        ExprKind::EffectBlock { .. } => "effect-block",
        ExprKind::Async { .. } => "async",
        ExprKind::Await { .. } => "await",
        ExprKind::Break { .. } => "break",
        ExprKind::Continue => "continue",
        ExprKind::Block { .. } => "block",
        ExprKind::Unsafe { .. } => "unsafe",
        ExprKind::Return { .. } => "return",
        ExprKind::Defer { .. } => "defer",
        ExprKind::Assign { .. } => "assign",
        ExprKind::InlineAsm(_) => "inline-asm",
        ExprKind::LlvmIr(_) => "llvm-ir",
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
    if matches!(tokens.get(next_idx), Some(token) if token.kind == TokenKind::LBrace) {
        return None;
    }
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

fn delimited_with_cut<P, O>(
    open: TokenKind,
    parser: P,
    close: TokenKind,
) -> impl ChumskyParser<TokenKind, O, Error = Simple<TokenKind>> + Clone
where
    P: ChumskyParser<TokenKind, O, Error = Simple<TokenKind>> + Clone,
{
    just(open)
        .ignore_then(parser.cut())
        .then_ignore(just(close).cut())
}

fn build_let_decl_parser<P, Q, R, S, T>(
    pattern_var: Q,
    lower_ident: S,
    params: T,
    type_parser: R,
    expr: P,
) -> impl ChumskyParser<TokenKind, Decl, Error = Simple<TokenKind>> + Clone
where
    P: ChumskyParser<TokenKind, Expr, Error = Simple<TokenKind>> + Clone,
    Q: ChumskyParser<TokenKind, Pattern, Error = Simple<TokenKind>> + Clone,
    R: ChumskyParser<TokenKind, TypeAnnot, Error = Simple<TokenKind>> + Clone,
    S: ChumskyParser<TokenKind, Ident, Error = Simple<TokenKind>> + Clone,
    T: ChumskyParser<TokenKind, Vec<Param>, Error = Simple<TokenKind>> + Clone,
{
    let let_fn_decl = just(TokenKind::KeywordLet)
        .ignore_then(lower_ident)
        .then(params)
        .then(
            just(TokenKind::Arrow)
                .ignore_then(type_parser.clone().cut())
                .or_not(),
        )
        .then_ignore(just(TokenKind::Assign))
        .then(expr.clone())
        .map_with_span(|(((name, params), ret_type), body), span: Range<usize>| {
            let lambda_span = Span::new(name.span.start, body.span().end);
            let lambda = Expr {
                span: lambda_span,
                kind: ExprKind::Lambda {
                    params,
                    ret_type,
                    body: Box::new(body),
                },
            };
            let pattern = Pattern {
                span: name.span,
                kind: PatternKind::Var(name),
            };
            Decl {
                attrs: Vec::new(),
                visibility: Visibility::Private,
                kind: DeclKind::Let {
                    pattern,
                    value: lambda,
                    type_annotation: None,
                },
                span: range_to_span(span),
            }
        });

    let let_decl = just(TokenKind::KeywordLet)
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
        });

    choice((let_fn_decl, let_decl))
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

fn build_stmt_parser<P, Q, R, S, T, U>(
    expr: P,
    pattern_var: Q,
    lower_ident: T,
    params: U,
    type_parser: R,
    assign_target: S,
) -> impl ChumskyParser<TokenKind, Stmt, Error = Simple<TokenKind>> + Clone
where
    P: ChumskyParser<TokenKind, Expr, Error = Simple<TokenKind>> + Clone,
    Q: ChumskyParser<TokenKind, Pattern, Error = Simple<TokenKind>> + Clone,
    R: ChumskyParser<TokenKind, TypeAnnot, Error = Simple<TokenKind>> + Clone,
    S: ChumskyParser<TokenKind, Expr, Error = Simple<TokenKind>> + Clone,
    T: ChumskyParser<TokenKind, Ident, Error = Simple<TokenKind>> + Clone,
    U: ChumskyParser<TokenKind, Vec<Param>, Error = Simple<TokenKind>> + Clone,
{
    let let_stmt_parser = build_let_decl_parser(
        pattern_var.clone(),
        lower_ident.clone(),
        params.clone(),
        type_parser.clone(),
        expr.clone(),
    );

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

    let assign_stmt = assign_target
        .clone()
        .then_ignore(just(TokenKind::ColonAssign).or(just(TokenKind::Assign)))
        .then(expr.clone().cut())
        .map_with_span(|(target, value), span: Range<usize>| Stmt {
            kind: StmtKind::Assign {
                target: Box::new(target),
                value: Box::new(value),
            },
            span: range_to_span(span),
        });

    let defer_stmt = just(TokenKind::KeywordDefer)
        .ignore_then(expr.clone().cut())
        .map_with_span(|expression, span: Range<usize>| Stmt {
            kind: StmtKind::Defer {
                expr: Box::new(expression),
            },
            span: range_to_span(span),
        });

    let expr_stmt = expr.map_with_span(|expression, span: Range<usize>| Stmt {
        kind: StmtKind::Expr {
            expr: Box::new(expression),
        },
        span: range_to_span(span),
    });

    choice((decl_stmt, defer_stmt, assign_stmt, expr_stmt))
}

fn build_attribute_parser<P, Q>(
    expr: P,
    ident: Q,
) -> impl ChumskyParser<TokenKind, Attribute, Error = Simple<TokenKind>> + Clone
where
    P: ChumskyParser<TokenKind, Expr, Error = Simple<TokenKind>> + Clone,
    Q: ChumskyParser<TokenKind, Ident, Error = Simple<TokenKind>> + Clone,
{
    let args = delimited_with_cut(
        TokenKind::LParen,
        expr.clone()
            .cut()
            .separated_by(just(TokenKind::Comma))
            .allow_trailing(),
        TokenKind::RParen,
    )
    .map_with_span(|values, span: Range<usize>| (values, Some(range_to_span(span))))
    .or_not();

    let at_attribute = just(TokenKind::At)
        .map_with_span(|_, span: Range<usize>| range_to_span(span))
        .then(ident.clone())
        .then(args.clone())
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
        });

    let hash_attribute = just(TokenKind::Hash)
        .ignore_then(just(TokenKind::LBracket))
        .ignore_then(ident)
        .then(args)
        .then_ignore(just(TokenKind::RBracket).cut())
        .map_with_span(|(name, args), span: Range<usize>| {
            let (args, _) = args.unwrap_or_else(|| (Vec::new(), None));
            Attribute {
                name,
                args,
                span: range_to_span(span),
            }
        });

    choice((hash_attribute, at_attribute))
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
                source: "fn pow(a, b, c) = a ^ b ^ c",
                expected_ast: Some("fn pow(a, b, c) = binary(var(a) ^ binary(var(b) ^ var(c)))"),
                expected_messages: &[],
            },
            Case {
                source: "fn mix(a, b, c) = a ^ b * c",
                expected_ast: Some("fn mix(a, b, c) = binary(binary(var(a) ^ var(b)) * var(c))"),
                expected_messages: &[],
            },
            Case {
                source: "fn logic(a, b, c) = a || b && c",
                expected_ast: Some("fn logic(a, b, c) = binary(var(a) || binary(var(b) && var(c)))"),
                expected_messages: &[],
            },
            Case {
                source: r#"effect ConsoleLog
fn emit(msg: String) = perform ConsoleLog(msg)
fn main() = emit("leak")"#,
                expected_ast: Some(
                    "effect ConsoleLog\nfn emit(msg: String) = perform ConsoleLog var(msg)\nfn main() = call(var(emit))[str(\"leak\")]",
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
            "ここで`!=`、`%`、`&&`、`(`、`)`、`*`、`+`、`,`、`-`、`.`、`..`、`/`、`:`、`<`、`<=`、`==`、`>`、`>=`、`?`、`^`、`|>` または `||`のいずれかが必要です"
        );
    }

    #[test]
    fn accepts_phase1_syntax_samples() {
        let source = r#"
const ConfigTriviaProfile::strict_json: ConfigTriviaProfile = todo
struct LexPack { profile: ConfigTriviaProfile, radix: 2|8|10|16 }
let sym(s) = s
fn demo() = {"fn", "let"}
"#;
        let result = ParserDriver::parse(source);
        assert!(
            result.diagnostics.is_empty(),
            "unexpected diagnostics: {:?}",
            result.diagnostics
        );
        assert!(result.value.is_some());
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
