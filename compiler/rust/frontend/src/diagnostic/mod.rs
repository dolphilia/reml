//! フロントエンドが出力する診断メッセージの骨格。

use crate::error::Recoverability;
use crate::span::Span;
use std::collections::BTreeMap;

pub mod recover;

pub use recover::{ExpectedToken, ExpectedTokenCollector, ExpectedTokensSummary};

pub(crate) const EXPECTED_PLACEHOLDER_TOKEN: &str = "解析継続トークン";
pub(crate) const EXPECTED_EMPTY_HUMANIZED: &str = "ここで解釈可能な構文が見つかりません";
pub(crate) const PARSE_EXPECTED_KEY: &str = "parse.expected";
pub(crate) const PARSE_EXPECTED_EMPTY_KEY: &str = "parse.expected.empty";

/// Rust フロントエンドが生成する診断レコードの最小構造。
/// W4 の診断互換試験に向け、`serde` スキーマと合わせて拡張する。
#[derive(Debug, Clone)]
pub struct FrontendDiagnostic {
    pub code: Option<String>,
    pub message: String,
    pub span: Option<Span>,
    pub recoverability: Recoverability,
    pub notes: Vec<DiagnosticNote>,
    pub expected_tokens: Vec<String>,
    pub expected_locale_args: Vec<String>,
    pub expected_humanized: Option<String>,
    pub expected_message_key: Option<String>,
}

impl FrontendDiagnostic {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            code: None,
            message: message.into(),
            span: None,
            recoverability: Recoverability::Fatal,
            notes: Vec::new(),
            expected_tokens: Vec::new(),
            expected_locale_args: Vec::new(),
            expected_humanized: None,
            expected_message_key: None,
        }
    }

    pub fn with_span(mut self, span: Span) -> Self {
        self.span = Some(span);
        self
    }

    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }

    pub fn with_recoverability(mut self, recoverability: Recoverability) -> Self {
        self.recoverability = recoverability;
        self
    }

    pub fn add_note(&mut self, note: DiagnosticNote) {
        self.notes.push(note);
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
        } else {
            self.expected_locale_args = tokens.clone();
            self.expected_tokens = tokens;
            self.expected_message_key = Some(PARSE_EXPECTED_KEY.to_string());
            self.expected_humanized = humanized;
        }
        self
    }

    pub fn apply_expected_summary(mut self, summary: &ExpectedTokensSummary) -> Self {
        if summary.has_alternatives() {
            self.expected_locale_args = summary.locale_args.clone();
            self.expected_tokens = summary.tokens();
            self.expected_message_key = summary
                .message_key
                .clone()
                .or_else(|| Some(PARSE_EXPECTED_KEY.to_string()));
            self.expected_humanized = summary.humanized.clone();
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
        }
        self
    }

    pub fn has_expected_tokens(&self) -> bool {
        !self.expected_tokens.is_empty()
    }

    /// Streaming Pending/Resume で recover が走らない場合でも、
    /// `ExpectedTokenCollector` による既定セットを診断へ埋め込む。
    pub fn ensure_streaming_expected(self) -> Self {
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
        self.apply_expected_summary(&summary)
    }
}

/// 診断の補足情報。
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

    pub fn push(&mut self, diagnostic: FrontendDiagnostic) {
        if self.merge_parse_expected {
            if let Some(key) = Self::parse_expected_key(&diagnostic) {
                if let Some(&index) = self.parse_expected_index.get(&key) {
                    self.diagnostics[index] = diagnostic;
                    return;
                }
                let index = self.diagnostics.len();
                self.diagnostics.push(diagnostic);
                self.parse_expected_index.insert(key, index);
                return;
            }
        }
        if let Some(key) = Self::parse_expected_key(&diagnostic) {
            if let Some(&index) = self.parse_expected_index.get(&key) {
                self.diagnostics[index] = diagnostic;
                return;
            }
            let index = self.diagnostics.len();
            self.diagnostics.push(diagnostic);
            self.parse_expected_index.insert(key, index);
        } else {
            self.diagnostics.push(diagnostic);
        }
    }

    pub fn extend<I>(&mut self, diagnostics: I)
    where
        I: IntoIterator<Item = FrontendDiagnostic>,
    {
        for diagnostic in diagnostics {
            self.push(diagnostic);
        }
    }

    pub fn into_vec(self) -> Vec<FrontendDiagnostic> {
        self.diagnostics
    }

    fn parse_expected_key(diagnostic: &FrontendDiagnostic) -> Option<(u32, u32)> {
        match diagnostic.expected_message_key.as_deref()? {
            key if key == PARSE_EXPECTED_KEY || key == PARSE_EXPECTED_EMPTY_KEY => {
                diagnostic.span.map(|span| (span.start, span.end))
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{DiagnosticBuilder, FrontendDiagnostic, PARSE_EXPECTED_KEY};
    use crate::span::Span;

    #[test]
    fn builder_merges_parse_expected_with_same_span() {
        let mut builder = DiagnosticBuilder::new();

        let mut first = FrontendDiagnostic::new("first").with_span(Span::new(10, 20));
        first.expected_message_key = Some(PARSE_EXPECTED_KEY.to_string());
        first.expected_tokens = vec!["fn".to_string()];
        builder.push(first);

        let mut second = FrontendDiagnostic::new("second").with_span(Span::new(10, 20));
        second.expected_message_key = Some(PARSE_EXPECTED_KEY.to_string());
        second.expected_tokens = vec!["let".to_string()];
        builder.push(second);

        let diags = builder.into_vec();
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].message, "second");
        assert_eq!(diags[0].expected_tokens, vec!["let".to_string()]);
    }

    #[test]
    fn builder_keeps_distinct_keys_or_spans() {
        let mut builder = DiagnosticBuilder::new();

        let mut expected = FrontendDiagnostic::new("expected").with_span(Span::new(0, 5));
        expected.expected_message_key = Some(PARSE_EXPECTED_KEY.to_string());
        builder.push(expected);

        builder.push(FrontendDiagnostic::new("other"));

        let mut different_span =
            FrontendDiagnostic::new("expected-other").with_span(Span::new(6, 10));
        different_span.expected_message_key = Some(PARSE_EXPECTED_KEY.to_string());
        builder.push(different_span);

        let diags = builder.into_vec();
        assert_eq!(diags.len(), 3);
    }

    #[test]
    fn ensure_streaming_expected_populates_tokens() {
        let diag = FrontendDiagnostic::new("streaming").ensure_streaming_expected();
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
