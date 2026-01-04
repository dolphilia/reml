use std::cell::Cell;

use crate::prelude::iter::{EffectLabels, EffectSet};

thread_local! {
    static IO_EFFECTS: Cell<EffectSet> = Cell::new(EffectSet::PURE);
    static WATCH_METRICS: Cell<WatchMetricsSnapshot> = Cell::new(WatchMetricsSnapshot::EMPTY);
}

/// ウォッチャーイベントのキュー指標。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WatchMetricsSnapshot {
    pub queue_size: u32,
    pub delay_ns: u64,
}

impl WatchMetricsSnapshot {
    pub const EMPTY: Self = Self {
        queue_size: 0,
        delay_ns: 0,
    };

    pub fn new(queue_size: usize, delay_ns: u64) -> Self {
        Self {
            queue_size: queue_size.min(u32::MAX as usize) as u32,
            delay_ns,
        }
    }
}

/// IO 操作が発生したことを記録する。
pub(crate) fn record_io_operation(_bytes: usize) {
    IO_EFFECTS.with(|slot| {
        let mut current = slot.get();
        current.mark_io_blocking();
        slot.set(current);
    });
}

/// fs.sync 系の操作を記録する。
pub(crate) fn record_fs_sync_operation() {
    IO_EFFECTS.with(|slot| {
        let mut current = slot.get();
        current.mark_fs_sync();
        slot.set(current);
    });
}

/// 非同期 IO 操作を記録する。
pub(crate) fn record_async_io_operation() {
    IO_EFFECTS.with(|slot| {
        let mut current = slot.get();
        current.mark_io_async();
        slot.set(current);
    });
}

/// セキュリティポリシー関連の効果を記録する。
#[allow(dead_code)]
pub(crate) fn record_security_event() {
    IO_EFFECTS.with(|slot| {
        let mut current = slot.get();
        current.mark_security();
        slot.set(current);
    });
}

pub(crate) fn record_buffer_allocation(bytes: usize) {
    record_mem_effect(bytes);
}

pub(crate) fn record_buffer_usage(bytes: usize) {
    record_mem_effect(bytes);
}

fn record_mem_effect(bytes: usize) {
    if bytes == 0 {
        return;
    }
    IO_EFFECTS.with(|slot| {
        let mut current = slot.get();
        current.mark_mem();
        current.record_mem_bytes(bytes);
        slot.set(current);
    });
}

/// 記録済みの効果を取り出し初期化する。テスト用のため `pub(crate)` とする。
#[allow(dead_code)]
pub(crate) fn take_recorded_effects() -> EffectSet {
    IO_EFFECTS.with(|slot| {
        let effects = slot.get();
        slot.set(EffectSet::PURE);
        effects
    })
}

/// IO API が記録した効果ラベルを取得し、内部状態をリセットする。
pub fn take_io_effects_snapshot() -> EffectLabels {
    take_recorded_effects().to_labels()
}

/// ウォッチャーのキュー統計を記録する。
pub(crate) fn record_watch_metrics(queue_size: usize, delay_ns: u64) {
    WATCH_METRICS.with(|slot| {
        slot.set(WatchMetricsSnapshot::new(queue_size, delay_ns));
    });
}

/// 記録済みのウォッチャー統計を取得しリセットする。
pub fn take_watch_metrics_snapshot() -> WatchMetricsSnapshot {
    WATCH_METRICS.with(|slot| {
        let snapshot = slot.get();
        slot.set(WatchMetricsSnapshot::EMPTY);
        snapshot
    })
}

pub(crate) fn blocking_io_effect_labels() -> EffectLabels {
    EffectLabels {
        mem: false,
        mutating: false,
        debug: false,
        async_pending: false,
        audit: false,
        cell: false,
        rc: false,
        unicode: false,
        io: true,
        io_blocking: true,
        io_async: false,
        security: false,
        transfer: false,
        fs_sync: false,
        mem_bytes: 0,
        predicate_calls: 0,
        rc_ops: 0,
        time: false,
        time_calls: 0,
        io_blocking_calls: 1,
        io_async_calls: 0,
        fs_sync_calls: 0,
        security_events: 0,
    }
}

pub(crate) fn fs_sync_effect_labels() -> EffectLabels {
    let mut labels = blocking_io_effect_labels();
    labels.fs_sync = true;
    labels.fs_sync_calls = 1;
    labels
}
