//! Reml ソースコードのトークン定義。

use crate::span::Span;
use serde::Serialize;

/// 整数リテラルの基数。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub enum IntBase {
    Binary,
    Octal,
    Decimal,
    Hexadecimal,
}

/// 文字列リテラルの種別。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub enum StringKind {
    /// 通常のダブルクォート文字列（C系のエスケープシーケンスを解釈）。
    Normal,
    /// `r"..."` 形式の Raw 文字列。
    Raw,
    /// `""" ... """` で囲む複数行文字列。
    Multiline,
}

/// リテラルに付随する追加情報。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub enum LiteralMetadata {
    Int { base: IntBase },
    Float,
    Char,
    String { kind: StringKind },
}

/// 字句解析で得られるトークン種別。
/// 仕様 1-1 §A.3〜A.4 に合わせて定義する。
#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub enum TokenKind {
    Identifier,
    UpperIdentifier,
    IntLiteral,
    FloatLiteral,
    CharLiteral,
    StringLiteral,

    KeywordModule,
    KeywordUse,
    KeywordAs,
    KeywordPub,
    KeywordSelf,
    KeywordSuper,
    KeywordLet,
    KeywordVar,
    KeywordConst,
    KeywordMut,
    KeywordMove,
    KeywordFn,
    KeywordType,
    KeywordStruct,
    KeywordEnum,
    KeywordAlias,
    KeywordNew,
    KeywordTrait,
    KeywordImpl,
    KeywordExtern,
    KeywordAsync,
    KeywordAwait,
    KeywordEffect,
    KeywordOperation,
    KeywordHandler,
    KeywordPattern,
    KeywordActor,
    KeywordSpec,
    KeywordMacro,
    KeywordConductor,
    KeywordChannels,
    KeywordExecution,
    KeywordMonitoring,
    KeywordIf,
    KeywordThen,
    KeywordElse,
    KeywordMatch,
    KeywordWhen,
    KeywordWith,
    KeywordFor,
    KeywordIn,
    KeywordWhile,
    KeywordLoop,
    KeywordReturn,
    KeywordDefer,
    KeywordUnsafe,
    KeywordPerform,
    KeywordDo,
    KeywordHandle,
    KeywordWhere,
    KeywordTrue,
    KeywordFalse,
    KeywordBreak,
    KeywordContinue,
    KeywordRec,
    FixityPrefix,
    FixityPostfix,
    FixityInfixLeft,
    FixityInfixRight,
    FixityInfixNonassoc,
    FixityTernary,

    PipeForward,
    ChannelPipe,
    Dot,
    Comma,
    Semicolon,
    Colon,
    At,
    Hash,
    Bar,
    Ampersand,
    Assign,
    ColonAssign,
    Arrow,
    DoubleArrow,
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Caret,
    EqEq,
    NotEqual,
    Lt,
    Le,
    Gt,
    Ge,
    LogicalAnd,
    LogicalOr,
    Not,
    Question,
    Ellipsis,
    DotDot,
    Underscore,

    Comment,
    Whitespace,
    EndOfFile,
    /// 未定義のトークン。診断で recover 可能な状態として扱う。
    Unknown,
}

impl TokenKind {
    /// キーワードの場合は対応する文字列表現を返す。
    pub fn keyword_literal(&self) -> Option<&'static str> {
        use TokenKind::*;
        let keyword = match self {
            KeywordModule => "module",
            KeywordUse => "use",
            KeywordAs => "as",
            KeywordPub => "pub",
            KeywordSelf => "self",
            KeywordSuper => "super",
            KeywordLet => "let",
            KeywordVar => "var",
            KeywordConst => "const",
            KeywordMut => "mut",
            KeywordMove => "move",
            KeywordFn => "fn",
            KeywordType => "type",
            KeywordStruct => "struct",
            KeywordEnum => "enum",
            KeywordAlias => "alias",
            KeywordNew => "new",
            KeywordTrait => "trait",
            KeywordImpl => "impl",
            KeywordExtern => "extern",
            KeywordAsync => "async",
            KeywordAwait => "await",
            KeywordEffect => "effect",
            KeywordOperation => "operation",
            KeywordHandler => "handler",
            KeywordPattern => "pattern",
            KeywordActor => "actor",
            KeywordSpec => "spec",
            KeywordMacro => "macro",
            KeywordConductor => "conductor",
            KeywordChannels => "channels",
            KeywordExecution => "execution",
            KeywordMonitoring => "monitoring",
            KeywordIf => "if",
            KeywordThen => "then",
            KeywordElse => "else",
            KeywordMatch => "match",
            KeywordWhen => "when",
            KeywordWith => "with",
            KeywordFor => "for",
            KeywordIn => "in",
            KeywordWhile => "while",
            KeywordLoop => "loop",
            KeywordReturn => "return",
            KeywordDefer => "defer",
            KeywordUnsafe => "unsafe",
            KeywordPerform => "perform",
            KeywordDo => "do",
            KeywordHandle => "handle",
            KeywordWhere => "where",
            KeywordTrue => "true",
            KeywordFalse => "false",
            KeywordBreak => "break",
            KeywordContinue => "continue",
            KeywordRec => "rec",
            FixityPrefix => ":prefix",
            FixityPostfix => ":postfix",
            FixityInfixLeft => ":infix_left",
            FixityInfixRight => ":infix_right",
            FixityInfixNonassoc => ":infix_nonassoc",
            FixityTernary => ":ternary",
            _ => return None,
        };
        Some(keyword)
    }
}

/// `TokenKind` に付随するメタデータ。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lexeme: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub literal: Option<LiteralMetadata>,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Self {
            kind,
            span,
            lexeme: None,
            literal: None,
        }
    }

    pub fn with_lexeme(kind: TokenKind, span: Span, lexeme: impl Into<String>) -> Self {
        Self {
            kind,
            span,
            lexeme: Some(lexeme.into()),
            literal: None,
        }
    }

    pub fn with_literal(
        kind: TokenKind,
        span: Span,
        lexeme: impl Into<String>,
        literal: LiteralMetadata,
    ) -> Self {
        Self {
            kind,
            span,
            lexeme: Some(lexeme.into()),
            literal: Some(literal),
        }
    }
}
