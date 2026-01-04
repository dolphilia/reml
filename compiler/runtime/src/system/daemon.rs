use crate::path::PathBuf;
use super::process::{ProcessError, ProcessErrorKind, ProcessId, ProcessResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonConfig {
    pub name: String,
    pub pid_file: Option<PathBuf>,
    pub user: Option<String>,
    pub group: Option<String>,
}

pub fn daemonize(_config: DaemonConfig) -> ProcessResult<()> {
    Err(ProcessError::new(
        ProcessErrorKind::Unsupported,
        "daemonize is not wired in this runtime",
    )
    .with_context("core.system.daemon.daemonize"))
}

pub fn write_pid_file(_path: PathBuf, _pid: ProcessId) -> ProcessResult<()> {
    Err(ProcessError::new(
        ProcessErrorKind::Unsupported,
        "write_pid_file is not wired in this runtime",
    )
    .with_context("core.system.daemon.write_pid_file"))
}

// TODO: Core.System.Daemon の拡張は Phase 5 で追加する。
