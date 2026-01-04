//! Core.Dsl.* の最小ランタイム API。
//! 仕様: `docs/spec/3-16-core-dsl-paradigm-kits.md`.

pub mod actor;
pub mod gc;
pub mod object;
pub mod vm;

use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use serde_json::{Map as JsonMap, Value};
use std::sync::Arc;

use crate::prelude::ensure::{DiagnosticSeverity, GuardDiagnostic, IntoDiagnostic};
pub use crate::runtime::async_bridge::ActorSystem;
pub use actor::{
    Actor, ActorDefinition, ActorError, ActorErrorKind, MailboxBridge, SupervisionBridge,
    SupervisorSpec,
};
pub use gc::{Gc, GcError, GcErrorKind, GcHeap, GcRef, GcStrategy, RootScope};
pub use object::{
    ClassBuilder, DispatchError, DispatchErrorKind, DispatchKind, DispatchTable, MethodCache,
    MethodCacheKey, MethodEntry, MethodId, Object, ObjectHandle, PrototypeBuilder,
};
pub use vm::{
    Bytecode, BytecodeBuilder, CallFrame, Vm, VmCore, VmError, VmErrorKind, VmState, VmTraceEvent,
};

/// Core.Dsl 全体で共有する Result 型。
pub type DslResult<T> = std::result::Result<T, DslError>;

/// Core.Dsl の共通エラー表現。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DslError {
    pub kind: DslErrorKind,
    pub message: String,
}

impl DslError {
    pub fn new(kind: DslErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

/// Core.Dsl のエラー種別。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DslErrorKind {
    NotImplemented,
    InvalidArgument,
    RuntimeFailure,
}

/// Core.Dsl の監査ペイロード。
#[derive(Debug, Clone)]
pub struct AuditPayload {
    pub event: String,
    pub metadata: JsonMap<String, Value>,
}

impl AuditPayload {
    pub fn new(event: impl Into<String>) -> Self {
        let event = event.into();
        let mut metadata = JsonMap::new();
        metadata.insert("event.kind".into(), Value::String(event.clone()));
        Self { event, metadata }
    }

    pub fn insert(&mut self, key: impl Into<String>, value: Value) {
        self.metadata.insert(key.into(), value);
    }
}

/// 監査イベント名（最小セット）。
pub const AUDIT_DSL_OBJECT_DISPATCH: &str = "dsl.object.dispatch";
pub const AUDIT_DSL_GC_ALLOC: &str = "dsl.gc.alloc";
pub const AUDIT_DSL_GC_ROOT: &str = "dsl.gc.root";
pub const AUDIT_DSL_GC_RELEASE: &str = "dsl.gc.release";
pub const AUDIT_DSL_ACTOR_MAILBOX: &str = "dsl.actor.mailbox";
pub const AUDIT_DSL_VM_EXECUTE: &str = "dsl.vm.execute";

/// DSL 監査フック。
pub type DslAuditHook = Arc<dyn Fn(AuditPayload) + Send + Sync>;

static DSL_AUDIT_HOOK: OnceCell<DslAuditHook> = OnceCell::new();

/// DSL 監査フックを登録する（1 回のみ）。
pub fn set_dsl_audit_hook(hook: DslAuditHook) -> DslResult<()> {
    DSL_AUDIT_HOOK
        .set(hook)
        .map_err(|_| DslError::new(DslErrorKind::InvalidArgument, "dsl audit hook already set"))
}

pub(crate) fn emit_audit(payload: AuditPayload) {
    if let Some(hook) = DSL_AUDIT_HOOK.get() {
        hook(payload);
    }
}

impl IntoDiagnostic for DslError {
    fn into_diagnostic(self) -> GuardDiagnostic {
        let code = match self.kind {
            DslErrorKind::NotImplemented => "dsl.not_implemented",
            DslErrorKind::InvalidArgument => "dsl.invalid_argument",
            DslErrorKind::RuntimeFailure => "dsl.runtime_failure",
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
