//! 診断メッセージのテンプレート郡。

pub mod config;
pub mod language;
pub mod pattern;

pub use config::{
    compatibility_unsupported, missing_manifest, schema_mismatch, ConfigDiagnosticMetadata,
};
pub use language::{find_language_message, language_messages, LanguageDiagnosticMessage};
pub use pattern::{find_pattern_message, pattern_messages, PatternDiagnosticMessage};

use crate::diagnostic::DiagnosticSeverity;

#[derive(Debug, Clone, Copy)]
pub struct DiagnosticMessageTemplate {
    pub code: &'static str,
    pub title: &'static str,
    pub message: &'static str,
    pub severity: DiagnosticSeverity,
}

impl DiagnosticMessageTemplate {
    fn from_pattern(message: &PatternDiagnosticMessage) -> Self {
        Self {
            code: message.code,
            title: message.title,
            message: message.message,
            severity: message.severity,
        }
    }

    fn from_language(message: &LanguageDiagnosticMessage) -> Self {
        Self {
            code: message.code,
            title: message.title,
            message: message.message,
            severity: message.severity,
        }
    }
}

pub fn find_message(code: &str) -> Option<DiagnosticMessageTemplate> {
    find_pattern_message(code)
        .map(DiagnosticMessageTemplate::from_pattern)
        .or_else(|| find_language_message(code).map(DiagnosticMessageTemplate::from_language))
}
