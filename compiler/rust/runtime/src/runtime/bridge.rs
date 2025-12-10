use once_cell::sync::Lazy;
use serde::Serialize;
use std::sync::Mutex;
use std::time::SystemTime;

use crate::stage::{StageId, StageRequirement};

/// Stage 検証の記録。
#[derive(Debug, Clone, Serialize)]
pub struct BridgeStageRecord {
    pub capability: &'static str,
    pub required: StageRequirement,
    pub actual: StageId,
    pub timestamp: SystemTime,
}

impl BridgeStageRecord {
    pub fn requirement_label(&self) -> String {
        match self.required {
            StageRequirement::Exact(stage) => stage.as_str().into(),
            StageRequirement::AtLeast(stage) => format!("at_least {}", stage.as_str()),
        }
    }
}

/// Runtime Bridge の Stage 記録を管理するレジストリ。
pub struct RuntimeBridgeRegistry {
    stage_records: Mutex<Vec<BridgeStageRecord>>,
}

static REGISTRY: Lazy<RuntimeBridgeRegistry> = Lazy::new(|| RuntimeBridgeRegistry {
    stage_records: Mutex::new(Vec::new()),
});
#[cfg(test)]
static BRIDGE_TEST_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

impl RuntimeBridgeRegistry {
    pub fn global() -> &'static Self {
        &REGISTRY
    }

    pub fn record_stage_probe(
        &self,
        capability: &'static str,
        requirement: StageRequirement,
        actual: StageId,
    ) {
        let mut records = self
            .stage_records
            .lock()
            .expect("RuntimeBridgeRegistry.stage_records poisoned");
        records.retain(|entry| entry.capability != capability);
        records.push(BridgeStageRecord {
            capability,
            required: requirement,
            actual,
            timestamp: SystemTime::now(),
        });
    }

    pub fn latest_stage_record(&self, capability: &str) -> Option<BridgeStageRecord> {
        let records = self
            .stage_records
            .lock()
            .expect("RuntimeBridgeRegistry.stage_records poisoned");
        records
            .iter()
            .rev()
            .find(|entry| entry.capability == capability)
            .cloned()
    }

    pub fn stage_records(&self) -> Vec<BridgeStageRecord> {
        self.stage_records
            .lock()
            .expect("RuntimeBridgeRegistry.stage_records poisoned")
            .clone()
    }

    pub fn clear(&self) {
        self.stage_records
            .lock()
            .expect("RuntimeBridgeRegistry.stage_records poisoned")
            .clear();
    }

    #[cfg(test)]
    pub(crate) fn test_lock() -> &'static Mutex<()> {
        &BRIDGE_TEST_LOCK
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_and_query_stage_probe() {
        let _guard = RuntimeBridgeRegistry::test_lock().lock().unwrap();
        let registry = RuntimeBridgeRegistry::global();
        registry.clear();
        registry.record_stage_probe(
            "io.fs.bridge_query",
            StageRequirement::AtLeast(StageId::Beta),
            StageId::Stable,
        );
        let record = registry
            .latest_stage_record("io.fs.bridge_query")
            .expect("stage record must exist");
        assert_eq!(record.capability, "io.fs.bridge_query");
        assert_eq!(record.actual, StageId::Stable);
        assert_eq!(
            record.requirement_label(),
            "at_least beta",
            "requirement label should be human readable"
        );
        assert!(
            registry.latest_stage_record("missing").is_none(),
            "unrecorded capability should return None"
        );
    }

    #[test]
    fn stage_records_replace_duplicates() {
        let _guard = RuntimeBridgeRegistry::test_lock().lock().unwrap();
        let registry = RuntimeBridgeRegistry::global();
        registry.clear();
        registry.record_stage_probe(
            "io.fs.bridge_replace",
            StageRequirement::Exact(StageId::Stable),
            StageId::Stable,
        );
        registry.record_stage_probe(
            "io.fs.bridge_replace",
            StageRequirement::AtLeast(StageId::Beta),
            StageId::Stable,
        );
        let records = registry.stage_records();
        assert_eq!(
            records.len(),
            1,
            "duplicate capabilities should be replaced"
        );
        assert_eq!(records[0].capability, "io.fs.bridge_replace");
        assert_eq!(
            records[0].requirement_label(),
            "at_least beta",
            "latest requirement should be stored"
        );
    }
}
