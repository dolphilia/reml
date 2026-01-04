use serde_json::{Map, Value};

/// アダプタ層サブシステムの Capability 情報。
#[derive(Copy, Clone, Debug)]
pub struct AdapterCapability {
    pub id: &'static str,
    pub stage: &'static str,
    pub effect_scope: &'static [&'static str],
    pub audit_key_prefix: &'static str,
}

impl AdapterCapability {
    /// `const` で宣言できる構築ヘルパ。
    pub const fn new(
        id: &'static str,
        stage: &'static str,
        effect_scope: &'static [&'static str],
        audit_key_prefix: &'static str,
    ) -> Self {
        Self {
            id,
            stage,
            effect_scope,
            audit_key_prefix,
        }
    }

    /// `adapter.*` 監査キーを満たすメタデータを生成する。
    pub fn audit_metadata(
        &self,
        operation: &str,
        status: &str,
    ) -> Map<String, Value> {
        let mut metadata = Map::new();
        metadata.insert(
            "capability.id".into(),
            Value::String(self.id.to_string()),
        );
        metadata.insert(
            "capability.stage".into(),
            Value::String(self.stage.to_string()),
        );
        metadata.insert(
            "capability.audit_prefix".into(),
            Value::String(self.audit_key_prefix.to_string()),
        );
        metadata.insert(
            "capability.effect_scope".into(),
            Value::Array(
                self.effect_scope
                    .iter()
                    .map(|scope| Value::String(scope.to_string()))
                    .collect(),
            ),
        );
        metadata.insert(
            format!("{}.operation", self.audit_key_prefix),
            Value::String(operation.to_string()),
        );
        metadata.insert(
            format!("{}.status", self.audit_key_prefix),
            Value::String(status.to_string()),
        );
        metadata
    }
}
