//! logos × chumsky フロントエンド PoC。入力ファイルを解析し JSON を出力する。

use std::env;
use std::fs;
use std::path::PathBuf;

use reml_frontend::diagnostic::{DiagnosticNote, FrontendDiagnostic};
use reml_frontend::error::Recoverability;
use reml_frontend::parser::ParserDriver;
use reml_frontend::span::Span;
use serde::Serialize;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let input_path = match parse_args() {
        Some(path) => path,
        None => {
            eprintln!("使用方法: poc_frontend <input.reml>");
            std::process::exit(1);
        }
    };
    let source = fs::read_to_string(&input_path)?;

    let result = ParserDriver::parse(&source);
    let diagnostics = result
        .diagnostics
        .iter()
        .map(DiagnosticJson::from)
        .collect::<Vec<_>>();

    let payload = serde_json::json!({
        "input": input_path,
        "ast_render": result.ast_render(),
        "diagnostics": diagnostics,
        "tokens": result.tokens.iter().map(|token| serde_json::json!({
            "kind": format!("{:?}", token.kind),
            "span": token.span,
            "lexeme": token.lexeme,
        })).collect::<Vec<_>>(),
    });

    println!("{}", serde_json::to_string_pretty(&payload)?);

    Ok(())
}

fn parse_args() -> Option<PathBuf> {
    env::args().nth(1).map(PathBuf::from)
}

#[derive(Debug, Serialize)]
struct DiagnosticJson {
    message: String,
    code: Option<String>,
    recoverability: String,
    span: Option<Span>,
    notes: Vec<NoteJson>,
}

impl From<&FrontendDiagnostic> for DiagnosticJson {
    fn from(value: &FrontendDiagnostic) -> Self {
        Self {
            message: value.message.clone(),
            code: value.code.clone(),
            recoverability: recoverability_label(value.recoverability).to_string(),
            span: value.span,
            notes: value.notes.iter().map(NoteJson::from).collect(),
        }
    }
}

#[derive(Debug, Serialize)]
struct NoteJson {
    label: String,
    message: String,
    span: Option<Span>,
}

impl From<&DiagnosticNote> for NoteJson {
    fn from(value: &DiagnosticNote) -> Self {
        Self {
            label: value.label.clone(),
            message: value.message.clone(),
            span: value.span,
        }
    }
}

fn recoverability_label(value: Recoverability) -> &'static str {
    match value {
        Recoverability::Recoverable => "recoverable",
        Recoverability::Fatal => "fatal",
    }
}
