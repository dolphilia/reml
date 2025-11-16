use crate::typeck::{RuntimeCapability, StageContext, StageTraceStep};
use serde_json::{json, Map, Value};

/// 効果ステージと Capability のメタ情報を JSON に焼き込むための文脈。
#[derive(Debug, Clone)]
pub struct EffectAuditContext {
    required_stage: Option<String>,
    actual_stage: Option<String>,
    runtime_capabilities: Vec<RuntimeCapability>,
    stage_trace: Vec<StageTraceStep>,
}

impl EffectAuditContext {
    /// 生のステージ文字列で初期化する。
    pub fn new(
        required_stage: Option<String>,
        actual_stage: Option<String>,
        runtime_capabilities: Vec<RuntimeCapability>,
        stage_trace: Vec<StageTraceStep>,
    ) -> Self {
        Self {
            required_stage,
            actual_stage,
            runtime_capabilities,
            stage_trace,
        }
    }

    /// StageContext と RuntimeCapability リストから構築する。
    pub fn from_stage_context(
        context: &StageContext,
        runtime_capabilities: &[RuntimeCapability],
    ) -> Self {
        Self {
            required_stage: Some(context.capability.label()),
            actual_stage: Some(context.runtime.label()),
            runtime_capabilities: runtime_capabilities.to_vec(),
            stage_trace: context.stage_trace.clone(),
        }
    }

    pub fn primary_capability(&self) -> Option<&str> {
        self.runtime_capabilities
            .first()
            .map(|cap| cap.id().as_str())
    }

    fn capability_ids_value(&self) -> Value {
        Value::Array(
            self.runtime_capabilities
                .iter()
                .map(|cap| Value::String(cap.id().to_string()))
                .collect(),
        )
    }

    fn capability_details(&self) -> Vec<Value> {
        self.runtime_capabilities
            .iter()
            .map(|cap| {
                let mut entry = Map::new();
                entry.insert("capability".to_string(), json!(cap.id()));
                entry.insert("stage".to_string(), json!(cap.stage().as_str()));
                Value::Object(entry)
            })
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
        for cap in &self.runtime_capabilities {
            trace.push(json!({
                "source": "runtime_capability",
                "capability": cap.id(),
                "stage": cap.stage().as_str(),
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
}

pub fn apply_extensions(context: &EffectAuditContext, extensions: &mut Map<String, Value>) {
    apply_effects_extension(context, extensions);
    apply_bridge_extension(context, extensions);
    apply_flattened_extension_keys(context, extensions);
    apply_contract_extensions(context, extensions);
}

pub fn apply_audit_metadata(context: &EffectAuditContext, metadata: &mut Map<String, Value>) {
    apply_effect_audit_metadata(context, metadata);
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
    stage_obj.insert("actual_capabilities".to_string(), capability_details.clone());
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
    stage_obj.insert("actual_capabilities".to_string(), capability_details.clone());
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

fn apply_flattened_extension_keys(context: &EffectAuditContext, extensions: &mut Map<String, Value>) {
    let ids_value = context.capability_ids();
    let capability_details = context.capability_details_value();
    extensions.insert("effect.capabilities".to_string(), ids_value.clone());
    extensions.insert("effect.required_capabilities".to_string(), ids_value.clone());
    extensions.insert("effect.stage.required_capabilities".to_string(), ids_value.clone());
    extensions.insert(
        "effect.actual_capabilities".to_string(),
        capability_details.clone(),
    );
    extensions.insert(
        "effect.stage.actual_capabilities".to_string(),
        capability_details.clone(),
    );
    if let Some(required) = context.required_stage_str() {
        extensions.insert(
            "effect.stage.required".to_string(),
            json!(required),
        );
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
}

fn apply_contract_extensions(context: &EffectAuditContext, extensions: &mut Map<String, Value>) {
    if let Some(required) = context.required_stage_str() {
        extensions.insert("effects.contract.stage.required".to_string(), json!(required));
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
    metadata.insert("effect.required_capabilities".to_string(), ids_value.clone());
    metadata.insert("effect.stage.required_capabilities".to_string(), ids_value.clone());
    metadata.insert("effect.actual_capabilities".to_string(), capability_details.clone());
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
        metadata.insert("effects.contract.stage.required".to_string(), json!(required));
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
