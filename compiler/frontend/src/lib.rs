//! Reml Rust フロントエンドの骨格モジュール。
//!
//! パーサ実行やストリーミング、期待集合の取り回しなどに必要な機能を
//! 段階的に整備するための雛形を提供する。

pub mod diagnostic;
pub mod effects;
pub mod error;
pub mod ffi_executor;
pub mod lexer;
pub mod output;
pub mod parser;
pub mod pipeline;
pub mod semantics;
pub mod span;
pub mod streaming;
pub mod token;
pub mod typeck;
pub mod unicode;

pub use error::{FrontendError, FrontendErrorKind, Recoverability};
pub use span::{Span, SpanTagged};
pub use token::{IntBase, LiteralMetadata, StringKind, Token, TokenKind};

/// フロントエンド共通で保持するソースファイル識別子。
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
