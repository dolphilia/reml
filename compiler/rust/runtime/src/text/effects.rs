use std::cell::Cell;

use crate::prelude::iter::EffectSet;

thread_local! {
  static TEXT_EFFECTS: Cell<EffectSet> = Cell::new(EffectSet::PURE);
}

/// メモリコピー系の処理で発生した `effect {mem}` を記録する。
pub(crate) fn record_mem_copy(bytes: usize) {
  if bytes == 0 {
    return;
  }
  TEXT_EFFECTS.with(|slot| {
    let mut current = slot.get();
    current.mark_mem();
    current.record_mem_bytes(bytes);
    slot.set(current);
  });
}

/// 記録済みの効果を取得してリセットする。テストおよび将来の監査ブリッジ向け。
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn take_recorded_effects() -> EffectSet {
    TEXT_EFFECTS.with(|slot| {
        let effects = slot.get();
        slot.set(EffectSet::PURE);
        effects
    })
}
