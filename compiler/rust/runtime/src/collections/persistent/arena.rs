use std::fmt;
use std::ops::Deref;
use std::sync::{Arc, Mutex};

/// 永続構造で共有されるノードをアロケートする簡易アリーナ。
/// `Arc` に包んだ値を保持し、呼び出し側からは `ArenaPtr` を通じて参照する。
pub struct PersistentArena<T> {
    storage: Arc<ArenaStorage<T>>,
}

impl<T> PersistentArena<T> {
    /// 新しいアリーナを作成する。
    pub fn new() -> Self {
        Self {
            storage: Arc::new(ArenaStorage {
                nodes: Mutex::new(Vec::new()),
            }),
        }
    }

    /// 値を確保し、共有可能なポインタを返す。
    pub fn alloc(&self, value: T) -> ArenaPtr<T> {
        let ptr = ArenaPtr {
            inner: Arc::new(value),
        };
        // 生成したポインタをアリーナに記録して寿命を管理する。
        self.storage
            .nodes
            .lock()
            .expect("arena poisoned")
            .push(ptr.inner.clone());
        ptr
    }
}

#[derive(Default)]
struct ArenaStorage<T> {
    nodes: Mutex<Vec<Arc<T>>>,
}

/// アリーナが管理するノードの共有ポインタ。
pub struct ArenaPtr<T> {
    inner: Arc<T>,
}

impl<T> ArenaPtr<T> {
    /// `Arc::strong_count` を透過的に公開する（監査・テスト向け）。
    pub fn strong_count(&self) -> usize {
        Arc::strong_count(&self.inner)
    }

    /// `Arc` の生ポインタ値を ID として返す（メトリクス用）。
    pub fn ptr_id(&self) -> usize {
        Arc::as_ptr(&self.inner) as usize
    }
}

impl<T> Deref for ArenaPtr<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T: fmt::Debug> fmt::Debug for ArenaPtr<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}

impl<T> AsRef<T> for ArenaPtr<T> {
    fn as_ref(&self) -> &T {
        &self.inner
    }
}

impl<T> Clone for PersistentArena<T> {
    fn clone(&self) -> Self {
        Self {
            storage: Arc::clone(&self.storage),
        }
    }
}

impl<T> Default for PersistentArena<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Clone for ArenaPtr<T> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}
