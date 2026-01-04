//! Core.Parse 関連モジュール。
//!
//! 現時点では OpBuilder DSL の構造表現とパーサーコンビネーターの足場を提供し、
//! 実行時に利用する優先度テーブルや Parser 型の基盤を Rust 側で構築できるようにする。

pub mod combinator;
pub mod cst;
pub mod embedded;
pub mod meta;
pub mod op_builder;

pub use combinator::{
    between, chainl1, chainr1, choice, cut_here, delimited, embedded_dsl, eof, fail, keyword,
    label, layout_token, lexeme, lookahead, not_followed_by, ok, parse_errors_to_guard_diagnostics,
    parse_result_to_guard_diagnostics, position, preceded, rule, run, run_shared, run_with_cst,
    run_with_cst_shared, run_with_default, run_with_recovery, run_with_recovery_config, spanned,
    symbol, sync_to, terminated, token, with_doc, BinaryOp, ExprBuilderConfig, ExprCommit,
    ExprOpLevel, Input, InputPosition, MemoEntry, MemoKey, MemoTable, ParseError, ParseFixIt,
    ParseResult, ParseState, Parser, ParserId, ParserProfile, RecoverAction, RecoverMeta, Reply,
    Span, UnaryOp,
};
pub use cst::{CstBuilder, CstChild, CstNode, CstOutput, Token as CstToken, Trivia, TriviaKind};
pub use embedded::{
    ContextBridge, ContextBridgeHandler, EmbeddedBoundary, EmbeddedDslSpec, EmbeddedMode,
    EmbeddedNode,
};
pub use meta::{normalize_doc, ObservedToken, ParseMetaRegistry, ParserMeta, ParserMetaKind};
pub use op_builder::{
    FixitySymbol, OpBuilder, OpBuilderError, OpBuilderErrorKind, OpLevel, OpTable, OperatorSpec,
};
