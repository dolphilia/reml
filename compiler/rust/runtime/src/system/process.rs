use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use crate::path::PathBuf;
use crate::runtime::api::guard_capability;
use crate::runtime::Signal;
use crate::stage::{StageId, StageRequirement};

#[cfg(any(feature = "core_time", feature = "metrics"))]
use crate::time::{Duration, Timestamp};
#[cfg(not(any(feature = "core_time", feature = "metrics")))]
use std::time::{Duration, SystemTime as Timestamp};

const CAP_PROCESS: &str = "core.process";
const EFFECTS_PROCESS: &[&str] = &["process"];
const EFFECTS_PROCESS_BLOCKING: &[&str] = &["process", "io.blocking"];
const EFFECTS_PROCESS_SIGNAL: &[&str] = &["process", "signal"];

pub type ProcessId = i64;
pub type ExitStatus = i64;
pub type ProcessResult<T> = Result<T, ProcessError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command {
    pub program: PathBuf,
    pub args: Vec<String>,
    pub cwd: Option<PathBuf>,
    pub env: Option<BTreeMap<String, String>>,
}

impl Command {
    pub fn new(program: PathBuf) -> Self {
        Self {
            program,
            args: Vec::new(),
            cwd: None,
            env: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnOptions {
    pub stdin: Option<PathBuf>,
    pub stdout: Option<PathBuf>,
    pub stderr: Option<PathBuf>,
    pub detach: bool,
}

impl Default for SpawnOptions {
    fn default() -> Self {
        Self {
            stdin: None,
            stdout: None,
            stderr: None,
            detach: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessHandle {
    pub pid: ProcessId,
    pub started_at: Option<Timestamp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessErrorKind {
    SpawnFailed,
    PermissionDenied,
    TimedOut,
    TerminatedBySignal,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessError {
    pub kind: ProcessErrorKind,
    pub message: String,
    pub context: Option<String>,
}

impl ProcessError {
    pub fn new(kind: ProcessErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            context: None,
        }
    }

    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }
}

impl fmt::Display for ProcessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.message)
    }
}

impl Error for ProcessError {}

pub fn spawn(_command: Command, _options: SpawnOptions) -> ProcessResult<ProcessHandle> {
    ensure_process_capability(EFFECTS_PROCESS)?;
    Err(ProcessError::new(
        ProcessErrorKind::Unsupported,
        "process spawn is not wired in this runtime",
    )
    .with_context("core.system.process.spawn"))
}

pub fn wait(_handle: ProcessHandle, _timeout: Option<Duration>) -> ProcessResult<ExitStatus> {
    ensure_process_capability(EFFECTS_PROCESS_BLOCKING)?;
    Err(ProcessError::new(
        ProcessErrorKind::Unsupported,
        "process wait is not wired in this runtime",
    )
    .with_context("core.system.process.wait"))
}

pub fn kill(_handle: ProcessHandle, _signal: Signal) -> ProcessResult<()> {
    ensure_process_capability(EFFECTS_PROCESS_SIGNAL)?;
    Err(ProcessError::new(
        ProcessErrorKind::Unsupported,
        "process kill is not wired in this runtime",
    )
    .with_context("core.system.process.kill"))
}

fn ensure_process_capability(required_effects: &[&str]) -> ProcessResult<()> {
    let requirement = StageRequirement::AtLeast(StageId::Experimental);
    guard_capability(CAP_PROCESS, requirement, required_effects)
        .map(|_| ())
        .map_err(|err| {
            ProcessError::new(ProcessErrorKind::Unsupported, err.detail().to_string())
                .with_context(CAP_PROCESS)
        })
}
