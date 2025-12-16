//! 診断メッセージのテンプレート郡。

pub mod config;
pub mod pattern;

pub use config::{
    compatibility_unsupported, missing_manifest, schema_mismatch, ConfigDiagnosticMetadata,
};
pub use pattern::{find_pattern_message, pattern_messages, PatternDiagnosticMessage};
