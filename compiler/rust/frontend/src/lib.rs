//! Reml Rust フロントエンドの骨格モジュール。
//!
//! OCaml 実装の `parser_driver`・`core_parse_streaming`・`parser_expectation`
//! 相当の機能を段階的に移植するための雛形を提供する。

pub mod diagnostic;
pub mod error;
pub mod lexer;
pub mod parser;
pub mod span;
pub mod streaming;
pub mod token;
pub mod typeck;

pub use error::{FrontendError, FrontendErrorKind, Recoverability};
pub use span::{Span, SpanTagged};
pub use token::{IntBase, LiteralMetadata, StringKind, Token, TokenKind};

/// フロントエンド共通で保持するソースファイル識別子。
/// OCaml 版の `Source_code.file_id` に対応する。
pub type SourceId = u32;

/// フロントエンド全体の初期化オプション。
/// W1 時点では構造のみを定義し、W2 以降の AST/IR 移植で拡張する。
#[derive(Debug, Default, Clone)]
pub struct FrontendConfig {
    /// Packrat キャッシュや span_trace を有効化するかどうか。
    pub enable_streaming_metrics: bool,
    /// 解析対象のソースファイル ID。
    pub source_id: Option<SourceId>,
}
