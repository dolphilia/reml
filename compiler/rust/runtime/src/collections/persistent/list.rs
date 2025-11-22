use std::fmt;

use super::arena::{ArenaPtr, PersistentArena};

/// Finger tree 風のノードを使った永続リスト。
#[derive(Clone)]
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

impl<T> List<T> {
    /// 空リストを返す。
    pub fn new() -> Self {
        Self::default()
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

    /// `Vec` へ変換する。要素はクローンされる。
    pub fn to_vec(&self) -> Vec<T>
    where
        T: Clone,
    {
        self.iter().collect()
    }

    /// 所有権を `Vec` へ移す。値のクローンを回避できない場合はコピーされる。
    pub fn into_vec(self) -> Vec<T>
    where
        T: Clone,
    {
        self.to_vec()
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

/// Finger tree ノード。
#[derive(Clone)]
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

    fn branch(
        left: ArenaPtr<FingerTreeNode<T>>,
        right: ArenaPtr<FingerTreeNode<T>>,
    ) -> Self {
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
            let value = values
                .pop()
                .expect("length checked");
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
