use once_cell::sync::Lazy;
use serde::Serialize;
use serde_json::{Map as JsonMap, Value};
use std::sync::Mutex;
use std::time::SystemTime;

use crate::stage::{StageId, StageRequirement};

/// Stage 検証の記録。
#[derive(Debug, Clone, Serialize)]
pub struct BridgeStageRecord {
    pub capability: String,
    pub required: StageRequirement,
    pub actual: StageId,
    pub timestamp: SystemTime,
    pub kind: Option<String>,
    pub engine: Option<String>,
    pub bundle_hash: Option<String>,
    pub module_hash: Option<String>,
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
        capability: impl Into<String>,
        requirement: StageRequirement,
        actual: StageId,
    ) {
        self.record_stage_probe_with_metadata(
            capability,
            requirement,
            actual,
            BridgeMetadata::none(),
        );
    }

    pub fn record_stage_probe_with_metadata(
        &self,
        capability: impl Into<String>,
        requirement: StageRequirement,
        actual: StageId,
        metadata: BridgeMetadata,
    ) {
        let capability = capability.into();
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
            kind: metadata.kind,
            engine: metadata.engine,
            bundle_hash: metadata.bundle_hash,
            module_hash: metadata.module_hash,
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

/// Runtime Bridge の Stage 記録を監査メタデータへ転写する。
pub fn attach_bridge_stage_metadata(
    bridge_id: &str,
    capability: &str,
    metadata: &mut JsonMap<String, Value>,
) {
    let snapshot = RuntimeBridgeRegistry::global().latest_stage_record(capability);
    let Some(snapshot) = snapshot else {
        return;
    };

    if metadata.contains_key("bridge.stage.required") {
        return;
    }

    metadata.insert("bridge.id".into(), Value::String(bridge_id.to_string()));
    metadata.insert(
        "bridge.capability".into(),
        Value::String(capability.to_string()),
    );
    metadata.insert(
        "bridge.stage.required".into(),
        Value::String(requirement_label(snapshot.required)),
    );
    metadata.insert(
        "bridge.stage.actual".into(),
        Value::String(snapshot.actual.as_str().into()),
    );
    if let Some(kind) = snapshot.kind.as_ref() {
        metadata.insert("bridge.kind".into(), Value::String(kind.to_string()));
    }
    if let Some(engine) = snapshot.engine.as_ref() {
        metadata.insert("bridge.engine".into(), Value::String(engine.to_string()));
    }
    if let Some(bundle_hash) = snapshot.bundle_hash.as_ref() {
        metadata.insert(
            "bridge.bundle_hash".into(),
            Value::String(bundle_hash.to_string()),
        );
    }
    if let Some(module_hash) = snapshot.module_hash.as_ref() {
        metadata.insert(
            "bridge.module_hash".into(),
            Value::String(module_hash.to_string()),
        );
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct BridgeMetadata {
    pub kind: Option<String>,
    pub engine: Option<String>,
    pub bundle_hash: Option<String>,
    pub module_hash: Option<String>,
}

impl BridgeMetadata {
    pub fn none() -> Self {
        Self {
            kind: None,
            engine: None,
            bundle_hash: None,
            module_hash: None,
        }
    }

    pub fn wasm(bundle_hash: Option<String>, module_hash: Option<String>) -> Self {
        Self {
            kind: Some("wasm".to_string()),
            engine: Some("wasmtime".to_string()),
            bundle_hash,
            module_hash,
        }
    }
}

fn requirement_label(requirement: StageRequirement) -> String {
    match requirement {
        StageRequirement::Exact(stage) => stage.as_str().into(),
        StageRequirement::AtLeast(stage) => format!("at_least {}", stage.as_str()),
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
