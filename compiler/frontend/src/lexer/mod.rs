//! Reml フロントエンドの字句解析器実装。
//!
//! 仕様 1-1 §A.3〜A.4（字句）に基づき、キーワード／演算子／
//! Unicode 識別子を Rust で再実装する。`IdentifierProfile` で ASCII 互換モードへ切り替え
//! できるようにし、RunConfig (`extensions.lex.identifier_profile`) から渡せる経路もここで確保する。

use logos::{Lexer as LogosLexer, Logos};
use reml_runtime::text::{self as unicode_text, LocaleId, Str as UnicodeStr, UnicodeErrorKind};
use std::str::FromStr;

use crate::error::{FrontendError, FrontendErrorKind, Recoverability};
use crate::span::Span;
use crate::token::{IntBase, LiteralMetadata, StringKind, Token, TokenKind};
use crate::unicode::UnicodeDetail;

/// 字句解析に利用する入力ソースの抽象化。
pub trait SourceBuffer {
    fn as_str(&self) -> &str;
}

impl SourceBuffer for String {
    fn as_str(&self) -> &str {
        String::as_str(self)
    }
}

impl<'a> SourceBuffer for &'a str {
    fn as_str(&self) -> &str {
        self
    }
}

/// Unicode 識別子プロファイルの切替。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentifierProfile {
    Unicode,
    AsciiCompat,
}

impl Default for IdentifierProfile {
    fn default() -> Self {
        IdentifierProfile::Unicode
    }
}

impl IdentifierProfile {
    pub fn as_str(&self) -> &'static str {
        match self {
            IdentifierProfile::Unicode => "unicode",
            IdentifierProfile::AsciiCompat => "ascii-compat",
        }
    }
}

impl FromStr for IdentifierProfile {
    type Err = ();

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input.trim().to_ascii_lowercase().as_str() {
            "unicode" => Ok(IdentifierProfile::Unicode),
            "ascii" | "ascii-compat" | "ascii_compat" => Ok(IdentifierProfile::AsciiCompat),
            _ => Err(()),
        }
    }
}

/// Lexer のオプション。
#[derive(Debug, Clone)]
pub struct LexerOptions {
    pub identifier_profile: IdentifierProfile,
    pub identifier_locale: Option<LocaleId>,
}

impl Default for LexerOptions {
    fn default() -> Self {
        Self {
            identifier_profile: IdentifierProfile::Unicode,
            identifier_locale: None,
        }
    }
}

/// `logos` が生成する内部トークン列挙。
#[derive(Debug, Clone, Copy, Logos, PartialEq, Eq)]
enum RawToken {
    #[regex(r"[ \t\r\n]+", logos::skip)]
    #[regex(r"//[^\n]*", logos::skip)]
    Skip,

    #[token("/*", lex_block_comment)]
    BlockComment,

    #[token("module")]
    KeywordModule,
    #[token("use")]
    KeywordUse,
    #[token("as")]
    KeywordAs,
    #[token("pub")]
    KeywordPub,
    #[token("self")]
    KeywordSelf,
    #[token("super")]
    KeywordSuper,
    #[token("let")]
    KeywordLet,
    #[token("var")]
    KeywordVar,
    #[token("const")]
    KeywordConst,
    #[token("mut")]
    KeywordMut,
    #[token("move")]
    KeywordMove,
    #[token("fn")]
    KeywordFn,
    #[token("type")]
    KeywordType,
    #[token("struct")]
    KeywordStruct,
    #[token("enum")]
    KeywordEnum,
    #[token("alias")]
    KeywordAlias,
    #[token("new")]
    KeywordNew,
    #[token("trait")]
    KeywordTrait,
    #[token("impl")]
    KeywordImpl,
    #[token("extern")]
    KeywordExtern,
    #[token("async")]
    KeywordAsync,
    #[token("await")]
    KeywordAwait,
    #[token("effect")]
    KeywordEffect,
    #[token("handler")]
    KeywordHandler,
    #[token("conductor")]
    KeywordConductor,
    #[token("actor")]
    KeywordActor,
    #[token("spec")]
    KeywordSpec,
    #[token("macro")]
    KeywordMacro,
    #[token("channels")]
    KeywordChannels,
    #[token("execution")]
    KeywordExecution,
    #[token("monitoring")]
    KeywordMonitoring,
    #[token("if")]
    KeywordIf,
    #[token("then")]
    KeywordThen,
    #[token("else")]
    KeywordElse,
    #[token("match")]
    KeywordMatch,
    #[token("when")]
    KeywordWhen,
    #[token("with")]
    KeywordWith,
    #[token("for")]
    KeywordFor,
    #[token("in")]
    KeywordIn,
    #[token("while")]
    KeywordWhile,
    #[token("loop")]
    KeywordLoop,
    #[token("return")]
    KeywordReturn,
    #[token("defer")]
    KeywordDefer,
    #[token("unsafe")]
    KeywordUnsafe,
    #[token("perform")]
    KeywordPerform,
    #[token("do")]
    KeywordDo,
    #[token("handle")]
    KeywordHandle,
    #[token("where")]
    KeywordWhere,
    #[token("true")]
    KeywordTrue,
    #[token("false")]
    KeywordFalse,
    #[token("break")]
    KeywordBreak,
    #[token("continue")]
    KeywordContinue,
    #[token("rec")]
    KeywordRec,

    #[token("|>")]
    PipeForward,
    #[token("~>")]
    ChannelPipe,
    #[token("...")]
    Ellipsis,
    #[token("..")]
    DotDot,
    #[token("=>")]
    DoubleArrow,
    #[token("->")]
    Arrow,
    #[token(":=")]
    ColonAssign,
    #[token("&&")]
    LogicalAnd,
    #[token("&")]
    Ampersand,
    #[token("||")]
    LogicalOr,
    #[token("==")]
    EqEq,
    #[token("!=")]
    NotEq,
    #[token("<=")]
    Le,
    #[token(">=")]
    Ge,

    #[token(".")]
    Dot,
    #[token(",")]
    Comma,
    #[token(";")]
    Semicolon,
    #[token(":prefix")]
    FixityPrefix,
    #[token(":postfix")]
    FixityPostfix,
    #[token(":infix_left")]
    FixityInfixLeft,
    #[token(":infix_right")]
    FixityInfixRight,
    #[token(":infix_nonassoc")]
    FixityInfixNonassoc,
    #[token(":ternary")]
    FixityTernary,
    #[token(":")]
    Colon,
    #[token("@")]
    At,
    #[token("#")]
    Hash,
    #[token("|")]
    Bar,
    #[token("=")]
    Assign,
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,
    #[token("%")]
    Percent,
    #[token("^")]
    Caret,
    #[token("<")]
    Lt,
    #[token(">")]
    Gt,
    #[token("!")]
    Not,
    #[token("?")]
    Question,
    #[token("_", priority = 4)]
    Underscore,

    #[regex(r"[0-9][0-9_]*\.[0-9_]+([eE][+-]?[0-9_]+)?", priority = 3)]
    #[regex(r"[0-9][0-9_]*[eE][+-]?[0-9_]+", priority = 2)]
    FloatLiteral,
    #[regex(r"0[bB][01_]+", priority = 4)]
    #[regex(r"0[oO][0-7_]+", priority = 4)]
    #[regex(r"0[xX][0-9a-fA-F_]+", priority = 4)]
    #[regex(r"[0-9][0-9_]*", priority = 2)]
    IntLiteral,

    #[token("\"\"\"", lex_multiline_string, priority = 2)]
    MultilineStringLiteral,
    #[regex(r#"r#*""#, lex_raw_string)]
    RawStringLiteral,
    #[token("\"", lex_string_literal)]
    StringLiteral,
    #[token("'", lex_char_literal)]
    CharLiteral,

    // 絵文字/Zwj と bidi 制御を字句で取り込み、識別子準備段階の診断に回す。
    #[regex(
        r"(?u)(?:_|\p{XID_Start})(?:\p{XID_Continue}|\u200D|\uFE0F|\p{Extended_Pictographic}|\p{Emoji_Component}|\u200E|\u200F|\u202A|\u202B|\u202C|\u202D|\u202E|\u2066|\u2067|\u2068|\u2069)*",
        priority = 1
    )]
    Identifier,
}

/// 字句解析結果。
pub struct LexOutput {
    pub tokens: Vec<Token>,
    pub errors: Vec<FrontendError>,
}

fn lex_block_comment(lex: &mut LogosLexer<RawToken>) -> Option<()> {
    let mut depth = 1usize;
    let mut offset = 0usize;
    let bytes = lex.remainder().as_bytes();
    while offset + 1 < bytes.len() {
        match (bytes[offset], bytes[offset + 1]) {
            (b'/', b'*') => {
                depth += 1;
                offset += 2;
            }
            (b'*', b'/') => {
                depth -= 1;
                offset += 2;
                if depth == 0 {
                    lex.bump(offset);
                    return Some(());
                }
            }
            _ => offset += 1,
        }
    }
    None
}

fn consume_skippable(src: &str) -> usize {
    let bytes = src.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        match bytes[i] {
            b' ' | b'\t' | b'\r' | b'\n' => i += 1,
            b'/' if i + 1 < bytes.len() && bytes[i + 1] == b'/' => {
                i += 2;
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
            }
            b'/' if i + 1 < bytes.len() && bytes[i + 1] == b'*' => {
                let mut depth = 1usize;
                i += 2;
                while i + 1 < bytes.len() {
                    match (bytes[i], bytes[i + 1]) {
                        (b'/', b'*') => {
                            depth += 1;
                            i += 2;
                        }
                        (b'*', b'/') => {
                            depth -= 1;
                            i += 2;
                            if depth == 0 {
                                break;
                            }
                        }
                        _ => i += 1,
                    }
                }
            }
            _ => break,
        }
    }
    i
}

fn lex_string_literal(lex: &mut LogosLexer<RawToken>) -> Option<()> {
    let mut escaped = false;
    for (idx, ch) in lex.remainder().char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match ch {
            '\\' => escaped = true,
            '"' => {
                lex.bump(idx + 1);
                return Some(());
            }
            '\n' | '\r' => return None,
            _ => {}
        }
    }
    None
}

fn lex_raw_string(lex: &mut LogosLexer<RawToken>) -> Option<()> {
    let prefix = lex.slice();
    let hash_count = prefix.chars().filter(|ch| *ch == '#').count();
    let remainder = lex.remainder().as_bytes();
    let mut offset = 0usize;
    while offset < remainder.len() {
        if remainder[offset] == b'"' {
            let mut matches_hash = true;
            for i in 0..hash_count {
                if remainder.get(offset + 1 + i) != Some(&b'#') {
                    matches_hash = false;
                    break;
                }
            }
            if matches_hash {
                lex.bump(offset + 1 + hash_count);
                return Some(());
            }
        }
        offset += 1;
    }
    None
}

fn lex_multiline_string(lex: &mut LogosLexer<RawToken>) -> Option<()> {
    let mut offset = 0usize;
    let remainder = lex.remainder().as_bytes();
    while offset + 2 < remainder.len() {
        if remainder[offset] == b'"'
            && remainder[offset + 1] == b'"'
            && remainder[offset + 2] == b'"'
        {
            lex.bump(offset + 3);
            return Some(());
        }
        offset += 1;
    }
    None
}

fn lex_char_literal(lex: &mut LogosLexer<RawToken>) -> Option<()> {
    let mut escaped = false;
    for (idx, ch) in lex.remainder().char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match ch {
            '\\' => escaped = true,
            '\'' => {
                lex.bump(idx + 1);
                return Some(());
            }
            '\n' | '\r' => return None,
            _ => {}
        }
    }
    None
}

/// `SourceBuffer` を解析し、`Token` の列を生成する。
pub fn lex_source(text: &str) -> LexOutput {
    lex_source_with_options(text, LexerOptions::default())
}

/// `SourceBuffer` を解析し、`Token` の列を生成する（プロファイル指定版）。
pub fn lex_source_with_options(text: &str, options: LexerOptions) -> LexOutput {
    let mut offset = 0usize;
    let mut tokens = Vec::new();
    let mut errors = Vec::new();

    while offset < text.len() {
        // logos の Skip 再帰を避けるため、空白/コメントは手動でまとめてスキップする。
        let skipped = consume_skippable(&text[offset..]);
        if skipped > 0 {
            offset += skipped;
            continue;
        }

        let mut lexer = RawToken::lexer(&text[offset..]);
        let result = match lexer.next() {
            Some(token) => token,
            None => break,
        };
        let range = lexer.span();
        let abs_range = (offset + range.start)..(offset + range.end);
        let span = Span::new(abs_range.start as u32, abs_range.end as u32);
        let consumed = range.end.max(1);

        match result {
            Ok(RawToken::KeywordModule) => {
                push_keyword(&mut tokens, span, TokenKind::KeywordModule, "module")
            }
            Ok(RawToken::KeywordUse) => {
                push_keyword(&mut tokens, span, TokenKind::KeywordUse, "use")
            }
            Ok(RawToken::KeywordAs) => push_keyword(&mut tokens, span, TokenKind::KeywordAs, "as"),
            Ok(RawToken::KeywordPub) => {
                push_keyword(&mut tokens, span, TokenKind::KeywordPub, "pub")
            }
            Ok(RawToken::KeywordSelf) => {
                push_keyword(&mut tokens, span, TokenKind::KeywordSelf, "self")
            }
            Ok(RawToken::KeywordSuper) => {
                push_keyword(&mut tokens, span, TokenKind::KeywordSuper, "super")
            }
            Ok(RawToken::KeywordLet) => {
                push_keyword(&mut tokens, span, TokenKind::KeywordLet, "let")
            }
            Ok(RawToken::KeywordVar) => {
                push_keyword(&mut tokens, span, TokenKind::KeywordVar, "var")
            }
            Ok(RawToken::KeywordConst) => {
                push_keyword(&mut tokens, span, TokenKind::KeywordConst, "const")
            }
            Ok(RawToken::KeywordMut) => {
                push_keyword(&mut tokens, span, TokenKind::KeywordMut, "mut")
            }
            Ok(RawToken::KeywordMove) => {
                push_keyword(&mut tokens, span, TokenKind::KeywordMove, "move")
            }
            Ok(RawToken::KeywordFn) => push_keyword(&mut tokens, span, TokenKind::KeywordFn, "fn"),
            Ok(RawToken::KeywordType) => {
                push_keyword(&mut tokens, span, TokenKind::KeywordType, "type")
            }
            Ok(RawToken::KeywordStruct) => {
                push_keyword(&mut tokens, span, TokenKind::KeywordStruct, "struct")
            }
            Ok(RawToken::KeywordEnum) => {
                push_keyword(&mut tokens, span, TokenKind::KeywordEnum, "enum")
            }
            Ok(RawToken::KeywordAlias) => {
                push_keyword(&mut tokens, span, TokenKind::KeywordAlias, "alias")
            }
            Ok(RawToken::KeywordNew) => {
                push_keyword(&mut tokens, span, TokenKind::KeywordNew, "new")
            }
            Ok(RawToken::KeywordTrait) => {
                push_keyword(&mut tokens, span, TokenKind::KeywordTrait, "trait")
            }
            Ok(RawToken::KeywordImpl) => {
                push_keyword(&mut tokens, span, TokenKind::KeywordImpl, "impl")
            }
            Ok(RawToken::KeywordExtern) => {
                push_keyword(&mut tokens, span, TokenKind::KeywordExtern, "extern")
            }
            Ok(RawToken::KeywordAsync) => {
                push_keyword(&mut tokens, span, TokenKind::KeywordAsync, "async")
            }
            Ok(RawToken::KeywordAwait) => {
                push_keyword(&mut tokens, span, TokenKind::KeywordAwait, "await")
            }
            Ok(RawToken::KeywordEffect) => {
                push_keyword(&mut tokens, span, TokenKind::KeywordEffect, "effect")
            }
            Ok(RawToken::KeywordHandler) => {
                push_keyword(&mut tokens, span, TokenKind::KeywordHandler, "handler")
            }
            Ok(RawToken::KeywordActor) => {
                push_keyword(&mut tokens, span, TokenKind::KeywordActor, "actor")
            }
            Ok(RawToken::KeywordSpec) => {
                push_keyword(&mut tokens, span, TokenKind::KeywordSpec, "spec")
            }
            Ok(RawToken::KeywordMacro) => {
                push_keyword(&mut tokens, span, TokenKind::KeywordMacro, "macro")
            }
            Ok(RawToken::KeywordConductor) => {
                push_keyword(&mut tokens, span, TokenKind::KeywordConductor, "conductor")
            }
            Ok(RawToken::KeywordChannels) => {
                push_keyword(&mut tokens, span, TokenKind::KeywordChannels, "channels")
            }
            Ok(RawToken::KeywordExecution) => {
                push_keyword(&mut tokens, span, TokenKind::KeywordExecution, "execution")
            }
            Ok(RawToken::KeywordMonitoring) => push_keyword(
                &mut tokens,
                span,
                TokenKind::KeywordMonitoring,
                "monitoring",
            ),
            Ok(RawToken::KeywordIf) => push_keyword(&mut tokens, span, TokenKind::KeywordIf, "if"),
            Ok(RawToken::KeywordThen) => {
                push_keyword(&mut tokens, span, TokenKind::KeywordThen, "then")
            }
            Ok(RawToken::KeywordElse) => {
                push_keyword(&mut tokens, span, TokenKind::KeywordElse, "else")
            }
            Ok(RawToken::KeywordMatch) => {
                push_keyword(&mut tokens, span, TokenKind::KeywordMatch, "match")
            }
            Ok(RawToken::KeywordWhen) => {
                push_keyword(&mut tokens, span, TokenKind::KeywordWhen, "when")
            }
            Ok(RawToken::KeywordWith) => {
                push_keyword(&mut tokens, span, TokenKind::KeywordWith, "with")
            }
            Ok(RawToken::KeywordFor) => {
                push_keyword(&mut tokens, span, TokenKind::KeywordFor, "for")
            }
            Ok(RawToken::KeywordIn) => push_keyword(&mut tokens, span, TokenKind::KeywordIn, "in"),
            Ok(RawToken::KeywordWhile) => {
                push_keyword(&mut tokens, span, TokenKind::KeywordWhile, "while")
            }
            Ok(RawToken::KeywordLoop) => {
                push_keyword(&mut tokens, span, TokenKind::KeywordLoop, "loop")
            }
            Ok(RawToken::KeywordReturn) => {
                push_keyword(&mut tokens, span, TokenKind::KeywordReturn, "return")
            }
            Ok(RawToken::KeywordDefer) => {
                push_keyword(&mut tokens, span, TokenKind::KeywordDefer, "defer")
            }
            Ok(RawToken::KeywordUnsafe) => {
                push_keyword(&mut tokens, span, TokenKind::KeywordUnsafe, "unsafe")
            }
            Ok(RawToken::KeywordPerform) => {
                push_keyword(&mut tokens, span, TokenKind::KeywordPerform, "perform")
            }
            Ok(RawToken::KeywordDo) => push_keyword(&mut tokens, span, TokenKind::KeywordDo, "do"),
            Ok(RawToken::KeywordHandle) => {
                push_keyword(&mut tokens, span, TokenKind::KeywordHandle, "handle")
            }
            Ok(RawToken::KeywordWhere) => {
                push_keyword(&mut tokens, span, TokenKind::KeywordWhere, "where")
            }
            Ok(RawToken::KeywordTrue) => {
                push_keyword(&mut tokens, span, TokenKind::KeywordTrue, "true")
            }
            Ok(RawToken::KeywordFalse) => {
                push_keyword(&mut tokens, span, TokenKind::KeywordFalse, "false")
            }
            Ok(RawToken::KeywordBreak) => {
                push_keyword(&mut tokens, span, TokenKind::KeywordBreak, "break")
            }
            Ok(RawToken::KeywordContinue) => {
                push_keyword(&mut tokens, span, TokenKind::KeywordContinue, "continue")
            }
            Ok(RawToken::KeywordRec) => {
                push_keyword(&mut tokens, span, TokenKind::KeywordRec, "rec")
            }
            Ok(RawToken::Identifier) => {
                let slice = lexer.slice();
                if options.identifier_profile == IdentifierProfile::AsciiCompat && !slice.is_ascii()
                {
                    push_ascii_error(span, slice, &mut errors, &mut tokens);
                    offset += consumed;
                    continue;
                }
                if slice == "_" {
                    tokens.push(Token::new(TokenKind::Underscore, span));
                    continue;
                }
                let first = slice.chars().next().unwrap_or('_');
                let kind = if is_upper_identifier_start(first) {
                    TokenKind::UpperIdentifier
                } else {
                    TokenKind::Identifier
                };
                match prepare_identifier_token(slice, span, &options) {
                    Ok(lexeme) => {
                        tokens.push(Token::with_lexeme(kind, span, lexeme));
                    }
                    Err(err) => {
                        errors.push(err);
                        tokens.push(Token::with_lexeme(TokenKind::Unknown, span, slice));
                    }
                }
            }
            Ok(RawToken::IntLiteral) => {
                let lexeme = lexer.slice();
                let base = detect_int_base(lexeme);
                tokens.push(Token::with_literal(
                    TokenKind::IntLiteral,
                    span,
                    lexeme,
                    LiteralMetadata::Int { base },
                ));
            }
            Ok(RawToken::FloatLiteral) => {
                tokens.push(Token::with_literal(
                    TokenKind::FloatLiteral,
                    span,
                    lexer.slice(),
                    LiteralMetadata::Float,
                ));
            }
            Ok(RawToken::StringLiteral) => tokens.push(Token::with_literal(
                TokenKind::StringLiteral,
                span,
                collect_string_lexeme(text, abs_range.clone()),
                LiteralMetadata::String {
                    kind: StringKind::Normal,
                },
            )),
            Ok(RawToken::RawStringLiteral) => tokens.push(Token::with_literal(
                TokenKind::StringLiteral,
                span,
                collect_string_lexeme(text, abs_range.clone()),
                LiteralMetadata::String {
                    kind: StringKind::Raw,
                },
            )),
            Ok(RawToken::MultilineStringLiteral) => tokens.push(Token::with_literal(
                TokenKind::StringLiteral,
                span,
                collect_string_lexeme(text, abs_range.clone()),
                LiteralMetadata::String {
                    kind: StringKind::Multiline,
                },
            )),
            Ok(RawToken::CharLiteral) => tokens.push(Token::with_literal(
                TokenKind::CharLiteral,
                span,
                collect_string_lexeme(text, abs_range.clone()),
                LiteralMetadata::Char,
            )),
            Ok(RawToken::PipeForward) => tokens.push(Token::new(TokenKind::PipeForward, span)),
            Ok(RawToken::ChannelPipe) => tokens.push(Token::new(TokenKind::ChannelPipe, span)),
            Ok(RawToken::Dot) => tokens.push(Token::new(TokenKind::Dot, span)),
            Ok(RawToken::Comma) => tokens.push(Token::new(TokenKind::Comma, span)),
            Ok(RawToken::Semicolon) => tokens.push(Token::new(TokenKind::Semicolon, span)),
            Ok(RawToken::FixityPrefix) => {
                push_keyword(&mut tokens, span, TokenKind::FixityPrefix, ":prefix")
            }
            Ok(RawToken::FixityPostfix) => {
                push_keyword(&mut tokens, span, TokenKind::FixityPostfix, ":postfix")
            }
            Ok(RawToken::FixityInfixLeft) => {
                push_keyword(&mut tokens, span, TokenKind::FixityInfixLeft, ":infix_left")
            }
            Ok(RawToken::FixityInfixRight) => push_keyword(
                &mut tokens,
                span,
                TokenKind::FixityInfixRight,
                ":infix_right",
            ),
            Ok(RawToken::FixityInfixNonassoc) => push_keyword(
                &mut tokens,
                span,
                TokenKind::FixityInfixNonassoc,
                ":infix_nonassoc",
            ),
            Ok(RawToken::FixityTernary) => {
                push_keyword(&mut tokens, span, TokenKind::FixityTernary, ":ternary")
            }
            Ok(RawToken::Colon) => tokens.push(Token::new(TokenKind::Colon, span)),
            Ok(RawToken::ColonAssign) => tokens.push(Token::new(TokenKind::ColonAssign, span)),
            Ok(RawToken::At) => tokens.push(Token::new(TokenKind::At, span)),
            Ok(RawToken::Hash) => tokens.push(Token::new(TokenKind::Hash, span)),
            Ok(RawToken::Bar) => tokens.push(Token::new(TokenKind::Bar, span)),
            Ok(RawToken::Assign) => tokens.push(Token::new(TokenKind::Assign, span)),
            Ok(RawToken::Arrow) => tokens.push(Token::new(TokenKind::Arrow, span)),
            Ok(RawToken::DoubleArrow) => tokens.push(Token::new(TokenKind::DoubleArrow, span)),
            Ok(RawToken::LParen) => tokens.push(Token::new(TokenKind::LParen, span)),
            Ok(RawToken::RParen) => tokens.push(Token::new(TokenKind::RParen, span)),
            Ok(RawToken::LBracket) => tokens.push(Token::new(TokenKind::LBracket, span)),
            Ok(RawToken::RBracket) => tokens.push(Token::new(TokenKind::RBracket, span)),
            Ok(RawToken::LBrace) => tokens.push(Token::new(TokenKind::LBrace, span)),
            Ok(RawToken::RBrace) => tokens.push(Token::new(TokenKind::RBrace, span)),
            Ok(RawToken::Plus) => tokens.push(Token::new(TokenKind::Plus, span)),
            Ok(RawToken::Minus) => tokens.push(Token::new(TokenKind::Minus, span)),
            Ok(RawToken::Star) => tokens.push(Token::new(TokenKind::Star, span)),
            Ok(RawToken::Slash) => tokens.push(Token::new(TokenKind::Slash, span)),
            Ok(RawToken::Percent) => tokens.push(Token::new(TokenKind::Percent, span)),
            Ok(RawToken::Caret) => tokens.push(Token::new(TokenKind::Caret, span)),
            Ok(RawToken::EqEq) => tokens.push(Token::new(TokenKind::EqEq, span)),
            Ok(RawToken::NotEq) => tokens.push(Token::new(TokenKind::NotEqual, span)),
            Ok(RawToken::Lt) => tokens.push(Token::new(TokenKind::Lt, span)),
            Ok(RawToken::Le) => tokens.push(Token::new(TokenKind::Le, span)),
            Ok(RawToken::Gt) => tokens.push(Token::new(TokenKind::Gt, span)),
            Ok(RawToken::Ge) => tokens.push(Token::new(TokenKind::Ge, span)),
            Ok(RawToken::LogicalAnd) => tokens.push(Token::new(TokenKind::LogicalAnd, span)),
            Ok(RawToken::Ampersand) => tokens.push(Token::new(TokenKind::Ampersand, span)),
            Ok(RawToken::LogicalOr) => tokens.push(Token::new(TokenKind::LogicalOr, span)),
            Ok(RawToken::Not) => tokens.push(Token::new(TokenKind::Not, span)),
            Ok(RawToken::Question) => tokens.push(Token::new(TokenKind::Question, span)),
            Ok(RawToken::Ellipsis) => tokens.push(Token::new(TokenKind::Ellipsis, span)),
            Ok(RawToken::DotDot) => tokens.push(Token::new(TokenKind::DotDot, span)),
            Ok(RawToken::Underscore) => tokens.push(Token::new(TokenKind::Underscore, span)),
            Ok(RawToken::BlockComment) => {}
            Ok(RawToken::Skip) => {}
            Err(_) => {
                errors.push(FrontendError::new(
                    FrontendErrorKind::UnknownToken { span },
                    Recoverability::Recoverable,
                ));
                tokens.push(Token::with_lexeme(TokenKind::Unknown, span, lexer.slice()));
            }
        }
        offset += consumed;
    }

    let eof_span = Span::new(text.len() as u32, text.len() as u32);
    tokens.push(Token::new(TokenKind::EndOfFile, eof_span));

    LexOutput { tokens, errors }
}

fn collect_string_lexeme(text: &str, range: std::ops::Range<usize>) -> String {
    text[range.start..range.end].to_string()
}

fn prepare_identifier_token(
    slice: &str,
    span: Span,
    options: &LexerOptions,
) -> Result<String, FrontendError> {
    let unicode = UnicodeStr::from(slice);
    let result = match options.identifier_locale.as_ref() {
        Some(locale) => unicode_text::prepare_identifier_with_locale(&unicode, Some(locale)),
        None => unicode_text::prepare_identifier(&unicode),
    };
    result.map(|text| text.into_std()).map_err(|err| {
        unicode_error_to_frontend(
            span,
            err,
            slice,
            options.identifier_locale.as_ref(),
            options.identifier_profile,
        )
    })
}

fn unicode_error_to_frontend(
    span: Span,
    err: unicode_text::UnicodeError,
    raw: &str,
    locale: Option<&LocaleId>,
    profile: IdentifierProfile,
) -> FrontendError {
    let mut detail = UnicodeDetail::from_error(&err)
        .with_phase("lex.identifier".to_string())
        .with_raw(raw.to_string())
        .with_profile(profile.as_str().to_string());
    if let Some(locale) = locale {
        detail = detail.with_locale(locale.canonical().to_string());
    }
    let mut message = match err.kind() {
        UnicodeErrorKind::InvalidIdentifier => {
            format!("Unicode 識別子の正規化に失敗しました: {}", err.message())
        }
        UnicodeErrorKind::UnsupportedLocale => {
            let requested = locale
                .map(|locale| locale.canonical().to_string())
                .unwrap_or_else(|| "und".to_string());
            format!(
                "lex.identifier_locale `{}` は未サポートです: {}",
                requested,
                err.message()
            )
        }
        other => format!(
            "Unicode 識別子処理で予期しないエラー ({other:?}): {}",
            err.message()
        ),
    };
    if let Some(offset) = err.offset() {
        message.push_str(&format!(" (offset {offset})"));
    }
    FrontendError::new(
        FrontendErrorKind::UnexpectedStructure {
            message,
            span: Some(span),
            unicode: Some(detail),
        },
        Recoverability::Recoverable,
    )
}

fn push_keyword(tokens: &mut Vec<Token>, span: Span, kind: TokenKind, text: &'static str) {
    tokens.push(Token::with_lexeme(kind, span, text));
}

fn detect_int_base(lexeme: &str) -> IntBase {
    if lexeme.len() >= 2 {
        let prefix = &lexeme[0..2];
        match prefix {
            "0b" | "0B" => return IntBase::Binary,
            "0o" | "0O" => return IntBase::Octal,
            "0x" | "0X" => return IntBase::Hexadecimal,
            _ => {}
        }
    }
    IntBase::Decimal
}

fn push_ascii_error(
    span: Span,
    lexeme: &str,
    errors: &mut Vec<FrontendError>,
    tokens: &mut Vec<Token>,
) {
    let invalid_code_point = lexeme
        .chars()
        .find(|ch| !ch.is_ascii())
        .unwrap_or('\u{FFFD}');
    let prefix = if lexeme
        .chars()
        .next()
        .map(|ch| !ch.is_ascii())
        .unwrap_or(false)
    {
        "識別子の先頭に使用できないコードポイント"
    } else {
        "識別子に使用できないコードポイント"
    };
    let message = format!(
        "{prefix} U+{:04X} (profile={})",
        invalid_code_point as u32,
        IdentifierProfile::AsciiCompat.as_str()
    );
    errors.push(FrontendError::new(
        FrontendErrorKind::UnexpectedStructure {
            message,
            span: Some(span),
            unicode: None,
        },
        Recoverability::Recoverable,
    ));
    tokens.push(Token::with_lexeme(TokenKind::Unknown, span, lexeme));
}

fn is_upper_identifier_start(ch: char) -> bool {
    if ch.is_ascii() {
        ch.is_ascii_uppercase()
    } else {
        ch.is_uppercase()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lex_handles_escaped_string_literal() {
        let source = "fn main() = emit(\"leak\")";
        let output = lex_source(source);
        let string_tokens: Vec<_> = output
            .tokens
            .iter()
            .filter(|token| token.kind == TokenKind::StringLiteral)
            .collect();
        assert!(
            output.errors.is_empty(),
            "lexer returned errors: {:?}",
            output
                .errors
                .iter()
                .map(|err| err.message())
                .collect::<Vec<_>>()
        );
        assert_eq!(
            string_tokens.len(),
            1,
            "expected exactly one string literal"
        );
    }

    #[test]
    fn lex_handles_hash_raw_string_literal() {
        let source = "r#\"foo\"# r##\"bar\"##";
        let output = lex_source(source);
        assert!(
            output.errors.is_empty(),
            "lexer returned errors: {:?}",
            output.errors
        );
        let string_tokens: Vec<_> = output
            .tokens
            .iter()
            .filter(|token| token.kind == TokenKind::StringLiteral)
            .collect();
        assert_eq!(string_tokens.len(), 2, "expected two string literals");
        let lexemes: Vec<&str> = string_tokens
            .iter()
            .map(|token| token.lexeme.as_deref().expect("lexeme should exist"))
            .collect();
        assert_eq!(lexemes, vec!["r#\"foo\"#", "r##\"bar\"##"]);
        for token in string_tokens {
            let literal = token.literal.as_ref().expect("literal metadata missing");
            match literal {
                LiteralMetadata::String { kind } => {
                    assert_eq!(*kind, StringKind::Raw);
                }
                other => panic!("unexpected literal metadata: {other:?}"),
            }
        }
    }
}

/// 字句解析器の薄いラッパー。
pub struct Lexer<'input> {
    source: &'input str,
}

impl<'input> Lexer<'input> {
    pub fn new(source: &'input impl SourceBuffer) -> Self {
        Self {
            source: source.as_str(),
        }
    }

    pub fn run(&self) -> LexOutput {
        lex_source(self.source)
    }

    pub fn run_with_options(&self, options: LexerOptions) -> LexOutput {
        lex_source_with_options(self.source, options)
    }
}
