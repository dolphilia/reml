use serde_json::{Map, Value};

use super::{DiagnosticSeverity, FrontendDiagnostic};

/// Stage 情報に応じて実験的診断の Severity を降格させる。
pub fn apply_experimental_stage_policy(
    diagnostic: &mut FrontendDiagnostic,
    extensions: &Map<String, Value>,
    ack_experimental: bool,
) {
    if should_downgrade_experimental(ack_experimental, extensions)
        && diagnostic.severity_or_default() == DiagnosticSeverity::Error
    {
        diagnostic.set_severity(DiagnosticSeverity::Warning);
    }
}

/// Stage 拡張から実験扱いかどうかを判定する。
pub fn should_downgrade_experimental(
    ack_experimental: bool,
    extensions: &Map<String, Value>,
) -> bool {
    if ack_experimental {
        return false;
    }
    stage_map_is_experimental(extension_stage_map(extensions, "effects"))
        || stage_map_is_experimental(extension_stage_map(extensions, "capability"))
        || stage_map_is_experimental(extension_stage_map(extensions, "bridge"))
        || any_flat_stage_is_experimental(extensions)
}

fn extension_stage_map<'a>(
    extensions: &'a Map<String, Value>,
    namespace: &str,
) -> Option<&'a Map<String, Value>> {
    extensions
        .get(namespace)?
        .as_object()?
        .get("stage")?
        .as_object()
}

fn stage_map_is_experimental(stage_map: Option<&Map<String, Value>>) -> bool {
    let Some(map) = stage_map else {
        return false;
    };
    map.get("required")
        .and_then(Value::as_str)
        .map_or(false, is_experimental_label)
        || map
            .get("actual")
            .and_then(Value::as_str)
            .map_or(false, is_experimental_label)
}

fn any_flat_stage_is_experimental(extensions: &Map<String, Value>) -> bool {
    const STAGE_KEYS: &[&str] = &[
        "effect.stage.required",
        "effect.stage.actual",
        "effects.contract.stage.required",
        "effects.contract.stage.actual",
    ];
    STAGE_KEYS.iter().any(|key| {
        extensions
            .get(*key)
            .and_then(Value::as_str)
            .map_or(false, is_experimental_label)
    })
}

fn is_experimental_label(label: &str) -> bool {
    label
        .to_ascii_lowercase()
        .contains("experimental")
}
