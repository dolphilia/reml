use serde_json::{Map as JsonMap, Value};

use crate::runtime::bridge::{
    attach_bridge_stage_metadata as attach_runtime_bridge_stage_metadata, RuntimeBridgeRegistry,
};
use crate::stage::{StageId, StageRequirement};

/// Capability 検証で得られた Stage をブリッジ監査用に記録する。
pub(crate) fn record_bridge_stage_probe(
    capability: &str,
    requirement: StageRequirement,
    actual: StageId,
) {
    RuntimeBridgeRegistry::global().record_stage_probe(capability, requirement, actual);
}

/// `IoError` の Audit メタデータへ `bridge.stage.*` 情報を注入する。
pub(crate) fn attach_bridge_stage_metadata(
    capability: &str,
    metadata: &mut JsonMap<String, Value>,
) {
    let bridge_id = format!("io::{capability}");
    attach_runtime_bridge_stage_metadata(&bridge_id, capability, metadata);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::bridge::RuntimeBridgeRegistry;

    #[test]
    fn attaches_bridge_stage_metadata_once_recorded() {
        let _guard = RuntimeBridgeRegistry::test_lock()
            .lock()
            .expect("RuntimeBridgeRegistry test lock poisoned");
        RuntimeBridgeRegistry::global().clear();
        let capability = "io.fs.read";
        record_bridge_stage_probe(
            capability,
            StageRequirement::AtLeast(StageId::Beta),
            StageId::Stable,
        );
        let mut metadata = JsonMap::new();
        attach_bridge_stage_metadata(capability, &mut metadata);
        assert_eq!(
            metadata.get("bridge.stage.required"),
            Some(&Value::String("at_least beta".into()))
        );
        assert_eq!(
            metadata.get("bridge.stage.actual"),
            Some(&Value::String("stable".into()))
        );
        assert_eq!(
            metadata.get("bridge.capability"),
            Some(&Value::String(capability.into()))
        );
    }
}
