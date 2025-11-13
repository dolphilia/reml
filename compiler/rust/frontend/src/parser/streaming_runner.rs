use crate::parser::api::{ParseResult, RunConfig};
use crate::parser::ast::Module;
use crate::parser::{ParserDriver, ParserOptions};
use crate::streaming::{StreamFlowMetrics, StreamFlowState, StreamMetrics};

/// ストリーミング処理のメタ情報。
#[derive(Debug, Clone)]
pub struct StreamMeta {
    pub metrics: StreamMetrics,
    pub flow: StreamFlowMetrics,
}

impl StreamMeta {
    fn from_result(result: &ParseResult<Module>, flow: &StreamFlowState) -> Self {
        Self {
            metrics: result.stream_metrics.clone(),
            flow: flow.metrics(),
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
            continuation: Continuation {
                buffer,
                parser_options,
                run_config,
                stream_flow,
            },
        }
    }

    pub fn run_stream(self) -> StreamOutcome {
        run_stream_from_continuation(self.continuation)
    }

    pub fn resume(mut self, more: &str) -> StreamOutcome {
        self.continuation.buffer.push_str(more);
        run_stream_from_continuation(self.continuation)
    }
}

fn run_stream_from_continuation(continuation: Continuation) -> StreamOutcome {
    let Continuation {
        buffer,
        parser_options,
        run_config,
        stream_flow,
    } = continuation;
    let mut options = parser_options;
    options.stream_flow = Some(stream_flow.clone());
    let result =
        ParserDriver::parse_with_options_and_run_config(&buffer, options, run_config.clone());
    let meta = StreamMeta::from_result(&result, &stream_flow);
    StreamOutcome::Completed { result, meta }
}
