use std::collections::HashSet;
use std::fmt;
use std::iter::FromIterator;
use std::mem;

use super::arena::{ArenaPtr, PersistentArena};
use crate::prelude::iter::{Iter, IterIntoIterator};

/// Finger tree 風のノードを使った永続リスト。
pub struct List<T> {
    arena: PersistentArena<FingerTreeNode<T>>,
    root: Option<ArenaPtr<FingerTreeNode<T>>>,
    len: usize,
}

impl<T> Default for List<T> {
    fn default() -> Self {
        Self {
            arena: PersistentArena::new(),
            root: None,
            len: 0,
        }
    }
}

impl<T> Clone for List<T> {
    fn clone(&self) -> Self {
        Self {
            arena: self.arena.clone(),
            root: self.root.clone(),
            len: self.len,
        }
    }
}

impl<T> List<T> {
    /// 空リストを返す。
    pub fn new() -> Self {
        Self::default()
    }

    /// 仕様上の `List.empty` に対応する別名。
    pub fn empty() -> Self {
        Self::new()
    }

    /// 単一要素のリストを生成する。
    pub fn singleton(value: T) -> Self {
        Self::from_vec(vec![value])
    }

    /// 任意のイテレータからリストを生成する。
    pub fn of_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        Self::from_vec(iter.into_iter().collect())
    }

    /// ベクタから永続リストを構築する。
    pub fn from_vec(values: Vec<T>) -> Self {
        let arena = PersistentArena::new();
        let len = values.len();
        let root = build_balanced(&arena, values);
        Self { arena, root, len }
    }

    /// 要素数を返す。
    pub fn len(&self) -> usize {
        self.len
    }

    /// 空かどうか。
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// 先頭に値を追加した新しいリストを返す。
    pub fn push_front(&self, value: T) -> Self {
        let arena = self.arena.clone();
        let leaf = arena.alloc(FingerTreeNode::leaf(value));
        let root = match &self.root {
            Some(existing) => Some(arena.alloc(FingerTreeNode::branch(leaf, existing.clone()))),
            None => Some(leaf),
        };
        Self {
            arena,
            root,
            len: self.len + 1,
        }
    }

    /// 末尾に値を追加した新しいリストを返す。
    pub fn push_back(&self, value: T) -> Self {
        let arena = self.arena.clone();
        let leaf = arena.alloc(FingerTreeNode::leaf(value));
        let root = match &self.root {
            Some(existing) => Some(arena.alloc(FingerTreeNode::branch(existing.clone(), leaf))),
            None => Some(leaf),
        };
        Self {
            arena,
            root,
            len: self.len + 1,
        }
    }

    /// 2 つのリストを結合する。
    pub fn concat(&self, other: &Self) -> Self {
        match (&self.root, &other.root) {
            (None, None) => Self::new(),
            (None, Some(_)) => other.clone(),
            (Some(_), None) => self.clone(),
            (Some(left), Some(right)) => {
                let arena = self.arena.clone();
                let root = arena.alloc(FingerTreeNode::branch(left.clone(), right.clone()));
                Self {
                    arena,
                    root: Some(root),
                    len: self.len + other.len,
                }
            }
        }
    }

    /// イテレータを返す。要素はクローンされる。
    pub fn iter(&self) -> ListIter<T> {
        ListIter::new(self.root.clone())
    }

    /// `Iter` 互換インタフェースを指すエイリアス。`List.iter()` と同義。
    pub fn to_iter(&self) -> ListIter<T> {
        self.iter()
    }

    /// `Vec` へ変換する。要素はクローンされる。
    pub fn to_vec(&self) -> Vec<T>
    where
        T: Clone,
    {
        self.iter().collect()
    }

    /// 仕様名 `List.as_vec` に対応するエイリアス。
    pub fn as_vec(&self) -> Vec<T>
    where
        T: Clone,
    {
        self.to_vec()
    }

    /// 所有権を `Vec` へ移す。値のクローンを回避できない場合はコピーされる。
    pub fn into_vec(self) -> Vec<T>
    where
        T: Clone,
    {
        self.to_vec()
    }

    /// Finger tree ノードの共有状況を集計する。
    pub fn sharing_stats_with<F>(&self, mut payload_size: F) -> ListSharingStats
    where
        F: FnMut(&T) -> usize,
    {
        const NODE_SHELL_BYTES: usize = mem::size_of::<FingerTreeNode<()>>();
        if self.root.is_none() {
            return ListSharingStats::empty();
        }
        let mut visited = HashSet::new();
        let mut stack = Vec::new();
        if let Some(root) = self.root.clone() {
            stack.push(root);
        }

        let mut payload_bytes = 0usize;
        let mut branch_nodes = 0usize;
        let mut leaf_nodes = 0usize;
        let mut shared_nodes = 0usize;
        let mut total_nodes = 0usize;

        while let Some(node_ptr) = stack.pop() {
            if !visited.insert(node_ptr.ptr_id()) {
                continue;
            }
            total_nodes += 1;
            if node_ptr.strong_count() > 1 {
                shared_nodes += 1;
            }
            match node_ptr.as_ref() {
                FingerTreeNode::Leaf(value) => {
                    leaf_nodes += 1;
                    payload_bytes += payload_size(value);
                }
                FingerTreeNode::Branch { left, right, .. } => {
                    branch_nodes += 1;
                    stack.push(left.clone());
                    stack.push(right.clone());
                }
            }
        }

        let shared_adjusted = (shared_nodes * NODE_SHELL_BYTES) / 2;
        let unique_nodes = total_nodes.saturating_sub(shared_nodes);
        let estimated_heap_bytes =
            unique_nodes * NODE_SHELL_BYTES + shared_adjusted + payload_bytes;
        ListSharingStats {
            len: self.len,
            total_nodes,
            branch_nodes,
            leaf_nodes,
            shared_nodes,
            payload_bytes,
            estimated_heap_bytes,
        }
    }
}

impl<T: Clone> List<T> {
    /// `map` を適用した新しいリストを返す。
    pub fn map<U, F>(&self, mut f: F) -> List<U>
    where
        F: FnMut(T) -> U,
    {
        let mapped: Vec<U> = self.iter().map(|value| f(value)).collect();
        List::from_vec(mapped)
    }

    /// 左畳み込みを行う。
    pub fn fold<U, F>(&self, init: U, mut f: F) -> U
    where
        F: FnMut(U, T) -> U,
    {
        let mut acc = init;
        for value in self.iter() {
            acc = f(acc, value);
        }
        acc
    }
}

impl<T: Clone> IntoIterator for List<T> {
    type Item = T;
    type IntoIter = IterIntoIterator<T>;

    fn into_iter(self) -> Self::IntoIter {
        Iter::from_persistent("List::into_iter", self.into_vec()).into_iter()
    }
}

impl<T> FromIterator<T> for List<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        List::of_iter(iter)
    }
}

impl<T: fmt::Debug + Clone> fmt::Debug for List<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl<T: PartialEq + Clone> PartialEq for List<T> {
    fn eq(&self, other: &Self) -> bool {
        self.iter().eq(other.iter())
    }
}

impl<T: Eq + Clone> Eq for List<T> {}

/// `List` 内部の構造共有メトリクス。
#[derive(Debug, Clone, Copy)]
pub struct ListSharingStats {
    pub len: usize,
    pub total_nodes: usize,
    pub branch_nodes: usize,
    pub leaf_nodes: usize,
    pub shared_nodes: usize,
    pub payload_bytes: usize,
    pub estimated_heap_bytes: usize,
}

impl ListSharingStats {
    fn empty() -> Self {
        Self {
            len: 0,
            total_nodes: 0,
            branch_nodes: 0,
            leaf_nodes: 0,
            shared_nodes: 0,
            payload_bytes: 0,
            estimated_heap_bytes: 0,
        }
    }

    /// ノード共有率（0.0〜1.0）を返す。
    pub fn reuse_ratio(&self) -> f64 {
        if self.total_nodes == 0 {
            return 0.0;
        }
        self.shared_nodes as f64 / self.total_nodes as f64
    }
}

/// Finger tree ノード。
enum FingerTreeNode<T> {
    Leaf(T),
    Branch {
        len: usize,
        left: ArenaPtr<FingerTreeNode<T>>,
        right: ArenaPtr<FingerTreeNode<T>>,
    },
}

impl<T> FingerTreeNode<T> {
    fn leaf(value: T) -> Self {
        FingerTreeNode::Leaf(value)
    }

    fn branch(left: ArenaPtr<FingerTreeNode<T>>, right: ArenaPtr<FingerTreeNode<T>>) -> Self {
        let len = left.len() + right.len();
        FingerTreeNode::Branch { len, left, right }
    }

    fn len(&self) -> usize {
        match self {
            FingerTreeNode::Leaf(_) => 1,
            FingerTreeNode::Branch { len, .. } => *len,
        }
    }
}

trait FingerTreeNodeExt<T> {
    fn len(&self) -> usize;
}

impl<T> FingerTreeNodeExt<T> for ArenaPtr<FingerTreeNode<T>> {
    fn len(&self) -> usize {
        self.as_ref().len()
    }
}

fn build_balanced<T>(
    arena: &PersistentArena<FingerTreeNode<T>>,
    values: Vec<T>,
) -> Option<ArenaPtr<FingerTreeNode<T>>> {
    if values.is_empty() {
        return None;
    }
    Some(build_subtree(arena, values))
}

fn build_subtree<T>(
    arena: &PersistentArena<FingerTreeNode<T>>,
    mut values: Vec<T>,
) -> ArenaPtr<FingerTreeNode<T>> {
    match values.len() {
        0 => unreachable!("caller guarantees non-empty vector"),
        1 => {
            let value = values.pop().expect("length checked");
            arena.alloc(FingerTreeNode::leaf(value))
        }
        _ => {
            let mid = values.len() / 2;
            let right_values = values.split_off(mid);
            let left_node = build_subtree(arena, values);
            let right_node = build_subtree(arena, right_values);
            arena.alloc(FingerTreeNode::branch(left_node, right_node))
        }
    }
}

/// Finger tree を末端まで辿るイテレータ。
pub struct ListIter<T> {
    stack: Vec<ArenaPtr<FingerTreeNode<T>>>,
}

impl<T> ListIter<T> {
    fn new(root: Option<ArenaPtr<FingerTreeNode<T>>>) -> Self {
        let mut stack = Vec::new();
        if let Some(node) = root {
            stack.push(node);
        }
        Self { stack }
    }
}

impl<T: Clone> Iterator for ListIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(node) = self.stack.pop() {
            match node.as_ref() {
                FingerTreeNode::Leaf(value) => return Some(value.clone()),
                FingerTreeNode::Branch { left, right, .. } => {
                    self.stack.push(right.clone());
                    self.stack.push(left.clone());
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::List;

    #[test]
    fn list_map_and_fold() {
        let list = List::from_vec(vec![1, 2, 3, 4]);
        let doubled = list.map(|value| value * 2);
        assert_eq!(doubled.to_vec(), vec![2, 4, 6, 8]);
        let sum = doubled.fold(0, |acc, value| acc + value);
        assert_eq!(sum, 20);
    }

    #[test]
    fn list_from_iter_and_concat() {
        let list_a: List<_> = (0..3).collect();
        let list_b = List::singleton(99);
        let joined = list_a.concat(&list_b);
        assert_eq!(joined.len(), 4);
        assert_eq!(joined.to_vec(), vec![0, 1, 2, 99]);
    }
}
