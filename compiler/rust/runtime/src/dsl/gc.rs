//! Core.Dsl.Gc の最小実装。

use std::sync::{atomic::{AtomicU64, Ordering}, Arc};

/// GC 戦略。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GcStrategy {
    Arena,
    RefCount,
    MarkAndSweep,
}

/// GC ヒープ。
#[derive(Debug, Clone)]
pub struct GcHeap {
    pub strategy: GcStrategy,
    heap_id: u64,
    next_id: Arc<AtomicU64>,
}

impl GcHeap {
    pub fn heap_id(&self) -> u64 {
        self.heap_id
    }
}

/// GC 参照。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GcRef<T> {
    pub heap_id: u64,
    pub handle: u64,
    _marker: std::marker::PhantomData<T>,
}

impl<T> GcRef<T> {
    fn new(heap_id: u64, handle: u64) -> Self {
        Self {
            heap_id,
            handle,
            _marker: std::marker::PhantomData,
        }
    }
}

/// ルートスコープ。
#[derive(Debug, Clone)]
pub struct RootScope {
    pub heap: GcHeap,
    pub roots: Vec<u64>,
}

/// GC エラー。
#[derive(Debug, Clone)]
pub struct GcError {
    pub kind: GcErrorKind,
    pub message: String,
}

impl GcError {
    pub fn new(kind: GcErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

/// GC エラー種別。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GcErrorKind {
    AllocationFailed,
    CollectFailed,
}

static HEAP_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Core.Dsl.Gc の名前空間。
pub struct Gc;

impl Gc {
    pub fn new(strategy: GcStrategy) -> GcHeap {
        let heap_id = HEAP_COUNTER.fetch_add(1, Ordering::Relaxed);
        GcHeap {
            strategy,
            heap_id,
            next_id: Arc::new(AtomicU64::new(1)),
        }
    }

    pub fn with_scope<T>(heap: GcHeap, f: impl FnOnce(RootScope) -> T) -> T {
        let scope = RootScope {
            heap,
            roots: Vec::new(),
        };
        f(scope)
    }

    pub fn alloc<T>(scope: &RootScope, value: T) -> Result<GcRef<T>, GcError> {
        let _ = value;
        let handle = scope.heap.next_id.fetch_add(1, Ordering::Relaxed);
        Ok(GcRef::new(scope.heap.heap_id, handle))
    }

    pub fn pin<T>(mut scope: RootScope, value: &GcRef<T>) -> RootScope {
        if scope.heap.heap_id == value.heap_id {
            scope.roots.push(value.handle);
        }
        scope
    }

    pub fn collect(_heap: &GcHeap) -> Result<(), GcError> {
        Ok(())
    }

    pub fn collect_if_needed(_heap: &GcHeap) -> Result<(), GcError> {
        Ok(())
    }
}
