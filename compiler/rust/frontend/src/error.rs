//! フロントエンドで発生するエラー種別の雛形。

use crate::{span::Span, unicode::UnicodeDetail};

/// エラーの復旧可否を示す分類。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Recoverability {
    /// 回復可能。診断を出力した上で解析継続を試みる。
    Recoverable,
    /// 回復不能。以降の解析は打ち切る。
    Fatal,
}

/// Rust フロントエンドにおけるエラーの種類。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrontendErrorKind {
    /// 字句解析フェーズでの未知トークン。
    UnknownToken { span: Span },
    /// パーサで期待したトークンが不足。
    MissingToken { expected: String, span: Span },
    /// 構文が不正で復旧不能。
    UnexpectedStructure {
        message: String,
        span: Option<Span>,
        unicode: Option<UnicodeDetail>,
    },
    /// Packrat キャッシュの内部矛盾など実装バグが疑われるケース。
    InternalState { message: String },
}

/// フロントエンド共通のエラー表現。
#[derive(Debug, Clone)]
pub struct FrontendError {
    pub kind: FrontendErrorKind,
    pub recoverability: Recoverability,
}

impl FrontendError {
    pub fn new(kind: FrontendErrorKind, recoverability: Recoverability) -> Self {
        Self {
            kind,
            recoverability,
        }
    }

    /// 診断向けにメッセージを抽出する。
    pub fn message(&self) -> String {
        match &self.kind {
            FrontendErrorKind::UnknownToken { .. } => "未定義のトークンを検出しました".to_string(),
            FrontendErrorKind::MissingToken { expected, .. } => {
                format!("`{expected}` が必要ですが別の入力が検出されました")
            }
            FrontendErrorKind::UnexpectedStructure { message, .. } => message.clone(),
            FrontendErrorKind::InternalState { message } => {
                format!("内部状態エラー: {message}")
            }
        }
    }
}
