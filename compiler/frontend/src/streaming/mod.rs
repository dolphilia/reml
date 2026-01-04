//! `core_parse_streaming` 相当の状態管理。
//!
//! Packrat キャッシュと `span_trace` の両機構を Rust 向けに再設計した骨格を提供する。
//! Packrat は `(ParserId, Range<u32>)` をキーに `IndexMap` で管理し、並列読み取りに
//! 対応するため `RwLock` で保護する。トレースは `VecDeque` に蓄積し、`trace_limit`
//! で上限を設ける。

pub mod flow;
pub use flow::{
    RuntimeBridgeSignal, RuntimeBridgeSignalKind, StreamFlowConfig, StreamFlowMetrics,
    StreamFlowState,
};

use crate::span::Span;
use indexmap::IndexMap;
use serde::Serialize;
use smallvec::SmallVec;
use smol_str::SmolStr;
use std::collections::VecDeque;
use std::ops::Range;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

/// Packrat キャッシュを有効化した際の既定バイト予算（4MiB）。
pub const DEFAULT_PACKRAT_BUDGET_BYTES: u64 = 4 * 1024 * 1024;

/// `span_trace` に保持するフレーム数の既定値。
pub const DEFAULT_TRACE_LIMIT: usize = 128;

/// ストリーミング状態の動作を制御する設定値。
#[derive(Debug, Clone)]
pub struct StreamingStateConfig {
    pub packrat_enabled: bool,
    pub packrat_budget_bytes: u64,
    pub trace_enabled: bool,
    pub trace_limit: usize,
}

impl Default for StreamingStateConfig {
    fn default() -> Self {
        Self {
            packrat_enabled: true,
            packrat_budget_bytes: DEFAULT_PACKRAT_BUDGET_BYTES,
            trace_enabled: true,
            trace_limit: DEFAULT_TRACE_LIMIT,
        }
    }
}

/// Packrat で収集した統計値。`collect-iterator-audit-metrics.py` から直接参照される。
#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct PackratStats {
    pub queries: u64,
    pub hits: u64,
    pub entries: u64,
    pub approx_bytes: u64,
    pub evictions: u64,
    pub pruned: u64,
    pub budget_drops: u64,
}

/// span_trace の統計値。CLI や診断で併記する。
#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct SpanTraceStats {
    pub retained: u64,
    pub dropped: u64,
}

/// Packrat と span_trace の統計値を束ねたスナップショット。
#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct StreamMetrics {
    pub packrat: PackratStats,
    pub span_trace: SpanTraceStats,
}

/// Packrat スナップショット。CLI `--emit parse-debug` 向けの簡易統計。
#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct PackratSnapshot {
    pub entries: u64,
    pub approx_bytes: u64,
}

/// Packrat キャッシュで保持するサンプルトークン。
#[derive(Debug, Clone, Serialize)]
pub struct TokenSample {
    pub kind: SmolStr,
    pub lexeme: SmolStr,
}

/// Packrat キャッシュに保存する期待値エントリ。
#[derive(Debug, Clone, Serialize)]
pub struct Expectation {
    pub description: SmolStr,
}

/// Packrat 要約。`Diagnostic.expectation_summary` と同役割。
#[derive(Debug, Clone, Serialize)]
pub struct ExpectationSummary {
    pub humanized: Option<SmolStr>,
    pub alternatives: Vec<SmolStr>,
}

/// Packrat キャッシュに保存する値。
#[derive(Debug, Clone, Serialize)]
pub struct PackratEntry {
    pub sample_tokens: SmallVec<[TokenSample; 8]>,
    pub expectations: Vec<Expectation>,
    pub summary: Option<ExpectationSummary>,
    approx_bytes: usize,
}

impl PackratEntry {
    pub fn new(
        sample_tokens: SmallVec<[TokenSample; 8]>,
        expectations: Vec<Expectation>,
        summary: Option<ExpectationSummary>,
    ) -> Self {
        let approx_bytes = Self::estimate_bytes(&sample_tokens, &expectations, summary.as_ref());
        Self {
            sample_tokens,
            expectations,
            summary,
            approx_bytes,
        }
    }

    pub fn approx_bytes(&self) -> usize {
        self.approx_bytes
    }

    fn estimate_bytes(
        tokens: &SmallVec<[TokenSample; 8]>,
        expectations: &[Expectation],
        summary: Option<&ExpectationSummary>,
    ) -> usize {
        let token_bytes: usize = tokens
            .iter()
            .map(|token| token.kind.len() + token.lexeme.len() + 16)
            .sum();
        let expectation_bytes: usize = expectations.iter().map(|e| e.description.len() + 24).sum();
        let summary_bytes = summary
            .map(|s| {
                let humanized = s.humanized.as_ref().map(|v| v.len()).unwrap_or(0);
                let alternatives = s.alternatives.iter().map(|alt| alt.len()).sum::<usize>();
                humanized + alternatives + 32
            })
            .unwrap_or(0);
        token_bytes + expectation_bytes + summary_bytes + 48
    }
}

/// Packrat キャッシュのキーデータとエントリをまとめたシリアライズ用レコード。
#[derive(Debug, Clone, Serialize)]
pub struct PackratCacheEntry {
    pub parser_id: u16,
    pub range_start: u32,
    pub range_end: u32,
    pub entry: PackratEntry,
}

/// span_trace の 1 フレーム。
#[derive(Debug, Clone, Serialize)]
pub struct TraceFrame {
    pub label: Option<SmolStr>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct PackratKey {
    parser_id: u16,
    range: Range<u32>,
}

impl PackratKey {
    fn new(parser_id: u16, range: Range<u32>) -> Self {
        Self { parser_id, range }
    }

    fn start(&self) -> u32 {
        self.range.start
    }
}

#[derive(Debug, Default)]
struct PackratMetricsAtomic {
    queries: AtomicU64,
    hits: AtomicU64,
    entries: AtomicU64,
    approx_bytes: AtomicU64,
    evictions: AtomicU64,
    pruned: AtomicU64,
    budget_drops: AtomicU64,
}

impl PackratMetricsAtomic {
    fn snapshot(&self) -> PackratStats {
        PackratStats {
            queries: self.queries.load(Ordering::Relaxed),
            hits: self.hits.load(Ordering::Relaxed),
            entries: self.entries.load(Ordering::Relaxed),
            approx_bytes: self.approx_bytes.load(Ordering::Relaxed),
            evictions: self.evictions.load(Ordering::Relaxed),
            pruned: self.pruned.load(Ordering::Relaxed),
            budget_drops: self.budget_drops.load(Ordering::Relaxed),
        }
    }

    fn observe_hit(&self) {
        self.queries.fetch_add(1, Ordering::Relaxed);
        self.hits.fetch_add(1, Ordering::Relaxed);
    }

    fn observe_miss(&self) {
        self.queries.fetch_add(1, Ordering::Relaxed);
    }

    fn record_entries(&self, len: usize) {
        self.entries.store(len as u64, Ordering::Relaxed);
    }

    fn add_bytes(&self, added: i64) {
        if added == 0 {
            return;
        }
        let mut current = self.approx_bytes.load(Ordering::Relaxed) as i64;
        loop {
            let next = (current + added).max(0);
            match self.approx_bytes.compare_exchange(
                current as u64,
                next as u64,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(actual) => current = actual as i64,
            }
        }
    }

    fn inc_evictions(&self, count: u64) {
        if count > 0 {
            self.evictions.fetch_add(count, Ordering::Relaxed);
        }
    }

    fn inc_pruned(&self, count: u64) {
        if count > 0 {
            self.pruned.fetch_add(count, Ordering::Relaxed);
        }
    }

    fn inc_budget_drops(&self, count: u64) {
        if count > 0 {
            self.budget_drops.fetch_add(count, Ordering::Relaxed);
        }
    }
}

#[derive(Debug, Default)]
struct SpanMetricsAtomic {
    retained: AtomicU64,
    dropped: AtomicU64,
}

impl SpanMetricsAtomic {
    fn snapshot(&self) -> SpanTraceStats {
        SpanTraceStats {
            retained: self.retained.load(Ordering::Relaxed),
            dropped: self.dropped.load(Ordering::Relaxed),
        }
    }

    fn inc_retained(&self) {
        self.retained.fetch_add(1, Ordering::Relaxed);
    }

    fn inc_dropped(&self) {
        self.dropped.fetch_add(1, Ordering::Relaxed);
    }
}

#[derive(Debug)]
struct StreamingStateShared {
    packrat_cache: RwLock<IndexMap<PackratKey, PackratEntry>>,
    span_trace: RwLock<VecDeque<TraceFrame>>,
    packrat_metrics: PackratMetricsAtomic,
    span_metrics: SpanMetricsAtomic,
}

impl Default for StreamingStateShared {
    fn default() -> Self {
        Self {
            packrat_cache: RwLock::new(IndexMap::new()),
            span_trace: RwLock::new(VecDeque::new()),
            packrat_metrics: PackratMetricsAtomic::default(),
            span_metrics: SpanMetricsAtomic::default(),
        }
    }
}

/// ストリーミング解析状態。`Arc` で共有可能。
#[derive(Debug, Clone)]
pub struct StreamingState {
    shared: Arc<StreamingStateShared>,
    config: StreamingStateConfig,
}

impl Default for StreamingState {
    fn default() -> Self {
        Self::new(StreamingStateConfig::default())
    }
}

impl StreamingState {
    pub fn new(config: StreamingStateConfig) -> Self {
        Self {
            shared: Arc::new(StreamingStateShared::default()),
            config,
        }
    }

    pub fn config(&self) -> &StreamingStateConfig {
        &self.config
    }

    pub fn packrat_enabled(&self) -> bool {
        self.config.packrat_enabled
    }

    pub fn trace_enabled(&self) -> bool {
        self.config.trace_enabled
    }

    /// Packrat キャッシュを検索し、ヒット/ミスを記録する。
    pub fn lookup_packrat(&self, parser_id: u16, range: Range<u32>) -> Option<PackratEntry> {
        if !self.config.packrat_enabled {
            return None;
        }
        let key = PackratKey::new(parser_id, range);
        let cache = self.shared.packrat_cache.read().ok()?;
        let result = cache.get(&key).cloned();
        drop(cache);
        if result.is_some() {
            self.shared.packrat_metrics.observe_hit();
        } else {
            self.shared.packrat_metrics.observe_miss();
        }
        result
    }

    /// Packrat キャッシュへ値を保存し、バイト予算超過時は古い要素を削除する。
    pub fn store_packrat(&self, parser_id: u16, range: Range<u32>, entry: PackratEntry) {
        if !self.config.packrat_enabled {
            return;
        }
        let key = PackratKey::new(parser_id, range);
        let mut cache = match self.shared.packrat_cache.write() {
            Ok(lock) => lock,
            Err(poisoned) => poisoned.into_inner(),
        };
        let added_bytes = entry.approx_bytes() as i64;
        let replaced_bytes = cache
            .insert(key, entry)
            .map(|previous| previous.approx_bytes() as i64)
            .unwrap_or(0);
        if replaced_bytes > 0 {
            self.shared.packrat_metrics.inc_evictions(1);
        }
        self.shared
            .packrat_metrics
            .add_bytes(added_bytes - replaced_bytes);
        self.shared.packrat_metrics.record_entries(cache.len());

        let mut evicted = 0u64;
        while self.config.packrat_budget_bytes > 0 {
            let current_bytes = self.shared.packrat_metrics.snapshot().approx_bytes;
            if current_bytes <= self.config.packrat_budget_bytes {
                break;
            }
            if let Some((_, removed)) = cache.shift_remove_index(0) {
                self.shared
                    .packrat_metrics
                    .add_bytes(-(removed.approx_bytes() as i64));
                evicted += 1;
            } else {
                break;
            }
        }
        if evicted > 0 {
            self.shared.packrat_metrics.record_entries(cache.len());
            self.shared.packrat_metrics.inc_evictions(evicted);
            self.shared.packrat_metrics.inc_budget_drops(evicted);
        }
    }

    /// 指定オフセットより前の Packrat エントリを削除する。
    pub fn prune_packrat_before(&self, offset: u32) -> usize {
        if !self.config.packrat_enabled {
            return 0;
        }
        let mut cache = match self.shared.packrat_cache.write() {
            Ok(lock) => lock,
            Err(poisoned) => poisoned.into_inner(),
        };
        let mut removed = 0usize;
        let mut released_bytes: i64 = 0;
        cache.retain(|key, value| {
            let retain = key.start() >= offset;
            if !retain {
                removed += 1;
                released_bytes += value.approx_bytes() as i64;
            }
            retain
        });
        if removed > 0 {
            self.shared.packrat_metrics.record_entries(cache.len());
            self.shared.packrat_metrics.add_bytes(-released_bytes);
            self.shared.packrat_metrics.inc_pruned(removed as u64);
            self.shared.packrat_metrics.inc_evictions(removed as u64);
        }
        removed
    }

    /// Packrat 全体のスナップショットを取得する。
    pub fn packrat_snapshot(&self) -> PackratSnapshot {
        let stats = self.shared.packrat_metrics.snapshot();
        PackratSnapshot {
            entries: stats.entries,
            approx_bytes: stats.approx_bytes,
        }
    }

    /// Packrat キャッシュ内の全エントリをコピーして取得する。
    pub fn packrat_cache_entries(&self) -> Option<Vec<PackratCacheEntry>> {
        if !self.config.packrat_enabled {
            return None;
        }
        let cache = match self.shared.packrat_cache.read() {
            Ok(lock) => lock,
            Err(poisoned) => poisoned.into_inner(),
        };
        let entries = cache
            .iter()
            .map(|(key, entry)| PackratCacheEntry {
                parser_id: key.parser_id,
                range_start: key.range.start,
                range_end: key.range.end,
                entry: entry.clone(),
            })
            .collect::<Vec<_>>();
        Some(entries)
    }

    /// Packrat 統計値を取得する。
    pub fn packrat_stats(&self) -> PackratStats {
        self.shared.packrat_metrics.snapshot()
    }

    /// span_trace にフレームを追加する。
    pub fn push_span_trace(&self, label: Option<SmolStr>, span: Span) {
        if !self.config.trace_enabled {
            return;
        }
        let mut trace = match self.shared.span_trace.write() {
            Ok(lock) => lock,
            Err(poisoned) => poisoned.into_inner(),
        };
        trace.push_back(TraceFrame { label, span });
        self.shared.span_metrics.inc_retained();
        let limit = self.config.trace_limit.max(1);
        while trace.len() > limit {
            trace.pop_front();
            self.shared.span_metrics.inc_dropped();
        }
    }

    /// span_trace を取り出してクリアする。
    pub fn drain_span_trace(&self) -> Vec<TraceFrame> {
        if !self.config.trace_enabled {
            return Vec::new();
        }
        let mut trace = match self.shared.span_trace.write() {
            Ok(lock) => lock,
            Err(poisoned) => poisoned.into_inner(),
        };
        trace.drain(..).collect()
    }

    /// span_trace の統計値を取得する。
    pub fn span_trace_stats(&self) -> SpanTraceStats {
        self.shared.span_metrics.snapshot()
    }

    /// Packrat と span_trace の統計値をまとめたスナップショットを取得する。
    pub fn metrics_snapshot(&self) -> StreamMetrics {
        StreamMetrics {
            packrat: self.packrat_stats(),
            span_trace: self.span_trace_stats(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_entry(bytes: usize) -> PackratEntry {
        let lexeme = SmolStr::new_inline("token");
        let mut tokens = SmallVec::<[TokenSample; 8]>::new();
        tokens.push(TokenSample {
            kind: SmolStr::new_inline("ident"),
            lexeme,
        });
        let expectations = vec![Expectation {
            description: SmolStr::new_inline("identifier"),
        }];
        let summary = Some(ExpectationSummary {
            humanized: Some(SmolStr::new_inline("identifier expected")),
            alternatives: vec![SmolStr::new_inline("IDENT")],
        });
        let entry = PackratEntry::new(tokens, expectations, summary);
        // `approx_bytes` はおおよそ 100 以上になるはずだが、テストではカスタム値で補正する。
        if bytes == 0 {
            entry
        } else {
            PackratEntry {
                approx_bytes: bytes,
                ..entry
            }
        }
    }

    #[test]
    fn lookup_updates_queries_and_hits() {
        let state = StreamingState::default();
        let key_range = 0..8;
        state.store_packrat(1, key_range.clone(), sample_entry(64));
        assert!(state.lookup_packrat(1, key_range.clone()).is_some());
        assert!(state.lookup_packrat(1, 8..16).is_none());
        let stats = state.packrat_stats();
        assert_eq!(stats.queries, 2);
        assert_eq!(stats.hits, 1);
    }

    #[test]
    fn enforce_budget() {
        let config = StreamingStateConfig {
            packrat_enabled: true,
            packrat_budget_bytes: 64,
            trace_enabled: false,
            trace_limit: DEFAULT_TRACE_LIMIT,
        };
        let state = StreamingState::new(config);
        for idx in 0..4 {
            state.store_packrat(1, (idx * 10)..(idx * 10 + 5), sample_entry(48));
        }
        let stats = state.packrat_stats();
        assert!(stats.approx_bytes <= 64);
        assert!(stats.budget_drops > 0);
    }

    #[test]
    fn prune_removes_older_entries() {
        let state = StreamingState::default();
        state.store_packrat(1, 0..5, sample_entry(32));
        state.store_packrat(1, 10..15, sample_entry(32));
        let removed = state.prune_packrat_before(5);
        assert_eq!(removed, 1);
        let stats = state.packrat_stats();
        assert_eq!(stats.entries, 1);
    }

    #[test]
    fn span_trace_respects_limit() {
        let config = StreamingStateConfig {
            packrat_enabled: false,
            packrat_budget_bytes: 0,
            trace_enabled: true,
            trace_limit: 2,
        };
        let state = StreamingState::new(config);
        state.push_span_trace(None, Span::new(0, 1));
        state.push_span_trace(None, Span::new(1, 2));
        state.push_span_trace(None, Span::new(2, 3));
        assert_eq!(state.drain_span_trace().len(), 2);
        let stats = state.span_trace_stats();
        assert_eq!(stats.retained, 3);
        assert_eq!(stats.dropped, 1);
    }

    #[test]
    fn packrat_stats_are_serializable() {
        let state = StreamingState::default();
        state.store_packrat(1, 0..5, sample_entry(32));
        let stats = state.packrat_stats();
        let value = serde_json::to_value(stats).expect("serialize stats");
        assert!(value.get("queries").is_some());
        assert!(value.get("hits").is_some());
    }
}
