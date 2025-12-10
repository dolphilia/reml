//! Core.Parse 関連モジュール。
//!
//! 現時点では OpBuilder DSL の構造表現のみを提供し、
//! 実行時に利用する優先度テーブルを Rust 側で構築できるようにする。

pub mod op_builder;

pub use op_builder::{
    FixitySymbol, OpBuilder, OpBuilderError, OpBuilderErrorKind, OpLevel, OpTable, OperatorSpec,
};
