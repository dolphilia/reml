//! Core.Parse 関連モジュール。
//!
//! 現時点では OpBuilder DSL の構造表現とパーサーコンビネーターの足場を提供し、
//! 実行時に利用する優先度テーブルや Parser 型の基盤を Rust 側で構築できるようにする。

pub mod combinator;
pub mod op_builder;

pub use combinator::{
    between, chainl1, chainr1, choice, cut_here, delimited, eof, fail, keyword, label, lexeme,
    lookahead, not_followed_by, ok, parse_errors_to_guard_diagnostics, parse_result_to_guard_diagnostics,
    position, preceded, rule, run, run_with_default, spanned, symbol, terminated, BinaryOp,
    ExprBuilderConfig, ExprCommit, ExprOpLevel, Input, InputPosition, MemoEntry, MemoKey,
    MemoTable, ParseError, ParseResult, ParseState, Parser, ParserId, Reply, Span, UnaryOp,
};
pub use op_builder::{
    FixitySymbol, OpBuilder, OpBuilderError, OpBuilderErrorKind, OpLevel, OpTable, OperatorSpec,
};
