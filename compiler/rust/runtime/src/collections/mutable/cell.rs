use std::cell::{Cell as EffectCell, UnsafeCell};
use std::fmt;

#[cfg(feature = "core_prelude")]
use crate::core_prelude::collectors::CollectorEffectMarkers;
#[cfg(feature = "core_prelude")]
use crate::core_prelude::iter::{EffectLabels, EffectSet};
#[cfg(not(feature = "core_prelude"))]
use crate::prelude::collectors::CollectorEffectMarkers;
#[cfg(not(feature = "core_prelude"))]
use crate::prelude::iter::{EffectLabels, EffectSet};

/// 内部可変性を提供する軽量コンテナ。
pub struct Cell<T: Copy> {
    inner: UnsafeCell<T>,
}

impl<T: Copy> Cell<T> {
    /// 新しい `Cell` を生成する。
    pub fn new(value: T) -> Self {
        Self {
            inner: UnsafeCell::new(value),
        }
    }

    /// 格納されている値を取得する。
    pub fn get(&self) -> T {
        // `Cell` は Copy 制約付きなので安全に読み出せる。
        unsafe { *self.inner.get() }
    }

    /// 値を上書きする。
    pub fn set(&self, value: T) {
        unsafe {
            *self.inner.get() = value;
        }
    }

    /// 値を置き換えて古い値を返す。
    pub fn replace(&self, value: T) -> T {
        let old = self.get();
        self.set(value);
        old
    }

    /// 内部値を取り出す。
    pub fn into_inner(self) -> T {
        self.inner.into_inner()
    }
}

impl<T: Copy + Default> Default for Cell<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T: Copy> fmt::Debug for Cell<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Cell").field("value", &self.get()).finish()
    }
}

unsafe impl<T: Copy + Send> Send for Cell<T> {}
unsafe impl<T: Copy + Send + Sync> Sync for Cell<T> {}

/// 効果計測付きの `Cell`。
pub struct EffectfulCell<T: Copy> {
    cell: Cell<T>,
    effects: EffectCell<EffectSet>,
}

impl<T: Copy> EffectfulCell<T> {
    /// 新しい `EffectfulCell` を生成する。
    pub fn new(value: T) -> Self {
        Self {
            cell: Cell::new(value),
            effects: EffectCell::new(EffectSet::PURE),
        }
    }

    /// 値を取得する。
    pub fn get(&self) -> T {
        self.cell.get()
    }

    fn record_cell_effect(&self) {
        let mut effects = self.effects.get();
        effects.mark_mut();
        effects.mark_cell();
        self.effects.set(effects);
    }

    /// 値を設定し effect を記録する。
    pub fn set(&self, value: T) {
        self.record_cell_effect();
        self.cell.set(value);
    }

    /// 値を置き換え effect を記録する。
    pub fn replace(&self, value: T) -> T {
        self.record_cell_effect();
        self.cell.replace(value)
    }

    /// 効果ラベルを取得する。
    pub fn effect_labels(&self) -> EffectLabels {
        self.effects.get().to_labels()
    }

    /// Collector 監査で `Cell` 操作が行われたことを記録する。
    pub fn record_cell_op(&self, markers: &mut CollectorEffectMarkers) {
        self.record_cell_effect();
        markers.record_cell_op();
    }

    /// 内部の `Cell` を取り出す。
    pub fn into_parts(self) -> (Cell<T>, EffectSet) {
        (self.cell, self.effects.into_inner())
    }
}

impl<T: Copy + Default> Default for EffectfulCell<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T: Copy> fmt::Debug for EffectfulCell<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EffectfulCell")
            .field("value", &self.get())
            .field("effects", &self.effects.get())
            .finish()
    }
}
