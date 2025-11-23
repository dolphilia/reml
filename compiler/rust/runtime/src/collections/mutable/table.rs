use std::{cell::Cell as EffectCell, collections::BTreeMap, fmt, iter::FromIterator};

#[cfg(feature = "core_prelude")]
use crate::core_prelude::iter::{EffectLabels, EffectSet};
#[cfg(not(feature = "core_prelude"))]
use crate::prelude::iter::{EffectLabels, EffectSet};

/// 挿入順序を保持する `Table`。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Table<K, V>
where
    K: Ord + Clone,
{
    entries: Vec<(K, V)>,
    index: BTreeMap<K, usize>,
}

impl<K, V> Default for Table<K, V>
where
    K: Ord + Clone,
{
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            index: BTreeMap::new(),
        }
    }
}

impl<K, V> Table<K, V>
where
    K: Ord + Clone,
{
    /// 新しいテーブルを生成する。
    pub fn new() -> Self {
        Self::default()
    }

    /// キー数を返す。
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 空かどうか。
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// 指定キーを含むか。
    pub fn contains_key(&self, key: &K) -> bool {
        self.index.contains_key(key)
    }

    /// 値を取得する。
    pub fn get(&self, key: &K) -> Option<&V> {
        self.index.get(key).map(|idx| &self.entries[*idx].1)
    }

    /// キーと値のペアを順序付きで参照する。
    pub fn entries(&self) -> &[(K, V)] {
        &self.entries
    }

    /// 要素を挿入する。既に存在する場合は値を置き換える。
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        if let Some(idx) = self.index.get(&key).copied() {
            let (_, slot) = &mut self.entries[idx];
            return Some(std::mem::replace(slot, value));
        }
        let idx = self.entries.len();
        self.entries.push((key.clone(), value));
        self.index.insert(key, idx);
        None
    }

    /// 要素を削除する。
    pub fn remove(&mut self, key: &K) -> Option<V> {
        let idx = self.index.remove(key)?;
        let (_, removed_value) = self.entries.remove(idx);
        for value in self.index.values_mut() {
            if *value > idx {
                *value -= 1;
            }
        }
        Some(removed_value)
    }

    /// すべての要素を削除する。
    pub fn clear(&mut self) {
        self.entries.clear();
        self.index.clear();
    }

    /// イテレータを返す。
    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.entries.iter().map(|(k, v)| (k, v))
    }

    /// エントリを所有権付きで取り出す。
    pub fn into_entries(self) -> Vec<(K, V)> {
        self.entries
    }

    /// `BTreeMap` へ変換する。
    pub fn to_map(&self) -> BTreeMap<K, V>
    where
        V: Clone,
    {
        self.entries.iter().cloned().collect()
    }
}

impl<K, V> FromIterator<(K, V)> for Table<K, V>
where
    K: Ord + Clone,
{
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        let mut table = Self::new();
        for (k, v) in iter {
            table.insert(k, v);
        }
        table
    }
}

/// 効果計測付きテーブル。
pub struct EffectfulTable<K, V>
where
    K: Ord + Clone,
{
    table: Table<K, V>,
    effects: EffectCell<EffectSet>,
}

impl<K, V> EffectfulTable<K, V>
where
    K: Ord + Clone,
{
    /// 新しいテーブルを生成する。
    pub fn new() -> Self {
        Self {
            table: Table::new(),
            effects: EffectCell::new(EffectSet::PURE),
        }
    }

    fn record_mut(&self) {
        let mut effects = self.effects.get();
        effects.mark_mut();
        self.effects.set(effects);
    }

    fn record_mem(&self) {
        let mut effects = self.effects.get();
        effects.mark_mem();
        self.effects.set(effects);
    }

    /// 要素を挿入する。
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        self.record_mut();
        let replaced = self.table.insert(key, value);
        if replaced.is_none() {
            self.record_mem();
        }
        replaced
    }

    /// 要素を削除する。
    pub fn remove(&mut self, key: &K) -> Option<V> {
        let removed = self.table.remove(key);
        if removed.is_some() {
            self.record_mut();
        }
        removed
    }

    /// すべての要素を削除する。
    pub fn clear(&mut self) {
        if self.table.is_empty() {
            return;
        }
        self.table.clear();
        self.record_mut();
    }

    /// 効果ラベルを返す。
    pub fn effect_labels(&self) -> EffectLabels {
        self.effects.get().to_labels()
    }

    /// 内部テーブル参照を返す。
    pub fn as_table(&self) -> &Table<K, V> {
        &self.table
    }

    /// 内部テーブルを取り出す。
    pub fn into_parts(self) -> (Table<K, V>, EffectSet) {
        (self.table, self.effects.into_inner())
    }
}

impl<K, V> fmt::Debug for EffectfulTable<K, V>
where
    K: Ord + Clone + fmt::Debug,
    V: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EffectfulTable")
            .field("entries", &self.table.entries)
            .field("effects", &self.effects.get())
            .finish()
    }
}
