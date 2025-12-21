//! Core.Dsl.* の最小ランタイム API。
//! 仕様: `docs/spec/3-16-core-dsl-paradigm-kits.md`.

pub mod actor;
pub mod gc;
pub mod object;
pub mod vm;

use serde::{Deserialize, Serialize};
use serde_json::{Map as JsonMap, Value};

pub use actor::{
    Actor, ActorDefinition, ActorError, ActorErrorKind, MailboxBridge, SupervisorSpec,
    SupervisionBridge,
};
pub use crate::runtime::async_bridge::ActorSystem;
pub use gc::{Gc, GcError, GcErrorKind, GcHeap, GcRef, GcStrategy, RootScope};
pub use object::{
    ClassBuilder, DispatchError, DispatchErrorKind, DispatchKind, DispatchTable, MethodCache,
    MethodCacheKey, MethodEntry, MethodId, Object, ObjectHandle, PrototypeBuilder,
};
pub use vm::{Bytecode, BytecodeBuilder, CallFrame, Vm, VmError, VmErrorKind, VmState};

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
pub const AUDIT_DSL_GC_ROOT: &str = "dsl.gc.root";
pub const AUDIT_DSL_ACTOR_MAILBOX: &str = "dsl.actor.mailbox";
pub const AUDIT_DSL_VM_EXECUTE: &str = "dsl.vm.execute";
