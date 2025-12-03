use once_cell::sync::Lazy;
use serde_json::{Map as JsonMap, Value};
use std::collections::HashMap;
use std::sync::Mutex;

use crate::stage::{StageId, StageRequirement};

#[derive(Clone, Copy)]
struct StageSnapshot {
    required: StageRequirement,
    actual: StageId,
}

static BRIDGE_STAGE_CACHE: Lazy<Mutex<HashMap<&'static str, StageSnapshot>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

/// Capability 検証で得られた Stage をブリッジ監査用に記録する。
pub(crate) fn record_bridge_stage_probe(
    capability: &'static str,
    requirement: StageRequirement,
    actual: StageId,
) {
    if let Ok(mut cache) = BRIDGE_STAGE_CACHE.lock() {
        cache.insert(
            capability,
            StageSnapshot {
                required: requirement,
                actual,
            },
        );
    }
}

/// `IoError` の Audit メタデータへ `bridge.stage.*` 情報を注入する。
pub(crate) fn attach_bridge_stage_metadata(
    capability: &str,
    metadata: &mut JsonMap<String, Value>,
) {
    let snapshot = {
        let cache = match BRIDGE_STAGE_CACHE.lock() {
            Ok(lock) => lock,
            Err(_) => return,
        };
        cache.get(capability).copied()
    };
    let Some(snapshot) = snapshot else {
        return;
    };

    if metadata.contains_key("bridge.stage.required") {
        return;
    }

    metadata.insert(
        "bridge.id".into(),
        Value::String(format!("io::{capability}")),
    );
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
    fn attaches_bridge_stage_metadata_once_recorded() {
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
