use crate::parser::api::{ParseResult, RunConfig};
use crate::parser::ast::Module;
use crate::parser::{ParserDriver, ParserOptions};
use crate::streaming::{RuntimeBridgeSignal, StreamFlowMetrics, StreamFlowState, StreamMetrics};

/// ストリーミング処理のメタ情報。
#[derive(Debug, Clone)]
pub struct StreamMeta {
    pub metrics: StreamMetrics,
    pub flow: StreamFlowMetrics,
    pub bridge_signal: Option<RuntimeBridgeSignal>,
}

impl StreamMeta {
    fn from_result(result: &ParseResult<Module>, flow: &StreamFlowState) -> Self {
        Self {
            metrics: result.stream_metrics.clone(),
            flow: flow.metrics(),
            bridge_signal: flow.latest_bridge_signal(),
        }
    }
}

/// 再開を待つ際にランナーが返すヒント。
#[derive(Debug, Clone)]
pub struct DemandHint {
    pub min_bytes: usize,
    pub preferred_bytes: Option<usize>,
    pub resume_hint: Option<String>,
    pub reason: Option<String>,
}

/// 継続再開に必要なコンテキスト。
#[derive(Debug, Clone)]
pub struct Continuation {
    pub buffer: String,
    pub parser_options: ParserOptions,
    pub run_config: RunConfig,
    pub stream_flow: StreamFlowState,
    pub cursor: usize,
    pub chunk_size: usize,
}

#[derive(Debug)]
pub enum StreamOutcome {
    Completed {
        result: ParseResult<Module>,
        meta: StreamMeta,
    },
    Pending {
        continuation: Continuation,
        demand: DemandHint,
        meta: StreamMeta,
    },
}

/// `ParserDriver` をラップして `run_stream` / `resume` API を提供するランナー。
#[derive(Debug, Clone)]
pub struct StreamingRunner {
    continuation: Continuation,
}

impl StreamingRunner {
    pub fn new(
        buffer: String,
        parser_options: ParserOptions,
        run_config: RunConfig,
        stream_flow: StreamFlowState,
    ) -> Self {
        Self {
            continuation: Continuation::new(buffer, parser_options, run_config, stream_flow),
        }
    }

    pub fn from_continuation(continuation: Continuation) -> Self {
        Self { continuation }
    }

    pub fn run_stream(self) -> StreamOutcome {
        run_stream_from_continuation(self.continuation)
    }

    pub fn record_bridge_signal(&self, signal: RuntimeBridgeSignal) {
        self.continuation.stream_flow.record_bridge_signal(signal);
    }

    pub fn resume(mut self, more: &str) -> StreamOutcome {
        self.continuation.buffer.push_str(more);
        run_stream_from_continuation(self.continuation)
    }
}

fn run_stream_from_continuation(continuation: Continuation) -> StreamOutcome {
    let mut continuation = continuation;
    let total_len = continuation.buffer.len();
    let chunk_size = continuation.chunk_size;
    let next_cursor = if chunk_size == 0 {
        total_len
    } else {
        (continuation.cursor + chunk_size).min(total_len)
    };

    let mut options = continuation.parser_options.clone();
    options.stream_flow = Some(continuation.stream_flow.clone());
    let result = ParserDriver::parse_with_options_and_run_config(
        &continuation.buffer,
        options,
        continuation.run_config.clone(),
    );
    let meta = StreamMeta::from_result(&result, &continuation.stream_flow);

    continuation.cursor = next_cursor;

    if chunk_size > 0 && continuation.cursor < total_len {
        let demand = DemandHint {
            min_bytes: chunk_size,
            preferred_bytes: Some(chunk_size),
            resume_hint: resume_hint_from_run_config(&continuation.run_config),
            reason: Some("stream.chunk".to_string()),
        };
        StreamOutcome::Pending {
            continuation,
            demand,
            meta,
        }
    } else {
        StreamOutcome::Completed { result, meta }
    }
}

impl Continuation {
    fn new(
        buffer: String,
        parser_options: ParserOptions,
        run_config: RunConfig,
        stream_flow: StreamFlowState,
    ) -> Self {
        let chunk_size = chunk_size_from_run_config(&run_config);
        Self {
            buffer,
            parser_options,
            run_config,
            stream_flow,
            cursor: 0,
            chunk_size,
        }
    }
}

fn chunk_size_from_run_config(run_config: &RunConfig) -> usize {
    run_config
        .extension("stream")
        .and_then(|value| value.get("chunk_size"))
        .and_then(|value| value.as_u64())
        .map(|value| value as usize)
        .unwrap_or_default()
}

fn resume_hint_from_run_config(run_config: &RunConfig) -> Option<String> {
    run_config
        .extension("stream")
        .and_then(|value| value.get("resume_hint"))
        .and_then(|value| value.as_str())
        .map(|hint| hint.to_string())
}
