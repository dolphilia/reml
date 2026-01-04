use std::cell::Cell;

use crate::prelude::iter::EffectSet;

thread_local! {
    static NUMERIC_EFFECTS: Cell<EffectSet> = Cell::new(EffectSet::PURE);
}

/// メモリ確保が発生した際に `effect {mem}` を記録する。
pub(crate) fn record_mem_copy(bytes: usize) {
    if bytes == 0 {
        return;
    }
    NUMERIC_EFFECTS.with(|slot| {
        let mut current = slot.get();
        current.mark_mem();
        current.record_mem_bytes(bytes);
        slot.set(current);
    });
}

/// 記録済みの効果情報を取得してリセットする。
pub(crate) fn take_recorded_effects() -> EffectSet {
    NUMERIC_EFFECTS.with(|slot| {
        let effects = slot.get();
        slot.set(EffectSet::PURE);
        effects
    })
}
