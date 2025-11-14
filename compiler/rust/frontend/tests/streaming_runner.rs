use reml_frontend::parser::{ParserOptions, RunConfig, StreamOutcome, StreamingRunner};
use reml_frontend::streaming::{StreamFlowConfig, StreamFlowState};
use serde_json::json;

const STREAMING_SOURCE: &str = "let streaming = true;\nlet x = 1; // extra text";

fn streaming_run_config_with_chunk(chunk_size: usize) -> RunConfig {
    let mut config = RunConfig::default();
    config = config.with_extension("stream", |_| {
        json!({
            "chunk_size": chunk_size,
        })
    });
    config
}

fn run_streaming_runner(run_config: RunConfig) -> StreamOutcome {
    let parser_options = ParserOptions::from_run_config(&run_config);
    let stream_flow = StreamFlowState::new(StreamFlowConfig::default());
    let runner = StreamingRunner::new(
        STREAMING_SOURCE.to_string(),
        parser_options,
        run_config,
        stream_flow,
    );
    runner.run_stream()
}

#[test]
fn chunked_streaming_runner_emits_pending() {
    let chunk_size = 8;
    let outcome = run_streaming_runner(streaming_run_config_with_chunk(chunk_size));
    let first_pending = match outcome {
        StreamOutcome::Pending {
            continuation,
            demand,
            ..
        } => {
            assert_eq!(demand.min_bytes, chunk_size);
            continuation
        }
        unexpected => panic!("expected pending outcome, got {unexpected:?}"),
    };

    assert_eq!(first_pending.cursor, chunk_size);

    let mut attempts = 0;
    let mut pending_continuation = first_pending;
    let mut next_outcome = StreamingRunner::from_continuation(pending_continuation).run_stream();
    loop {
        match next_outcome {
            StreamOutcome::Completed { .. } => break,
            StreamOutcome::Pending {
                continuation,
                demand,
                ..
            } => {
                assert!(demand.min_bytes > 0);
                pending_continuation = continuation;
                attempts += 1;
                next_outcome =
                    StreamingRunner::from_continuation(pending_continuation).run_stream();
            }
        }
        if attempts > 10 {
            panic!("streaming runner did not complete after many attempts");
        }
    }
}

#[test]
fn streaming_runner_without_chunk_size_completes_immediately() {
    let run_config = RunConfig::default();
    let outcome = run_streaming_runner(run_config);
    assert!(
        matches!(outcome, StreamOutcome::Completed { .. }),
        "expected completed outcome when chunk_size is absent"
    );
}
