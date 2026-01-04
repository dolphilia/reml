//! フロントエンドが出力する診断メッセージの骨格。

use crate::error::Recoverability;
use crate::span::Span;
use crate::streaming::TraceFrame;
use crate::unicode::UnicodeDetail;
use serde_json::{Map, Value};
use std::borrow::Cow;
use std::collections::BTreeMap;
use thiserror::Error;
use uuid::Uuid;

use super::recover::{self, ExpectedToken, ExpectedTokenCollector, ExpectedTokensSummary};

pub(crate) const EXPECTED_PLACEHOLDER_TOKEN: &str = "解析継続トークン";
pub(crate) const EXPECTED_EMPTY_HUMANIZED: &str = "ここで解釈可能な構文が見つかりません";
pub(crate) const PARSE_EXPECTED_KEY: &str = "parse.expected";
pub(crate) const PARSE_EXPECTED_EMPTY_KEY: &str = "parse.expected.empty";

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone)]
pub struct AuditEnvelope {
    pub metadata: Map<String, Value>,
    pub audit_id: Option<Uuid>,
    pub change_set: Option<Value>,
    pub capability: Option<String>,
}

impl AuditEnvelope {
    pub fn new() -> Self {
        Self {
            metadata: Map::new(),
            audit_id: None,
            change_set: None,
            capability: None,
        }
    }

    pub fn from_parts(
        metadata: Map<String, Value>,
        audit_id: Option<Uuid>,
        change_set: Option<Value>,
        capability: Option<String>,
    ) -> Self {
        Self {
            metadata,
            audit_id,
            change_set,
            capability,
        }
    }
}

impl Default for AuditEnvelope {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Info,
    Hint,
}

impl DiagnosticSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            DiagnosticSeverity::Error => "error",
            DiagnosticSeverity::Warning => "warning",
            DiagnosticSeverity::Info => "info",
            DiagnosticSeverity::Hint => "hint",
        }
    }
}

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeverityHint {
    Rollback,
    Retry,
    Ignore,
    Escalate,
}

impl SeverityHint {
    pub fn as_str(&self) -> &'static str {
        match self {
            SeverityHint::Rollback => "rollback",
            SeverityHint::Retry => "retry",
            SeverityHint::Ignore => "ignore",
            SeverityHint::Escalate => "escalate",
        }
    }
}

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagnosticDomain {
    Effect,
    Target,
    Plugin,
    Lsp,
    Runtime,
    Parser,
    Type,
    Config,
    Network,
    Data,
    Audit,
    Security,
    Cli,
    Other(String),
}

impl DiagnosticDomain {
    pub fn label(&self) -> Cow<'static, str> {
        match self {
            DiagnosticDomain::Effect => Cow::Borrowed("effect"),
            DiagnosticDomain::Target => Cow::Borrowed("target"),
            DiagnosticDomain::Plugin => Cow::Borrowed("plugin"),
            DiagnosticDomain::Lsp => Cow::Borrowed("lsp"),
            DiagnosticDomain::Runtime => Cow::Borrowed("runtime"),
            DiagnosticDomain::Parser => Cow::Borrowed("parser"),
            DiagnosticDomain::Type => Cow::Borrowed("type"),
            DiagnosticDomain::Config => Cow::Borrowed("config"),
            DiagnosticDomain::Network => Cow::Borrowed("network"),
            DiagnosticDomain::Data => Cow::Borrowed("data"),
            DiagnosticDomain::Audit => Cow::Borrowed("audit"),
            DiagnosticDomain::Security => Cow::Borrowed("security"),
            DiagnosticDomain::Cli => Cow::Borrowed("cli"),
            DiagnosticDomain::Other(ref value) => Cow::Owned(value.clone()),
        }
    }
}

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone)]
pub struct DiagnosticSpanLabel {
    pub span: Option<Span>,
    pub message: Option<String>,
}

impl DiagnosticSpanLabel {
    pub fn new(span: Option<Span>, message: Option<String>) -> Self {
        Self { span, message }
    }
}

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone)]
pub struct DiagnosticHint {
    pub message: Option<String>,
    pub actions: Vec<DiagnosticFixIt>,
    pub id: Option<String>,
    pub title: Option<String>,
    pub kind: Option<String>,
    pub span: Option<Span>,
    pub payload: Option<Value>,
}

impl DiagnosticHint {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: Some(message.into()),
            actions: Vec::new(),
            id: None,
            title: None,
            kind: None,
            span: None,
            payload: None,
        }
    }

    pub fn with_actions(mut self, actions: Vec<DiagnosticFixIt>) -> Self {
        self.actions = actions;
        self
    }

    pub fn push_action(&mut self, action: DiagnosticFixIt) {
        self.actions.push(action);
    }

    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn with_kind(mut self, kind: impl Into<String>) -> Self {
        self.kind = Some(kind.into());
        self
    }

    pub fn with_span(mut self, span: Span) -> Self {
        self.span = Some(span);
        self
    }

    pub fn set_payload(&mut self, payload: Value) {
        self.payload = Some(payload);
    }
}

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone)]
pub enum DiagnosticFixIt {
    Insert { span: Span, text: String },
    Replace { span: Span, text: String },
    Delete { span: Span },
}

impl DiagnosticFixIt {
    pub fn insert(span: Span, text: impl Into<String>) -> Self {
        Self::Insert {
            span,
            text: text.into(),
        }
    }

    pub fn replace(span: Span, text: impl Into<String>) -> Self {
        Self::Replace {
            span,
            text: text.into(),
        }
    }

    pub fn delete(span: Span) -> Self {
        Self::Delete { span }
    }

    pub fn span(&self) -> Span {
        match self {
            DiagnosticFixIt::Insert { span, .. } => *span,
            DiagnosticFixIt::Replace { span, .. } => *span,
            DiagnosticFixIt::Delete { span } => *span,
        }
    }

    pub fn kind(&self) -> &'static str {
        match self {
            DiagnosticFixIt::Insert { .. } => "insert",
            DiagnosticFixIt::Replace { .. } => "replace",
            DiagnosticFixIt::Delete { .. } => "delete",
        }
    }

    pub fn text(&self) -> Option<&str> {
        match self {
            DiagnosticFixIt::Insert { text, .. } => Some(text),
            DiagnosticFixIt::Replace { text, .. } => Some(text),
            DiagnosticFixIt::Delete { .. } => None,
        }
    }
}

/// Rust フロントエンドが生成する診断レコードの最小構造。
/// W4 の診断互換試験に向け、`serde` スキーマと合わせて拡張する。
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone)]
pub struct FrontendDiagnostic {
    pub id: Option<Uuid>,
    pub code: Option<String>,
    pub codes: Vec<String>,
    pub message: String,
    pub timestamp: String,
    pub severity: Option<DiagnosticSeverity>,
    pub severity_hint: Option<SeverityHint>,
    pub domain: Option<DiagnosticDomain>,
    span: Span,
    has_primary_span: bool,
    pub span_trace: Vec<TraceFrame>,
    pub secondary_spans: Vec<DiagnosticSpanLabel>,
    pub recoverability: Recoverability,
    pub notes: Vec<DiagnosticNote>,
    pub hints: Vec<DiagnosticHint>,
    pub fixits: Vec<DiagnosticFixIt>,
    pub expected_tokens: Vec<String>,
    pub expected_locale_args: Vec<String>,
    pub expected_humanized: Option<String>,
    pub expected_message_key: Option<String>,
    pub expected_alternatives: Vec<ExpectedToken>,
    pub expected_summary: Option<ExpectedTokensSummary>,
    pub audit_metadata: Map<String, Value>,
    pub audit: AuditEnvelope,
    pub unicode: Option<UnicodeDetail>,
    pub extensions: Map<String, Value>,
}

impl FrontendDiagnostic {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            id: None,
            code: None,
            codes: Vec::new(),
            message: message.into(),
            severity: None,
            severity_hint: None,
            domain: None,
            span: Span::default(),
            has_primary_span: false,
            span_trace: Vec::new(),
            secondary_spans: Vec::new(),
            recoverability: Recoverability::Fatal,
            notes: Vec::new(),
            hints: Vec::new(),
            fixits: Vec::new(),
            expected_tokens: Vec::new(),
            expected_locale_args: Vec::new(),
            expected_humanized: None,
            expected_message_key: None,
            expected_alternatives: Vec::new(),
            expected_summary: None,
            timestamp: String::new(),
            audit_metadata: Map::new(),
            audit: AuditEnvelope::new(),
            unicode: None,
            extensions: Map::new(),
        }
    }

    pub fn with_id(mut self, id: Uuid) -> Self {
        self.id = Some(id);
        self
    }

    pub fn set_id(&mut self, id: Uuid) {
        self.id = Some(id);
    }

    pub fn ensure_id(&mut self) -> Uuid {
        if let Some(id) = self.id {
            id
        } else {
            let generated = Uuid::new_v4();
            self.id = Some(generated);
            generated
        }
    }

    pub fn with_span(mut self, span: Span) -> Self {
        self.set_span(span);
        self
    }

    pub fn set_span(&mut self, span: Span) {
        self.span = span;
        self.has_primary_span = true;
    }

    pub fn clear_primary_span(&mut self) {
        self.span = Span::default();
        self.has_primary_span = false;
    }

    pub fn has_primary_span(&self) -> bool {
        self.has_primary_span
    }

    pub fn primary_span(&self) -> Option<Span> {
        if self.has_primary_span {
            Some(self.span)
        } else {
            None
        }
    }

    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.push_code_value(code.into());
        self
    }

    pub fn push_code(&mut self, code: impl Into<String>) {
        self.push_code_value(code.into());
    }

    fn push_code_value(&mut self, code: String) {
        if self.code.is_none() {
            self.code = Some(code.clone());
        }
        if !self.codes.iter().any(|existing| existing == &code) {
            self.codes.push(code);
        }
    }

    pub fn with_severity(mut self, severity: DiagnosticSeverity) -> Self {
        self.severity = Some(severity);
        self
    }

    pub fn with_severity_hint(mut self, hint: SeverityHint) -> Self {
        self.severity_hint = Some(hint);
        self
    }

    pub fn set_severity(&mut self, severity: DiagnosticSeverity) {
        self.severity = Some(severity);
    }

    pub fn severity_or_default(&self) -> DiagnosticSeverity {
        self.severity.unwrap_or(DiagnosticSeverity::Error)
    }

    pub fn with_domain(mut self, domain: DiagnosticDomain) -> Self {
        self.domain = Some(domain);
        self
    }

    pub fn with_recoverability(mut self, recoverability: Recoverability) -> Self {
        self.recoverability = recoverability;
        self
    }

    pub fn add_note(&mut self, note: DiagnosticNote) {
        self.notes.push(note);
    }

    pub fn with_span_trace(mut self, trace: Vec<TraceFrame>) -> Self {
        self.span_trace = trace;
        self
    }

    pub fn set_span_trace(&mut self, trace: Vec<TraceFrame>) {
        self.span_trace = trace;
    }

    pub fn add_secondary_span(&mut self, span_label: DiagnosticSpanLabel) {
        self.secondary_spans.push(span_label);
    }

    pub fn with_secondary_span(mut self, span_label: DiagnosticSpanLabel) -> Self {
        self.add_secondary_span(span_label);
        self
    }

    pub fn add_hint(&mut self, hint: DiagnosticHint) {
        self.hints.push(hint);
    }

    pub fn add_fixit(&mut self, fixit: DiagnosticFixIt) {
        self.fixits.push(fixit);
    }

    pub fn set_unicode_detail(&mut self, detail: UnicodeDetail) {
        self.unicode = Some(detail);
    }

    pub fn with_unicode_detail(mut self, detail: UnicodeDetail) -> Self {
        self.set_unicode_detail(detail);
        self
    }

    pub fn extensions_mut(&mut self) -> &mut Map<String, Value> {
        &mut self.extensions
    }

    pub fn set_extensions(&mut self, extensions: Map<String, Value>) {
        self.extensions = extensions;
    }

    pub fn merge_extensions(&mut self, other: &Map<String, Value>) {
        for (key, value) in other {
            self.extensions.insert(key.clone(), value.clone());
        }
    }

    pub fn set_timestamp(&mut self, timestamp: impl Into<String>) {
        self.timestamp = timestamp.into();
    }

    pub fn set_expected_tokens(mut self, tokens: Vec<String>, humanized: Option<String>) -> Self {
        if tokens.is_empty() {
            self.expected_tokens = vec![EXPECTED_PLACEHOLDER_TOKEN.to_string()];
            self.expected_locale_args = Vec::new();
            self.expected_message_key = Some(PARSE_EXPECTED_EMPTY_KEY.to_string());
            self.expected_humanized = match humanized {
                Some(text) if !text.trim().is_empty() => Some(text),
                _ => Some(EXPECTED_EMPTY_HUMANIZED.to_string()),
            };
            self.expected_alternatives.clear();
        } else {
            self.expected_locale_args = tokens.clone();
            self.expected_tokens = tokens;
            self.expected_message_key = Some(PARSE_EXPECTED_KEY.to_string());
            self.expected_humanized = humanized;
            self.expected_alternatives = self
                .expected_tokens
                .iter()
                .cloned()
                .map(ExpectedToken::custom)
                .collect();
        }
        self.expected_summary = None;
        self
    }

    pub fn apply_expected_summary(mut self, summary: &ExpectedTokensSummary) -> Self {
        self.overwrite_expected_summary(summary);
        self
    }

    pub fn has_expected_tokens(&self) -> bool {
        !self.expected_tokens.is_empty()
    }

    pub fn expected_alternatives(&self) -> &[ExpectedToken] {
        &self.expected_alternatives
    }

    pub fn merge_expected_summary(&mut self, summary: &ExpectedTokensSummary) {
        if summary.alternatives.is_empty() {
            return;
        }
        let mut collector = ExpectedTokenCollector::with_capacity(
            self.expected_alternatives.len() + summary.alternatives.len(),
        );
        if !self.expected_alternatives.is_empty() {
            collector.extend(self.expected_alternatives.clone());
        } else {
            for token in &self.expected_tokens {
                collector.push(ExpectedToken::custom(token.clone()));
            }
        }
        collector.extend(summary.alternatives.clone());
        let merged = collector.summarize();
        self.overwrite_expected_summary(&merged);
    }

    fn overwrite_expected_summary(&mut self, summary: &ExpectedTokensSummary) {
        if summary.has_alternatives() {
            self.expected_locale_args = summary.locale_args.clone();
            self.expected_tokens = summary.tokens();
            self.expected_message_key = summary
                .message_key
                .clone()
                .or_else(|| Some(PARSE_EXPECTED_KEY.to_string()));
            self.expected_humanized = summary.humanized.clone();
            self.expected_alternatives = summary.alternatives.clone();
        } else {
            self.expected_tokens = vec![EXPECTED_PLACEHOLDER_TOKEN.to_string()];
            self.expected_locale_args.clear();
            self.expected_message_key = summary
                .message_key
                .clone()
                .or_else(|| Some(PARSE_EXPECTED_EMPTY_KEY.to_string()));
            self.expected_humanized = Some(
                summary
                    .humanized
                    .clone()
                    .unwrap_or_else(|| EXPECTED_EMPTY_HUMANIZED.to_string()),
            );
            self.expected_alternatives.clear();
        }
        self.expected_summary = Some(summary.clone());
    }

    /// Streaming Pending/Resume で recover が走らない場合でも、
    /// `ExpectedTokenCollector` による既定セットを診断へ埋め込む。
    pub fn ensure_streaming_expected(mut self) -> Self {
        let needs_override = self.expected_tokens.is_empty()
            || (self.expected_tokens.len() == 1
                && self.expected_tokens[0] == EXPECTED_PLACEHOLDER_TOKEN);
        if !needs_override {
            return self;
        }
        let summary = recover::streaming_expression_summary();
        debug_assert!(
            summary.has_alternatives(),
            "streaming_expression_summary should provide alternatives"
        );
        self.overwrite_expected_summary(&summary);
        self
    }

    /// Streaming recover では既存 `expected_tokens` があっても
    /// placeholder セットへ強制上書きする。
    pub fn force_streaming_expected(mut self) -> Self {
        let summary = recover::streaming_expression_summary();
        self.overwrite_expected_summary(&summary);
        self
    }
}

/// 診断の補足情報。
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone)]
pub struct DiagnosticNote {
    pub label: String,
    pub message: String,
    pub span: Option<Span>,
}

impl DiagnosticNote {
    pub fn new(label: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            message: message.into(),
            span: None,
        }
    }

    pub fn with_span(mut self, span: Span) -> Self {
        self.span = Some(span);
        self
    }
}

/// `parser_expectation` 相当の重複排除ルールを適用しながら診断を構築するビルダー。
#[derive(Debug)]
pub struct DiagnosticBuilder {
    diagnostics: Vec<FrontendDiagnostic>,
    parse_expected_index: BTreeMap<(u32, u32), usize>,
    merge_parse_expected: bool,
}

/// `docs/spec/3-6-core-diagnostics-audit.md` §1 の必須フィールド表に基づき、
/// Severity/Domain/Code の欠落を検知するためのエラー種別。
#[derive(Debug, Error, PartialEq, Eq)]
pub enum DiagnosticBuilderError {
    #[error("diagnostic.severity が未設定です（3-6-core-diagnostics-audit.md §1 参照）")]
    MissingSeverity,
    #[error("diagnostic.domain が未設定です（3-6-core-diagnostics-audit.md §1 参照）")]
    MissingDomain,
    #[error("diagnostic.code が未設定です（3-6-core-diagnostics-audit.md §1 参照）")]
    MissingCode,
}

impl DiagnosticBuilder {
    pub fn new() -> Self {
        Self::with_merge_parse_expected(true)
    }

    pub fn with_merge_parse_expected(merge: bool) -> Self {
        Self {
            diagnostics: Vec::new(),
            parse_expected_index: BTreeMap::new(),
            merge_parse_expected: merge,
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        let mut builder = Self::with_merge_parse_expected(true);
        builder.diagnostics = Vec::with_capacity(capacity);
        builder
    }

    pub fn push(&mut self, diagnostic: FrontendDiagnostic) -> Result<(), DiagnosticBuilderError> {
        self.push_internal(diagnostic, false).map(|_| ())
    }

    pub fn push_with_index(
        &mut self,
        diagnostic: FrontendDiagnostic,
    ) -> Result<usize, DiagnosticBuilderError> {
        self.push_internal(diagnostic, true)
            .map(|index| index.expect("push_with_index must return an index"))
    }

    fn push_internal(
        &mut self,
        diagnostic: FrontendDiagnostic,
        wants_index: bool,
    ) -> Result<Option<usize>, DiagnosticBuilderError> {
        let mut diagnostic = diagnostic;
        Self::ensure_fields(&diagnostic)?;
        diagnostic.ensure_id();
        if self.merge_parse_expected {
            if let Some(key) = Self::parse_expected_key(&diagnostic) {
                if let Some(&index) = self.parse_expected_index.get(&key) {
                    self.diagnostics[index] = diagnostic;
                    return Ok(if wants_index { Some(index) } else { None });
                }
                let index = self.diagnostics.len();
                self.diagnostics.push(diagnostic);
                self.parse_expected_index.insert(key, index);
                return Ok(if wants_index { Some(index) } else { None });
            }
        }
        if let Some(key) = Self::parse_expected_key(&diagnostic) {
            if let Some(&index) = self.parse_expected_index.get(&key) {
                self.diagnostics[index] = diagnostic;
                return Ok(if wants_index { Some(index) } else { None });
            }
            let index = self.diagnostics.len();
            self.diagnostics.push(diagnostic);
            self.parse_expected_index.insert(key, index);
            return Ok(if wants_index { Some(index) } else { None });
        }
        let index = self.diagnostics.len();
        self.diagnostics.push(diagnostic);
        Ok(if wants_index { Some(index) } else { None })
    }

    pub fn extend<I>(&mut self, diagnostics: I) -> Result<(), DiagnosticBuilderError>
    where
        I: IntoIterator<Item = FrontendDiagnostic>,
    {
        for diagnostic in diagnostics {
            self.push(diagnostic)?;
        }
        Ok(())
    }

    pub fn into_vec(self) -> Vec<FrontendDiagnostic> {
        self.diagnostics
    }

    pub fn merge_expected_summary_at(&mut self, index: usize, summary: &ExpectedTokensSummary) {
        if let Some(diagnostic) = self.diagnostics.get_mut(index) {
            diagnostic.merge_expected_summary(summary);
        }
    }

    fn parse_expected_key(diagnostic: &FrontendDiagnostic) -> Option<(u32, u32)> {
        match diagnostic.expected_message_key.as_deref()? {
            key if key == PARSE_EXPECTED_KEY || key == PARSE_EXPECTED_EMPTY_KEY => {
                diagnostic.primary_span().map(|span| (span.start, span.end))
            }
            _ => None,
        }
    }

    fn ensure_fields(diagnostic: &FrontendDiagnostic) -> Result<(), DiagnosticBuilderError> {
        if diagnostic.severity.is_none() {
            return Err(DiagnosticBuilderError::MissingSeverity);
        }
        if diagnostic.domain.is_none() {
            return Err(DiagnosticBuilderError::MissingDomain);
        }
        if diagnostic.code.is_none() && diagnostic.codes.is_empty() {
            return Err(DiagnosticBuilderError::MissingCode);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DiagnosticBuilder, DiagnosticDomain, DiagnosticSeverity, FrontendDiagnostic,
        PARSE_EXPECTED_KEY,
    };
    use crate::span::Span;

    fn parser_diag(label: &str) -> FrontendDiagnostic {
        FrontendDiagnostic::new(label)
            .with_severity(DiagnosticSeverity::Error)
            .with_domain(DiagnosticDomain::Parser)
            .with_code("parser.test")
    }

    #[test]
    fn builder_merges_parse_expected_with_same_span() {
        let mut builder = DiagnosticBuilder::new();

        let mut first = parser_diag("first").with_span(Span::new(10, 20));
        first.expected_message_key = Some(PARSE_EXPECTED_KEY.to_string());
        first.expected_tokens = vec!["fn".to_string()];
        builder.push(first).expect("first diagnostic");

        let mut second = parser_diag("second").with_span(Span::new(10, 20));
        second.expected_message_key = Some(PARSE_EXPECTED_KEY.to_string());
        second.expected_tokens = vec!["let".to_string()];
        builder.push(second).expect("second diagnostic");

        let diags = builder.into_vec();
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].message, "second");
        assert_eq!(diags[0].expected_tokens, vec!["let".to_string()]);
    }

    #[test]
    fn builder_keeps_distinct_keys_or_spans() {
        let mut builder = DiagnosticBuilder::new();

        let mut expected = parser_diag("expected").with_span(Span::new(0, 5));
        expected.expected_message_key = Some(PARSE_EXPECTED_KEY.to_string());
        builder.push(expected).expect("expected diagnostic");

        builder
            .push(parser_diag("other"))
            .expect("other diagnostic");

        let mut different_span = parser_diag("expected-other").with_span(Span::new(6, 10));
        different_span.expected_message_key = Some(PARSE_EXPECTED_KEY.to_string());
        builder.push(different_span).expect("different span");

        let diags = builder.into_vec();
        assert_eq!(diags.len(), 3);
    }

    #[test]
    fn ensure_streaming_expected_populates_tokens() {
        let diag = parser_diag("streaming").ensure_streaming_expected();
        assert!(
            !diag.expected_tokens.is_empty(),
            "streaming_expected should supply non-empty tokens"
        );
        assert_eq!(
            diag.expected_message_key.as_deref(),
            Some(PARSE_EXPECTED_KEY)
        );
    }
}
