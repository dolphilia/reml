//! Reml フロントエンドの字句解析器スケルトン。

use crate::error::{FrontendError, FrontendErrorKind, Recoverability};
use crate::span::Span;
use crate::token::{Token, TokenKind};

/// 字句解析に利用する入力ソースの抽象化。
pub trait SourceBuffer {
    fn as_bytes(&self) -> &[u8];
}

impl SourceBuffer for String {
    fn as_bytes(&self) -> &[u8] {
        String::as_bytes(self)
    }
}

impl<'a> SourceBuffer for &'a str {
    fn as_bytes(&self) -> &[u8] {
        str::as_bytes(self)
    }
}

/// 字句解析器の最小構造。
pub struct Lexer<'input> {
    input: &'input [u8],
    offset: u32,
}

impl<'input> Lexer<'input> {
    /// 新しい `Lexer` を生成する。
    pub fn new(source: &'input impl SourceBuffer) -> Self {
        Self {
            input: source.as_bytes(),
            offset: 0,
        }
    }

    /// 次のトークンを取得する。スケルトンでは ASCII 文字に限定した粗い判定を行う。
    pub fn next_token(&mut self) -> Result<Token, FrontendError> {
        while let Some(byte) = self.peek() {
            match byte {
                b' ' | b'\t' | b'\n' | b'\r' => {
                    self.consume();
                    continue;
                }
                b'a'..=b'z' | b'A'..=b'Z' | b'_' => {
                    let start = self.offset;
                    self.consume();
                    while matches!(self.peek(), Some(b'0'..=b'9' | b'a'..=b'z' | b'A'..=b'Z' | b'_')) {
                        self.consume();
                    }
                    let span = Span::new(start, self.offset);
                    return Ok(Token::new(TokenKind::Identifier, span));
                }
                b'0'..=b'9' => {
                    let start = self.offset;
                    self.consume();
                    while matches!(self.peek(), Some(b'0'..=b'9')) {
                        self.consume();
                    }
                    let span = Span::new(start, self.offset);
                    return Ok(Token::new(TokenKind::IntLiteral, span));
                }
                _ => {
                    let span = Span::new(self.offset, self.offset + 1);
                    self.consume();
                    return Err(FrontendError::new(
                        FrontendErrorKind::UnknownToken { span },
                        Recoverability::Recoverable,
                    ));
                }
            }
        }

        Ok(Token::new(
            TokenKind::EndOfFile,
            Span::new(self.offset, self.offset),
        ))
    }

    fn peek(&self) -> Option<u8> {
        self.input.get(self.offset as usize).copied()
    }

    fn consume(&mut self) {
        self.offset = self.offset.saturating_add(1);
    }
}
