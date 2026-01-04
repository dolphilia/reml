use std::{
    cell::Cell as EffectCell,
    collections::{BTreeMap, VecDeque},
    fs::File,
    hash::{BuildHasher, Hasher},
    io::{self, BufRead, BufReader},
    mem,
    path::Path,
};

use indexmap::IndexMap;

#[cfg(feature = "core_prelude")]
use crate::core_prelude::iter::{EffectLabels, EffectSet};
#[cfg(not(feature = "core_prelude"))]
use crate::prelude::iter::{EffectLabels, EffectSet};

#[cfg(feature = "core_prelude")]
use crate::register_table_csv_capability;

const DETERMINISTIC_SEED: u64 = 0x9E377_9B97_F4A7_C15;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct EntryId(u64);

#[derive(Clone, Debug)]
struct TableEntry<V> {
    id: EntryId,
    value: V,
}

#[derive(Clone, Debug)]
struct TableInner<K, V>
where
    K: Eq + std::hash::Hash + Clone,
{
    storage: TableMap<K, V>,
    order: VecDeque<(EntryId, K)>,
    next_id: EntryId,
}

impl<K, V> TableInner<K, V>
where
    K: Eq + std::hash::Hash + Clone,
{
    fn new() -> Self {
        Self {
            storage: IndexMap::with_hasher(DeterministicHasherBuilder::default()),
            order: VecDeque::new(),
            next_id: EntryId(0),
        }
    }

    fn allocate(&mut self) -> EntryId {
        let id = self.next_id;
        self.next_id = EntryId(id.0.wrapping_add(1));
        id
    }

    fn push_order(&mut self, id: EntryId, key: K) {
        self.order.push_back((id, key));
    }

    fn remove_order(&mut self, key: &K, id: EntryId) {
        self.order
            .retain(|(entry_id, stored_key)| !(entry_id == &id && stored_key == key));
    }
}

type TableMap<K, V> = IndexMap<K, TableEntry<V>, DeterministicHasherBuilder>;

#[derive(Clone, Debug)]
struct DeterministicHasher(u64);

impl DeterministicHasher {
    fn mix(&mut self, bytes: &[u8]) {
        let mut state = self.0;
        for &byte in bytes {
            state = state
                .wrapping_mul(0x9E377_9B97_F4A7_C15)
                .wrapping_add(byte as u64);
        }
        self.0 = state;
    }
}

impl Default for DeterministicHasher {
    fn default() -> Self {
        Self(DETERMINISTIC_SEED)
    }
}

impl Hasher for DeterministicHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        self.mix(bytes);
    }
}

#[derive(Clone, Default)]
struct DeterministicHasherBuilder;

impl BuildHasher for DeterministicHasherBuilder {
    type Hasher = DeterministicHasher;

    fn build_hasher(&self) -> Self::Hasher {
        DeterministicHasher::default()
    }
}

/// 挿入順序を保持する `Table<K, V>`。
#[derive(Clone, Debug)]
pub struct Table<K, V>
where
    K: Eq + std::hash::Hash + Clone,
{
    inner: TableInner<K, V>,
}

impl<K, V> Table<K, V>
where
    K: Eq + std::hash::Hash + Clone,
{
    /// 新しいテーブルを生成する。
    pub fn new() -> Self {
        Self {
            inner: TableInner::new(),
        }
    }

    /// 要素数を返す。
    pub fn len(&self) -> usize {
        self.inner.storage.len()
    }

    /// 空かどうか。
    pub fn is_empty(&self) -> bool {
        self.inner.storage.is_empty()
    }

    /// 指定キーを含むか。
    pub fn contains_key(&self, key: &K) -> bool {
        self.inner.storage.contains_key(key)
    }

    /// 値を取得する。
    pub fn get(&self, key: &K) -> Option<&V> {
        self.inner.storage.get(key).map(|entry| &entry.value)
    }

    /// 挿入順を保持したイテレータを返す。
    pub fn iter(&self) -> TableIter<'_, K, V> {
        TableIter::new(&self.inner.order, &self.inner.storage)
    }

    /// 要素を挿入する。既存キーは値を置き換える。
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        if let Some(entry) = self.inner.storage.get_mut(&key) {
            return Some(mem::replace(&mut entry.value, value));
        }
        let id = self.inner.allocate();
        self.inner.push_order(id, key.clone());
        self.inner.storage.insert(key, TableEntry { id, value });
        None
    }

    /// 要素を削除する。
    pub fn remove(&mut self, key: &K) -> Option<V> {
        let entry = self.inner.storage.remove(key)?;
        self.inner.remove_order(key, entry.id);
        Some(entry.value)
    }

    /// すべての要素を削除する。
    pub fn clear(&mut self) {
        if self.is_empty() {
            return;
        }
        self.inner.storage.clear();
        self.inner.order.clear();
    }

    /// 所有権付きでエントリ一覧を返す。
    pub fn into_entries(self) -> Vec<(K, V)> {
        let TableInner {
            mut storage,
            mut order,
            ..
        } = self.inner;
        let mut result = Vec::with_capacity(storage.len());
        while let Some((_, key)) = order.pop_front() {
            if let Some(entry) = storage.remove(&key) {
                result.push((key, entry.value));
            }
        }
        result
    }

    /// `Vec` へコピーした列挙を返す。
    pub fn entries(&self) -> Vec<(K, V)>
    where
        K: Clone,
        V: Clone,
    {
        self.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
    }

    /// `BTreeMap` へ変換する（キー昇順）。
    pub fn to_map(&self) -> BTreeMap<K, V>
    where
        K: Ord + Clone,
        V: Clone,
    {
        let mut map = BTreeMap::new();
        for (key, value) in self.iter() {
            map.insert(key.clone(), value.clone());
        }
        map
    }
}

impl<K, V> Default for Table<K, V>
where
    K: Eq + std::hash::Hash + Clone,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V> PartialEq for Table<K, V>
where
    K: Eq + std::hash::Hash + Clone + PartialEq,
    V: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        if self.len() != other.len() {
            return false;
        }
        self.iter()
            .zip(other.iter())
            .all(|((left_key, left_val), (right_key, right_val))| {
                left_key == right_key && left_val == right_val
            })
    }
}

impl<K, V> Eq for Table<K, V>
where
    K: Eq + std::hash::Hash + Clone + PartialEq,
    V: Eq,
{
}

impl<K, V> FromIterator<(K, V)> for Table<K, V>
where
    K: Eq + std::hash::Hash + Clone,
{
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iterable: I) -> Self {
        let mut table = Self::new();
        for (key, value) in iterable {
            table.insert(key, value);
        }
        table
    }
}

/// 挿入順をトラバースするイテレータ。
pub struct TableIter<'a, K, V> {
    order_iter: std::collections::vec_deque::Iter<'a, (EntryId, K)>,
    storage: &'a TableMap<K, V>,
}

impl<'a, K, V> TableIter<'a, K, V>
where
    K: Eq + std::hash::Hash + Clone,
{
    fn new(order: &'a VecDeque<(EntryId, K)>, storage: &'a TableMap<K, V>) -> Self {
        Self {
            order_iter: order.iter(),
            storage,
        }
    }
}

impl<'a, K, V> Iterator for TableIter<'a, K, V>
where
    K: Eq + std::hash::Hash + Clone,
{
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        while let Some((_, key)) = self.order_iter.next() {
            if let Some(entry) = self.storage.get(key) {
                return Some((key, &entry.value));
            }
        }
        None
    }
}

/// 効果計測付きテーブル。
pub struct EffectfulTable<K, V>
where
    K: Eq + std::hash::Hash + Clone,
{
    table: Table<K, V>,
    effects: EffectCell<EffectSet>,
}

impl<K, V> EffectfulTable<K, V>
where
    K: Eq + std::hash::Hash + Clone,
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

    fn record_mem_bytes(&self, bytes: usize) {
        if bytes == 0 {
            return;
        }
        let mut effects = self.effects.get();
        effects.mark_mem();
        effects.record_mem_bytes(bytes);
        self.effects.set(effects);
    }

    fn record_io(&self) {
        let mut effects = self.effects.get();
        effects.mark_io();
        self.effects.set(effects);
    }

    /// 要素を挿入する。
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        self.record_mut();
        let replaced = self.table.insert(key, value);
        if replaced.is_none() {
            self.record_mem_bytes(mem::size_of::<K>() + mem::size_of::<V>());
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

    /// 所有権付きのエントリ一覧を取り出す。
    pub fn into_parts(self) -> (Table<K, V>, EffectSet) {
        (self.table, self.effects.into_inner())
    }

    /// 挿入順で要素を辿るイテレータを返す。
    pub fn iter(&self) -> TableIter<'_, K, V> {
        self.table.iter()
    }

    /// `BTreeMap` へ変換する。
    pub fn to_map(&self) -> BTreeMap<K, V>
    where
        K: Ord + Clone,
        V: Clone,
    {
        let map = self.table.to_map();
        let key_bytes = mem::size_of::<K>();
        let value_bytes = mem::size_of::<V>();
        let total = self.table.len() * (key_bytes + value_bytes);
        self.record_mem_bytes(total);
        map
    }
}

impl Table<String, String> {
    /// CSV ファイルを読み込み、キー-値ペアを挿入する。
    pub fn load_csv<P>(path: P) -> io::Result<Self>
    where
        P: AsRef<Path>,
    {
        #[cfg(feature = "core_prelude")]
        register_table_csv_capability();

        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut effectful = EffectfulTable::new();
        effectful.record_io();
        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let mut parts = line.splitn(2, ',');
            let key = parts.next().unwrap_or("").trim().to_string();
            let value = parts.next().unwrap_or("").trim().to_string();
            effectful.insert(key, value);
        }
        let (table, _) = effectful.into_parts();
        Ok(table)
    }
}
