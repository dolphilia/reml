//! 永続 `Map` / `Set` 実装。赤黒木（Left-Leaning Red-Black Tree）を簡易移植し、
//! `PersistentArena` 上でノードを共有する。

use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet, HashSet},
    fmt,
    iter::FromIterator,
    mem,
    sync::Arc,
};

use serde::Serialize;

use crate::prelude::iter::{Iter, IterIntoIterator};

use crate::collections::audit_bridge::{self, AuditBridgeError, ChangeSet};

use super::arena::{ArenaPtr, PersistentArena};

/// 永続マップ（`@pure`）。操作は O(log n) で構造共有を維持する。
#[derive(Clone)]
pub struct PersistentMap<K, V> {
    arena: PersistentArena<Node<K, V>>,
    root: Option<ArenaPtr<Node<K, V>>>,
    len: usize,
}

impl<K: Ord, V> Default for PersistentMap<K, V> {
    fn default() -> Self {
        Self {
            arena: PersistentArena::new(),
            root: None,
            len: 0,
        }
    }
}

impl<K: Ord, V> PersistentMap<K, V> {
    /// 空のマップを生成する。
    pub fn new() -> Self {
        Self::default()
    }

    /// 要素数を返す。
    pub fn len(&self) -> usize {
        self.len
    }

    /// 空かどうか。
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// キーに対応する値を取得する。
    pub fn get(&self, key: &K) -> Option<&V> {
        let mut cursor = self.root.as_ref();
        while let Some(ptr) = cursor {
            let node = &**ptr;
            match key.cmp(node.key()) {
                Ordering::Less => {
                    cursor = node.left.as_ref();
                }
                Ordering::Greater => {
                    cursor = node.right.as_ref();
                }
                Ordering::Equal => return Some(node.value()),
            }
        }
        None
    }

    /// キーの存在可否を返す。
    pub fn contains_key(&self, key: &K) -> bool {
        self.get(key).is_some()
    }

    /// 値を挿入する（既存キーは上書き）。
    pub fn insert(&self, key: K, value: V) -> Self {
        let arena = self.arena.clone();
        let (root, _) = insert_node(&arena, self.root.clone(), key, value);
        let root = make_black(&arena, root);
        let len = subtree_size(Some(&root));
        Self {
            arena,
            root: Some(root),
            len,
        }
    }

    /// 既存の `BTreeMap` から永続マップを構築する。
    pub fn from_map(map: BTreeMap<K, V>) -> Self {
        map.into_iter()
            .fold(Self::new(), |acc, (k, v)| acc.insert(k, v))
    }

    /// `BTreeMap` へ変換する（コピー）。
    pub fn into_map(self) -> BTreeMap<K, V>
    where
        K: Ord + Clone,
        V: Clone,
    {
        let mut map = BTreeMap::new();
        self.for_each_entry(|k, v| {
            map.insert(k.clone(), v.clone());
        });
        map
    }

    /// キー一覧を取得する。
    pub fn keys(&self) -> Vec<K>
    where
        K: Clone,
    {
        let mut keys = Vec::with_capacity(self.len());
        self.for_each_entry(|key, _| keys.push(key.clone()));
        keys
    }

    /// 内部ノードを昇順で走査するユーティリティ。
    fn for_each_entry<'a, F>(&'a self, mut visit: F)
    where
        F: FnMut(&'a K, &'a V),
    {
        fn traverse<'a, K: Ord, V, F: FnMut(&'a K, &'a V)>(
            node: Option<&'a ArenaPtr<Node<K, V>>>,
            visit: &mut F,
        ) {
            if let Some(ptr) = node {
                let node_ref = &**ptr;
                traverse(node_ref.left.as_ref(), visit);
                visit(node_ref.key(), node_ref.value());
                traverse(node_ref.right.as_ref(), visit);
            }
        }

        traverse(self.root.as_ref(), &mut visit);
    }

    /// `self` と `other` の差分を `ChangeSet` として取得する。
    pub fn diff_change_set(&self, other: &Self) -> Result<ChangeSet, AuditBridgeError>
    where
        K: Ord + Clone + Serialize,
        V: Clone + Serialize,
    {
        audit_bridge::map_diff_to_changes(self, other)
    }

    /// `delta` の要素を取り込みつつ、競合時は `resolver` で値を決定する。
    pub fn merge_with<F>(&self, delta: &Self, mut resolver: F) -> Self
    where
        K: Ord + Clone,
        V: Clone,
        F: FnMut(&K, &V, &V) -> V,
    {
        let mut result = self.clone();
        delta.for_each_entry(|key, value| {
            if let Some(existing) = result.get(key) {
                let merged = resolver(key, existing, value);
                result = result.insert(key.clone(), merged);
            } else {
                result = result.insert(key.clone(), value.clone());
            }
        });
        result
    }

    /// `merge_with` の結果と差分情報を同時に取得する。
    pub fn merge_with_change_set<F>(
        &self,
        delta: &Self,
        mut resolver: F,
    ) -> Result<(Self, ChangeSet), AuditBridgeError>
    where
        K: Ord + Clone + Serialize,
        V: Clone + Serialize,
        F: FnMut(&K, &V, &V) -> V,
    {
        let merged = self.merge_with(delta, |key, left, right| resolver(key, left, right));
        let change_set = self.diff_change_set(&merged)?;
        Ok((merged, change_set))
    }
}

impl<K: Ord + Clone, V: Clone> IntoIterator for PersistentMap<K, V> {
    type Item = (K, V);
    type IntoIter = IterIntoIterator<(K, V)>;

    fn into_iter(self) -> Self::IntoIter {
        let entries = self.into_map().into_iter().collect();
        Iter::from_persistent("PersistentMap::into_iter", entries).into_iter()
    }
}

impl<K: Ord, V> FromIterator<(K, V)> for PersistentMap<K, V> {
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        iter.into_iter()
            .fold(Self::new(), |acc, (k, v)| acc.insert(k, v))
    }
}

impl<K: Ord + fmt::Debug, V: fmt::Debug> fmt::Debug for PersistentMap<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut entries = Vec::new();
        self.for_each_entry(|k, v| entries.push((k, v)));
        f.debug_map().entries(entries).finish()
    }
}

impl<K: Ord, V> PersistentMap<K, V> {
    /// 赤黒木ノードの構造共有状況を返す。
    pub fn sharing_stats_with<F>(&self, mut payload_size: F) -> PersistentMapSharingStats
    where
        F: FnMut(&K, &V) -> usize,
    {
        const NODE_SHELL_BYTES: usize = mem::size_of::<Node<(), ()>>();
        if self.root.is_none() {
            return PersistentMapSharingStats::empty(self.len);
        }

        let mut visited = HashSet::new();
        let mut stack = Vec::new();
        if let Some(root) = self.root.clone() {
            stack.push(root);
        }

        let mut total_nodes = 0usize;
        let mut shared_nodes = 0usize;
        let mut payload_bytes = 0usize;

        while let Some(node_ptr) = stack.pop() {
            if !visited.insert(node_ptr.ptr_id()) {
                continue;
            }
            total_nodes += 1;
            if node_ptr.strong_count() > 1 {
                shared_nodes += 1;
            }
            let node_ref = node_ptr.as_ref();
            payload_bytes += payload_size(node_ref.key(), node_ref.value());
            if let Some(left) = node_ref.left.clone() {
                stack.push(left);
            }
            if let Some(right) = node_ref.right.clone() {
                stack.push(right);
            }
        }

        let shared_adjusted = (shared_nodes * NODE_SHELL_BYTES) / 2;
        let unique_nodes = total_nodes.saturating_sub(shared_nodes);
        let estimated_heap_bytes =
            unique_nodes * NODE_SHELL_BYTES + shared_adjusted + payload_bytes;
        PersistentMapSharingStats {
            len: self.len,
            total_nodes,
            shared_nodes,
            payload_bytes,
            estimated_heap_bytes,
        }
    }
}

/// `PersistentMap` の共有メトリクス。
#[derive(Debug, Clone, Copy)]
pub struct PersistentMapSharingStats {
    pub len: usize,
    pub total_nodes: usize,
    pub shared_nodes: usize,
    pub payload_bytes: usize,
    pub estimated_heap_bytes: usize,
}

impl PersistentMapSharingStats {
    fn empty(len: usize) -> Self {
        Self {
            len,
            total_nodes: 0,
            shared_nodes: 0,
            payload_bytes: 0,
            estimated_heap_bytes: 0,
        }
    }

    pub fn reuse_ratio(&self) -> f64 {
        if self.total_nodes == 0 {
            return 0.0;
        }
        self.shared_nodes as f64 / self.total_nodes as f64
    }
}

/// 永続 Set。内部的には `PersistentMap<T, ()>` を利用する。
#[derive(Clone)]
pub struct PersistentSet<T> {
    map: PersistentMap<T, ()>,
}

impl<T: Ord> Default for PersistentSet<T> {
    fn default() -> Self {
        Self {
            map: PersistentMap::new(),
        }
    }
}

impl<T: Ord> PersistentSet<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn contains(&self, value: &T) -> bool {
        self.map.contains_key(value)
    }

    pub fn insert(&self, value: T) -> Self {
        Self {
            map: self.map.insert(value, ()),
        }
    }

    pub fn from_set(set: BTreeSet<T>) -> Self {
        set.into_iter().fold(Self::new(), |acc, v| acc.insert(v))
    }

    pub fn into_set(self) -> BTreeSet<T>
    where
        T: Ord + Clone,
    {
        let mut set = BTreeSet::new();
        self.map.for_each_entry(|key, _| {
            set.insert(key.clone());
        });
        set
    }

    /// 差集合 (`self` - `other`) を返す。
    pub fn diff(&self, other: &Self) -> Self
    where
        T: Ord + Clone,
    {
        let mut result = Self::new();
        self.map.for_each_entry(|key, _| {
            if !other.contains(key) {
                result = result.insert(key.clone());
            }
        });
        result
    }

    /// 集合を predicate で 2 つに分割する。
    pub fn partition<F>(&self, mut pred: F) -> (Self, Self)
    where
        T: Ord + Clone,
        F: FnMut(&T) -> bool,
    {
        let mut left = Self::new();
        let mut right = Self::new();
        self.map.for_each_entry(|key, _| {
            if pred(key) {
                left = left.insert(key.clone());
            } else {
                right = right.insert(key.clone());
            }
        });
        (left, right)
    }

    /// 差分を `ChangeSet` として取得する。
    pub fn diff_change_set(&self, other: &Self) -> Result<ChangeSet, AuditBridgeError>
    where
        T: Ord + Clone + Serialize,
    {
        audit_bridge::set_diff_to_changes(self, other)
    }
}

impl<T: Ord + Clone> IntoIterator for PersistentSet<T> {
    type Item = T;
    type IntoIter = IterIntoIterator<T>;

    fn into_iter(self) -> Self::IntoIter {
        let entries = self.into_set().into_iter().collect();
        Iter::from_persistent("PersistentSet::into_iter", entries).into_iter()
    }
}

impl<T: Ord + fmt::Debug> fmt::Debug for PersistentSet<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut entries = Vec::new();
        self.map.for_each_entry(|key, _| entries.push(key));
        f.debug_set().entries(entries).finish()
    }
}

/// LLRB ノードの色。
#[derive(Clone, Copy, PartialEq, Eq)]
enum Color {
    Red,
    Black,
}

impl Color {
    fn is_red(self) -> bool {
        matches!(self, Color::Red)
    }

    fn flip(self) -> Self {
        match self {
            Color::Red => Color::Black,
            Color::Black => Color::Red,
        }
    }
}

#[derive(Clone)]
struct Entry<K, V> {
    key: K,
    value: V,
}

/// 木ノード。`entry` を `Arc` で保持して構造共有する。
#[derive(Clone)]
struct Node<K, V> {
    entry: Arc<Entry<K, V>>,
    left: Option<ArenaPtr<Node<K, V>>>,
    right: Option<ArenaPtr<Node<K, V>>>,
    color: Color,
    size: usize,
}

impl<K, V> Node<K, V> {
    fn key(&self) -> &K {
        &self.entry.key
    }

    fn value(&self) -> &V {
        &self.entry.value
    }
}

fn insert_node<K: Ord, V>(
    arena: &PersistentArena<Node<K, V>>,
    node: Option<ArenaPtr<Node<K, V>>>,
    key: K,
    value: V,
) -> (ArenaPtr<Node<K, V>>, bool) {
    match node {
        None => {
            let entry = Arc::new(Entry { key, value });
            let new_node = build_node(arena, entry, None, None, Color::Red);
            (new_node, true)
        }
        Some(ptr) => {
            let node_ref = &*ptr;
            let (next_ptr, inserted) = match key.cmp(node_ref.key()) {
                Ordering::Less => {
                    let (left, inserted) = insert_node(arena, node_ref.left.clone(), key, value);
                    (
                        build_node(
                            arena,
                            Arc::clone(&node_ref.entry),
                            Some(left),
                            node_ref.right.clone(),
                            node_ref.color,
                        ),
                        inserted,
                    )
                }
                Ordering::Greater => {
                    let (right, inserted) = insert_node(arena, node_ref.right.clone(), key, value);
                    (
                        build_node(
                            arena,
                            Arc::clone(&node_ref.entry),
                            node_ref.left.clone(),
                            Some(right),
                            node_ref.color,
                        ),
                        inserted,
                    )
                }
                Ordering::Equal => {
                    let entry = Arc::new(Entry { key, value });
                    (
                        build_node(
                            arena,
                            entry,
                            node_ref.left.clone(),
                            node_ref.right.clone(),
                            node_ref.color,
                        ),
                        false,
                    )
                }
            };
            (fix_up(arena, next_ptr), inserted)
        }
    }
}

fn fix_up<K: Ord, V>(
    arena: &PersistentArena<Node<K, V>>,
    node: ArenaPtr<Node<K, V>>,
) -> ArenaPtr<Node<K, V>> {
    let mut current = node;

    if is_red(current.right.as_ref()) && !is_red(current.left.as_ref()) {
        current = rotate_left(arena, current);
    }
    if is_red(current.left.as_ref())
        && current
            .left
            .as_ref()
            .map(|left| is_red(left.left.as_ref()))
            .unwrap_or(false)
    {
        current = rotate_right(arena, current);
    }
    if is_red(current.left.as_ref()) && is_red(current.right.as_ref()) {
        current = flip_colors(arena, current);
    }
    current
}

fn rotate_left<K: Ord, V>(
    arena: &PersistentArena<Node<K, V>>,
    node: ArenaPtr<Node<K, V>>,
) -> ArenaPtr<Node<K, V>> {
    let node_ref = &*node;
    let right = node_ref
        .right
        .clone()
        .expect("rotate_left requires right child");
    let right_ref = &*right;

    let left_child = build_node(
        arena,
        Arc::clone(&node_ref.entry),
        node_ref.left.clone(),
        right_ref.left.clone(),
        Color::Red,
    );
    build_node(
        arena,
        Arc::clone(&right_ref.entry),
        Some(left_child),
        right_ref.right.clone(),
        node_ref.color,
    )
}

fn rotate_right<K: Ord, V>(
    arena: &PersistentArena<Node<K, V>>,
    node: ArenaPtr<Node<K, V>>,
) -> ArenaPtr<Node<K, V>> {
    let node_ref = &*node;
    let left = node_ref
        .left
        .clone()
        .expect("rotate_right requires left child");
    let left_ref = &*left;

    let right_child = build_node(
        arena,
        Arc::clone(&node_ref.entry),
        left_ref.right.clone(),
        node_ref.right.clone(),
        Color::Red,
    );

    build_node(
        arena,
        Arc::clone(&left_ref.entry),
        left_ref.left.clone(),
        Some(right_child),
        node_ref.color,
    )
}

fn flip_colors<K: Ord, V>(
    arena: &PersistentArena<Node<K, V>>,
    node: ArenaPtr<Node<K, V>>,
) -> ArenaPtr<Node<K, V>> {
    let node_ref = &*node;
    let left = node_ref.left.clone().map(|left| {
        let color = left.color;
        set_color(arena, left, color.flip())
    });
    let right = node_ref.right.clone().map(|right| {
        let color = right.color;
        set_color(arena, right, color.flip())
    });
    build_node(
        arena,
        Arc::clone(&node_ref.entry),
        left,
        right,
        node_ref.color.flip(),
    )
}

fn set_color<K, V>(
    arena: &PersistentArena<Node<K, V>>,
    node: ArenaPtr<Node<K, V>>,
    color: Color,
) -> ArenaPtr<Node<K, V>> {
    let node_ref = &*node;
    build_node(
        arena,
        Arc::clone(&node_ref.entry),
        node_ref.left.clone(),
        node_ref.right.clone(),
        color,
    )
}

fn make_black<K, V>(
    arena: &PersistentArena<Node<K, V>>,
    node: ArenaPtr<Node<K, V>>,
) -> ArenaPtr<Node<K, V>> {
    set_color(arena, node, Color::Black)
}

fn build_node<K, V>(
    arena: &PersistentArena<Node<K, V>>,
    entry: Arc<Entry<K, V>>,
    left: Option<ArenaPtr<Node<K, V>>>,
    right: Option<ArenaPtr<Node<K, V>>>,
    color: Color,
) -> ArenaPtr<Node<K, V>> {
    let size = 1 + subtree_size(left.as_ref()) + subtree_size(right.as_ref());
    arena.alloc(Node {
        entry,
        left,
        right,
        color,
        size,
    })
}

fn subtree_size<K, V>(node: Option<&ArenaPtr<Node<K, V>>>) -> usize {
    node.map(|ptr| ptr.size).unwrap_or(0)
}

fn is_red<K, V>(node: Option<&ArenaPtr<Node<K, V>>>) -> bool {
    node.map(|ptr| ptr.color.is_red()).unwrap_or(false)
}
