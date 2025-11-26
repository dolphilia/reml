use std::cell::Cell;

use crate::prelude::iter::EffectSet;

thread_local! {
    static IO_EFFECTS: Cell<EffectSet> = Cell::new(EffectSet::PURE);
}

/// IO 操作が発生したことを記録する。
pub(crate) fn record_io_operation(_bytes: usize) {
    IO_EFFECTS.with(|slot| {
        let mut current = slot.get();
        current.mark_io();
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
