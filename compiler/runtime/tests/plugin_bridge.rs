use reml_runtime::{
    runtime::bridge::RuntimeBridgeRegistry,
    runtime::plugin::PluginError,
    runtime::plugin_bridge::{
        NativePluginExecutionBridge, PluginExecutionBridge, PluginInstance, PluginInvokeRequest,
    },
    stage::{StageId, StageRequirement},
};

#[test]
fn native_bridge_invokes_echo_entrypoint() {
    let bridge = NativePluginExecutionBridge::new();
    let instance = PluginInstance {
        plugin_id: "plugin.demo".to_string(),
    };
    let request = PluginInvokeRequest {
        entrypoint: "plugin.echo".to_string(),
        payload: vec![1, 2, 3],
    };
    let response = bridge
        .invoke(&instance, request)
        .expect("plugin.echo should respond");
    assert_eq!(response.payload, vec![1, 2, 3]);
}

#[test]
fn plugin_error_into_diagnostic_includes_bridge_metadata() {
    let registry = RuntimeBridgeRegistry::global();
    registry.clear();
    registry.record_stage_probe(
        "plugin.demo.invoke",
        StageRequirement::AtLeast(StageId::Beta),
        StageId::Stable,
    );
    let diagnostic = PluginError::VerificationFailed {
        message: "bad".to_string(),
    }
    .into_diagnostic_with_bridge(Some("plugin::invoke"), Some("plugin.demo.invoke"));

    assert_eq!(
        diagnostic
            .audit_metadata
            .get("bridge.stage.required")
            .and_then(|value| value.as_str()),
        Some("at_least beta")
    );
    assert_eq!(
        diagnostic
            .audit_metadata
            .get("bridge.stage.actual")
            .and_then(|value| value.as_str()),
        Some("stable")
    );
}
