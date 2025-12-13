//! Core.Parse 関連モジュール。
//!
//! 現時点では OpBuilder DSL の構造表現とパーサーコンビネーターの足場を提供し、
//! 実行時に利用する優先度テーブルや Parser 型の基盤を Rust 側で構築できるようにする。

pub mod combinator;
pub mod op_builder;

pub use combinator::{
    run, run_with_default, Input, InputPosition, MemoEntry, MemoKey, MemoTable, ParseError,
    ParseResult, ParseState, Parser, ParserId, Reply, Span,
};
pub use op_builder::{
    FixitySymbol, OpBuilder, OpBuilderError, OpBuilderErrorKind, OpLevel, OpTable, OperatorSpec,
};
