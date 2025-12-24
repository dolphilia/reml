//! Core.Dsl.Gc の最小実装。

use std::collections::{HashMap, HashSet};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, Mutex,
};

use serde_json::{Map as JsonMap, Value};

use crate::dsl::{
    emit_audit, AuditPayload, AUDIT_DSL_GC_ALLOC, AUDIT_DSL_GC_RELEASE, AUDIT_DSL_GC_ROOT,
};
use crate::prelude::ensure::{DiagnosticSeverity, GuardDiagnostic, IntoDiagnostic};

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
    state: Arc<Mutex<GcState>>,
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
#[derive(Debug)]
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

#[derive(Debug, Default)]
struct GcState {
    entries: HashMap<u64, GcEntry>,
    roots: HashSet<u64>,
}

#[derive(Debug, Clone)]
struct GcEntry {
    root_count: u64,
    bytes: usize,
}

/// Core.Dsl.Gc の名前空間。
pub struct Gc;

impl Gc {
    pub fn new(strategy: GcStrategy) -> GcHeap {
        let heap_id = HEAP_COUNTER.fetch_add(1, Ordering::Relaxed);
        GcHeap {
            strategy,
            heap_id,
            next_id: Arc::new(AtomicU64::new(1)),
            state: Arc::new(Mutex::new(GcState::default())),
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
        let handle = scope.heap.next_id.fetch_add(1, Ordering::Relaxed);
        let bytes = std::mem::size_of_val(&value);
        let mut state = scope
            .heap
            .state
            .lock()
            .map_err(|_| GcError::new(GcErrorKind::AllocationFailed, "gc state lock failed"))?;
        state.entries.insert(
            handle,
            GcEntry {
                root_count: 0,
                bytes,
            },
        );
        let mut payload = AuditPayload::new(AUDIT_DSL_GC_ALLOC);
        payload.insert("dsl.gc.heap_id", Value::from(scope.heap.heap_id));
        payload.insert(
            "dsl.gc.strategy",
            Value::String(format!("{:?}", scope.heap.strategy)),
        );
        payload.insert("dsl.gc.handle", Value::from(handle));
        payload.insert("dsl.gc.bytes", Value::from(bytes as u64));
        emit_audit(payload);
        Ok(GcRef::new(scope.heap.heap_id, handle))
    }

    pub fn pin<T>(mut scope: RootScope, value: &GcRef<T>) -> RootScope {
        if scope.heap.heap_id == value.heap_id {
            scope.roots.push(value.handle);
            if let Ok(mut state) = scope.heap.state.lock() {
                if let Some(entry) = state.entries.get_mut(&value.handle) {
                    entry.root_count = entry.root_count.saturating_add(1);
                    state.roots.insert(value.handle);
                }
            }
            let mut payload = AuditPayload::new(AUDIT_DSL_GC_ROOT);
            payload.insert("dsl.gc.heap_id", Value::from(scope.heap.heap_id));
            payload.insert("dsl.gc.handle", Value::from(value.handle));
            payload.insert("dsl.gc.action", Value::String("register".into()));
            emit_audit(payload);
        }
        scope
    }

    pub fn collect(heap: &GcHeap) -> Result<(), GcError> {
        let mut state = heap
            .state
            .lock()
            .map_err(|_| GcError::new(GcErrorKind::CollectFailed, "gc state lock failed"))?;
        let mut release_targets = Vec::new();
        for (handle, entry) in state.entries.iter() {
            if entry.root_count == 0 {
                release_targets.push(*handle);
            }
        }
        for handle in &release_targets {
            state.entries.remove(handle);
            state.roots.remove(handle);
        }
        let released = release_targets.len() as u64;
        let mut payload = AuditPayload::new(AUDIT_DSL_GC_RELEASE);
        payload.insert("dsl.gc.heap_id", Value::from(heap.heap_id));
        payload.insert(
            "dsl.gc.strategy",
            Value::String(format!("{:?}", heap.strategy)),
        );
        payload.insert("dsl.gc.released", Value::from(released));
        emit_audit(payload);
        Ok(())
    }

    pub fn collect_if_needed(heap: &GcHeap) -> Result<(), GcError> {
        Gc::collect(heap)
    }
}

impl Drop for RootScope {
    fn drop(&mut self) {
        if self.roots.is_empty() {
            return;
        }
        if let Ok(mut state) = self.heap.state.lock() {
            for handle in self.roots.drain(..) {
                if let Some(entry) = state.entries.get_mut(&handle) {
                    if entry.root_count > 0 {
                        entry.root_count -= 1;
                    }
                    if entry.root_count == 0 {
                        state.roots.remove(&handle);
                    }
                }
            }
            let mut payload = AuditPayload::new(AUDIT_DSL_GC_ROOT);
            payload.insert("dsl.gc.heap_id", Value::from(self.heap.heap_id));
            payload.insert("dsl.gc.action", Value::String("release".into()));
            emit_audit(payload);
        }
    }
}

impl IntoDiagnostic for GcError {
    fn into_diagnostic(self) -> GuardDiagnostic {
        let code = match self.kind {
            GcErrorKind::AllocationFailed => "dsl.gc.allocation_failed",
            GcErrorKind::CollectFailed => "dsl.gc.collect_failed",
        };
        GuardDiagnostic {
            code,
            domain: "dsl",
            severity: DiagnosticSeverity::Error,
            message: self.message,
            notes: Vec::new(),
            extensions: JsonMap::new(),
            audit_metadata: JsonMap::new(),
        }
    }
}
