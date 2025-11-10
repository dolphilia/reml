//! フロントエンドが出力する診断メッセージの骨格。

use crate::error::Recoverability;
use crate::span::Span;

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
