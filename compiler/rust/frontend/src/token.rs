//! Reml ソースコードのトークン定義。

use crate::span::Span;

/// 字句解析で得られるトークン種別。
/// 実際の列挙子は W2 以降の AST 対応表と同期しながら拡張する。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TokenKind {
    Identifier,
    IntLiteral,
    FloatLiteral,
    StringLiteral,
    KeywordLet,
    KeywordFn,
    KeywordEffect,
    KeywordModule,
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Comma,
    Colon,
    Semi,
    Arrow,
    Assign,
    Operator,
    Comment,
    Whitespace,
    EndOfFile,
    /// 未知のトークン。診断で recover 可能な状態として扱う。
    Unknown,
}

/// `TokenKind` に付随するメタデータ。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }
}
