use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use crate::diagnostic::DiagnosticBuilder;
use crate::parser::StreamingRecoverController;
use crate::typeck::StageTraceStep;
use serde::Serialize;

/// Stream ランタイム設定（CLI フラグ由来）をまとめた構造体。
#[derive(Debug, Clone, Default)]
pub struct StreamFlowConfig {
    pub enabled: bool,
    pub packrat_enabled: bool,
    pub resume_hint: Option<String>,
    pub checkpoint: Option<String>,
    pub flow_policy: Option<String>,
    pub flow_max_lag: Option<u64>,
    pub demand_min_bytes: Option<u64>,
    pub demand_preferred_bytes: Option<u64>,
}

const BRIDGE_SIGNAL_HISTORY_LIMIT: usize = 16;

#[derive(Debug, Clone, Copy, Default)]
pub struct StreamFlowMetrics {
    pub checkpoints_closed: u32,
    pub await_count: u32,
    pub resume_count: u32,
    pub backpressure_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RuntimeBridgeSignalKind {
    Await,
    Resume,
    Backpressure,
}

impl RuntimeBridgeSignalKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            RuntimeBridgeSignalKind::Await => "await",
            RuntimeBridgeSignalKind::Resume => "resume",
            RuntimeBridgeSignalKind::Backpressure => "backpressure",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeBridgeSignal {
    pub kind: RuntimeBridgeSignalKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parser_offset: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_sequence: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capability: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub stage_trace: Vec<StageTraceStep>,
}

impl RuntimeBridgeSignal {
    pub fn normalized_reason(&self) -> String {
        self.note
            .as_deref()
            .map(|note| note.to_owned())
            .unwrap_or_else(|| self.kind.as_str().to_string())
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct BridgeMetrics {
    await_count: u32,
    resume_count: u32,
    backpressure_count: u32,
}

impl BridgeMetrics {
    fn record(&mut self, kind: RuntimeBridgeSignalKind) {
        match kind {
            RuntimeBridgeSignalKind::Await => self.await_count = self.await_count.saturating_add(1),
            RuntimeBridgeSignalKind::Resume => {
                self.resume_count = self.resume_count.saturating_add(1)
            }
            RuntimeBridgeSignalKind::Backpressure => {
                self.backpressure_count = self.backpressure_count.saturating_add(1)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct StreamFlowState {
    inner: Arc<Mutex<StreamFlowInner>>,
}

#[derive(Debug)]
struct StreamFlowInner {
    config: StreamFlowConfig,
    checkpoints_closed: u32,
    bridge_metrics: BridgeMetrics,
    bridge_signals: VecDeque<RuntimeBridgeSignal>,
}

impl StreamFlowState {
    pub fn new(config: StreamFlowConfig) -> Self {
        Self {
            inner: Arc::new(Mutex::new(StreamFlowInner {
                config,
                checkpoints_closed: 0,
                bridge_metrics: BridgeMetrics::default(),
                bridge_signals: VecDeque::new(),
            })),
        }
    }

    pub fn enabled(&self) -> bool {
        self.with_inner(|inner| inner.config.enabled)
            .unwrap_or(false)
    }

    pub fn config(&self) -> Option<StreamFlowConfig> {
        self.with_inner(|inner| inner.config.clone())
    }

    pub fn checkpoint_end(
        &self,
        controller: &mut StreamingRecoverController,
        diagnostics: &mut DiagnosticBuilder,
    ) {
        controller.checkpoint_end(diagnostics);
        if let Ok(mut inner) = self.inner.lock() {
            inner.checkpoints_closed += 1;
        }
    }

    pub fn metrics(&self) -> StreamFlowMetrics {
        self.with_inner(|inner| StreamFlowMetrics {
            checkpoints_closed: inner.checkpoints_closed,
            await_count: inner.bridge_metrics.await_count,
            resume_count: inner.bridge_metrics.resume_count,
            backpressure_count: inner.bridge_metrics.backpressure_count,
        })
        .unwrap_or_default()
    }

    pub fn record_bridge_signal(&self, signal: RuntimeBridgeSignal) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.record_bridge_signal(signal);
        }
    }

    pub fn latest_bridge_signal(&self) -> Option<RuntimeBridgeSignal> {
        self.with_inner(|inner| inner.bridge_signals.back().cloned())
            .flatten()
    }

    fn with_inner<T>(&self, f: impl FnOnce(&StreamFlowInner) -> T) -> Option<T> {
        self.inner.lock().ok().map(|inner| f(&inner))
    }
}

impl StreamFlowInner {
    fn record_bridge_signal(&mut self, signal: RuntimeBridgeSignal) {
        self.bridge_metrics.record(signal.kind);
        self.bridge_signals.push_back(signal);
        if self.bridge_signals.len() > BRIDGE_SIGNAL_HISTORY_LIMIT {
            self.bridge_signals.pop_front();
        }
    }
}
