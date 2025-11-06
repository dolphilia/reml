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
    let args = parse_args()?;
    let input_path = args.input.clone();
    let source = fs::read_to_string(&input_path)?;

    let result = ParserDriver::parse(&source);
    let diagnostics = result
        .diagnostics
        .iter()
        .map(DiagnosticJson::from)
        .collect::<Vec<_>>();

    let parse_result = serde_json::json!({
        "packrat_stats": result.packrat_stats,
        "span_trace": result.span_trace,
    });

    let stream_meta = serde_json::json!({
        "packrat": result.stream_metrics.packrat,
        "span_trace": result.stream_metrics.span_trace,
    });

    let payload = serde_json::json!({
        "input": input_path,
        "ast_render": result.ast_render(),
        "parse_result": parse_result.clone(),
        "stream_meta": stream_meta.clone(),
        "diagnostics": diagnostics,
        "tokens": result.tokens.iter().map(|token| serde_json::json!({
            "kind": format!("{:?}", token.kind),
            "span": token.span,
            "lexeme": token.lexeme,
        })).collect::<Vec<_>>(),
    });

    println!("{}", serde_json::to_string_pretty(&payload)?);

    if let Some(path) = args.parse_debug_output {
        let diagnostics_json = payload
            .get("diagnostics")
            .cloned()
            .unwrap_or_else(|| serde_json::Value::Array(vec![]));
        let parse_debug = serde_json::json!({
            "run_config": {
                "switches": {
                    "require_eof": true,
                    "packrat": true,
                    "left_recursion": "auto",
                    "trace": false,
                    "merge_warnings": true,
                    "legacy_result": false,
                },
                "extensions": {
                    "stream": {
                        "enabled": true,
                        "checkpoint": "poc_frontend",
                        "resume_hint": "n/a",
                        "chunk_size": 0,
                    }
                }
            },
            "input": input_path,
            "diagnostics": diagnostics_json,
            "parse_result": parse_result,
            "stream_meta": stream_meta,
        });
        fs::write(path, serde_json::to_string_pretty(&parse_debug)?)?;
    }

    Ok(())
}

struct CliArgs {
    input: PathBuf,
    parse_debug_output: Option<PathBuf>,
}

fn parse_args() -> Result<CliArgs, Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let mut input = None;
    let mut parse_debug = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--emit-parse-debug" => {
                let path = args
                    .next()
                    .ok_or_else(|| "--emit-parse-debug は出力パスを伴う必要があります")?;
                parse_debug = Some(PathBuf::from(path));
            }
            _ if arg.starts_with("--") => {
                return Err(format!("未知のオプション: {arg}").into());
            }
            _ => {
                if input.is_some() {
                    return Err("入力ファイルは 1 つのみ指定できます".into());
                }
                input = Some(PathBuf::from(arg));
            }
        }
    }

    let input = match input {
        Some(path) => path,
        None => {
            eprintln!("使用方法: poc_frontend [--emit-parse-debug <path>] <input.reml>");
            std::process::exit(1);
        }
    };

    Ok(CliArgs {
        input,
        parse_debug_output: parse_debug,
    })
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
