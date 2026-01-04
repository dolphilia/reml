use std::{
    io,
    process::{Child, Command},
};

use serde_json::{Map, Value};

use crate::capability::AdapterCapability;

const PROCESS_EFFECT_SCOPE: &[&str] = &["effect {process}", "effect {security}"];

/// プロセス API の Capability。
pub const PROCESS_CAPABILITY: AdapterCapability = AdapterCapability::new(
    "adapter.process",
    "beta",
    PROCESS_EFFECT_SCOPE,
    "adapter.process",
);

/// 現在プロセスの PID。
pub fn current_pid() -> u32 {
    std::process::id()
}

/// サブプロセスを起動するヘルパ。
pub fn spawn(program: &str, args: &[&str]) -> io::Result<Child> {
    Command::new(program).args(args).spawn()
}

/// 監査メタデータ。
pub fn audit_metadata(operation: &str, status: &str) -> Map<String, Value> {
    PROCESS_CAPABILITY.audit_metadata(operation, status)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_reports_process_ids() {
        let metadata = audit_metadata("pid", "success");
        assert_eq!(metadata["capability.id"], "adapter.process");
        assert_eq!(metadata["adapter.process.operation"], "pid");
    }

    #[test]
    fn current_pid_matches_std() {
        assert_eq!(current_pid(), std::process::id());
    }
}
