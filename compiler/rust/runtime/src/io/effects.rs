use std::cell::Cell;

use crate::prelude::iter::{EffectLabels, EffectSet};

thread_local! {
    static IO_EFFECTS: Cell<EffectSet> = Cell::new(EffectSet::PURE);
}

/// IO 操作が発生したことを記録する。
pub(crate) fn record_io_operation(_bytes: usize) {
    IO_EFFECTS.with(|slot| {
        let mut current = slot.get();
        current.mark_io_blocking();
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
pub(crate) fn record_security_event() {
    IO_EFFECTS.with(|slot| {
        let mut current = slot.get();
        current.mark_security();
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
        mem_bytes: 0,
        predicate_calls: 0,
        rc_ops: 0,
        time: false,
        time_calls: 0,
        io_blocking_calls: 1,
        io_async_calls: 0,
        security_events: 0,
    }
}
