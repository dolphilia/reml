use serde::{Deserialize, Serialize};
#[cfg(test)]
use serde_json::json;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};

/// `Core.Data` のスキーマを表現する。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Schema {
    pub name: String,
    pub description: Option<String>,
    pub version: Option<SchemaVersion>,
    #[serde(default)]
    pub fields: BTreeMap<String, Field>,
    #[serde(default)]
    pub metadata: Map<String, Value>,
}

impl Schema {
    /// スキーマビルダーを初期化する。
    pub fn builder(name: impl Into<String>) -> SchemaBuilder {
        SchemaBuilder::new(name)
    }

    /// 登録済みフィールドを参照する。
    pub fn field(&self, name: &str) -> Option<&Field> {
        self.fields.get(name)
    }

    /// スキーマ差分を計算するユーティリティ。
    pub fn diff(old: &Schema, new: &Schema) -> SchemaDiff {
        SchemaDiff::between(old, new)
    }

    /// フィールド数を返す。
    pub fn len(&self) -> usize {
        self.fields.len()
    }

    /// フィールドが空かどうか。
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }
}

impl Default for Schema {
    fn default() -> Self {
        Self {
            name: String::new(),
            description: None,
            version: None,
            fields: BTreeMap::new(),
            metadata: Map::new(),
        }
    }
}

/// スキーマバージョン表現。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct SchemaVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl SchemaVersion {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    pub fn as_string(&self) -> String {
        format!("{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// スキーマのビルダー。
#[derive(Debug, Clone)]
pub struct SchemaBuilder {
    schema: Schema,
}

impl SchemaBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            schema: Schema {
                name: name.into(),
                ..Schema::default()
            },
        }
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.schema.description = Some(description.into());
        self
    }

    pub fn version(mut self, version: SchemaVersion) -> Self {
        self.schema.version = Some(version);
        self
    }

    pub fn metadata(mut self, key: impl Into<String>, value: Value) -> Self {
        self.schema.metadata.insert(key.into(), value);
        self
    }

    pub fn field(mut self, field: Field) -> Self {
        self.schema.fields.insert(field.name.clone(), field);
        self
    }

    pub fn field_with(self, builder: FieldBuilder) -> Self {
        self.field(builder.finish())
    }

    pub fn finish(self) -> Schema {
        self.schema
    }
}

/// スキーマフィールド。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Field {
    pub name: String,
    pub data_type: SchemaDataType,
    #[serde(default = "Field::default_required")]
    pub required: bool,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub default_value: Option<Value>,
    #[serde(default)]
    pub examples: Vec<Value>,
    #[serde(default)]
    pub rules: Vec<ValidationRule>,
    #[serde(default)]
    pub metadata: Map<String, Value>,
}

impl Field {
    fn default_required() -> bool {
        true
    }

    pub fn builder(name: impl Into<String>, data_type: SchemaDataType) -> FieldBuilder {
        FieldBuilder::new(name.into(), data_type)
    }
}

/// フィールドビルダー。
#[derive(Debug, Clone)]
pub struct FieldBuilder {
    field: Field,
}

impl FieldBuilder {
    pub fn new(name: String, data_type: SchemaDataType) -> Self {
        Self {
            field: Field {
                name,
                data_type,
                required: true,
                description: None,
                default_value: None,
                examples: Vec::new(),
                rules: Vec::new(),
                metadata: Map::new(),
            },
        }
    }

    pub fn required(mut self, required: bool) -> Self {
        self.field.required = required;
        self
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.field.description = Some(description.into());
        self
    }

    pub fn default_value(mut self, value: Value) -> Self {
        self.field.default_value = Some(value);
        self
    }

    pub fn example(mut self, value: Value) -> Self {
        self.field.examples.push(value);
        self
    }

    pub fn metadata(mut self, key: impl Into<String>, value: Value) -> Self {
        self.field.metadata.insert(key.into(), value);
        self
    }

    pub fn rule(mut self, rule: ValidationRule) -> Self {
        self.field.rules.push(rule);
        self
    }

    pub fn finish(self) -> Field {
        self.field
    }
}

/// フィールドの基本データ型。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SchemaDataType {
    Boolean,
    Integer,
    Number,
    String,
    Bytes,
    Timestamp,
    Date,
    List {
        items: Box<SchemaDataType>,
    },
    Map {
        key: Box<SchemaDataType>,
        value: Box<SchemaDataType>,
    },
    Enum {
        values: Vec<Value>,
    },
    Object {
        fields: BTreeMap<String, Field>,
    },
    Reference {
        schema: String,
    },
    Any,
}

impl SchemaDataType {
    pub fn reference(name: impl Into<String>) -> Self {
        Self::Reference {
            schema: name.into(),
        }
    }

    pub fn list(items: SchemaDataType) -> Self {
        Self::List {
            items: Box::new(items),
        }
    }

    pub fn map(key: SchemaDataType, value: SchemaDataType) -> Self {
        Self::Map {
            key: Box::new(key),
            value: Box::new(value),
        }
    }
}

/// バリデーションルール。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ValidationRule {
    pub id: String,
    pub kind: ValidationRuleKind,
    #[serde(default)]
    pub severity: ValidationRuleSeverity,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub params: Map<String, Value>,
}

impl ValidationRule {
    pub fn builder(id: impl Into<String>, kind: ValidationRuleKind) -> ValidationRuleBuilder {
        ValidationRuleBuilder::new(id.into(), kind)
    }
}

/// バリデーションルールのビルダー。
#[derive(Debug, Clone)]
pub struct ValidationRuleBuilder {
    rule: ValidationRule,
}

impl ValidationRuleBuilder {
    pub fn new(id: String, kind: ValidationRuleKind) -> Self {
        Self {
            rule: ValidationRule {
                id,
                kind,
                severity: ValidationRuleSeverity::Error,
                message: None,
                params: Map::new(),
            },
        }
    }

    pub fn severity(mut self, severity: ValidationRuleSeverity) -> Self {
        self.rule.severity = severity;
        self
    }

    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.rule.message = Some(message.into());
        self
    }

    pub fn param(mut self, key: impl Into<String>, value: Value) -> Self {
        self.rule.params.insert(key.into(), value);
        self
    }

    pub fn finish(self) -> ValidationRule {
        self.rule
    }
}

/// バリデーションルールの種類。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "rule", rename_all = "snake_case")]
pub enum ValidationRuleKind {
    Range {
        #[serde(default)]
        min: Option<Value>,
        #[serde(default)]
        max: Option<Value>,
    },
    Regex {
        pattern: String,
    },
    Length {
        #[serde(default)]
        min: Option<u64>,
        #[serde(default)]
        max: Option<u64>,
    },
    Enum {
        values: Vec<Value>,
    },
    Custom,
}

/// ルールの重大度。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ValidationRuleSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

impl Default for ValidationRuleSeverity {
    fn default() -> Self {
        Self::Error
    }
}

/// スキーマ差分。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SchemaDiff {
    pub added: Vec<Field>,
    pub removed: Vec<Field>,
    pub changed: Vec<FieldChange>,
}

impl SchemaDiff {
    pub fn between(old: &Schema, new: &Schema) -> Self {
        let mut added = Vec::new();
        let mut removed = Vec::new();
        let mut changed = Vec::new();
        for (name, old_field) in &old.fields {
            match new.fields.get(name) {
                Some(next) => {
                    if old_field != next {
                        changed.push(FieldChange::from_fields(name, old_field, next));
                    }
                }
                None => removed.push(old_field.clone()),
            }
        }
        for (name, field) in &new.fields {
            if !old.fields.contains_key(name) {
                added.push(field.clone());
            }
        }
        added.sort_by(|a, b| a.name.cmp(&b.name));
        removed.sort_by(|a, b| a.name.cmp(&b.name));
        changed.sort_by(|a, b| a.name.cmp(&b.name));
        Self {
            added,
            removed,
            changed,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty() && self.changed.is_empty()
    }
}

/// フィールド差分。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FieldChange {
    pub name: String,
    pub previous: Field,
    pub current: Field,
    pub attributes: BTreeSet<FieldAttribute>,
}

impl FieldChange {
    fn from_fields(name: &str, previous: &Field, current: &Field) -> Self {
        let mut attributes = BTreeSet::new();
        if previous.data_type != current.data_type {
            attributes.insert(FieldAttribute::DataType);
        }
        if previous.required != current.required {
            attributes.insert(FieldAttribute::Required);
        }
        if previous.description != current.description {
            attributes.insert(FieldAttribute::Description);
        }
        if previous.default_value != current.default_value {
            attributes.insert(FieldAttribute::DefaultValue);
        }
        if previous.examples != current.examples {
            attributes.insert(FieldAttribute::Examples);
        }
        if previous.rules != current.rules {
            attributes.insert(FieldAttribute::Rules);
        }
        if previous.metadata != current.metadata {
            attributes.insert(FieldAttribute::Metadata);
        }
        Self {
            name: name.to_string(),
            previous: previous.clone(),
            current: current.clone(),
            attributes,
        }
    }
}

/// 差分対象のフィールド属性。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum FieldAttribute {
    DataType,
    Required,
    Description,
    DefaultValue,
    Examples,
    Rules,
    Metadata,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_builder_constructs_fields() {
        let schema = Schema::builder("User")
            .description("ユーザプロファイル")
            .version(SchemaVersion::new(1, 0, 0))
            .field(
                Field::builder("id", SchemaDataType::String)
                    .description("主キー")
                    .finish(),
            )
            .field(
                Field::builder("score", SchemaDataType::Number)
                    .required(false)
                    .default_value(json!(0))
                    .example(json!(42))
                    .rule(
                        ValidationRule::builder(
                            "rule.score.range",
                            ValidationRuleKind::Range {
                                min: Some(json!(0)),
                                max: Some(json!(100)),
                            },
                        )
                        .severity(ValidationRuleSeverity::Warning)
                        .message("スコアは 0〜100 の間で指定してください")
                        .finish(),
                    )
                    .finish(),
            )
            .finish();
        assert_eq!(schema.name, "User");
        assert_eq!(schema.description, Some("ユーザプロファイル".into()));
        assert_eq!(
            schema
                .version
                .as_ref()
                .map(|version| version.as_string())
                .unwrap(),
            "1.0.0"
        );
        assert_eq!(schema.fields.len(), 2);
        assert!(schema.field("id").is_some());
    }

    #[test]
    fn schema_diff_detects_changes() {
        let legacy = Schema::builder("Config")
            .field(Field::builder("host", SchemaDataType::String).finish())
            .field(
                Field::builder("port", SchemaDataType::Integer)
                    .default_value(json!(8080))
                    .finish(),
            )
            .finish();
        let updated = Schema::builder("Config")
            .field(Field::builder("host", SchemaDataType::String).finish())
            .field(
                Field::builder("port", SchemaDataType::Integer)
                    .default_value(json!(9000))
                    .finish(),
            )
            .field(
                Field::builder("tls", SchemaDataType::Boolean)
                    .required(false)
                    .default_value(Value::Bool(false))
                    .finish(),
            )
            .finish();
        let diff = Schema::diff(&legacy, &updated);
        assert_eq!(diff.added.len(), 1);
        assert_eq!(diff.removed.len(), 0);
        assert_eq!(diff.changed.len(), 1);
        let change = diff.changed.first().expect("port diff");
        assert!(change.attributes.contains(&FieldAttribute::DefaultValue));
        assert_eq!(change.current.default_value, Some(json!(9000)));
        assert_eq!(diff.added.first().unwrap().name, "tls");
        assert!(!diff.is_empty());
    }
}
