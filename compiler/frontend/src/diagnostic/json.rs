use reml_runtime::text::{span_highlight, SpanHighlight};
use serde_json::{json, Map, Value};
use std::path::Path;

use super::{
    DiagnosticFixIt, DiagnosticHint, DiagnosticNote, DiagnosticSpanLabel, ExpectedToken,
    ExpectedTokensSummary, FrontendDiagnostic,
};
use crate::span::Span;
use crate::streaming::TraceFrame;

/// ソースの行・列インデクスを保持する軽量型。
#[derive(Debug, Clone)]
pub struct LineIndex {
    starts: Vec<usize>,
    len: usize,
}

impl LineIndex {
    pub fn new(source: &str) -> Self {
        let mut starts = vec![0];
        for (idx, ch) in source.char_indices() {
            if ch == '\n' {
                starts.push(idx + ch.len_utf8());
            }
        }
        Self {
            starts,
            len: source.len(),
        }
    }

    pub fn line_col(&self, offset: usize) -> (u32, u32) {
        let clamped = offset.min(self.len);
        let idx = match self.starts.binary_search(&clamped) {
            Ok(pos) => pos,
            Err(pos) => pos.saturating_sub(1),
        };
        let line_start = self.starts[idx];
        (
            idx as u32 + 1,
            (clamped.saturating_sub(line_start)) as u32 + 1,
        )
    }
}

/// フロントエンド診断を JSON に変換するためのあらまし。
pub struct FrontendDiagnosticPayload<'a> {
    pub diag: &'a FrontendDiagnostic,
    pub timestamp: &'a str,
    pub domain_label: &'a str,
    pub line_index: &'a LineIndex,
    pub input_path: &'a Path,
    pub source: &'a str,
    pub extensions: Map<String, Value>,
    pub audit_metadata: Map<String, Value>,
    pub audit: Value,
    pub recoverability: &'a str,
    pub expected: Value,
    pub schema_version: &'a str,
}

/// フロントエンド診断をスキーマ準拠の JSON オブジェクトに組み立てる。
pub fn build_frontend_diagnostic(payload: FrontendDiagnosticPayload<'_>) -> Value {
    let severity_hint = payload.diag.severity_hint.map(|hint| hint.as_str());
    let codes = effective_codes(payload.diag);
    let primary_span = payload.diag.primary_span();
    let location = span_to_location_opt(primary_span, payload.line_index, payload.input_path);
    let primary = span_to_primary_value(
        primary_span,
        payload.line_index,
        payload.input_path,
        payload.source,
    );
    let notes = payload
        .diag
        .notes
        .iter()
        .map(|note| note_to_json(note, payload.line_index, payload.input_path))
        .collect::<Vec<_>>();
    let secondary = payload
        .diag
        .secondary_spans
        .iter()
        .map(|label| secondary_span_to_json(label, payload.line_index, payload.input_path))
        .collect::<Vec<_>>();
    let hints = payload
        .diag
        .hints
        .iter()
        .map(|hint| diagnostic_hint_to_json(hint, payload.line_index, payload.input_path))
        .collect::<Vec<_>>();
    let structured_hints =
        structured_hints_to_json(payload.diag, payload.line_index, payload.input_path);
    let fixits = payload
        .diag
        .fixits
        .iter()
        .map(|fixit| diagnostic_fixit_to_json(fixit, payload.line_index, payload.input_path))
        .collect::<Vec<_>>();
    let span_trace_value = span_trace_to_json(
        &payload.diag.span_trace,
        payload.line_index,
        payload.input_path,
        payload.source,
    );

    json!({
    "schema_version": payload.schema_version,
    "timestamp": payload.timestamp,
    "id": payload.diag.id.map(|id| id.to_string()),
    "message": payload.diag.message,
    "severity": payload.diag.severity_or_default().as_str(),
    "severity_hint": severity_hint,
    "domain": payload.domain_label,
    "primary": primary,
    "location": location,
    "span_trace": span_trace_value,
    "extensions": Value::Object(payload.extensions),
    "audit_metadata": Value::Object(payload.audit_metadata),
    "audit": payload.audit,
    "notes": notes,
    "secondary": secondary,
    "hints": hints,
    "structured_hints": structured_hints,
    "fixits": fixits,
    "recoverability": payload.recoverability,
    "code": payload.diag.code.clone(),
    "codes": codes,
    "expected": payload.expected,
    })
}

fn span_trace_to_json(
    frames: &[TraceFrame],
    line_index: &LineIndex,
    input_path: &Path,
    source: &str,
) -> Value {
    let entries = frames
        .iter()
        .map(|frame| {
            json!({
                "label": frame.label.as_ref().map(|label| label.to_string()),
                "span": span_to_primary_value(Some(frame.span), line_index, input_path, source),
            })
        })
        .collect::<Vec<_>>();
    Value::Array(entries)
}

pub fn build_recover_extension(diag: &FrontendDiagnostic) -> Option<Value> {
    if let Some(summary) = diag.expected_summary.as_ref() {
        if summary_has_recover_payload(summary) {
            return Some(recover_extension_payload_from_summary(summary));
        }
    }
    if diag.has_expected_tokens() {
        let message = diag
            .expected_humanized
            .clone()
            .unwrap_or_else(|| default_expected_message(&diag.expected_tokens));
        let tokens: Vec<Value> = if !diag.expected_alternatives().is_empty() {
            diag.expected_alternatives()
                .iter()
                .map(expected_token_object_from_expected)
                .collect()
        } else {
            diag.expected_tokens
                .iter()
                .map(|token| expected_token_object_from_label(token))
                .collect()
        };
        Some(json!({
            "message": message,
            "expected_tokens": tokens,
        }))
    } else {
        diag.notes.iter().find_map(|note| {
            if note.label == "recover.expected_tokens" {
                Some(json!({
                    "message": note.message,
                    "expected_tokens": [],
                }))
            } else {
                None
            }
        })
    }
}

pub fn build_expected_field(diag: &FrontendDiagnostic) -> Value {
    if !diag.has_expected_tokens() {
        return Value::Null;
    }
    let message_key = diag
        .expected_message_key
        .clone()
        .unwrap_or_else(|| "parse.expected".to_string());
    let alternatives: Vec<Value> = if !diag.expected_alternatives().is_empty() {
        diag.expected_alternatives()
            .iter()
            .map(expected_token_object_from_expected)
            .collect()
    } else {
        diag.expected_tokens
            .iter()
            .map(|token| expected_token_object_from_label(token))
            .collect()
    };
    let humanized = diag
        .expected_humanized
        .clone()
        .unwrap_or_else(|| default_expected_message(&diag.expected_tokens));
    let locale_args = if diag.expected_locale_args.is_empty() {
        diag.expected_tokens.clone()
    } else {
        diag.expected_locale_args.clone()
    };
    let mut map = Map::new();
    map.insert("message_key".to_string(), json!(message_key));
    map.insert("humanized".to_string(), json!(humanized));
    map.insert("locale_args".to_string(), json!(locale_args));
    map.insert("alternatives".to_string(), json!(alternatives));
    if let Some(summary) = diag.expected_summary.as_ref() {
        if let Some(context) = summary.context_note.as_ref() {
            if !context.trim().is_empty() {
                map.insert("context_note".to_string(), json!(context));
            }
        }
    }
    Value::Object(map)
}

pub fn expected_payload_from_summary(summary: &ExpectedTokensSummary) -> Value {
    let expected_tokens = expected_tokens_array_from_summary(summary);
    let message = summary
        .humanized
        .clone()
        .unwrap_or_else(|| summary.tokens().join(", "));
    json!({
        "message": message,
        "expected_tokens": expected_tokens,
    })
}

pub fn recover_extension_payload_from_summary(summary: &ExpectedTokensSummary) -> Value {
    let expected_tokens = expected_tokens_array_from_summary(summary);
    let mut map = Map::new();
    map.insert("expected_tokens".to_string(), Value::Array(expected_tokens));
    if let Some(message) = non_blank_string(summary.humanized.as_ref()) {
        map.insert("message".to_string(), json!(message));
    }
    if let Some(context) = non_blank_string(summary.context_note.as_ref()) {
        map.insert("context".to_string(), json!(context));
    }
    Value::Object(map)
}

fn expected_tokens_array_from_summary(summary: &ExpectedTokensSummary) -> Vec<Value> {
    if summary.alternatives.is_empty() {
        summary
            .tokens()
            .iter()
            .map(|token| expected_token_object_from_label(token))
            .collect()
    } else {
        summary
            .alternatives
            .iter()
            .map(expected_token_object_from_expected)
            .collect()
    }
}

fn summary_has_recover_payload(summary: &ExpectedTokensSummary) -> bool {
    summary.has_alternatives()
        || non_blank_string(summary.humanized.as_ref()).is_some()
        || non_blank_string(summary.context_note.as_ref()).is_some()
}

pub fn span_to_primary_value(
    span: Option<Span>,
    index: &LineIndex,
    input_path: &Path,
    source: &str,
) -> Value {
    let map = match span {
        Some(span) => primary_map_from_span(span, index, input_path, source),
        None => default_primary(input_path),
    };
    Value::Object(map)
}

pub fn span_to_location_opt(span: Option<Span>, index: &LineIndex, input_path: &Path) -> Value {
    span.map(|span| span_to_location(span, index, input_path))
        .unwrap_or(Value::Null)
}

pub fn span_to_location(span: Span, index: &LineIndex, input_path: &Path) -> Value {
    let (line, column) = index.line_col(span.start as usize);
    let (end_line, end_column) = index.line_col(span.end as usize);
    json!({
        "file": input_path,
        "line": line,
        "column": column,
        "endLine": end_line,
        "endColumn": end_column,
    })
}

fn primary_map_from_span(
    span: Span,
    index: &LineIndex,
    input_path: &Path,
    source: &str,
) -> Map<String, Value> {
    let (start_line, start_col) = index.line_col(span.start as usize);
    let (end_line, end_col) = index.line_col(span.end as usize);
    let mut map = Map::new();
    map.insert("file".to_string(), json!(input_path));
    map.insert("start_line".to_string(), json!(start_line));
    map.insert("start_col".to_string(), json!(start_col));
    map.insert("end_line".to_string(), json!(end_line));
    map.insert("end_col".to_string(), json!(end_col));
    if let Some(highlight) = span_highlight(source, span.start as usize, span.end as usize) {
        map.insert("highlight".to_string(), highlight_to_value(&highlight));
    }
    map
}

fn default_primary(input_path: &Path) -> Map<String, Value> {
    let mut map = Map::new();
    map.insert("file".to_string(), json!(input_path));
    map.insert("start_line".to_string(), json!(0));
    map.insert("start_col".to_string(), json!(0));
    map.insert("end_line".to_string(), json!(0));
    map.insert("end_col".to_string(), json!(0));
    map
}

fn highlight_to_value(highlight: &SpanHighlight) -> Value {
    json!({
        "line": highlight.line,
        "column_start": highlight.column_start,
        "column_end": highlight.column_end,
        "line_text": highlight.line_text,
        "highlight_text": highlight.highlight_text,
        "indicator": highlight.indicator,
    })
}

fn note_to_json(note: &DiagnosticNote, index: &LineIndex, input_path: &Path) -> Value {
    let span_value = note
        .span
        .map(|span| span_to_location(span, index, input_path))
        .unwrap_or(Value::Null);
    json!({
        "label": note.label,
        "message": note.message,
        "span": span_value,
    })
}

fn secondary_span_to_json(
    label: &DiagnosticSpanLabel,
    index: &LineIndex,
    input_path: &Path,
) -> Value {
    let span_value = label
        .span
        .map(|span| span_to_location(span, index, input_path))
        .unwrap_or(Value::Null);
    json!({
        "span": span_value,
        "message": label.message.clone(),
    })
}

fn diagnostic_hint_to_json(hint: &DiagnosticHint, index: &LineIndex, input_path: &Path) -> Value {
    let actions = hint
        .actions
        .iter()
        .map(|action| diagnostic_fixit_to_json(action, index, input_path))
        .collect::<Vec<_>>();
    json!({
        "message": hint.message.clone(),
        "actions": actions,
    })
}

fn structured_hints_to_json(
    diag: &FrontendDiagnostic,
    index: &LineIndex,
    input_path: &Path,
) -> Value {
    let entries = diag
        .hints
        .iter()
        .enumerate()
        .map(|(idx, hint)| structured_hint_to_json(idx, hint, index, input_path))
        .collect::<Vec<_>>();
    Value::Array(entries)
}

fn structured_hint_to_json(
    index: usize,
    hint: &DiagnosticHint,
    line_index: &LineIndex,
    input_path: &Path,
) -> Value {
    let id = hint.id.clone().unwrap_or_else(|| format!("hint-{index}"));
    let title = hint
        .title
        .clone()
        .or_else(|| hint.message.clone())
        .unwrap_or_else(|| format!("Hint {}", index + 1));
    let kind = hint
        .kind
        .clone()
        .unwrap_or_else(|| default_structured_hint_kind(hint).to_string());
    let span_value = hint
        .span
        .or_else(|| hint.actions.first().map(|action| action.span()))
        .map(|span| span_to_location(span, line_index, input_path))
        .unwrap_or(Value::Null);
    let payload = hint.payload.clone().unwrap_or(Value::Null);
    let actions = hint
        .actions
        .iter()
        .map(|action| diagnostic_fixit_to_json(action, line_index, input_path))
        .collect::<Vec<_>>();
    json!({
        "id": id,
        "title": title,
        "kind": kind,
        "span": span_value,
        "payload": payload,
        "actions": actions,
    })
}

fn default_structured_hint_kind(hint: &DiagnosticHint) -> &'static str {
    if hint.actions.is_empty() {
        "information"
    } else {
        "quick_fix"
    }
}

fn diagnostic_fixit_to_json(
    fixit: &DiagnosticFixIt,
    index: &LineIndex,
    input_path: &Path,
) -> Value {
    let mut map = Map::new();
    map.insert(
        "span".to_string(),
        span_to_location(fixit.span(), index, input_path),
    );
    map.insert("kind".to_string(), json!(fixit.kind()));
    if let Some(text) = fixit.text() {
        map.insert("text".to_string(), json!(text));
    }
    Value::Object(map)
}

fn expected_token_object_from_expected(token: &ExpectedToken) -> Value {
    let label = token.raw_label();
    let hint = token.kind_label();
    json!({
        "token": label,
        "label": label,
        "hint": hint,
        "kind": hint,
    })
}

fn expected_token_object_from_label(token: &str) -> Value {
    let hint = classify_expected_token(token);
    json!({
        "token": token,
        "label": token,
        "hint": hint,
        "kind": hint,
    })
}

fn classify_expected_token(token: &str) -> &'static str {
    let trimmed = token.trim();
    if trimmed.is_empty() {
        "token"
    } else if trimmed.contains("identifier")
        || trimmed.ends_with("literal")
        || trimmed.ends_with("-literal")
    {
        "class"
    } else if trimmed
        .chars()
        .all(|ch| ch.is_ascii_alphabetic() && ch.is_lowercase())
    {
        "keyword"
    } else if trimmed.chars().all(|ch| ch.is_ascii_uppercase()) {
        "class"
    } else {
        "token"
    }
}

fn default_expected_message(tokens: &[String]) -> String {
    if tokens.is_empty() {
        return "ここで解釈可能な構文が見つかりません".to_string();
    }
    let formatted = tokens
        .iter()
        .map(|token| format!("`{}`", token))
        .collect::<Vec<_>>()
        .join("、");
    format!("ここで{}のいずれかが必要です", formatted)
}

fn non_blank_string(value: Option<&String>) -> Option<String> {
    value.and_then(|text| {
        if text.trim().is_empty() {
            None
        } else {
            Some(text.clone())
        }
    })
}

fn effective_codes(diag: &FrontendDiagnostic) -> Vec<String> {
    if !diag.codes.is_empty() {
        diag.codes.clone()
    } else if let Some(code) = diag.code.as_ref() {
        vec![code.clone()]
    } else {
        vec!["unknown".to_string()]
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_recover_extension, ExpectedToken, ExpectedTokensSummary, FrontendDiagnostic,
    };

    #[test]
    fn recover_extension_from_summary_includes_context() {
        let summary = ExpectedTokensSummary {
            message_key: Some("parse.expected".to_string()),
            locale_args: vec!["fn".to_string()],
            humanized: Some("ここで `fn` が必要です".to_string()),
            context_note: Some("式の中でここに来ました".to_string()),
            alternatives: vec![ExpectedToken::keyword("fn")],
        };
        let diag = FrontendDiagnostic::new("oops").apply_expected_summary(&summary);
        let payload = build_recover_extension(&diag).expect("recover extension must exist");
        assert_eq!(
            payload.get("context").and_then(|value| value.as_str()),
            Some("式の中でここに来ました")
        );
        assert_eq!(
            payload.get("message").and_then(|value| value.as_str()),
            Some("ここで `fn` が必要です")
        );
        assert_eq!(
            payload
                .get("expected_tokens")
                .and_then(|value| value.as_array())
                .map(|arr| arr.len()),
            Some(1)
        );
    }

    #[test]
    fn recover_extension_obtains_kind_from_summary_alternatives() {
        let summary = ExpectedTokensSummary {
            message_key: Some("parse.expected".to_string()),
            locale_args: Vec::new(),
            humanized: None,
            context_note: None,
            alternatives: vec![
                ExpectedToken::keyword("fn"),
                ExpectedToken::not("identifier"),
                ExpectedToken::class("identifier"),
            ],
        };
        let diag = FrontendDiagnostic::new("oops").apply_expected_summary(&summary);
        let payload = build_recover_extension(&diag).expect("recover extension must exist");
        let expected_tokens = payload
            .get("expected_tokens")
            .and_then(|value| value.as_array())
            .expect("expected tokens array present");
        assert_eq!(
            expected_tokens[0]
                .get("kind")
                .and_then(|value| value.as_str()),
            Some("keyword")
        );
        assert_eq!(
            expected_tokens[1]
                .get("kind")
                .and_then(|value| value.as_str()),
            Some("not")
        );
        assert_eq!(
            expected_tokens[2]
                .get("kind")
                .and_then(|value| value.as_str()),
            Some("class")
        );
    }
}
