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

/// IO モジュールなど別 ThreadLocal の計測結果を統合する。
pub(crate) fn merge_effects(extra: EffectSet) {
    if extra == EffectSet::PURE {
        return;
    }
    TEXT_EFFECTS.with(|slot| {
        let mut current = slot.get();
        current = current.union(extra);
        slot.set(current);
    });
}

/// ゼロコピー転送が発生したことを記録する。
pub(crate) fn record_transfer() {
    TEXT_EFFECTS.with(|slot| {
        let mut current = slot.get();
        current.mark_transfer();
        slot.set(current);
    });
}

/// Unicode 変換イベントを記録する。
pub(crate) fn record_unicode_event(bytes: usize) {
    let _ = bytes;
    TEXT_EFFECTS.with(|slot| {
        let mut current = slot.get();
        current.mark_unicode();
        slot.set(current);
    });
}

/// 監査ログ連携を行ったことを記録する。`log_grapheme_stats` などで利用する。
pub(crate) fn record_audit_event() {
    TEXT_EFFECTS.with(|slot| {
        let mut current = slot.get();
        current.mark_audit();
        slot.set(current);
    });
}
