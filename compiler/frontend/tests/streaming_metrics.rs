use reml_frontend::diagnostic::recover::streaming_expression_summary;
use reml_frontend::diagnostic::FrontendDiagnostic;
use reml_frontend::parser::{ParserOptions, RunConfig, StreamOutcome, StreamingRunner};
use reml_frontend::span::Span;
use reml_frontend::streaming::{
    Expectation, ExpectationSummary, PackratEntry, RuntimeBridgeSignal, RuntimeBridgeSignalKind,
    StreamFlowConfig, StreamFlowState, StreamingState, StreamingStateConfig, TokenSample,
};
use serde_json::json;
use smallvec::smallvec;
use smol_str::SmolStr;

macro_rules! assert_matches {
    ($expression:expr, $pattern:pat $(if $guard:expr)? $(,)?) => {
        match $expression {
            $pattern $(if $guard)? => {}
            value => panic!(
                "assertion failed: `{:?}` does not match `{}`",
                value,
                stringify!($pattern)
            ),
        }
    };
}

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

#[derive(Copy, Clone, Debug)]
enum SampleCase {
    UseNested,
    EffectHandler,
}

impl SampleCase {
    fn name(self) -> &'static str {
        match self {
            SampleCase::UseNested => "use_nested",
            SampleCase::EffectHandler => "effect_handler",
        }
    }

    fn source(self) -> &'static str {
        match self {
            SampleCase::UseNested => {
                include_str!("../../../examples/docs-examples/spec/1-1-syntax/use_nested.reml")
            }
            SampleCase::EffectHandler => {
                include_str!(
                    "../../../examples/docs-examples/spec/1-1-syntax/effect_handler.reml"
                )
            }
        }
    }
}

struct StreamingSampleResult {
    outcome: StreamOutcome,
    flow_state: StreamFlowState,
}

fn run_streaming_sample(case: SampleCase, chunk: Option<usize>) -> StreamingSampleResult {
    let mut run_config = RunConfig::default();
    if let Some(chunk_size) = chunk {
        run_config = run_config.with_extension("stream", |_| {
            json!({
                "chunk_size": chunk_size,
                "resume_hint": format!("sample:{}", case.name()),
            })
        });
    }

    let flow_config = StreamFlowConfig {
        enabled: true,
        demand_min_bytes: chunk.map(|size| size as u64),
        demand_preferred_bytes: chunk.map(|size| size as u64),
        resume_hint: Some(format!("sample:{}", case.name())),
        ..StreamFlowConfig::default()
    };
    let flow_state = StreamFlowState::new(flow_config);

    let parser_options = ParserOptions::from_run_config(&run_config)
        .with_stream_flow(Some(flow_state.clone()))
        .with_streaming_enabled(true);
    let runner = StreamingRunner::new(
        case.source().to_string(),
        parser_options,
        run_config,
        flow_state.clone(),
    );
    let outcome = runner.run_stream();

    StreamingSampleResult {
        outcome,
        flow_state,
    }
}

#[test]
fn module_header_acceptance_streaming_completes_once() {
    let StreamingSampleResult { outcome, .. } = run_streaming_sample(SampleCase::UseNested, None);
    let meta = match outcome {
        StreamOutcome::Completed { meta, .. } => meta,
        unexpected => panic!("expected completed outcome, got {unexpected:?}"),
    };

    assert_eq!(
        meta.flow.checkpoints_closed, 1,
        "streaming module header acceptance should close exactly one checkpoint"
    );
    assert!(
        meta.bridge_signal.is_none(),
        "module sample does not emit bridge signals"
    );
}

#[test]
fn effect_handler_acceptance_streaming_records_resume_signal() {
    let chunk_size = SampleCase::EffectHandler.source().len().saturating_sub(1);
    let StreamingSampleResult {
        outcome,
        flow_state,
    } = run_streaming_sample(SampleCase::EffectHandler, Some(chunk_size));
    let continuation = match outcome {
        StreamOutcome::Pending {
            continuation,
            demand,
            meta,
        } => {
            assert_eq!(
                demand.min_bytes, chunk_size,
                "chunked streaming demand should mirror chunk size"
            );
            assert_eq!(
                meta.flow.checkpoints_closed, 1,
                "first streaming pass closes one checkpoint"
            );
            continuation
        }
        unexpected => panic!("expected pending outcome, got {unexpected:?}"),
    };

    let resumed_outcome = StreamingRunner::from_continuation(continuation).run_stream();
    let completed_meta = match resumed_outcome {
        StreamOutcome::Completed { meta, .. } => meta,
        unexpected => panic!("expected completed outcome after resume, got {unexpected:?}"),
    };
    assert!(
        completed_meta.flow.checkpoints_closed >= 2,
        "resume pass should advance StreamFlow checkpoints"
    );

    flow_state.record_bridge_signal(RuntimeBridgeSignal {
        kind: RuntimeBridgeSignalKind::Resume,
        parser_offset: Some(0),
        stream_sequence: Some(1),
        stage: Some("effect.handler".to_string()),
        capability: Some("runtime.bridge".to_string()),
        note: Some("resume handler".to_string()),
        stage_trace: Vec::new(),
    });
    assert_matches!(
        flow_state.latest_bridge_signal(),
        Some(signal) if signal.kind == RuntimeBridgeSignalKind::Resume
    );
}

#[test]
fn bridge_signal_roundtrip_keeps_latest_signal() {
    let StreamingSampleResult {
        outcome,
        flow_state,
    } = run_streaming_sample(SampleCase::UseNested, None);
    match outcome {
        StreamOutcome::Completed { .. } => {}
        unexpected => panic!("expected completed outcome, got {unexpected:?}"),
    }

    flow_state.record_bridge_signal(RuntimeBridgeSignal {
        kind: RuntimeBridgeSignalKind::Await,
        parser_offset: Some(8),
        stream_sequence: Some(1),
        stage: Some("syntax.module".to_string()),
        capability: None,
        note: Some("await sample chunk".to_string()),
        stage_trace: Vec::new(),
    });
    flow_state.record_bridge_signal(RuntimeBridgeSignal {
        kind: RuntimeBridgeSignalKind::Resume,
        parser_offset: Some(16),
        stream_sequence: Some(2),
        stage: Some("syntax.module".to_string()),
        capability: Some("runtime.bridge".to_string()),
        note: None,
        stage_trace: Vec::new(),
    });
    assert_matches!(
        flow_state.latest_bridge_signal(),
        Some(signal)
            if signal.kind == RuntimeBridgeSignalKind::Resume
                && signal.parser_offset == Some(16)
    );
}
