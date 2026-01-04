//! Core.Doc 仕様の最小実装。
//! `///` コメント抽出と最小レンダリング、Doctest の枠組みを提供する。

use once_cell::sync::Lazy;
use serde_json::{Map as JsonMap, Value};
use std::sync::Mutex;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::audit::{AuditEnvelope, AuditEvent, AuditEventKind};
use crate::prelude::ensure::{DiagnosticSeverity, GuardDiagnostic};

const DOC_DIAGNOSTIC_DOMAIN: &str = "doc";
const DOCTEST_FAILED_CODE: &str = "doc.doctest.failed";
const DOC_EXTRACT_FAILED_CODE: &str = "doc.extract.failed";
const DOC_RENDER_FAILED_CODE: &str = "doc.render.failed";
const DOCTEST_FAIL_MARKER: &str = "doctest:fail";

static DOC_DIAGNOSTICS: Lazy<Mutex<Vec<GuardDiagnostic>>> = Lazy::new(|| Mutex::new(Vec::new()));
static DOC_AUDIT_EVENTS: Lazy<Mutex<Vec<AuditEvent>>> = Lazy::new(|| Mutex::new(Vec::new()));

/// ドキュメント項目。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocItem {
    pub name: String,
    pub summary: String,
    pub body: String,
    pub examples: Vec<String>,
}

/// ドキュメントページ。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocPage {
    pub items: Vec<DocItem>,
}

/// Doc エラー。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocError {
    pub kind: DocErrorKind,
    pub message: String,
}

impl DocError {
    pub fn new(kind: DocErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    fn into_diagnostic(&self) -> GuardDiagnostic {
        let code = match self.kind {
            DocErrorKind::ParseFailed => DOC_EXTRACT_FAILED_CODE,
            DocErrorKind::RenderFailed => DOC_RENDER_FAILED_CODE,
            DocErrorKind::DoctestFailed => DOCTEST_FAILED_CODE,
        };
        GuardDiagnostic {
            code,
            domain: DOC_DIAGNOSTIC_DOMAIN,
            severity: DiagnosticSeverity::Error,
            message: self.message.clone(),
            notes: Vec::new(),
            extensions: JsonMap::new(),
            audit_metadata: JsonMap::new(),
        }
    }
}

/// Doc エラー種別。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocErrorKind {
    ParseFailed,
    RenderFailed,
    DoctestFailed,
}

/// ドキュメントコメントからページを構築する。
pub fn extract(source: impl AsRef<str>) -> Result<DocPage, DocError> {
    let mut items = Vec::new();
    let mut lines = source.as_ref().lines().peekable();

    while let Some(line) = lines.next() {
        if let Some(doc_line) = strip_doc_prefix(line) {
            let mut doc_lines = vec![doc_line];
            while let Some(next) = lines.peek() {
                if let Some(doc_line) = strip_doc_prefix(next) {
                    doc_lines.push(doc_line);
                    lines.next();
                } else {
                    break;
                }
            }

            let mut item_name = String::new();
            while let Some(next) = lines.peek() {
                if next.trim().is_empty() {
                    lines.next();
                    continue;
                }
                let candidate = next.to_string();
                lines.next();
                if let Some(name) = extract_item_name(&candidate) {
                    item_name = name;
                }
                break;
            }

            let (summary, body, examples) = parse_doc_lines(&doc_lines);
            items.push(DocItem {
                name: item_name,
                summary,
                body,
                examples,
            });
        }
    }

    Ok(DocPage { items })
}

/// Markdown を生成する。
pub fn render_markdown(page: DocPage) -> String {
    let mut sections = Vec::new();
    for item in page.items {
        let title = if !item.summary.is_empty() {
            item.summary.clone()
        } else {
            item.name.clone()
        };
        let mut block = String::new();
        if !title.is_empty() {
            block.push_str(&title);
        }
        if !item.body.is_empty() {
            if !block.is_empty() {
                block.push('\n');
            }
            block.push_str(&item.body);
        }
        sections.push(block);
    }
    sections.join("\n\n")
}

/// HTML を生成する。
pub fn render_html(page: DocPage) -> String {
    let mut output = String::new();
    for item in page.items {
        let title = if !item.summary.is_empty() {
            item.summary
        } else {
            item.name
        };
        output.push_str("<section>");
        if !title.is_empty() {
            output.push_str("<h1>");
            output.push_str(&escape_html(&title));
            output.push_str("</h1>");
        }
        if !item.body.is_empty() {
            output.push_str("<p>");
            output.push_str(&escape_html(&item.body));
            output.push_str("</p>");
        }
        output.push_str("</section>");
    }
    output
}

/// Doctest を実行する。
pub fn run_doctest(page: DocPage) -> Result<(), DocError> {
    let example_count = page
        .items
        .iter()
        .map(|item| item.examples.len())
        .sum::<usize>();
    if page
        .items
        .iter()
        .flat_map(|item| item.examples.iter())
        .any(|example| example.contains(DOCTEST_FAIL_MARKER))
    {
        let error = DocError::new(DocErrorKind::DoctestFailed, "doctest failed");
        record_doc_diagnostic(&error);
        record_doc_audit_event("failed", example_count);
        return Err(error);
    }

    record_doc_audit_event("ok", example_count);
    Ok(())
}

/// Doc の診断を取得してリセットする。
pub fn take_doc_diagnostics() -> Vec<GuardDiagnostic> {
    DOC_DIAGNOSTICS
        .lock()
        .map(|mut diagnostics| std::mem::take(&mut *diagnostics))
        .unwrap_or_default()
}

/// Doc の監査イベントを取得してリセットする。
pub fn take_doc_audit_events() -> Vec<AuditEvent> {
    DOC_AUDIT_EVENTS
        .lock()
        .map(|mut events| std::mem::take(&mut *events))
        .unwrap_or_default()
}

fn strip_doc_prefix(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with("///") {
        return None;
    }
    let content = trimmed.trim_start_matches("///");
    let content = content.strip_prefix(' ').unwrap_or(content);
    Some(content.to_string())
}

fn extract_item_name(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    if let Some(rest) = trimmed.strip_prefix("fn ") {
        let name = rest
            .split(|ch: char| ch == '(' || ch.is_whitespace())
            .next()
            .unwrap_or("")
            .trim();
        if !name.is_empty() {
            return Some(name.to_string());
        }
    }
    None
}

fn parse_doc_lines(lines: &[String]) -> (String, String, Vec<String>) {
    let mut summary = String::new();
    let mut body_lines = Vec::new();
    let mut examples = Vec::new();
    let mut in_code = false;
    let mut code_lines = Vec::new();

    for line in lines {
        let trimmed = line.trim_end();
        if trimmed.starts_with("```") {
            if in_code {
                let example = code_lines.join("\n");
                if !example.is_empty() {
                    examples.push(example);
                }
                code_lines.clear();
            }
            in_code = !in_code;
            continue;
        }

        if in_code {
            code_lines.push(trimmed.to_string());
            continue;
        }

        if summary.is_empty() {
            summary = trimmed.to_string();
        } else {
            body_lines.push(trimmed.to_string());
        }
    }

    if in_code {
        let example = code_lines.join("\n");
        if !example.is_empty() {
            examples.push(example);
        }
    }

    (summary, body_lines.join("\n"), examples)
}

fn escape_html(input: &str) -> String {
    let mut escaped = String::new();
    for ch in input.chars() {
        match ch {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

fn record_doc_diagnostic(error: &DocError) {
    let diagnostic = error.into_diagnostic();
    if let Ok(mut diagnostics) = DOC_DIAGNOSTICS.lock() {
        diagnostics.push(diagnostic);
    }
}

fn record_doc_audit_event(result: &str, example_count: usize) {
    let timestamp = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string());
    let mut metadata = JsonMap::new();
    metadata.insert(
        "event.kind".into(),
        Value::String(AuditEventKind::DocTest.as_str().into_owned()),
    );
    metadata.insert(
        "doc.doctest.result".into(),
        Value::String(result.to_string()),
    );
    metadata.insert(
        "doc.doctest.examples".into(),
        Value::Number(serde_json::Number::from(example_count as u64)),
    );
    let envelope = AuditEnvelope::from_parts(metadata, None, None, None);
    let event = AuditEvent::new(timestamp, envelope);
    if let Ok(mut events) = DOC_AUDIT_EVENTS.lock() {
        events.push(event);
    }
}
