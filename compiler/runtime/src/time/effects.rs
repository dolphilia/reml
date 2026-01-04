use std::cell::Cell;
use std::time::Duration as StdDuration;

use crate::prelude::iter::EffectSet;
use serde::{Deserialize, Serialize};

thread_local! {
    static TIME_EFFECTS: Cell<EffectSet> = Cell::new(EffectSet::PURE);
    static TIME_METRICS: Cell<TimeSyscallMetrics> = Cell::new(TimeSyscallMetrics::default());
}

/// 時刻 API が観測したシステムコール統計。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct TimeSyscallMetrics {
    pub calls: u64,
    pub total_latency_ns: u128,
    pub max_latency_ns: u128,
}

impl TimeSyscallMetrics {
    /// 平均レイテンシを返す（記録が無い場合は `None`）。
    pub fn average_latency_ns(&self) -> Option<u128> {
        if self.calls == 0 {
            return None;
        }
        Some(self.total_latency_ns / self.calls as u128)
    }
}

/// `effect {time}` を記録し、Syscall メトリクスを更新する。
pub(crate) fn record_time_call(latency: StdDuration) {
    TIME_EFFECTS.with(|slot| {
        let mut current = slot.get();
        current.record_time_call();
        slot.set(current);
    });

    let latency_ns = latency.as_nanos();
    TIME_METRICS.with(|slot| {
        let mut metrics = slot.get();
        metrics.calls = metrics.calls.saturating_add(1);
        metrics.total_latency_ns = metrics.total_latency_ns.saturating_add(latency_ns);
        metrics.max_latency_ns = metrics.max_latency_ns.max(latency_ns);
        slot.set(metrics);
    });
}

pub(crate) fn take_recorded_effects() -> EffectSet {
    TIME_EFFECTS.with(|slot| {
        let effects = slot.get();
        slot.set(EffectSet::PURE);
        effects
    })
}

pub(crate) fn take_syscall_metrics() -> TimeSyscallMetrics {
    TIME_METRICS.with(|slot| {
        let snapshot = slot.get();
        slot.set(TimeSyscallMetrics::default());
        snapshot
    })
}
