use super::Str;

/// `Span` を CLI/LSP 表示用にハイライトする際の情報。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpanHighlight {
    pub line: u32,
    pub column_start: u32,
    pub column_end: u32,
    pub line_text: std::string::String,
    pub highlight_text: std::string::String,
    pub indicator: std::string::String,
}

/// 入力全体とバイトオフセットから 1 行分のハイライト情報を生成する。
pub fn span_highlight(source: &str, start: usize, end: usize) -> Option<SpanHighlight> {
    if source.is_empty() {
        return None;
    }
    let len = source.len();
    let start = start.min(len);
    let end = end.min(len);
    let (line_start, line_number) = line_start_index(source, start);
    let line_end = line_end_index(source, line_start);
    let highlight_start = start.min(line_end);
    let highlight_end = end.max(highlight_start).min(line_end);
    let line_slice = &source[line_start..line_end];
    let prefix_slice = &source[line_start..highlight_start];
    let highlight_slice = &source[highlight_start..highlight_end];
    let prefix_graphemes = Str::from(prefix_slice).iter_graphemes().count() as u32;
    let highlight_graphemes = Str::from(highlight_slice).iter_graphemes().count() as u32;
    let column_start = prefix_graphemes + 1;
    let column_end = column_start + highlight_graphemes;
    let marker_len = if highlight_graphemes == 0 {
        1
    } else {
        highlight_graphemes as usize
    };
    let mut indicator = String::new();
    indicator.extend(std::iter::repeat(' ').take(column_start.saturating_sub(1) as usize));
    indicator.extend(std::iter::repeat('~').take(marker_len));

    Some(SpanHighlight {
        line: line_number,
        column_start,
        column_end,
        line_text: line_slice.to_string(),
        highlight_text: highlight_slice.to_string(),
        indicator,
    })
}

fn line_start_index(source: &str, offset: usize) -> (usize, u32) {
    let mut line_start = 0usize;
    let mut line_number = 1u32;
    for (idx, ch) in source.char_indices() {
        if idx >= offset {
            break;
        }
        if ch == '\n' {
            line_start = idx + ch.len_utf8();
            line_number += 1;
        }
    }
    (line_start, line_number)
}

fn line_end_index(source: &str, start: usize) -> usize {
    source[start..]
        .find('\n')
        .map(|rel| start + rel)
        .unwrap_or_else(|| source.len())
}

#[cfg(test)]
mod tests {
    use super::span_highlight;

    #[test]
    fn expect_span_highlight() {
        let source = "let 名 = \"👨‍💻\";\n";
        let start = source.find("名").unwrap();
        let end = start + "名".len();
        let highlight = span_highlight(source, start, end).expect("highlight");
        assert_eq!(highlight.line, 1);
        assert_eq!(highlight.column_start, 5);
        assert_eq!(highlight.column_end, 6);
        assert_eq!(highlight.highlight_text, "名");
        assert!(highlight.indicator.ends_with("~"));
    }

    #[test]
    fn span_highlight_handles_grapheme_clusters() {
        let source = "prefix 👩‍💻 suffix\n";
        let start = source.find("👩").unwrap();
        // `👩‍💻` は 1 grapheme だが複数コードポイントを含む
        let end = start + "👩‍💻".len();
        let highlight = span_highlight(source, start, end).expect("highlight");
        assert_eq!(highlight.column_start, 8);
        assert_eq!(highlight.column_end, 9);
        assert_eq!(highlight.highlight_text, "👩‍💻");
        assert_eq!(highlight.indicator.trim(), "~");
    }
}
