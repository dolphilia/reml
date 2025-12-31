use std::{error::Error, fmt};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{
    actor::ActorCapability, async_runtime::AsyncCapability, audit::AuditCapability,
    collections::CollectionsCapability, gc::GcCapability, hardware::HardwareCapability,
    io::IoCapability, memory::MemoryCapability, metrics::MetricsCapability,
    native::NativeCapability, plugin::PluginCapability, process::ProcessCapability,
    realtime::RealtimeCapability, security::SecurityCapability, signal::SignalCapability,
    system::SystemCapability, CapabilityDescriptor,
};

/// Capability ごとの型付きハンドル。
#[derive(Debug, Clone)]
pub enum CapabilityHandle {
    Gc(GcCapability),
    Io(IoCapability),
    Async(AsyncCapability),
    Collections(CollectionsCapability),
    Audit(AuditCapability),
    Metrics(MetricsCapability),
    Memory(MemoryCapability),
    Security(SecurityCapability),
    Native(NativeCapability),
    Plugin(PluginCapability),
    Actor(ActorCapability),
    Process(ProcessCapability),
    System(SystemCapability),
    Signal(SignalCapability),
    Hardware(HardwareCapability),
    Realtime(RealtimeCapability),
}

impl CapabilityHandle {
    pub fn descriptor(&self) -> &CapabilityDescriptor {
        match self {
            CapabilityHandle::Gc(handle) => handle.descriptor(),
            CapabilityHandle::Io(handle) => handle.descriptor(),
            CapabilityHandle::Async(handle) => handle.descriptor(),
            CapabilityHandle::Collections(handle) => handle.descriptor(),
            CapabilityHandle::Audit(handle) => handle.descriptor(),
            CapabilityHandle::Metrics(handle) => handle.descriptor(),
            CapabilityHandle::Memory(handle) => handle.descriptor(),
            CapabilityHandle::Security(handle) => handle.descriptor(),
            CapabilityHandle::Native(handle) => handle.descriptor(),
            CapabilityHandle::Plugin(handle) => handle.descriptor(),
            CapabilityHandle::Actor(handle) => handle.descriptor(),
            CapabilityHandle::Process(handle) => handle.descriptor(),
            CapabilityHandle::System(handle) => handle.descriptor(),
            CapabilityHandle::Signal(handle) => handle.descriptor(),
            CapabilityHandle::Hardware(handle) => handle.descriptor(),
            CapabilityHandle::Realtime(handle) => handle.descriptor(),
        }
    }

    pub fn kind(&self) -> CapabilityHandleKind {
        match self {
            CapabilityHandle::Gc(_) => CapabilityHandleKind::Gc,
            CapabilityHandle::Io(_) => CapabilityHandleKind::Io,
            CapabilityHandle::Async(_) => CapabilityHandleKind::Async,
            CapabilityHandle::Collections(_) => CapabilityHandleKind::Collections,
            CapabilityHandle::Audit(_) => CapabilityHandleKind::Audit,
            CapabilityHandle::Metrics(_) => CapabilityHandleKind::Metrics,
            CapabilityHandle::Memory(_) => CapabilityHandleKind::Memory,
            CapabilityHandle::Security(_) => CapabilityHandleKind::Security,
            CapabilityHandle::Native(_) => CapabilityHandleKind::Native,
            CapabilityHandle::Plugin(_) => CapabilityHandleKind::Plugin,
            CapabilityHandle::Actor(_) => CapabilityHandleKind::Actor,
            CapabilityHandle::Process(_) => CapabilityHandleKind::Process,
            CapabilityHandle::System(_) => CapabilityHandleKind::System,
            CapabilityHandle::Signal(_) => CapabilityHandleKind::Signal,
            CapabilityHandle::Hardware(_) => CapabilityHandleKind::Hardware,
            CapabilityHandle::Realtime(_) => CapabilityHandleKind::Realtime,
        }
    }

    pub fn as_gc(&self) -> Option<&GcCapability> {
        match self {
            CapabilityHandle::Gc(handle) => Some(handle),
            _ => None,
        }
    }

    pub fn as_io(&self) -> Option<&IoCapability> {
        match self {
            CapabilityHandle::Io(handle) => Some(handle),
            _ => None,
        }
    }

    pub fn as_async(&self) -> Option<&AsyncCapability> {
        match self {
            CapabilityHandle::Async(handle) => Some(handle),
            _ => None,
        }
    }

    pub fn as_collections(&self) -> Option<&CollectionsCapability> {
        match self {
            CapabilityHandle::Collections(handle) => Some(handle),
            _ => None,
        }
    }

    pub fn as_audit(&self) -> Option<&AuditCapability> {
        match self {
            CapabilityHandle::Audit(handle) => Some(handle),
            _ => None,
        }
    }

    pub fn as_metrics(&self) -> Option<&MetricsCapability> {
        match self {
            CapabilityHandle::Metrics(handle) => Some(handle),
            _ => None,
        }
    }

    pub fn as_memory(&self) -> Option<&MemoryCapability> {
        match self {
            CapabilityHandle::Memory(handle) => Some(handle),
            _ => None,
        }
    }

    pub fn as_security(&self) -> Option<&SecurityCapability> {
        match self {
            CapabilityHandle::Security(handle) => Some(handle),
            _ => None,
        }
    }

    pub fn as_native(&self) -> Option<&NativeCapability> {
        match self {
            CapabilityHandle::Native(handle) => Some(handle),
            _ => None,
        }
    }

    pub fn as_plugin(&self) -> Option<&PluginCapability> {
        match self {
            CapabilityHandle::Plugin(handle) => Some(handle),
            _ => None,
        }
    }

    pub fn as_actor(&self) -> Option<&ActorCapability> {
        match self {
            CapabilityHandle::Actor(handle) => Some(handle),
            _ => None,
        }
    }

    pub fn as_process(&self) -> Option<&ProcessCapability> {
        match self {
            CapabilityHandle::Process(handle) => Some(handle),
            _ => None,
        }
    }

    pub fn as_system(&self) -> Option<&SystemCapability> {
        match self {
            CapabilityHandle::System(handle) => Some(handle),
            _ => None,
        }
    }

    pub fn as_signal(&self) -> Option<&SignalCapability> {
        match self {
            CapabilityHandle::Signal(handle) => Some(handle),
            _ => None,
        }
    }

    pub fn as_hardware(&self) -> Option<&HardwareCapability> {
        match self {
            CapabilityHandle::Hardware(handle) => Some(handle),
            _ => None,
        }
    }

    pub fn as_realtime(&self) -> Option<&RealtimeCapability> {
        match self {
            CapabilityHandle::Realtime(handle) => Some(handle),
            _ => None,
        }
    }
}

impl From<GcCapability> for CapabilityHandle {
    fn from(value: GcCapability) -> Self {
        CapabilityHandle::Gc(value)
    }
}

impl From<IoCapability> for CapabilityHandle {
    fn from(value: IoCapability) -> Self {
        CapabilityHandle::Io(value)
    }
}

impl From<AsyncCapability> for CapabilityHandle {
    fn from(value: AsyncCapability) -> Self {
        CapabilityHandle::Async(value)
    }
}

impl From<CollectionsCapability> for CapabilityHandle {
    fn from(value: CollectionsCapability) -> Self {
        CapabilityHandle::Collections(value)
    }
}

impl From<AuditCapability> for CapabilityHandle {
    fn from(value: AuditCapability) -> Self {
        CapabilityHandle::Audit(value)
    }
}

impl From<MetricsCapability> for CapabilityHandle {
    fn from(value: MetricsCapability) -> Self {
        CapabilityHandle::Metrics(value)
    }
}

impl From<MemoryCapability> for CapabilityHandle {
    fn from(value: MemoryCapability) -> Self {
        CapabilityHandle::Memory(value)
    }
}

impl From<SecurityCapability> for CapabilityHandle {
    fn from(value: SecurityCapability) -> Self {
        CapabilityHandle::Security(value)
    }
}

impl From<NativeCapability> for CapabilityHandle {
    fn from(value: NativeCapability) -> Self {
        CapabilityHandle::Native(value)
    }
}

impl From<PluginCapability> for CapabilityHandle {
    fn from(value: PluginCapability) -> Self {
        CapabilityHandle::Plugin(value)
    }
}

impl From<ActorCapability> for CapabilityHandle {
    fn from(value: ActorCapability) -> Self {
        CapabilityHandle::Actor(value)
    }
}

impl From<ProcessCapability> for CapabilityHandle {
    fn from(value: ProcessCapability) -> Self {
        CapabilityHandle::Process(value)
    }
}

impl From<SystemCapability> for CapabilityHandle {
    fn from(value: SystemCapability) -> Self {
        CapabilityHandle::System(value)
    }
}

impl From<SignalCapability> for CapabilityHandle {
    fn from(value: SignalCapability) -> Self {
        CapabilityHandle::Signal(value)
    }
}

impl From<HardwareCapability> for CapabilityHandle {
    fn from(value: HardwareCapability) -> Self {
        CapabilityHandle::Hardware(value)
    }
}

impl From<RealtimeCapability> for CapabilityHandle {
    fn from(value: RealtimeCapability) -> Self {
        CapabilityHandle::Realtime(value)
    }
}

/// CapabilityHandle の種別。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityHandleKind {
    Gc,
    Io,
    Async,
    Collections,
    Audit,
    Metrics,
    Memory,
    Security,
    Native,
    Plugin,
    Actor,
    Process,
    System,
    Signal,
    Hardware,
    Realtime,
}

/// ハンドル種別不一致。
#[derive(Debug, Clone)]
pub struct CapabilityHandleTypeError {
    expected: CapabilityHandleKind,
    actual: CapabilityHandleKind,
}

impl CapabilityHandleTypeError {
    pub fn new(expected: CapabilityHandleKind, actual: CapabilityHandleKind) -> Self {
        Self { expected, actual }
    }

    pub fn expected(&self) -> CapabilityHandleKind {
        self.expected
    }

    pub fn actual(&self) -> CapabilityHandleKind {
        self.actual
    }
}

impl fmt::Display for CapabilityHandleTypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "capability handle mismatch: expected {:?}, got {:?}",
            self.expected, self.actual
        )
    }
}

impl Error for CapabilityHandleTypeError {}

macro_rules! impl_try_from_handle {
    ($ty:ty, $variant:ident, $kind:ident) => {
        impl TryFrom<CapabilityHandle> for $ty {
            type Error = CapabilityHandleTypeError;

            fn try_from(value: CapabilityHandle) -> Result<Self, Self::Error> {
                if let CapabilityHandle::$variant(inner) = value {
                    Ok(inner)
                } else {
                    Err(CapabilityHandleTypeError::new(
                        CapabilityHandleKind::$kind,
                        value.kind(),
                    ))
                }
            }
        }

        impl<'a> TryFrom<&'a CapabilityHandle> for &'a $ty {
            type Error = CapabilityHandleTypeError;

            fn try_from(value: &'a CapabilityHandle) -> Result<Self, Self::Error> {
                if let CapabilityHandle::$variant(inner) = value {
                    Ok(inner)
                } else {
                    Err(CapabilityHandleTypeError::new(
                        CapabilityHandleKind::$kind,
                        value.kind(),
                    ))
                }
            }
        }
    };
}

impl_try_from_handle!(GcCapability, Gc, Gc);
impl_try_from_handle!(IoCapability, Io, Io);
impl_try_from_handle!(AsyncCapability, Async, Async);
impl_try_from_handle!(CollectionsCapability, Collections, Collections);
impl_try_from_handle!(AuditCapability, Audit, Audit);
impl_try_from_handle!(MetricsCapability, Metrics, Metrics);
impl_try_from_handle!(MemoryCapability, Memory, Memory);
impl_try_from_handle!(SecurityCapability, Security, Security);
impl_try_from_handle!(NativeCapability, Native, Native);
impl_try_from_handle!(PluginCapability, Plugin, Plugin);
impl_try_from_handle!(ActorCapability, Actor, Actor);
impl_try_from_handle!(ProcessCapability, Process, Process);
impl_try_from_handle!(SystemCapability, System, System);
impl_try_from_handle!(SignalCapability, Signal, Signal);
impl_try_from_handle!(HardwareCapability, Hardware, Hardware);
impl_try_from_handle!(RealtimeCapability, Realtime, Realtime);
