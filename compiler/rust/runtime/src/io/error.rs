use std::error::Error;
use std::fmt;
use std::path::PathBuf;

#[cfg(any(feature = "core_time", feature = "metrics"))]
use crate::time::{self, Timestamp};
use crate::prelude::iter::EffectLabels;
#[cfg(not(any(feature = "core_time", feature = "metrics")))]
use std::time::SystemTime as Timestamp;

/// IO 操作共通の結果型。
pub type IoResult<T> = Result<T, IoError>;

/// Core.IO 互換エラー。
#[derive(Debug, Clone)]
pub struct IoError {
    kind: IoErrorKind,
    message: String,
    path: Option<PathBuf>,
    context: Option<IoContext>,
}

impl IoError {
    pub fn new(kind: IoErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            path: None,
            context: None,
        }
    }

    pub fn with_path(mut self, path: impl Into<PathBuf>) -> Self {
        let path_buf = path.into();
        if let Some(context) = self.context.as_mut() {
            context.set_path(path_buf.clone());
        }
        self.path = Some(path_buf);
        self
    }

    pub fn with_context(mut self, context: IoContext) -> Self {
        if self.path.is_none() {
            if let Some(path) = context.path() {
                self.path = Some(path.to_path_buf());
            }
        }
        self.context = Some(context);
        self
    }

    pub fn map_context<F>(mut self, f: F) -> Self
    where
        F: FnOnce(IoContext) -> IoContext,
    {
        if let Some(context) = self.context.take() {
            self.context = Some(f(context));
        }
        self
    }

    pub fn kind(&self) -> IoErrorKind {
        self.kind
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn path(&self) -> Option<&PathBuf> {
        self.path.as_ref()
    }

    pub fn context(&self) -> Option<&IoContext> {
        self.context.as_ref()
    }

    pub fn from_std(error: std::io::Error, context: IoContext) -> Self {
        let kind = IoErrorKind::from(error.kind());
        IoError::new(kind, error.to_string()).with_context(context)
    }
}

impl fmt::Display for IoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.message)
    }
}

impl Error for IoError {}

/// 仕様に沿った IO エラー種別。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoErrorKind {
    NotFound,
    PermissionDenied,
    ConnectionRefused,
    InvalidInput,
    TimedOut,
    WriteZero,
    Interrupted,
    UnexpectedEof,
    OutOfMemory,
    SecurityViolation,
    UnsupportedPlatform,
}

impl From<std::io::ErrorKind> for IoErrorKind {
    fn from(kind: std::io::ErrorKind) -> Self {
        use std::io::ErrorKind as Std;
        match kind {
            Std::NotFound => IoErrorKind::NotFound,
            Std::PermissionDenied => IoErrorKind::PermissionDenied,
            Std::ConnectionRefused | Std::ConnectionReset | Std::ConnectionAborted => {
                IoErrorKind::ConnectionRefused
            }
            Std::BrokenPipe | Std::NotConnected | Std::AddrInUse | Std::AddrNotAvailable => {
                IoErrorKind::InvalidInput
            }
            Std::TimedOut => IoErrorKind::TimedOut,
            Std::WriteZero => IoErrorKind::WriteZero,
            Std::Interrupted => IoErrorKind::Interrupted,
            Std::UnexpectedEof => IoErrorKind::UnexpectedEof,
            Std::OutOfMemory => IoErrorKind::OutOfMemory,
            Std::Unsupported => IoErrorKind::UnsupportedPlatform,
            Std::WouldBlock | Std::InvalidInput | Std::InvalidData => IoErrorKind::InvalidInput,
            _ => IoErrorKind::InvalidInput,
        }
    }
}

/// IO 操作の文脈情報。
#[derive(Debug, Clone)]
pub struct IoContext {
    operation: &'static str,
    path: Option<PathBuf>,
    capability: Option<&'static str>,
    bytes_processed: Option<u64>,
    timestamp: Timestamp,
    effects: EffectLabels,
}

impl IoContext {
    pub fn new(operation: &'static str) -> Self {
        Self {
            operation,
            path: None,
            capability: None,
            bytes_processed: None,
            timestamp: current_timestamp(),
            effects: empty_effect_labels(),
        }
    }

    pub fn with_bytes_processed(mut self, bytes: u64) -> Self {
        self.bytes_processed = Some(bytes);
        self
    }

    pub fn with_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.path = Some(path.into());
        self
    }

    pub fn with_capability(mut self, capability: &'static str) -> Self {
        self.capability = Some(capability);
        self
    }

    pub fn set_path(&mut self, path: PathBuf) {
        self.path = Some(path);
    }

    pub fn operation(&self) -> &'static str {
        self.operation
    }

    pub fn path(&self) -> Option<&PathBuf> {
        self.path.as_ref()
    }

    pub fn capability(&self) -> Option<&'static str> {
        self.capability
    }

    pub fn bytes_processed(&self) -> Option<u64> {
        self.bytes_processed
    }

    pub fn timestamp(&self) -> Timestamp {
        self.timestamp
    }

    pub fn effects(&self) -> EffectLabels {
        self.effects
    }

    pub fn with_effects(mut self, effects: EffectLabels) -> Self {
        self.effects = effects;
        self
    }
}

fn empty_effect_labels() -> EffectLabels {
    EffectLabels {
        mem: false,
        mutating: false,
        debug: false,
        async_pending: false,
        audit: false,
        cell: false,
        rc: false,
        unicode: false,
        io: false,
        io_blocking: false,
        io_async: false,
        security: false,
        transfer: false,
        mem_bytes: 0,
        predicate_calls: 0,
        rc_ops: 0,
        time: false,
        time_calls: 0,
        io_blocking_calls: 0,
        io_async_calls: 0,
        security_events: 0,
    }
}

fn current_timestamp() -> Timestamp {
    #[cfg(any(feature = "core_time", feature = "metrics"))]
    {
        time::now().unwrap_or_else(|_| Timestamp::unix_epoch())
    }
    #[cfg(not(any(feature = "core_time", feature = "metrics")))]
    {
        Timestamp::now()
    }
}
