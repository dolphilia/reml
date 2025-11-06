//! OCaml 版 `parser_driver` に相当するドライバの雛形。

use crate::diagnostic::{DiagnosticNote, FrontendDiagnostic};
use crate::error::{FrontendError, Recoverability};
use crate::lexer::Lexer;
use crate::token::{Token, TokenKind};

/// パース結果の簡易表現。W2 以降で AST/IR 構造に置き換える。
#[derive(Debug, Default)]
pub struct ParsedModule {
    pub tokens: Vec<Token>,
    pub diagnostics: Vec<FrontendDiagnostic>,
}

/// Rust フロントエンドのパーサドライバ。
pub struct ParserDriver;

impl ParserDriver {
    pub fn parse(source: &str) -> ParsedModule {
        let mut lexer = Lexer::new(&source);
        let mut module = ParsedModule::default();

        loop {
            match lexer.next_token() {
                Ok(token) if token.kind == TokenKind::EndOfFile => {
                    module.tokens.push(token);
                    break;
                }
                Ok(token) => module.tokens.push(token),
                Err(error) => {
                    let diagnostic = Self::error_to_diagnostic(error);
                    module.diagnostics.push(diagnostic);
                }
            }
        }

        module
    }

    fn error_to_diagnostic(error: FrontendError) -> FrontendDiagnostic {
        let mut diagnostic = FrontendDiagnostic::new(error.message());

        match error.recoverability {
            Recoverability::Recoverable => {
                diagnostic = diagnostic.with_recoverability(Recoverability::Recoverable);
            }
            Recoverability::Fatal => {}
        }

        match error.kind {
            crate::error::FrontendErrorKind::UnknownToken { span } => {
                diagnostic = diagnostic.with_span(span);
                diagnostic.add_note(
                    DiagnosticNote::new("lexer", "未定義のトークンをスキップします")
                        .with_span(span),
                );
            }
            crate::error::FrontendErrorKind::MissingToken { span, .. } => {
                diagnostic = diagnostic.with_span(span);
            }
            crate::error::FrontendErrorKind::UnexpectedStructure { span, .. } => {
                if let Some(span) = span {
                    diagnostic = diagnostic.with_span(span);
                }
            }
            crate::error::FrontendErrorKind::InternalState { .. } => {}
        }

        diagnostic
    }
}
