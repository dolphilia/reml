use crate::streaming::RuntimeBridgeSignal;
use crate::typeck::{RuntimeCapability, StageContext, StageTraceStep};
use reml_runtime::{CapabilityDescriptor, CapabilityRegistry};
use serde_json::{json, Map, Value};

#[derive(Debug, Clone)]
struct CapabilityMetadataEntry {
    id: String,
    stage: String,
    provider: Option<Value>,
}

impl CapabilityMetadataEntry {
    fn from_descriptor(descriptor: &CapabilityDescriptor) -> Self {
        let provider = serde_json::to_value(descriptor.metadata().provider.clone()).ok();
        Self {
            id: descriptor.id.clone(),
            stage: descriptor.stage().as_str().to_string(),
            provider,
        }
    }

    fn from_runtime_capability(capability: &RuntimeCapability) -> Self {
        Self {
            id: capability.id().to_string(),
            stage: capability.stage().as_str().to_string(),
            provider: None,
        }
    }

    fn detail_value(&self) -> Value {
        let mut entry = Map::new();
        entry.insert("capability".to_string(), json!(self.id));
        entry.insert("stage".to_string(), json!(self.stage));
        if let Some(provider) = &self.provider {
            entry.insert("provider".to_string(), provider.clone());
        }
        Value::Object(entry)
    }

    fn provider_value(&self) -> Option<Value> {
        self.provider.clone()
    }
}

/// 効果ステージと Capability のメタ情報を JSON に焼き込むための文脈。
#[derive(Debug, Clone)]
pub struct EffectAuditContext {
    required_stage: Option<String>,
    actual_stage: Option<String>,
    capability_metadata: Vec<CapabilityMetadataEntry>,
    stage_trace: Vec<StageTraceStep>,
    bridge_signal: Option<RuntimeBridgeSignal>,
}

impl EffectAuditContext {
    /// 生のステージ文字列で初期化する。
    fn new(
        required_stage: Option<String>,
        actual_stage: Option<String>,
        capability_metadata: Vec<CapabilityMetadataEntry>,
        stage_trace: Vec<StageTraceStep>,
        bridge_signal: Option<RuntimeBridgeSignal>,
    ) -> Self {
        Self {
            required_stage,
            actual_stage,
            capability_metadata,
            stage_trace,
            bridge_signal,
        }
    }

    /// StageContext と RuntimeCapability リストから構築する。
    pub fn from_stage_context(
        context: &StageContext,
        runtime_capabilities: &[RuntimeCapability],
        bridge_signal: Option<RuntimeBridgeSignal>,
    ) -> Self {
        Self {
            required_stage: Some(context.capability.label()),
            actual_stage: Some(context.runtime.label()),
            capability_metadata: collect_capability_metadata(runtime_capabilities),
            stage_trace: context.stage_trace.clone(),
            bridge_signal,
        }
    }

    pub fn bridge_signal(&self) -> Option<&RuntimeBridgeSignal> {
        self.bridge_signal.as_ref()
    }

    pub fn primary_capability(&self) -> Option<&str> {
        self.capability_metadata.first().map(|cap| cap.id.as_str())
    }

    fn primary_metadata(&self) -> Option<&CapabilityMetadataEntry> {
        self.capability_metadata.first()
    }

    fn capability_ids_value(&self) -> Value {
        Value::Array(
            self.capability_metadata
                .iter()
                .map(|cap| Value::String(cap.id.clone()))
                .collect(),
        )
    }

    fn capability_details(&self) -> Vec<Value> {
        self.capability_metadata
            .iter()
            .map(|entry| entry.detail_value())
            .collect()
    }

    fn stage_trace(&self) -> Vec<Value> {
        let mut trace = self
            .stage_trace
            .iter()
            .map(|step| step.to_value())
            .collect::<Vec<_>>();
        if let Some(required) = &self.required_stage {
            trace.push(json!({
                "source": "cli_option",
                "stage": required,
                "note": "--effect-stage",
            }));
        }
        if let Some(actual) = &self.actual_stage {
            trace.push(json!({
                "source": "runtime",
                "stage": actual,
            }));
        }
        for cap in &self.capability_metadata {
            trace.push(json!({
                "source": "runtime_capability",
                "capability": cap.id,
                "stage": cap.stage,
            }));
        }
        trace
    }

    fn required_stage_str(&self) -> Option<&str> {
        self.required_stage.as_deref()
    }

    fn actual_stage_str(&self) -> Option<&str> {
        self.actual_stage.as_deref()
    }

    fn capability_ids(&self) -> Value {
        self.capability_ids_value()
    }

    fn capability_details_value(&self) -> Value {
        Value::Array(self.capability_details())
    }

    fn provider_values(&self) -> Vec<Value> {
        self.capability_metadata
            .iter()
            .map(|entry| entry.provider_value().unwrap_or(Value::Null))
            .collect()
    }
}

/// Stage/Capability メタデータを診断・監査経路へ展開するための共通コンテナ。
#[derive(Debug, Clone)]
pub struct StageAuditPayload {
    required_stage: Option<String>,
    actual_stage: Option<String>,
    capability_metadata: Vec<CapabilityMetadataEntry>,
    stage_trace: Vec<StageTraceStep>,
    bridge_signal: Option<RuntimeBridgeSignal>,
}

impl StageAuditPayload {
    pub fn new(
        context: &StageContext,
        capabilities: &[RuntimeCapability],
        bridge_signal: Option<RuntimeBridgeSignal>,
    ) -> Self {
        let mut trace = context.stage_trace.clone();
        if let Some(signal) = &bridge_signal {
            trace.extend(signal.stage_trace.clone());
        }
        Self {
            required_stage: Some(context.capability.label()),
            actual_stage: Some(context.runtime.label()),
            capability_metadata: collect_capability_metadata(capabilities),
            stage_trace: trace,
            bridge_signal,
        }
    }

    pub fn effect_context(&self) -> EffectAuditContext {
        EffectAuditContext::new(
            self.required_stage.clone(),
            self.actual_stage.clone(),
            self.capability_metadata.clone(),
            self.stage_trace.clone(),
            self.bridge_signal.clone(),
        )
    }

    pub fn apply_extensions(&self, extensions: &mut Map<String, Value>) {
        crate::diagnostic::effects::apply_extensions(&self.effect_context(), extensions);
    }

    pub fn apply_audit_metadata(&self, metadata: &mut Map<String, Value>) {
        crate::diagnostic::effects::apply_audit_metadata(&self.effect_context(), metadata);
    }

    pub fn primary_capability(&self) -> Option<&str> {
        self.capability_metadata.first().map(|cap| cap.id.as_str())
    }

    pub fn bridge_signal(&self) -> Option<&RuntimeBridgeSignal> {
        self.bridge_signal.as_ref()
    }

    pub fn stage_trace(&self) -> &[StageTraceStep] {
        &self.stage_trace
    }

    pub fn extend_stage_trace(&mut self, extra: &[StageTraceStep]) {
        self.stage_trace.extend(extra.iter().cloned());
    }

    pub fn required_stage_label(&self) -> Option<&str> {
        self.required_stage.as_deref()
    }

    pub fn actual_stage_label(&self) -> Option<&str> {
        self.actual_stage.as_deref()
    }
}

fn collect_capability_metadata(capabilities: &[RuntimeCapability]) -> Vec<CapabilityMetadataEntry> {
    let registry = CapabilityRegistry::registry();
    capabilities
        .iter()
        .map(|cap| match registry.describe(cap.id().as_str()) {
            Ok(descriptor) => CapabilityMetadataEntry::from_descriptor(&descriptor),
            Err(_) => CapabilityMetadataEntry::from_runtime_capability(cap),
        })
        .collect()
}

pub fn apply_extensions(context: &EffectAuditContext, extensions: &mut Map<String, Value>) {
    apply_effects_extension(context, extensions);
    apply_bridge_extension(context, extensions);
    apply_bridge_signal_extensions(context, extensions);
    apply_flattened_extension_keys(context, extensions);
    apply_contract_extensions(context, extensions);
}

pub fn apply_audit_metadata(context: &EffectAuditContext, metadata: &mut Map<String, Value>) {
    apply_effect_audit_metadata(context, metadata);
    apply_bridge_signal_audit_metadata(context, metadata);
    apply_contract_audit_metadata(context, metadata);
}

fn apply_effects_extension(context: &EffectAuditContext, extensions: &mut Map<String, Value>) {
    let ids_value = context.capability_ids();
    let capability_details = context.capability_details_value();
    let stage_trace = context.stage_trace();
    let effects_entry = extensions
        .entry("effects".to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    let effects_obj = ensure_object(effects_entry);
    effects_obj.insert("capabilities".to_string(), ids_value.clone());
    if let Some(primary) = context.primary_capability() {
        effects_obj.insert("capability".to_string(), json!(primary));
    }
    let stage_entry = effects_obj
        .entry("stage".to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    let stage_obj = ensure_object(stage_entry);
    if let Some(required) = context.required_stage_str() {
        stage_obj.insert("required".to_string(), json!(required));
    }
    if let Some(actual) = context.actual_stage_str() {
        stage_obj.insert("actual".to_string(), json!(actual));
    }
    stage_obj.insert("required_capabilities".to_string(), ids_value.clone());
    stage_obj.insert(
        "actual_capabilities".to_string(),
        capability_details.clone(),
    );
    if let Some(primary) = context.primary_capability() {
        stage_obj.insert("capability".to_string(), json!(primary));
    }
    if !stage_trace.is_empty() {
        let trace_value = Value::Array(stage_trace.clone());
        stage_obj.insert("trace".to_string(), trace_value.clone());
        effects_obj.insert("stage_trace".to_string(), trace_value);
    }
}

fn apply_bridge_extension(context: &EffectAuditContext, extensions: &mut Map<String, Value>) {
    let ids_value = context.capability_ids();
    let capability_details = context.capability_details_value();
    let stage_trace = context.stage_trace();
    let bridge_entry = extensions
        .entry("bridge".to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    let bridge_obj = ensure_object(bridge_entry);
    if let Some(primary) = context.primary_capability() {
        bridge_obj.insert("capability".to_string(), json!(primary));
    }
    let stage_entry = bridge_obj
        .entry("stage".to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    let stage_obj = ensure_object(stage_entry);
    stage_obj.insert("required_capabilities".to_string(), ids_value.clone());
    stage_obj.insert(
        "actual_capabilities".to_string(),
        capability_details.clone(),
    );
    if let Some(required) = context.required_stage_str() {
        stage_obj.insert("required".to_string(), json!(required));
    }
    if let Some(actual) = context.actual_stage_str() {
        stage_obj.insert("actual".to_string(), json!(actual));
    }
    if let Some(primary) = context.primary_capability() {
        stage_obj.insert("capability".to_string(), json!(primary));
    }
    if !stage_trace.is_empty() {
        stage_obj.insert("trace".to_string(), Value::Array(stage_trace));
    }
}

fn apply_bridge_signal_extensions(
    context: &EffectAuditContext,
    extensions: &mut Map<String, Value>,
) {
    if let Some(signal) = context.bridge_signal() {
        let signal_value = Value::Object(bridge_signal_payload(signal));
        let bridge_entry = extensions
            .entry("bridge".to_string())
            .or_insert_with(|| Value::Object(Map::new()));
        let bridge_obj = ensure_object(bridge_entry);
        bridge_obj.insert("signal".to_string(), signal_value.clone());
        let stage_entry = bridge_obj
            .entry("stage".to_string())
            .or_insert_with(|| Value::Object(Map::new()));
        let stage_obj = ensure_object(stage_entry);
        stage_obj.insert("signal".to_string(), signal_value);
    }
}

fn apply_flattened_extension_keys(
    context: &EffectAuditContext,
    extensions: &mut Map<String, Value>,
) {
    let ids_value = context.capability_ids();
    let capability_details = context.capability_details_value();
    let provider_values = context.provider_values();
    extensions.insert("effect.capabilities".to_string(), ids_value.clone());
    extensions.insert(
        "effect.required_capabilities".to_string(),
        ids_value.clone(),
    );
    extensions.insert(
        "effect.stage.required_capabilities".to_string(),
        ids_value.clone(),
    );
    extensions.insert(
        "effect.actual_capabilities".to_string(),
        capability_details.clone(),
    );
    extensions.insert(
        "effect.stage.actual_capabilities".to_string(),
        capability_details.clone(),
    );
    if let Some(required) = context.required_stage_str() {
        extensions.insert("effect.stage.required".to_string(), json!(required));
    }
    if let Some(actual) = context.actual_stage_str() {
        extensions.insert("effect.stage.actual".to_string(), json!(actual));
    }
    if let Some(primary) = context.primary_capability() {
        extensions.insert("effect.capability".to_string(), json!(primary));
    }
    let mut capability_ext = Map::new();
    capability_ext.insert("ids".to_string(), ids_value.clone());
    if let Some(primary) = context.primary_capability() {
        capability_ext.insert("primary".to_string(), json!(primary));
    }
    if !provider_values.is_empty() {
        capability_ext.insert(
            "providers".to_string(),
            Value::Array(provider_values.clone()),
        );
    }
    capability_ext.insert(
        "stage".to_string(),
        json!({
            "required": context.required_stage_str(),
            "actual": context.actual_stage_str(),
        }),
    );
    capability_ext.insert("detail".to_string(), capability_details.clone());
    capability_ext.insert("required_capabilities".to_string(), ids_value);
    extensions.insert("capability".to_string(), Value::Object(capability_ext));
    if let Some(primary_entry) = context.primary_metadata() {
        extensions.insert("capability.id".to_string(), json!(primary_entry.id.clone()));
        extensions.insert(
            "capability.stage".to_string(),
            json!(primary_entry.stage.clone()),
        );
        if let Some(provider) = primary_entry.provider_value() {
            extensions.insert("capability.provider".to_string(), provider);
        }
    }
}

fn apply_contract_extensions(context: &EffectAuditContext, extensions: &mut Map<String, Value>) {
    if let Some(required) = context.required_stage_str() {
        extensions.insert(
            "effects.contract.stage.required".to_string(),
            json!(required),
        );
    }
    if let Some(actual) = context.actual_stage_str() {
        extensions.insert("effects.contract.stage.actual".to_string(), json!(actual));
    }
    if let Some(primary) = context.primary_capability() {
        extensions.insert("effects.contract.capability".to_string(), json!(primary));
    }
    let stage_trace = context.stage_trace();
    if !stage_trace.is_empty() {
        extensions.insert(
            "effects.contract.stage_trace".to_string(),
            Value::Array(stage_trace),
        );
    }
}

fn apply_bridge_signal_audit_metadata(
    context: &EffectAuditContext,
    metadata: &mut Map<String, Value>,
) {
    if let Some(signal) = context.bridge_signal() {
        metadata.insert(
            "bridge.stage.signal".to_string(),
            Value::Object(bridge_signal_payload(signal)),
        );
        metadata.insert(
            "bridge.stage.reason".to_string(),
            json!(signal.normalized_reason()),
        );
    }
}

fn bridge_signal_payload(signal: &RuntimeBridgeSignal) -> Map<String, Value> {
    let mut payload = Map::new();
    payload.insert("kind".to_string(), json!(signal.kind.as_str()));
    if let Some(offset) = signal.parser_offset {
        payload.insert("parser_offset".to_string(), json!(offset));
    }
    if let Some(sequence) = signal.stream_sequence {
        payload.insert("stream_sequence".to_string(), json!(sequence));
    }
    if let Some(stage) = &signal.stage {
        payload.insert("stage".to_string(), json!(stage));
    }
    if let Some(capability) = &signal.capability {
        payload.insert("capability".to_string(), json!(capability));
    }
    if let Some(note) = &signal.note {
        payload.insert("note".to_string(), json!(note));
    }
    if !signal.stage_trace.is_empty() {
        let trace = signal
            .stage_trace
            .iter()
            .map(|step| step.to_value())
            .collect::<Vec<_>>();
        payload.insert("stage_trace".to_string(), Value::Array(trace));
    }
    payload
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::streaming::RuntimeBridgeSignalKind;
    use crate::typeck::StageTraceStep;

    fn sample_stage_trace_step() -> StageTraceStep {
        StageTraceStep {
            source: "runtime".to_string(),
            stage: Some("beta".to_string()),
            capability: Some("ffi.bridge".to_string()),
            note: Some("test-span".to_string()),
            file: None,
            target: None,
        }
    }

    #[test]
    fn bridge_signal_extensions_emit_expected_metadata() {
        let stage_step = sample_stage_trace_step();
        let bridge_signal = RuntimeBridgeSignal {
            kind: RuntimeBridgeSignalKind::Backpressure,
            parser_offset: Some(42),
            stream_sequence: Some(7),
            stage: Some("beta".to_string()),
            capability: Some("ffi.bridge".to_string()),
            note: Some("overload".to_string()),
            stage_trace: vec![stage_step.clone()],
        };
        let context = EffectAuditContext::new(
            Some("beta".to_string()),
            Some("stable".to_string()),
            vec![],
            vec![stage_step.clone()],
            Some(bridge_signal.clone()),
        );
        let mut extensions = Map::new();
        apply_extensions(&context, &mut extensions);

        let bridge_ext = extensions
            .get("bridge")
            .and_then(Value::as_object)
            .expect("bridge extension is present");
        let signal_ext = bridge_ext
            .get("signal")
            .and_then(Value::as_object)
            .expect("signal payload inserted");
        assert_eq!(signal_ext.get("kind"), Some(&json!("backpressure")));
        assert_eq!(
            bridge_ext
                .get("stage")
                .and_then(Value::as_object)
                .and_then(|entry| entry.get("signal"))
                .and_then(Value::as_object)
                .and_then(|entry| entry.get("parser_offset")),
            Some(&json!(42))
        );

        let mut metadata = Map::new();
        apply_audit_metadata(&context, &mut metadata);
        let signal_meta = metadata
            .get("bridge.stage.signal")
            .and_then(Value::as_object)
            .expect("bridge.stage.signal metadata");
        assert_eq!(signal_meta.get("kind"), Some(&json!("backpressure")));
        assert_eq!(signal_meta.get("parser_offset"), Some(&json!(42)));
        assert_eq!(
            signal_meta
                .get("stage_trace")
                .and_then(Value::as_array)
                .and_then(|entries| entries.get(0))
                .and_then(Value::as_object)
                .and_then(|entry| entry.get("source")),
            Some(&json!("runtime"))
        );
        assert_eq!(
            metadata.get("bridge.stage.reason").and_then(Value::as_str),
            Some("overload")
        );
        let contract_trace_len = metadata
            .get("effects.contract.stage_trace")
            .and_then(Value::as_array)
            .map(|arr| arr.len())
            .unwrap_or_default();
        assert!(contract_trace_len > 0);
    }
}

fn apply_effect_audit_metadata(context: &EffectAuditContext, metadata: &mut Map<String, Value>) {
    if let Some(required) = context.required_stage_str() {
        metadata.insert("effect.stage.required".to_string(), json!(required));
    }
    if let Some(actual) = context.actual_stage_str() {
        metadata.insert("effect.stage.actual".to_string(), json!(actual));
    }
    let ids_value = context.capability_ids();
    let capability_details = context.capability_details_value();
    metadata.insert("capability.ids".to_string(), ids_value.clone());
    metadata.insert(
        "effect.required_capabilities".to_string(),
        ids_value.clone(),
    );
    metadata.insert(
        "effect.stage.required_capabilities".to_string(),
        ids_value.clone(),
    );
    metadata.insert(
        "effect.actual_capabilities".to_string(),
        capability_details.clone(),
    );
    metadata.insert(
        "capability.providers".to_string(),
        Value::Array(context.provider_values()),
    );
    if let Some(primary_entry) = context.primary_metadata() {
        metadata.insert("capability.id".to_string(), json!(primary_entry.id.clone()));
        metadata.insert(
            "capability.stage".to_string(),
            json!(primary_entry.stage.clone()),
        );
        metadata.insert(
            "capability.stage.actual".to_string(),
            json!(primary_entry.stage.clone()),
        );
        if let Some(provider) = primary_entry.provider_value() {
            metadata.insert("capability.provider".to_string(), provider);
        }
    }
    metadata.insert(
        "effect.stage.actual_capabilities".to_string(),
        capability_details.clone(),
    );
    metadata.insert(
        "bridge.stage.required_capabilities".to_string(),
        ids_value.clone(),
    );
    metadata.insert(
        "bridge.stage.actual_capabilities".to_string(),
        capability_details.clone(),
    );
    if let Some(primary) = context.primary_capability() {
        metadata.insert("bridge.stage.capability".to_string(), json!(primary));
        metadata.insert("effect.capability".to_string(), json!(primary));
    }
    let stage_trace = context.stage_trace();
    if !stage_trace.is_empty() {
        let trace_value = Value::Array(stage_trace.clone());
        metadata.insert("stage.trace".to_string(), trace_value.clone());
        metadata.insert("effect.stage.trace".to_string(), trace_value);
    }
}

fn apply_contract_audit_metadata(context: &EffectAuditContext, metadata: &mut Map<String, Value>) {
    if let Some(required) = context.required_stage_str() {
        metadata.insert(
            "effects.contract.stage.required".to_string(),
            json!(required),
        );
    }
    if let Some(actual) = context.actual_stage_str() {
        metadata.insert("effects.contract.stage.actual".to_string(), json!(actual));
    }
    if let Some(primary) = context.primary_capability() {
        metadata.insert("effects.contract.capability".to_string(), json!(primary));
    }
    let stage_trace = context.stage_trace();
    if !stage_trace.is_empty() {
        metadata.insert(
            "effects.contract.stage_trace".to_string(),
            Value::Array(stage_trace),
        );
    }
}

pub fn ensure_object(value: &mut Value) -> &mut Map<String, Value> {
    if !value.is_object() {
        *value = Value::Object(Map::new());
    }
    value
        .as_object_mut()
        .expect("value should be converted into an object")
}
