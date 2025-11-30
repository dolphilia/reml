use std::path::PathBuf;

use super::effects::WatchMetricsSnapshot;
use crate::prelude::iter::EffectLabels;
#[cfg(any(feature = "core_time", feature = "metrics"))]
use crate::time::{self, Timestamp};
#[cfg(not(any(feature = "core_time", feature = "metrics")))]
use std::time::SystemTime as Timestamp;

/// IO 操作の文脈情報。
#[derive(Debug, Clone)]
pub struct IoContext {
    operation: &'static str,
    path: Option<PathBuf>,
    capability: Option<&'static str>,
    bytes_processed: Option<u64>,
    timestamp: Timestamp,
    effects: EffectLabels,
    buffer: Option<BufferStats>,
    watch: Option<WatchStats>,
    glob: Option<GlobStats>,
}

impl IoContext {
    pub fn new(operation: &'static str) -> Self {
        Self {
            operation,
            path: None,
            capability: None,
            bytes_processed: None,
            timestamp: current_timestamp(),
            effects: empty_effect_labels(),
            buffer: None,
            watch: None,
            glob: None,
        }
    }

    pub fn with_bytes_processed(mut self, bytes: u64) -> Self {
        self.bytes_processed = Some(bytes);
        self
    }

    pub fn with_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.path = Some(path.into());
        self
    }

    pub fn with_capability(mut self, capability: &'static str) -> Self {
        self.capability = Some(capability);
        self
    }

    pub fn with_timestamp(mut self, timestamp: Timestamp) -> Self {
        self.timestamp = timestamp;
        self
    }

    pub fn set_timestamp(&mut self, timestamp: Timestamp) {
        self.timestamp = timestamp;
    }

    pub fn set_path(&mut self, path: PathBuf) {
        self.path = Some(path);
    }

    pub fn operation(&self) -> &'static str {
        self.operation
    }

    pub fn path(&self) -> Option<&PathBuf> {
        self.path.as_ref()
    }

    pub fn capability(&self) -> Option<&'static str> {
        self.capability
    }

    pub fn bytes_processed(&self) -> Option<u64> {
        self.bytes_processed
    }

    pub fn timestamp(&self) -> Timestamp {
        self.timestamp
    }

    pub fn effects(&self) -> EffectLabels {
        self.effects
    }

    pub fn with_effects(mut self, effects: EffectLabels) -> Self {
        self.effects = effects;
        self
    }

    pub fn set_effects(&mut self, effects: EffectLabels) {
        self.effects = effects;
    }

    pub fn buffer(&self) -> Option<&BufferStats> {
        self.buffer.as_ref()
    }

    pub fn watch_stats(&self) -> Option<&WatchStats> {
        self.watch.as_ref()
    }

    pub fn with_buffer_stats(mut self, stats: BufferStats) -> Self {
        self.buffer = Some(stats);
        self
    }

    pub fn set_buffer_stats(&mut self, stats: BufferStats) {
        self.buffer = Some(stats);
    }

    pub fn update_buffer_usage(&mut self, capacity: usize, fill: usize) {
        let stats = self
            .buffer
            .get_or_insert_with(|| BufferStats::new(capacity));
        stats.update(capacity, fill);
    }

    pub fn with_watch_stats_from_metrics(mut self, metrics: WatchMetricsSnapshot) -> Self {
        self.watch = Some(WatchStats::from(metrics));
        self
    }

    pub fn set_watch_stats_from_metrics(&mut self, metrics: WatchMetricsSnapshot) {
        self.watch = Some(WatchStats::from(metrics));
    }

    pub fn set_watch_stats(&mut self, stats: WatchStats) {
        self.watch = Some(stats);
    }

    pub fn glob(&self) -> Option<&GlobStats> {
        self.glob.as_ref()
    }

    pub fn with_glob_pattern(mut self, pattern: impl Into<String>) -> Self {
        self.glob = Some(GlobStats::new(pattern));
        self
    }

    pub fn set_glob_pattern(&mut self, pattern: impl Into<String>) {
        match self.glob.as_mut() {
            Some(glob) => glob.set_pattern(pattern),
            None => self.glob = Some(GlobStats::new(pattern)),
        }
    }

    pub fn set_glob_offending_path(&mut self, path: impl Into<String>) {
        if let Some(glob) = self.glob.as_mut() {
            glob.set_offending_path(path);
        } else {
            let mut stats = GlobStats::new(String::new());
            stats.set_offending_path(path);
            self.glob = Some(stats);
        }
    }
}

#[derive(Debug, Clone)]
pub struct BufferStats {
    capacity: u32,
    fill: u32,
    last_fill_timestamp: Timestamp,
}

impl BufferStats {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.min(u32::MAX as usize) as u32,
            fill: 0,
            last_fill_timestamp: current_timestamp(),
        }
    }

    fn update(&mut self, capacity: usize, fill: usize) {
        self.capacity = capacity.min(u32::MAX as usize) as u32;
        self.fill = fill.min(self.capacity as usize) as u32;
        self.last_fill_timestamp = current_timestamp();
    }

    pub fn capacity(&self) -> u32 {
        self.capacity
    }

    pub fn fill(&self) -> u32 {
        self.fill
    }

    pub fn last_fill_timestamp(&self) -> Timestamp {
        self.last_fill_timestamp
    }
}

fn empty_effect_labels() -> EffectLabels {
    EffectLabels {
        mem: false,
        mutating: false,
        debug: false,
        async_pending: false,
        audit: false,
        cell: false,
        rc: false,
        unicode: false,
        io: false,
        io_blocking: false,
        io_async: false,
        security: false,
        transfer: false,
        fs_sync: false,
        mem_bytes: 0,
        predicate_calls: 0,
        rc_ops: 0,
        time: false,
        time_calls: 0,
        io_blocking_calls: 0,
        io_async_calls: 0,
        fs_sync_calls: 0,
        security_events: 0,
    }
}

#[derive(Debug, Clone)]
pub struct GlobStats {
    pattern: String,
    offending_path: Option<String>,
}

impl GlobStats {
    pub fn new(pattern: impl Into<String>) -> Self {
        Self {
            pattern: pattern.into(),
            offending_path: None,
        }
    }

    pub fn pattern(&self) -> &str {
        &self.pattern
    }

    pub fn offending_path(&self) -> Option<&str> {
        self.offending_path.as_deref()
    }

    pub fn set_pattern(&mut self, pattern: impl Into<String>) {
        self.pattern = pattern.into();
    }

    pub fn set_offending_path(&mut self, path: impl Into<String>) {
        self.offending_path = Some(path.into());
    }
}

fn current_timestamp() -> Timestamp {
    #[cfg(any(feature = "core_time", feature = "metrics"))]
    {
        time::now().unwrap_or_else(|_| Timestamp::unix_epoch())
    }
    #[cfg(not(any(feature = "core_time", feature = "metrics")))]
    {
        Timestamp::now()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct WatchStats {
    queue_size: u32,
    delay_ns: u64,
}

impl WatchStats {
    pub fn new(queue_size: u32, delay_ns: u64) -> Self {
        Self {
            queue_size,
            delay_ns,
        }
    }

    pub fn queue_size(&self) -> u32 {
        self.queue_size
    }

    pub fn delay_ns(&self) -> u64 {
        self.delay_ns
    }
}

impl From<WatchMetricsSnapshot> for WatchStats {
    fn from(value: WatchMetricsSnapshot) -> Self {
        WatchStats {
            queue_size: value.queue_size,
            delay_ns: value.delay_ns,
        }
    }
}
