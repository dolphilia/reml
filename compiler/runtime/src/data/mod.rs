//! Core.Data の基盤モジュール。
//! 仕様 3.7 で定義されるスキーマ・データモデリング API を
//! Rust 実装から段階的に提供していく。

pub mod change_set;
pub mod schema;

pub use change_set::{ChangeEntry, ChangeOperation, ChangeSet, ChangeSeverity, ChangeSummary};
pub use schema::{
    Field, FieldAttribute, FieldBuilder, Schema, SchemaBuilder, SchemaDataType, SchemaDiff,
    SchemaVersion, ValidationRule, ValidationRuleBuilder, ValidationRuleKind,
    ValidationRuleSeverity,
};
