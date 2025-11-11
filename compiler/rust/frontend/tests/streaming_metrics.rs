use reml_frontend::diagnostic::recover::streaming_expression_summary;
use reml_frontend::diagnostic::FrontendDiagnostic;
use reml_frontend::span::Span;
use reml_frontend::streaming::{
    Expectation, ExpectationSummary, PackratEntry, StreamingState, StreamingStateConfig,
    TokenSample,
};
use smallvec::smallvec;
use smol_str::SmolStr;

const EXPECTED_STREAMING_TOKENS: [&str; 27] = [
    "continue",
    "defer",
    "do",
    "false",
    "for",
    "handle",
    "if",
    "loop",
    "match",
    "perform",
    "return",
    "self",
    "true",
    "unsafe",
    "while",
    "!",
    "(",
    "-",
    "[",
    "{",
    "|",
    "char-literal",
    "float-literal",
    "identifier",
    "integer-literal",
    "string-literal",
    "upper-identifier",
];

fn make_entry() -> PackratEntry {
    let tokens = smallvec![TokenSample {
        kind: SmolStr::new_inline("ident"),
        lexeme: SmolStr::new_inline("foo"),
    }];
    let expectations = vec![Expectation {
        description: SmolStr::new_inline("identifier"),
    }];
    let summary = Some(ExpectationSummary {
        humanized: Some(SmolStr::new_inline("identifier expected")),
        alternatives: vec![SmolStr::new_inline("IDENT")],
    });
    PackratEntry::new(tokens, expectations, summary)
}

#[test]
fn packrat_metrics_update() {
    let state = StreamingState::default();
    assert!(state.lookup_packrat(10, 0..8).is_none());
    state.store_packrat(10, 0..8, make_entry());
    assert!(state.lookup_packrat(10, 0..8).is_some());
    assert!(state.lookup_packrat(10, 8..16).is_none());

    let stats = state.packrat_stats();
    assert_eq!(stats.queries, 3);
    assert_eq!(stats.hits, 1);
    assert_eq!(stats.entries, 1);
    assert!(stats.approx_bytes > 0);
}

#[test]
fn packrat_budget_enforced() {
    let config = StreamingStateConfig {
        packrat_enabled: true,
        packrat_budget_bytes: 96,
        trace_enabled: false,
        trace_limit: 0,
    };
    let state = StreamingState::new(config);
    for idx in 0..5 {
        state.store_packrat(1, (idx * 10)..(idx * 10 + 3), make_entry());
    }
    let stats = state.packrat_stats();
    assert!(stats.approx_bytes <= 96);
    assert!(stats.budget_drops > 0 || stats.evictions > 0);
}

#[test]
fn span_trace_collects_frames() {
    let config = StreamingStateConfig {
        packrat_enabled: false,
        packrat_budget_bytes: 0,
        trace_enabled: true,
        trace_limit: 2,
    };
    let state = StreamingState::new(config);
    state.push_span_trace(Some(SmolStr::new_inline("ruleA")), Span::new(0, 1));
    state.push_span_trace(None, Span::new(1, 2));
    state.push_span_trace(Some(SmolStr::new_inline("ruleB")), Span::new(2, 3));
    let frames = state.drain_span_trace();
    assert_eq!(frames.len(), 2);
    assert_eq!(frames[0].span, Span::new(1, 2));
    assert_eq!(frames[1].label.as_deref(), Some("ruleB"));
    let trace_stats = state.span_trace_stats();
    assert_eq!(trace_stats.retained, 3);
    assert_eq!(trace_stats.dropped, 1);
}

#[test]
fn streaming_expected_token_snapshot_matches() {
    let summary = streaming_expression_summary();
    let tokens: Vec<String> = summary.tokens();
    let expected: Vec<String> = EXPECTED_STREAMING_TOKENS
        .iter()
        .map(|token| token.to_string())
        .collect();
    assert_eq!(
        tokens, expected,
        "streaming expected-token summary deviated from snapshot"
    );
}

#[test]
fn streaming_diagnostics_inject_expected_tokens() {
    let diag = FrontendDiagnostic::new("streaming placeholder").ensure_streaming_expected();
    let expected: Vec<String> = EXPECTED_STREAMING_TOKENS
        .iter()
        .map(|token| token.to_string())
        .collect();
    assert_eq!(
        diag.expected_tokens, expected,
        "streaming diagnostics should embed ExpectedTokenCollector summary"
    );
    assert_eq!(
        diag.expected_message_key.as_deref(),
        Some("parse.expected"),
        "streaming diagnostics should emit the standard parse.expected key"
    );
}
