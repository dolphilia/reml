//! Core.Lsp 仕様の最小実装。
//! LSP 型と JSON-RPC 用の最小ヘルパを提供する。

use serde::Serialize;
use std::collections::BTreeMap;

use crate::prelude::ensure::DiagnosticSeverity as CoreSeverity;
use crate::prelude::ensure::GuardDiagnostic;

pub mod derive;
pub mod embedded;

pub use embedded::{EmbeddedLspRegistry, EmbeddedLspRoute};

/// 0-based の位置情報。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Position {
    pub line: i64,
    pub character: i64,
}

/// 位置情報の範囲。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

/// LSP 機能フラグ。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct LspCapabilities {
    pub completion: bool,
    pub outline: bool,
    pub semantic_tokens: bool,
    pub hover: bool,
}

/// 最小 LSP サーバー表現。
#[derive(Debug, Clone)]
pub struct LspServer {
    capabilities: LspCapabilities,
}

impl LspServer {
    pub fn new() -> Self {
        Self {
            capabilities: LspCapabilities::default(),
        }
    }

    pub fn with_capabilities(mut self, capabilities: LspCapabilities) -> Self {
        self.capabilities = capabilities;
        self
    }

    pub fn capabilities(&self) -> &LspCapabilities {
        &self.capabilities
    }
}

/// LSP 診断の Severity。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Information,
    Hint,
}

/// LSP 診断。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LspDiagnostic {
    pub range: Range,
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub code: Option<String>,
}

/// JSON-RPC メッセージ。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonRpcMessage {
    pub method: String,
    pub params: BTreeMap<String, String>,
}

/// LSP エラー。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LspError {
    pub kind: LspErrorKind,
    pub message: String,
}

/// LSP エラー種別。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LspErrorKind {
    DecodeFailed,
    UnsupportedMethod,
}

impl LspError {
    pub fn new(kind: LspErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

/// 位置情報を生成する。
pub fn position(line: i64, character: i64) -> Position {
    Position {
        line: line.max(0),
        character: character.max(0),
    }
}

/// 位置範囲を生成する。
pub fn range(start_line: i64, start_char: i64, end_line: i64, end_char: i64) -> Range {
    Range {
        start: position(start_line, start_char),
        end: position(end_line, end_char),
    }
}

/// 診断を生成する。
pub fn diagnostic(
    range: Range,
    severity: DiagnosticSeverity,
    message: impl Into<String>,
) -> LspDiagnostic {
    LspDiagnostic {
        range,
        severity,
        message: message.into(),
        code: None,
    }
}

/// Core.Diagnostics の診断を LSP 形式へ変換する。
pub fn to_lsp(range: Range, diagnostic: &GuardDiagnostic) -> LspDiagnostic {
    LspDiagnostic {
        range,
        severity: match diagnostic.severity {
            CoreSeverity::Error => DiagnosticSeverity::Error,
            CoreSeverity::Warning => DiagnosticSeverity::Warning,
            CoreSeverity::Info => DiagnosticSeverity::Information,
            CoreSeverity::Hint => DiagnosticSeverity::Hint,
        },
        message: diagnostic.message.clone(),
        code: Some(diagnostic.code.to_string()),
    }
}

/// `textDocument/publishDiagnostics` をエンコードする。
pub fn encode_publish(uri: impl Into<String>, diagnostics: Vec<LspDiagnostic>) -> String {
    let params = PublishParams {
        uri: uri.into(),
        diagnostics: diagnostics.iter().map(LspDiagnosticJson::from).collect(),
    };
    let payload = PublishNotification {
        method: "textDocument/publishDiagnostics",
        params,
    };
    serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string())
}

/// JSON-RPC メッセージをデコードする。
pub fn decode_message(payload: impl AsRef<str>) -> Result<JsonRpcMessage, LspError> {
    let value: serde_json::Value = serde_json::from_str(payload.as_ref())
        .map_err(|err| LspError::new(LspErrorKind::DecodeFailed, err.to_string()))?;
    let method = value
        .get("method")
        .and_then(|value| value.as_str())
        .ok_or_else(|| {
            LspError::new(
                LspErrorKind::DecodeFailed,
                "method フィールドが見つかりません",
            )
        })?;
    let mut params = BTreeMap::new();
    if let Some(obj) = value.get("params").and_then(|value| value.as_object()) {
        for (key, value) in obj.iter() {
            let value = match value.as_str() {
                Some(text) => text.to_string(),
                None => value.to_string(),
            };
            params.insert(key.clone(), value);
        }
    }
    Ok(JsonRpcMessage {
        method: method.to_string(),
        params,
    })
}

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
enum DiagnosticSeverityJson {
    Error,
    Warning,
    Information,
    Hint,
}

#[derive(Serialize)]
struct LspDiagnosticJson {
    message: String,
    severity: DiagnosticSeverityJson,
    range: Range,
    #[serde(skip_serializing_if = "Option::is_none")]
    code: Option<String>,
}

impl From<&LspDiagnostic> for LspDiagnosticJson {
    fn from(value: &LspDiagnostic) -> Self {
        Self {
            message: value.message.clone(),
            severity: match value.severity {
                DiagnosticSeverity::Error => DiagnosticSeverityJson::Error,
                DiagnosticSeverity::Warning => DiagnosticSeverityJson::Warning,
                DiagnosticSeverity::Information => DiagnosticSeverityJson::Information,
                DiagnosticSeverity::Hint => DiagnosticSeverityJson::Hint,
            },
            range: value.range,
            code: value.code.clone(),
        }
    }
}

#[derive(Serialize)]
struct PublishParams {
    uri: String,
    diagnostics: Vec<LspDiagnosticJson>,
}

#[derive(Serialize)]
struct PublishNotification {
    method: &'static str,
    params: PublishParams,
}
