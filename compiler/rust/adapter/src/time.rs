use std::{thread, time::{Duration, Instant, SystemTime}};

use serde_json::{Map, Value};

use crate::capability::AdapterCapability;

const TIME_EFFECT_SCOPE: &[&str] = &["effect {io.timer}"];

/// 時刻 API の Capability。
pub const TIME_CAPABILITY: AdapterCapability = AdapterCapability::new(
    "adapter.time",
    "stable",
    TIME_EFFECT_SCOPE,
    "adapter.time",
);

/// モノトニック時計。
pub fn monotonic_now() -> Instant {
    Instant::now()
}

/// システム時刻。
pub fn system_now() -> SystemTime {
    SystemTime::now()
}

/// 休止。
pub fn sleep(duration: Duration) {
    thread::sleep(duration);
}

/// 監査メタデータ。
pub fn audit_metadata(operation: &str, status: &str) -> Map<String, Value> {
    TIME_CAPABILITY.audit_metadata(operation, status)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn metadata_reflects_time_operation() {
        let metadata = audit_metadata("monotonic", "success");
        assert_eq!(metadata["capability.id"], "adapter.time");
        assert_eq!(metadata["adapter.time.operation"], "monotonic");
    }

    #[test]
    fn monotonic_progresses() {
        let start = monotonic_now();
        sleep(Duration::from_millis(1));
        assert!(monotonic_now() >= start);
    }
}
