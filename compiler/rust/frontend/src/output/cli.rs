use super::localization::LocalizationKey;
use reml_runtime::lsp::derive::{DeriveModel, LspDeriveEnvelope};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::io::{self, Write};
use std::path::Path;
use uuid::Uuid;

#[derive(Clone, Copy, Debug)]
pub enum OutputFormat {
    Human,
    Json,
    Lsp,
    LspDerive,
}

impl OutputFormat {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value.trim().to_ascii_lowercase().as_str() {
            "human" => Ok(Self::Human),
            "json" => Ok(Self::Json),
            "lsp" => Ok(Self::Lsp),
            "lsp-derive" => Ok(Self::LspDerive),
            other => Err(format!(
                "--output に指定した値 `{other}` は human/json/lsp/lsp-derive のいずれかである必要があります"
            )),
        }
    }
}

impl Default for OutputFormat {
    fn default() -> Self {
        Self::Json
    }
}

#[derive(Clone, Copy)]
pub enum CliCommandKind {
    Check,
}

impl CliCommandKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            CliCommandKind::Check => "Check",
        }
    }
}

impl Default for CliCommandKind {
    fn default() -> Self {
        Self::Check
    }
}

#[derive(Clone, Copy)]
pub enum CliPhaseKind {
    Reporting,
}

impl CliPhaseKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            CliPhaseKind::Reporting => "Reporting",
        }
    }
}

impl Default for CliPhaseKind {
    fn default() -> Self {
        Self::Reporting
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CliDiagnosticEnvelope {
    command: String,
    phase: String,
    run_id: String,
    diagnostics: Vec<Value>,
    summary: CliSummary,
    exit_code: CliExitCode,
}

impl CliDiagnosticEnvelope {
    pub fn new(
        command: &CliCommandKind,
        phase: &CliPhaseKind,
        run_id: Uuid,
        diagnostics: Vec<Value>,
        summary: CliSummary,
        exit_code: CliExitCode,
    ) -> Self {
        Self {
            command: command.as_str().to_string(),
            phase: phase.as_str().to_string(),
            run_id: run_id.to_string(),
            diagnostics,
            summary,
            exit_code,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CliSummary {
    pub inputs: Vec<String>,
    pub started_at: String,
    pub finished_at: String,
    pub artifact: Option<String>,
    pub stats: Map<String, Value>,
    pub dsl_embeddings: Vec<CliDslEmbedding>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CliExitCode {
    label: String,
    value: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CliDslEmbedding {
    pub dsl_id: String,
    pub span: Option<Value>,
    pub mode: Option<String>,
}

impl CliExitCode {
    pub fn success() -> Self {
        Self {
            label: "success".to_string(),
            value: 0,
        }
    }

    pub fn warning() -> Self {
        Self {
            label: "warning".to_string(),
            value: 2,
        }
    }

    pub fn failure() -> Self {
        Self {
            label: "failure".to_string(),
            value: 1,
        }
    }

    pub fn value(&self) -> i32 {
        self.value
    }

    pub fn label(&self) -> &str {
        self.label.as_str()
    }
}

pub fn emit_cli_output(
    format: OutputFormat,
    envelope: &CliDiagnosticEnvelope,
    input_path: &Path,
    lsp_derive: Option<&DeriveModel>,
) -> Result<(), Box<dyn std::error::Error>> {
    match format {
        OutputFormat::Json => {
            let line = serde_json::to_string(envelope)?;
            println!("{line}");
        }
        OutputFormat::Human => {
            let mut stderr = io::stderr();
            render_human_output(&mut stderr, envelope)?;
        }
        OutputFormat::Lsp => emit_lsp_output(envelope, input_path)?,
        OutputFormat::LspDerive => emit_lsp_derive_output(lsp_derive, input_path)?,
    }
    Ok(())
}

fn render_human_output<W: Write>(
    writer: &mut W,
    envelope: &CliDiagnosticEnvelope,
) -> io::Result<()> {
    for diag in &envelope.diagnostics {
        let severity = diag
            .get("severity")
            .and_then(|value| value.as_str())
            .unwrap_or("unknown");
        let message = diag
            .get("message")
            .and_then(|value| value.as_str())
            .unwrap_or("");
        writeln!(writer, "{severity}: {message}")?;
        let location = diagnostic_location_label(diag);
        if !location.is_empty() {
            writeln!(writer, "  --> {location}")?;
        }
        if let Some(code) = diag.get("code").and_then(|value| value.as_str()) {
            if !code.trim().is_empty() {
                writeln!(writer, "  code: {code}")?;
            }
        }
        if let Some(label) = LocalizationKey::from_diagnostic(diag).display_label() {
            writeln!(writer, "  localization: {label}")?;
        }
        writeln!(writer)?;
    }
    writeln!(
        writer,
        "[{}] diagnostics={}, exit={}",
        envelope.command,
        envelope.diagnostics.len(),
        envelope.exit_code.label()
    )?;
    Ok(())
}

/// テストやスナップショット取得向けに、Human 出力を文字列として取得する。
pub fn render_human_output_to_string(envelope: &CliDiagnosticEnvelope) -> io::Result<String> {
    let mut buffer = Vec::new();
    render_human_output(&mut buffer, envelope)?;
    Ok(String::from_utf8(buffer).expect("human output must be utf8"))
}

fn emit_lsp_output(
    envelope: &CliDiagnosticEnvelope,
    input_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let uri = path_to_uri(input_path);
    let diagnostics = envelope
        .diagnostics
        .iter()
        .map(convert_to_lsp_diagnostic)
        .collect::<Vec<_>>();
    let publish = json!({
        "jsonrpc": "2.0",
        "method": "textDocument/publishDiagnostics",
        "params": {
            "uri": uri,
            "diagnostics": diagnostics,
            "version": 1,
        }
    });
    println!("{}", serde_json::to_string(&publish)?);
    let log_message = json!({
        "jsonrpc": "2.0",
        "method": "window/logMessage",
        "params": {
            "type": 4,
            "message": format!(
                "[{}] diagnostics={}, exit={}",
                envelope.command,
                envelope.diagnostics.len(),
                envelope.exit_code.label()
            ),
        }
    });
    println!("{}", serde_json::to_string(&log_message)?);
    Ok(())
}

fn emit_lsp_derive_output(
    model: Option<&DeriveModel>,
    input_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let model = model.ok_or("--output lsp-derive は parse driver 実行時のみ利用できます")?;
    let envelope = LspDeriveEnvelope::from_model(input_path.display().to_string(), model);
    println!("{}", serde_json::to_string(&envelope)?);
    Ok(())
}

fn convert_to_lsp_diagnostic(diag: &Value) -> Value {
    let severity = diag
        .get("severity")
        .and_then(|value| value.as_str())
        .unwrap_or("error");
    let severity_value = match severity {
        "warning" => 2,
        "info" => 3,
        "hint" => 4,
        _ => 1,
    };
    let message = diag
        .get("message")
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .to_string();
    let code = diag.get("code").cloned().unwrap_or(Value::Null);
    let range = lsp_range_from_primary(diag.get("primary"));
    let structured_hints = diag
        .get("structured_hints")
        .cloned()
        .unwrap_or_else(|| Value::Array(Vec::new()));
    let localization = LocalizationKey::from_diagnostic(diag).to_value();
    let mut data = Map::new();
    data.insert("diagnostic".to_string(), diag.clone());
    data.insert("structured_hints".to_string(), structured_hints);
    if !localization.is_null() {
        data.insert("localization".to_string(), localization);
    }
    json!({
        "range": range,
        "severity": severity_value,
        "code": code,
        "source": "reml_frontend",
        "message": message,
        "data": Value::Object(data),
    })
}

fn lsp_range_from_primary(primary: Option<&Value>) -> Value {
    if let Some(Value::Object(map)) = primary {
        let start_line = map
            .get("start_line")
            .and_then(|value| value.as_i64())
            .unwrap_or(1)
            .saturating_sub(1);
        let start_col = map
            .get("start_col")
            .and_then(|value| value.as_i64())
            .unwrap_or(1)
            .saturating_sub(1);
        let end_line = map
            .get("end_line")
            .and_then(|value| value.as_i64())
            .unwrap_or(start_line + 1)
            .saturating_sub(1);
        let end_col = map
            .get("end_col")
            .and_then(|value| value.as_i64())
            .unwrap_or(start_col + 1)
            .saturating_sub(1);
        return json!({
            "start": { "line": start_line, "character": start_col },
            "end": { "line": end_line, "character": end_col },
        });
    }
    json!({
        "start": { "line": 0, "character": 0 },
        "end": { "line": 0, "character": 0 },
    })
}

fn diagnostic_location_label(diag: &Value) -> String {
    if let Some(primary) = diag.get("primary").and_then(|value| value.as_object()) {
        let file = primary
            .get("file")
            .and_then(|value| value.as_str())
            .unwrap_or("<unknown>");
        let line = primary
            .get("start_line")
            .and_then(|value| value.as_i64())
            .unwrap_or(0);
        let column = primary
            .get("start_col")
            .and_then(|value| value.as_i64())
            .unwrap_or(0);
        return format!("{file}:{line}:{column}");
    }
    String::new()
}

fn path_to_uri(path: &Path) -> String {
    if path.is_absolute() {
        format!("file://{}", path.display())
    } else if let Ok(absolute) = path.canonicalize() {
        format!("file://{}", absolute.display())
    } else {
        format!("file://{}", path.display())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn localization_is_embedded_in_lsp_data() {
        let diag = json!({
            "message": "expected token",
            "severity": "error",
            "code": "E0001",
            "primary": null,
            "message_key": "parse.expected",
            "locale": "ja-JP",
            "locale_args": ["fn"]
        });
        let value = convert_to_lsp_diagnostic(&diag);
        assert_eq!(
            value["data"]["localization"]["message_key"],
            json!("parse.expected")
        );
        assert_eq!(value["data"]["localization"]["locale"], json!("ja-JP"));
    }
}
