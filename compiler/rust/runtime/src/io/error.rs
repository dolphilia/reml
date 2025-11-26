use std::error::Error;
use std::fmt;
use std::path::PathBuf;
use std::time::SystemTime;

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
        self.path = Some(path.into());
        self
    }

    pub fn with_context(mut self, context: IoContext) -> Self {
        self.context = Some(context);
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

    pub fn from_std(error: std::io::Error, operation: &'static str) -> Self {
        let kind = IoErrorKind::from(error.kind());
        IoError::new(kind, error.to_string()).with_context(IoContext::new(operation))
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
    pub operation: &'static str,
    pub bytes_processed: Option<u64>,
    pub timestamp: SystemTime,
}

impl IoContext {
    pub fn new(operation: &'static str) -> Self {
        Self {
            operation,
            bytes_processed: None,
            timestamp: SystemTime::now(),
        }
    }

    pub fn with_bytes_processed(mut self, bytes: u64) -> Self {
        self.bytes_processed = Some(bytes);
        self
    }
}
