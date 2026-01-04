use std::{cmp, collections::TryReserveError, iter::FromIterator, mem, slice};

use crate::collections::persistent::list::List;
#[cfg(feature = "core_prelude")]
use crate::core_prelude::collectors::{CollectError, Collector, VecCollector};
#[cfg(feature = "core_prelude")]
use crate::core_prelude::iter::{EffectLabels, EffectSet};
#[cfg(not(feature = "core_prelude"))]
use crate::prelude::collectors::{CollectError, Collector, VecCollector};
#[cfg(not(feature = "core_prelude"))]
use crate::prelude::iter::{EffectLabels, EffectSet};

pub mod error;

/// ランタイムが公開する標準 `Vec` 型。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CoreVec<T> {
    inner: Vec<T>,
}

impl<T> CoreVec<T> {
    /// 空の `Vec` を生成する。
    pub fn new() -> Self {
        Self { inner: Vec::new() }
    }

    /// 事前確保付きで生成する。
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            inner: Vec::with_capacity(capacity),
        }
    }

    pub(crate) fn bytes_for(len: usize) -> usize {
        let single = cmp::max(mem::size_of::<T>(), 1);
        single.saturating_mul(len)
    }

    pub fn try_reserve(&mut self, additional: usize) -> Result<(), TryReserveError> {
        self.inner.try_reserve(additional)
    }

    /// 要素数を返す。
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// 空かどうか。
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// 要素を追加する。
    pub fn push(&mut self, value: T) {
        self.inner.push(value);
    }

    /// 末尾から要素を取り出す。
    pub fn pop(&mut self) -> Option<T> {
        self.inner.pop()
    }

    /// 追加容量を確保する。
    pub fn reserve(&mut self, additional: usize) {
        self.inner.reserve(additional);
    }

    /// 内部バッファを縮小する。
    pub fn shrink_to_fit(&mut self) {
        self.inner.shrink_to_fit();
    }

    /// スライスを取得する。
    pub fn as_slice(&self) -> &[T] {
        &self.inner
    }

    /// ミュータブルスライスを取得する。
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        &mut self.inner
    }

    /// イテレータを返す。
    pub fn iter(&self) -> slice::Iter<'_, T> {
        self.inner.iter()
    }

    /// `Vec` を永続 `List` へ変換する。
    pub fn to_list(self) -> List<T> {
        List::from_vec(self.inner)
    }

    /// 内部ベクタを取り出す。
    pub fn into_inner(self) -> Vec<T> {
        self.inner
    }

    /// 任意のイテレータから構築する。`CollectError::OutOfMemory` を
    /// `VecCollector::reserve` から取り込み、`Result` で返す。
    pub fn collect_from<I>(iter: I) -> Result<Self, CollectError>
    where
        I: IntoIterator<Item = T>,
    {
        let mut collector = VecCollector::new();
        for value in iter {
            collector.push(value)?;
        }
        let (core_vec, _) = collector.finish().into_parts();
        Ok(core_vec)
    }
}

impl<T> From<Vec<T>> for CoreVec<T> {
    fn from(value: Vec<T>) -> Self {
        Self { inner: value }
    }
}

impl<T> From<CoreVec<T>> for Vec<T> {
    fn from(value: CoreVec<T>) -> Self {
        value.inner
    }
}

impl<T> FromIterator<T> for CoreVec<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self::collect_from(iter)
            .unwrap_or_else(|err| panic!("CoreVec::collect_from unexpectedly failed: {err:?}"))
    }
}

/// 効果測定付き `Vec` ラッパー。
#[derive(Clone, Debug)]
pub struct EffectfulVec<T> {
    core: CoreVec<T>,
    effects: EffectSet,
}

impl<T> EffectfulVec<T> {
    /// 新規インスタンスを生成する。
    pub fn new() -> Self {
        Self {
            core: CoreVec::new(),
            effects: EffectSet::PURE,
        }
    }

    /// 容量を指定して生成する。
    pub fn with_capacity(capacity: usize) -> Self {
        let mut effects = EffectSet::PURE;
        if capacity > 0 {
            effects.mark_mut();
            effects.mark_mem();
            effects.record_mem_bytes(CoreVec::<T>::bytes_for(capacity));
        }
        Self {
            core: CoreVec::with_capacity(capacity),
            effects,
        }
    }

    /// `CoreVec` を包み直す。
    pub fn from_core(core: CoreVec<T>) -> Self {
        let mut effects = EffectSet::PURE;
        if !core.is_empty() {
            effects.mark_mut();
            effects.mark_mem();
            effects.record_mem_bytes(CoreVec::<T>::bytes_for(core.len()));
        }
        Self { core, effects }
    }

    /// イテレータ収集から生成する。
    pub fn collect_from<I>(iter: I) -> Result<Self, CollectError>
    where
        I: IntoIterator<Item = T>,
    {
        Ok(Self::from_core(CoreVec::collect_from(iter)?))
    }

    /// 現在の効果ラベルを取得する。
    pub fn effects(&self) -> EffectLabels {
        self.effects.to_labels()
    }

    fn mark_mem_for_len(&mut self) {
        self.effects.mark_mem();
        self.effects
            .record_mem_bytes(CoreVec::<T>::bytes_for(self.core.len()));
    }

    /// 要素を追加する。
    pub fn push(&mut self, value: T) {
        self.core.push(value);
        self.effects.mark_mut();
        self.mark_mem_for_len();
    }

    /// 要素を取り出す。
    pub fn pop(&mut self) -> Option<T> {
        let popped = self.core.pop();
        if popped.is_some() {
            self.effects.mark_mut();
        }
        popped
    }

    /// 追加容量を確保する。
    pub fn reserve(&mut self, additional: usize) {
        if additional == 0 {
            return;
        }
        self.core.reserve(additional);
        self.effects.mark_mut();
        self.effects.mark_mem();
        self.effects
            .record_mem_bytes(CoreVec::<T>::bytes_for(additional));
    }

    /// バッファを縮小する。
    pub fn shrink_to_fit(&mut self) {
        self.core.shrink_to_fit();
        self.effects.mark_mut();
    }

    /// バッファを読み取り専用で参照する。
    pub fn as_slice(&self) -> &[T] {
        self.core.as_slice()
    }

    /// イテレータを返す。
    pub fn iter(&self) -> slice::Iter<'_, T> {
        self.core.iter()
    }

    /// 永続リストへコピーする。
    pub fn to_list(&mut self) -> List<T>
    where
        T: Clone,
    {
        self.effects.mark_mem();
        self.effects
            .record_mem_bytes(CoreVec::<T>::bytes_for(self.core.len()));
        List::of_iter(self.core.iter().cloned())
    }

    /// 内部 `CoreVec` と効果集合を取り出す。
    pub fn into_parts(self) -> (CoreVec<T>, EffectSet) {
        (self.core, self.effects)
    }
}

impl<T> Default for EffectfulVec<T> {
    fn default() -> Self {
        Self::new()
    }
}
