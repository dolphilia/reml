use crate::diagnostic::FrontendDiagnostic;
use crate::span::Span;
use crate::unicode::UnicodeDetail;
use reml_runtime::text::{insert_grapheme_stats_metadata, log_grapheme_stats, Str as UnicodeStr};
use serde_json::{json, Map, Value};
use unicode_width::UnicodeWidthStr;

#[derive(Debug, Clone)]
pub struct UnicodeSpanMetrics {
    refined_span: Option<Span>,
    original_span: Option<Span>,
    absolute_offset: Option<u32>,
    relative_offset: Option<u32>,
    display_width: Option<u32>,
    grapheme_start: Option<u32>,
    grapheme_end: Option<u32>,
    snippet: Option<String>,
}

impl UnicodeSpanMetrics {
    fn new(detail: &UnicodeDetail, span: Option<Span>, source: &str) -> Self {
        if span.is_none() || source.is_empty() {
            return Self {
                refined_span: span,
                original_span: span,
                absolute_offset: None,
                relative_offset: detail.offset(),
                display_width: None,
                grapheme_start: None,
                grapheme_end: None,
                snippet: None,
            };
        }
        let span_value = span.expect("span.is_some() checked");
        let (base_start, base_end) = clamp_span(span_value, source.len());
        let mut refined_span = Span::new(base_start as u32, base_end as u32);
        let mut snippet = None;
        let mut absolute_offset = Some(base_start as u32);
        let mut grapheme_start = Some(0);
        let mut grapheme_end = Some(0);
        let mut display_width = None;
        if base_start < base_end {
            let (highlight_start, highlight_end, text, override_offset) =
                select_highlight_slice(detail, source, base_start, base_end);
            refined_span = Span::new(highlight_start as u32, highlight_end as u32);
            snippet = text;
            absolute_offset = Some(highlight_start as u32);
            display_width = snippet
                .as_deref()
                .map(|value| UnicodeWidthStr::width(value) as u32);
            let prefix = &source[..highlight_start];
            let prefix_graphemes = UnicodeStr::from(prefix).iter_graphemes().count() as u32;
            let highlight_graphemes = snippet
                .as_deref()
                .map(|value| UnicodeStr::from(value).iter_graphemes().count() as u32)
                .unwrap_or(0);
            grapheme_start = Some(prefix_graphemes);
            grapheme_end = Some(prefix_graphemes.saturating_add(highlight_graphemes));
            if let Some(value) = override_offset {
                absolute_offset = Some(value as u32);
            }
        }
        Self {
            refined_span: Some(refined_span),
            original_span: span,
            absolute_offset,
            relative_offset: detail.offset(),
            display_width,
            grapheme_start,
            grapheme_end,
            snippet,
        }
    }

    fn grapheme_span(&self) -> Option<(u32, u32)> {
        match (self.grapheme_start, self.grapheme_end) {
            (Some(start), Some(end)) => Some((start, end)),
            _ => None,
        }
    }
}

fn clamp_span(span: Span, len: usize) -> (usize, usize) {
    let start = span.start.min(span.end) as usize;
    let end = span.end.max(span.start) as usize;
    (start.min(len), end.min(len))
}

fn select_highlight_slice(
    detail: &UnicodeDetail,
    source: &str,
    base_start: usize,
    base_end: usize,
) -> (usize, usize, Option<String>, Option<usize>) {
    if let Some(relative) = detail.offset() {
        let rel = relative as usize;
        let highlight_start = base_start.saturating_add(rel).min(base_end);
        let tail = &source[highlight_start..base_end];
        if let Some(first) = UnicodeStr::from(tail).iter_graphemes().next() {
            let end = highlight_start + first.len();
            return (
                highlight_start,
                end,
                Some(first.to_string()),
                Some(highlight_start),
            );
        }
    }
    let slice = &source[base_start..base_end];
    (
        base_start,
        base_end,
        Some(slice.to_string()),
        Some(base_start),
    )
}

pub fn integrate_unicode_metadata(
    diag: &mut FrontendDiagnostic,
    source: &str,
    extensions: &mut Map<String, Value>,
    metadata: &mut Map<String, Value>,
) {
    let detail = match diag.unicode.clone() {
        Some(detail) => detail,
        None => return,
    };
    let metrics = UnicodeSpanMetrics::new(&detail, diag.primary_span(), source);
    if let Some(span) = metrics.refined_span {
        diag.set_span(span);
    }
    extensions.insert(
        "unicode".to_string(),
        build_unicode_extension(&detail, &metrics),
    );
    apply_unicode_metadata(&detail, &metrics, metadata);
    attach_grapheme_stats(metadata, source);
}

fn build_unicode_extension(detail: &UnicodeDetail, metrics: &UnicodeSpanMetrics) -> Value {
    let mut map = Map::new();
    map.insert("kind".to_string(), json!(detail.kind_label()));
    map.insert("phase".to_string(), json!(detail.phase()));
    if let Some(offset) = metrics.absolute_offset {
        map.insert("offset".to_string(), json!(offset));
    }
    if let Some(relative) = metrics.relative_offset {
        map.insert("relative_offset".to_string(), json!(relative));
    }
    if let Some(span) = metrics.refined_span {
        map.insert(
            "span".to_string(),
            json!({ "start": span.start, "end": span.end }),
        );
    }
    if let Some(original) = metrics.original_span {
        map.insert(
            "original_span".to_string(),
            json!({ "start": original.start, "end": original.end }),
        );
    }
    if let Some((start, end)) = metrics.grapheme_span() {
        map.insert(
            "grapheme_span".to_string(),
            json!({ "start": start, "end": end }),
        );
    }
    if let Some(width) = metrics.display_width {
        map.insert("display_width".to_string(), json!(width));
    }
    if let Some(snippet) = metrics.snippet.as_ref() {
        map.insert("snippet".to_string(), json!(snippet));
    }
    if let Some(raw) = detail.raw() {
        map.insert("raw".to_string(), json!(raw));
    }
    if let Some(locale) = detail.locale() {
        map.insert("locale".to_string(), json!(locale));
    }
    if let Some(profile) = detail.profile() {
        map.insert("profile".to_string(), json!(profile));
    }
    Value::Object(map)
}

fn apply_unicode_metadata(
    detail: &UnicodeDetail,
    metrics: &UnicodeSpanMetrics,
    metadata: &mut Map<String, Value>,
) {
    metadata.insert("unicode.error.kind".to_string(), json!(detail.kind_label()));
    metadata.insert("unicode.error.phase".to_string(), json!(detail.phase()));
    if let Some(offset) = metrics.absolute_offset {
        metadata.insert("unicode.error.offset".to_string(), json!(offset));
    }
    if let Some(relative) = metrics.relative_offset {
        metadata.insert("unicode.error.relative_offset".to_string(), json!(relative));
    }
    if let Some(span) = metrics.refined_span {
        metadata.insert("unicode.span.start".to_string(), json!(span.start));
        metadata.insert("unicode.span.end".to_string(), json!(span.end));
    }
    if let Some(original) = metrics.original_span {
        metadata.insert(
            "unicode.span.original_start".to_string(),
            json!(original.start),
        );
        metadata.insert("unicode.span.original_end".to_string(), json!(original.end));
    }
    if let Some((start, end)) = metrics.grapheme_span() {
        metadata.insert("unicode.grapheme.start".to_string(), json!(start));
        metadata.insert("unicode.grapheme.end".to_string(), json!(end));
    }
    if let Some(width) = metrics.display_width {
        metadata.insert("unicode.display_width".to_string(), json!(width));
    }
    if let Some(snippet) = metrics.snippet.as_ref() {
        metadata.insert("unicode.snippet".to_string(), json!(snippet));
    }
    if let Some(raw) = detail.raw() {
        metadata.insert("unicode.identifier.raw".to_string(), json!(raw));
    }
    if let Some(locale) = detail.locale() {
        metadata.insert("unicode.locale.requested".to_string(), json!(locale));
    }
    if let Some(profile) = detail.profile() {
        metadata.insert("unicode.identifier.profile".to_string(), json!(profile));
    }
}

fn attach_grapheme_stats(metadata: &mut Map<String, Value>, source: &str) {
    if source.is_empty() || metadata.contains_key("text.grapheme_stats") {
        return;
    }
    let str_ref = UnicodeStr::from(source);
    if let Ok(stats) = log_grapheme_stats(&str_ref) {
        insert_grapheme_stats_metadata(metadata, &stats);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::FrontendDiagnostic;
    use crate::span::Span;
    use reml_runtime::text::UnicodeErrorKind;

    #[test]
    fn integrates_unicode_span_and_metadata() {
        let source = "let café = 1";
        let span = Span::new(4, 9);
        let detail = UnicodeDetail::new(UnicodeErrorKind::InvalidIdentifier)
            .with_phase("lex.identifier".to_string())
            .with_raw("café".to_string())
            .with_offset(Some(3));
        let mut diag = FrontendDiagnostic::new("unicode")
            .with_span(span)
            .with_unicode_detail(detail);
        let mut extensions = Map::new();
        let mut metadata = Map::new();
        integrate_unicode_metadata(&mut diag, source, &mut extensions, &mut metadata);
        let refined = diag.primary_span().expect("span exists");
        assert_eq!(refined.start, 7);
        assert_eq!(refined.end, 9);
        let unicode_ext = extensions
            .get("unicode")
            .and_then(|value| value.as_object())
            .expect("unicode extension");
        assert_eq!(
            unicode_ext.get("kind").and_then(|value| value.as_str()),
            Some("invalid_identifier")
        );
        assert_eq!(
            metadata
                .get("unicode.display_width")
                .and_then(|value| value.as_u64()),
            Some(1)
        );
        assert!(metadata.contains_key("unicode.identifier.raw"));
    }
}
