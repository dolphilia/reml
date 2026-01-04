use serde::{Deserialize, Serialize};
use std::env;
use std::error::Error;
use std::fmt;

pub type EnvResult<T> = Result<T, EnvError>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlatformInfo {
    pub os: Option<String>,
    pub arch: Option<String>,
    pub family: Option<String>,
}

impl PlatformInfo {
    pub fn from_current() -> Self {
        Self {
            os: Some(std::env::consts::OS.into()),
            arch: Some(std::env::consts::ARCH.into()),
            family: Some(std::env::consts::FAMILY.into()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnvContext {
    pub operation_name: String,
    pub platform: PlatformInfo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EnvErrorKind {
    NotFound,
    PermissionDenied,
    InvalidEncoding,
    UnsupportedPlatform,
    IoFailure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnvError {
    pub kind: EnvErrorKind,
    pub message: String,
    pub key: Option<String>,
    pub context: Option<EnvContext>,
}

impl EnvError {
    pub fn new(kind: EnvErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            key: None,
            context: None,
        }
    }

    pub fn with_key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }

    pub fn with_context(mut self, context: EnvContext) -> Self {
        self.context = Some(context);
        self
    }
}

impl fmt::Display for EnvError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.message)
    }
}

impl Error for EnvError {}

pub fn get_env(key: &str) -> EnvResult<Option<String>> {
    let context = context_for("get");
    match env::var(key) {
        Ok(value) => Ok(Some(value)),
        Err(env::VarError::NotPresent) => Ok(None),
        Err(env::VarError::NotUnicode(_)) => Err(EnvError::new(
            EnvErrorKind::InvalidEncoding,
            "environment variable is not valid UTF-8",
        )
        .with_key(key)
        .with_context(context)),
    }
}

pub fn set_env(key: &str, value: &str) -> EnvResult<()> {
    let _context = context_for("set");
    env::set_var(key, value);
    Ok(())
}

pub fn remove_env(key: &str) -> EnvResult<()> {
    let _context = context_for("remove");
    env::remove_var(key);
    Ok(())
}

fn context_for(operation: &str) -> EnvContext {
    EnvContext {
        operation_name: operation.to_string(),
        platform: PlatformInfo::from_current(),
    }
}
