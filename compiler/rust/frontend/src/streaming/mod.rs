//! `core_parse_streaming` 相当の状態管理スケルトン。

use crate::span::Span;

/// Packrat キャッシュのメトリクス。P1 W5 までに実測値を収集する。
#[derive(Debug, Default, Clone)]
pub struct StreamMetrics {
    pub cache_hit: u64,
    pub cache_miss: u64,
    pub replay_count: u64,
}

/// ストリーミング解析で使用するチェックポイント。
#[derive(Debug, Clone)]
pub struct StreamCheckpoint {
    pub position: Span,
    pub committed: bool,
}

/// ストリーミング解析全体の状態。
#[derive(Debug, Default)]
pub struct StreamingState {
    pub metrics: StreamMetrics,
    pub checkpoints: Vec<StreamCheckpoint>,
}

impl StreamingState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Packrat キャッシュのヒット回数を増加させる。
    pub fn record_cache_hit(&mut self) {
        self.metrics.cache_hit = self.metrics.cache_hit.saturating_add(1);
    }

    /// Packrat キャッシュのミス回数を増加させる。
    pub fn record_cache_miss(&mut self) {
        self.metrics.cache_miss = self.metrics.cache_miss.saturating_add(1);
    }

    pub fn push_checkpoint(&mut self, span: Span) {
        self.checkpoints.push(StreamCheckpoint {
            position: span,
            committed: false,
        });
    }

    pub fn commit_last(&mut self) {
        if let Some(last) = self.checkpoints.last_mut() {
            last.committed = true;
        }
    }
}
