//! Reml フロントエンドの字句解析器実装（W1 PoC）。

use logos::Logos;

use crate::error::{FrontendError, FrontendErrorKind, Recoverability};
use crate::span::Span;
use crate::token::{Token, TokenKind};

/// 字句解析に利用する入力ソースの抽象化。
pub trait SourceBuffer {
    fn as_str(&self) -> &str;
}

impl SourceBuffer for String {
    fn as_str(&self) -> &str {
        self.as_str()
    }
}

impl<'a> SourceBuffer for &'a str {
    fn as_str(&self) -> &str {
        self
    }
}

/// `logos` が生成する内部トークン列挙。
#[derive(Debug, Clone, Copy, Logos, PartialEq, Eq)]
enum RawToken {
    #[regex(r"[ \t\r\n]+", logos::skip)]
    #[regex(r"//[^\n]*", logos::skip)]
    #[error]
    Error,

    #[token("fn")]
    KeywordFn,
    #[token("let")]
    KeywordLet,
    #[token("module")]
    KeywordModule,
    #[token("effect")]
    KeywordEffect,

    #[regex(r"[A-Za-z_][A-Za-z0-9_]*")]
    Identifier,
    #[regex(r"[0-9]+")]
    IntLiteral,

    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,
    #[token(",")]
    Comma,
    #[token(":")]
    Colon,
    #[token(";")]
    Semi,
    #[token("->")]
    Arrow,
    #[token("=")]
    Assign,
    #[token("+")]
    Plus,
    #[regex(r"[^\s]", priority = 0)]
    Other,
}

/// 字句解析結果。
pub struct LexOutput {
    pub tokens: Vec<Token>,
    pub errors: Vec<FrontendError>,
}

/// `SourceBuffer` を `logos` で解析し、`Token` の列を生成する。
pub fn lex_source(text: &str) -> LexOutput {
    let mut lexer = RawToken::lexer(text);
    let mut tokens = Vec::new();
    let mut errors = Vec::new();

    while let Some(raw) = lexer.next() {
        let span = lexer.span();
        let span = Span::new(span.start as u32, span.end as u32);
        match raw {
            RawToken::KeywordFn => {
                tokens.push(Token::with_lexeme(TokenKind::KeywordFn, span, "fn"));
            }
            RawToken::KeywordLet => {
                tokens.push(Token::with_lexeme(TokenKind::KeywordLet, span, "let"));
            }
            RawToken::KeywordModule => {
                tokens.push(Token::with_lexeme(TokenKind::KeywordModule, span, "module"));
            }
            RawToken::KeywordEffect => {
                tokens.push(Token::with_lexeme(TokenKind::KeywordEffect, span, "effect"));
            }
            RawToken::Identifier => {
                tokens.push(Token::with_lexeme(
                    TokenKind::Identifier,
                    span,
                    lexer.slice(),
                ));
            }
            RawToken::IntLiteral => {
                tokens.push(Token::with_lexeme(
                    TokenKind::IntLiteral,
                    span,
                    lexer.slice(),
                ));
            }
            RawToken::LParen => tokens.push(Token::new(TokenKind::LParen, span)),
            RawToken::RParen => tokens.push(Token::new(TokenKind::RParen, span)),
            RawToken::LBrace => tokens.push(Token::new(TokenKind::LBrace, span)),
            RawToken::RBrace => tokens.push(Token::new(TokenKind::RBrace, span)),
            RawToken::LBracket => tokens.push(Token::new(TokenKind::LBracket, span)),
            RawToken::RBracket => tokens.push(Token::new(TokenKind::RBracket, span)),
            RawToken::Comma => tokens.push(Token::new(TokenKind::Comma, span)),
            RawToken::Colon => tokens.push(Token::new(TokenKind::Colon, span)),
            RawToken::Semi => tokens.push(Token::new(TokenKind::Semi, span)),
            RawToken::Arrow => tokens.push(Token::new(TokenKind::Arrow, span)),
            RawToken::Assign => tokens.push(Token::new(TokenKind::Assign, span)),
            RawToken::Plus => tokens.push(Token::with_lexeme(TokenKind::Operator, span, "+")),
            RawToken::Other | RawToken::Error => {
                errors.push(FrontendError::new(
                    FrontendErrorKind::UnknownToken { span },
                    Recoverability::Recoverable,
                ));
            }
        }
    }

    let eof_span = Span::new(text.len() as u32, text.len() as u32);
    tokens.push(Token::new(TokenKind::EndOfFile, eof_span));

    LexOutput { tokens, errors }
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
}
