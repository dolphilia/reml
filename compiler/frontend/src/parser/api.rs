#![allow(dead_code)]
//! FRP Parser API を仕様文書 `docs/spec/2-1-parser-type.md` に沿って整理した共通型群。
//!
//! Phase 2-5 では以下の型を実装することで外部 API を整備し、
//! `Parser<T>` / `RunConfig` / `ParseResult` の共通利用を目指します。

use super::ParserTraceEvent;
use crate::diagnostic::{ExpectedToken, FrontendDiagnostic};
use crate::span::Span;
use crate::streaming::{
    PackratCacheEntry, PackratSnapshot, PackratStats, StreamFlowState, StreamMetrics, TraceFrame,
};
use crate::token::Token;
use crate::unicode::UnicodeDetail;
use indexmap::IndexMap;
use reml_runtime::config::ResolvedConfigCompatibility;
use serde_json::Value;
use std::str::FromStr;

/// 左再帰のモード。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeftRecursionMode {
    Off,
    On,
    Auto,
}

impl Default for LeftRecursionMode {
    fn default() -> Self {
        Self::Auto
    }
}

impl FromStr for LeftRecursionMode {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "off" => Ok(LeftRecursionMode::Off),
            "on" => Ok(LeftRecursionMode::On),
            "auto" => Ok(LeftRecursionMode::Auto),
            _ => Err(()),
        }
    }
}

/// ランナー向け実行設定。
#[derive(Debug, Clone)]
pub struct RunConfig {
    pub require_eof: bool,
    pub packrat: bool,
    pub left_recursion: LeftRecursionMode,
    pub trace: bool,
    pub merge_warnings: bool,
    pub allow_top_level_expr: bool,
    pub ack_experimental_diagnostics: bool,
    pub legacy_result: bool,
    pub locale: Option<String>,
    pub extensions: IndexMap<String, Value>,
    pub config_compat: Option<ResolvedConfigCompatibility>,
}

impl Default for RunConfig {
    fn default() -> Self {
        Self {
            require_eof: false,
            packrat: true,
            left_recursion: LeftRecursionMode::Auto,
            trace: true,
            merge_warnings: true,
            allow_top_level_expr: false,
            ack_experimental_diagnostics: false,
            legacy_result: false,
            locale: None,
            extensions: IndexMap::new(),
            config_compat: None,
        }
    }
}

impl RunConfig {
    /// 指定されたネームスペースをイミュータブルに更新し、新しい `RunConfig` を返す。
    pub fn with_extension(
        &self,
        key: impl Into<String>,
        update: impl FnOnce(Option<&Value>) -> Value,
    ) -> Self {
        let mut extensions = self.extensions.clone();
        let key_string = key.into();
        let updated = update(extensions.get(&key_string));
        extensions.insert(key_string, updated);
        let mut next = self.clone();
        next.extensions = extensions;
        next
    }

    /// 拡張データを上書きするユーティリティ。
    pub fn insert_extension(mut self, key: impl Into<String>, value: Value) -> Self {
        self.extensions.insert(key.into(), value);
        self
    }

    /// 指定されたネームスペースを取得する。
    pub fn extension(&self, key: &str) -> Option<&Value> {
        self.extensions.get(key)
    }

    /// 解析時に使用した互換プロファイルを保持する。
    pub fn set_config_compat(&mut self, resolved: ResolvedConfigCompatibility) {
        self.config_compat = Some(resolved);
    }

    pub fn config_compat(&self) -> Option<&ResolvedConfigCompatibility> {
        self.config_compat.as_ref()
    }
}

/// パーサ入力ビュー（仕様上の `Input`）。
#[derive(Debug, Clone)]
pub struct Input<'src> {
    pub source: &'src str,
    pub offset: usize,
}

impl<'src> Input<'src> {
    pub fn new(source: &'src str) -> Self {
        Self { source, offset: 0 }
    }

    pub fn rest(&self) -> &'src str {
        &self.source[self.offset..]
    }

    pub fn advance(&self, bytes: usize) -> Self {
        Self {
            source: self.source,
            offset: self.offset.saturating_add(bytes),
        }
    }
}

/// パーサ状態（補助）。
#[derive(Debug)]
pub struct State<'src> {
    pub input: Input<'src>,
    pub config: RunConfig,
    pub diagnostics: Vec<FrontendDiagnostic>,
    pub trace: Vec<TraceFrame>,
    pub memo: (),
}

/// 外部から呼び出すパーサ関数。
pub type Parser<'src, T> = fn(&mut State<'src>) -> Reply<T>;

/// 最小限の失敗表現。
#[derive(Debug, Clone)]
pub struct ParseError {
    pub at: Span,
    pub expected: Vec<ExpectedToken>,
    pub context: Vec<String>,
    pub committed: bool,
    pub notes: Vec<String>,
    pub unicode: Option<UnicodeDetail>,
    pub span_trace: Vec<Span>,
}

impl ParseError {
    pub fn new(at: Span, expected: Vec<ExpectedToken>) -> Self {
        Self {
            at,
            expected,
            context: Vec::new(),
            committed: false,
            notes: Vec::new(),
            unicode: None,
            span_trace: Vec::new(),
        }
    }
}

impl<'src> State<'src> {
    /// 指定されたソースと設定から状態を構築する。
    pub fn new(source: &'src str, config: RunConfig) -> Self {
        Self {
            input: Input::new(source),
            config,
            diagnostics: Vec::new(),
            trace: Vec::new(),
            memo: (),
        }
    }

    /// 残り入力を末尾まで進める。
    pub fn consume_to_end(&mut self) {
        let len = self.input.source.len();
        self.input = self.input.advance(len);
    }

    /// 診断情報を更新する。
    pub fn record_diagnostics(&mut self, diagnostics: Vec<FrontendDiagnostic>) {
        self.diagnostics = diagnostics;
    }

    /// スパン トレースを書き込む。
    pub fn record_span_trace(&mut self, trace: Vec<TraceFrame>) {
        self.trace = trace;
    }
}

/// `Parser<T>` の実行結果。仕様上の `Reply` を模倣。
#[derive(Debug)]
pub enum Reply<T> {
    Ok {
        value: T,
        span: Span,
        consumed: bool,
    },
    Err {
        error: ParseError,
        consumed: bool,
        committed: bool,
    },
}

/// パーサランナーが CLI/LSP へ返す結果。
#[derive(Debug, Clone)]
pub struct ParseResult<T> {
    pub value: Option<T>,
    pub span: Option<Span>,
    pub diagnostics: Vec<FrontendDiagnostic>,
    pub recovered: bool,
    pub legacy_error: Option<ParseError>,
    pub farthest_error_offset: Option<u32>,
    pub packrat_cache: Option<Vec<PackratCacheEntry>>,
    pub tokens: Vec<Token>,
    pub packrat_stats: PackratStats,
    pub packrat_snapshot: PackratSnapshot,
    pub stream_metrics: StreamMetrics,
    pub span_trace: Vec<TraceFrame>,
    pub stream_flow_state: Option<StreamFlowState>,
    pub run_config: RunConfig,
    pub trace_events: Vec<ParserTraceEvent>,
}

impl<T> ParseResult<T> {
    pub fn new(
        value: Option<T>,
        span: Option<Span>,
        diagnostics: Vec<FrontendDiagnostic>,
        recovered: bool,
        legacy_error: Option<ParseError>,
        farthest_error_offset: Option<u32>,
        packrat_cache: Option<Vec<PackratCacheEntry>>,
        tokens: Vec<Token>,
        packrat_stats: PackratStats,
        packrat_snapshot: PackratSnapshot,
        stream_metrics: StreamMetrics,
        span_trace: Vec<TraceFrame>,
        stream_flow_state: Option<StreamFlowState>,
        run_config: RunConfig,
        trace_events: Vec<ParserTraceEvent>,
    ) -> Self {
        Self {
            value,
            span,
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
        }
    }
}

/// 残り入力を含む部分パース結果。
#[derive(Debug, Clone)]
pub struct ParseResultWithRest<T> {
    pub result: ParseResult<T>,
    pub rest: Option<String>,
}
