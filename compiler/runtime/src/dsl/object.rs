//! Core.Dsl.Object の最小実装。

use std::collections::HashMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::Arc;

use serde_json::{Map as JsonMap, Value as JsonValue};

use crate::dsl::{emit_audit, AuditPayload, AUDIT_DSL_OBJECT_DISPATCH};
use crate::prelude::ensure::{DiagnosticSeverity, GuardDiagnostic, IntoDiagnostic};

/// メソッド識別子。
pub type MethodId = String;

/// ディスパッチ方式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchKind {
    ClassBased,
    PrototypeBased,
}

/// メソッド本体。
pub type MethodEntry<Payload> = fn(ObjectHandle<Payload>, Vec<Payload>) -> DispatchResult<Payload>;

/// ディスパッチテーブル。
#[derive(Clone)]
pub struct DispatchTable<Payload> {
    pub kind: DispatchKind,
    pub name: String,
    pub parent: Option<Arc<DispatchTable<Payload>>>,
    pub methods: HashMap<MethodId, MethodEntry<Payload>>,
}

impl<Payload> DispatchTable<Payload> {
    pub fn new(
        kind: DispatchKind,
        name: impl Into<String>,
        parent: Option<Arc<DispatchTable<Payload>>>,
        methods: HashMap<MethodId, MethodEntry<Payload>>,
    ) -> Self {
        Self {
            kind,
            name: name.into(),
            parent,
            methods,
        }
    }
}

/// オブジェクトハンドル。
#[derive(Clone)]
pub struct ObjectHandle<Value> {
    pub table: Arc<DispatchTable<Value>>,
    pub payload: Value,
    pub shape_id: u64,
}

impl<Payload> ObjectHandle<Payload> {
    /// GC と接続する場合は `payload` に `GcRef<T>` を入れ、`RootScope` を保持すること。
    pub fn new(table: Arc<DispatchTable<Payload>>, payload: Payload, shape_id: u64) -> Self {
        Self {
            table,
            payload,
            shape_id,
        }
    }

    pub fn map_payload(self, f: impl FnOnce(Payload) -> Payload) -> Self {
        Self {
            table: self.table,
            payload: f(self.payload),
            shape_id: self.shape_id,
        }
    }
}

/// メソッドキャッシュのキー。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MethodCacheKey {
    pub shape_id: u64,
    pub name: MethodId,
}

/// 最小のメソッドキャッシュ。
#[derive(Default)]
pub struct MethodCache<Payload> {
    entries: HashMap<MethodCacheKey, MethodEntry<Payload>>,
}

impl<Payload> MethodCache<Payload> {
    pub fn lookup(&self, key: &MethodCacheKey) -> Option<MethodEntry<Payload>> {
        self.entries.get(key).cloned()
    }

    pub fn record(&mut self, key: MethodCacheKey, entry: MethodEntry<Payload>) {
        self.entries.insert(key, entry);
    }

    pub fn invalidate(&mut self, _table: &DispatchTable<Payload>) {
        self.entries.clear();
    }
}

/// ディスパッチエラー。
#[derive(Debug, Clone)]
pub struct DispatchError {
    pub kind: DispatchErrorKind,
    pub message: String,
}

impl DispatchError {
    pub fn new(kind: DispatchErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

/// ディスパッチエラー種別。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchErrorKind {
    MethodNotFound,
    ArityMismatch,
    RuntimeFailure,
}

pub type DispatchResult<T> = Result<T, DispatchError>;

/// Core.Dsl.Object の名前空間。
pub struct Object;

impl Object {
    pub fn call<Payload>(
        obj: ObjectHandle<Payload>,
        name: &str,
        args: Vec<Payload>,
        cache: Option<&mut MethodCache<Payload>>,
    ) -> DispatchResult<Payload> {
        let mut payload = AuditPayload::new(AUDIT_DSL_OBJECT_DISPATCH);
        payload.insert("dsl.object.method", JsonValue::String(name.to_string()));
        payload.insert("dsl.object.shape_id", JsonValue::from(obj.shape_id));
        payload.insert(
            "dsl.object.table",
            JsonValue::String(obj.table.name.clone()),
        );
        payload.insert(
            "dsl.object.kind",
            JsonValue::String(format!("{:?}", obj.table.kind)),
        );
        emit_audit(payload);

        let key = MethodCacheKey {
            shape_id: obj.shape_id,
            name: name.to_string(),
        };
        if let Some(cache) = cache.as_ref() {
            if let Some(entry) = cache.lookup(&key) {
                return invoke_entry(entry, obj, args);
            }
        }
        let entry = lookup(obj.table.as_ref(), name).ok_or_else(|| {
            DispatchError::new(
                DispatchErrorKind::MethodNotFound,
                format!("method not found: {name}"),
            )
        })?;
        if let Some(cache) = cache {
            cache.record(key, entry.clone());
        }
        invoke_entry(entry, obj, args)
    }

    pub fn lookup<Payload>(
        table: &DispatchTable<Payload>,
        name: &str,
    ) -> Option<MethodEntry<Payload>> {
        lookup(table, name)
    }

    pub fn class_builder<Payload>(name: impl Into<String>) -> ClassBuilder<Payload> {
        ClassBuilder::new(name)
    }

    pub fn prototype_builder<Payload>(name: impl Into<String>) -> PrototypeBuilder<Payload> {
        PrototypeBuilder::new(name)
    }
}

fn lookup<Payload>(table: &DispatchTable<Payload>, name: &str) -> Option<MethodEntry<Payload>> {
    if let Some(entry) = table.methods.get(name) {
        return Some(entry.clone());
    }
    table
        .parent
        .as_ref()
        .and_then(|parent| lookup(parent, name))
}

fn invoke_entry<Payload>(
    entry: MethodEntry<Payload>,
    obj: ObjectHandle<Payload>,
    args: Vec<Payload>,
) -> DispatchResult<Payload> {
    catch_unwind(AssertUnwindSafe(|| entry(obj, args))).unwrap_or_else(|_| {
        Err(DispatchError::new(
            DispatchErrorKind::RuntimeFailure,
            "method dispatch panicked",
        ))
    })
}

/// クラスビルダー。
#[derive(Clone)]
pub struct ClassBuilder<Value> {
    name: String,
    parent: Option<Arc<DispatchTable<Value>>>,
    methods: HashMap<MethodId, MethodEntry<Value>>,
}

impl<Payload> ClassBuilder<Payload> {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            parent: None,
            methods: HashMap::new(),
        }
    }

    pub fn method(mut self, name: impl Into<String>, entry: MethodEntry<Payload>) -> Self {
        self.methods.insert(name.into(), entry);
        self
    }

    pub fn extend(mut self, parent: Arc<DispatchTable<Payload>>) -> Self {
        self.parent = Some(parent);
        self
    }

    pub fn build(self) -> DispatchTable<Payload> {
        DispatchTable::new(
            DispatchKind::ClassBased,
            self.name,
            self.parent,
            self.methods,
        )
    }
}

/// プロトタイプビルダー。
#[derive(Clone)]
pub struct PrototypeBuilder<Payload> {
    name: String,
    parent: Option<Arc<DispatchTable<Payload>>>,
    methods: HashMap<MethodId, MethodEntry<Payload>>,
}

impl<Payload> PrototypeBuilder<Payload> {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            parent: None,
            methods: HashMap::new(),
        }
    }

    pub fn method(mut self, name: impl Into<String>, entry: MethodEntry<Payload>) -> Self {
        self.methods.insert(name.into(), entry);
        self
    }

    pub fn delegate(mut self, parent: Arc<DispatchTable<Payload>>) -> Self {
        self.parent = Some(parent);
        self
    }

    pub fn build(self) -> DispatchTable<Payload> {
        DispatchTable::new(
            DispatchKind::PrototypeBased,
            self.name,
            self.parent,
            self.methods,
        )
    }
}

impl IntoDiagnostic for DispatchError {
    fn into_diagnostic(self) -> GuardDiagnostic {
        let code = match self.kind {
            DispatchErrorKind::MethodNotFound | DispatchErrorKind::ArityMismatch => {
                "dsl.object.dispatch_failed"
            }
            DispatchErrorKind::RuntimeFailure => "dsl.object.runtime_failure",
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
