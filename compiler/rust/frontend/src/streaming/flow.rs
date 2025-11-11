use std::sync::{Arc, Mutex};

use crate::diagnostic::DiagnosticBuilder;
use crate::parser::StreamingRecoverController;

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

#[derive(Debug, Clone, Copy, Default)]
pub struct StreamFlowMetrics {
    pub checkpoints_closed: u32,
}

#[derive(Debug, Clone)]
pub struct StreamFlowState {
    inner: Arc<Mutex<StreamFlowInner>>,
}

#[derive(Debug)]
struct StreamFlowInner {
    config: StreamFlowConfig,
    checkpoints_closed: u32,
}

impl StreamFlowState {
    pub fn new(config: StreamFlowConfig) -> Self {
        Self {
            inner: Arc::new(Mutex::new(StreamFlowInner {
                config,
                checkpoints_closed: 0,
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
        })
        .unwrap_or_default()
    }

    fn with_inner<T>(&self, f: impl FnOnce(&StreamFlowInner) -> T) -> Option<T> {
        self.inner.lock().ok().map(|inner| f(&inner))
    }
}
