//! 診断メッセージのテンプレート郡。

pub mod config;

pub use config::{
    compatibility_unsupported, missing_manifest, schema_mismatch, ConfigDiagnosticMetadata,
};
