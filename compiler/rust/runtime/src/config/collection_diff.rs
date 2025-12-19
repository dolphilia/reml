use serde::Serialize;
use serde_json::{json, Number, Value};

use crate::{
    collections::audit_bridge::{AuditBridgeError, ChangeSet, ChangeSetKind},
    prelude::iter::EffectLabels,
};

const DEFAULT_ORIGIN: &str = "core.collections";
const DEFAULT_POLICY: &str = "core.collections.audit.v1";
const DEFAULT_STAGE: &str = "stable";
const DEFAULT_CATEGORY: &str = "collections.diff";

/// Config/Data 側で利用する SchemaDiff 互換の差分表現。
#[derive(Debug, Clone, Serialize)]
pub struct SchemaDiff {
    pub kind: ChangeSetKind,
    pub metadata: SchemaDiffMetadata,
    pub changes: Vec<ConfigChange>,
}

impl SchemaDiff {
    /// `ChangeSet` と optional な効果ラベルを受け取り、Config 仕様の差分に変換する。
    pub fn from_change_set(change_set: &ChangeSet, effects: Option<EffectLabels>) -> Self {
        let value = change_set.to_value();
        let kind = change_set.kind();
        let mem_bytes = effects.map(|labels| labels.mem_bytes).unwrap_or(0);
        let metadata = SchemaDiffMetadata::from_value(&value, mem_bytes);
        let changes = value
            .get("items")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .map(|item| {
                        ConfigChange::from_value(item, mem_bytes)
                            .expect("ChangeSet should produce valid ConfigChange")
                    })
                    .collect()
            })
            .unwrap_or_default();
        Self {
            kind,
            metadata,
            changes,
        }
    }

    /// JSON 表現を `ChangeSet` に再構築する。
    pub fn to_change_set(&self) -> Result<ChangeSet, AuditBridgeError> {
        ChangeSet::from_value(self.to_value())
    }

    /// Config/Data で利用する JSON 表現を生成する。
    pub fn to_value(&self) -> Value {
        let (added, removed, updated) = self.counts();
        json!({
            "origin": self.metadata.origin,
            "policy": self.metadata.policy,
            "category": self.metadata.category,
            "stage": self.metadata.stage,
            "kind": self.kind.label(),
            "summary": {
                "added": added,
                "removed": removed,
                "updated": updated,
                "total": self.metadata.total,
            },
            "metadata": {
                "stage": self.metadata.stage,
                "category": self.metadata.category,
                "kind": self.kind.label(),
                "origin": self.metadata.origin,
            },
            "items": self
                .changes
                .iter()
                .map(ConfigChange::into_value)
                .collect::<Vec<_>>(),
            "total": self.metadata.total,
        })
    }

    fn counts(&self) -> (usize, usize, usize) {
        let mut added = 0;
        let mut removed = 0;
        let mut updated = 0;
        for change in &self.changes {
            match change.kind {
                ChangeKind::Added => added += 1,
                ChangeKind::Removed => removed += 1,
                ChangeKind::Updated => updated += 1,
            }
        }
        (added, removed, updated)
    }
}

/// `ChangeSet` に含まれる差分エントリのメタデータ。
#[derive(Debug, Clone, Serialize)]
pub struct SchemaDiffMetadata {
    pub origin: String,
    pub policy: String,
    pub category: String,
    pub stage: String,
    pub total: usize,
    pub mem_bytes: usize,
}

impl SchemaDiffMetadata {
    fn from_value(value: &Value, mem_bytes: usize) -> Self {
        let origin = value
            .get("origin")
            .and_then(Value::as_str)
            .unwrap_or(DEFAULT_ORIGIN)
            .to_string();
        let policy = value
            .get("policy")
            .and_then(Value::as_str)
            .unwrap_or(DEFAULT_POLICY)
            .to_string();
        let category = value
            .get("category")
            .and_then(Value::as_str)
            .unwrap_or(DEFAULT_CATEGORY)
            .to_string();
        let stage = value
            .get("stage")
            .and_then(Value::as_str)
            .or_else(|| {
                value
                    .get("metadata")
                    .and_then(Value::as_object)
                    .and_then(|map| map.get("stage"))
                    .and_then(Value::as_str)
            })
            .unwrap_or(DEFAULT_STAGE)
            .to_string();
        let total = value.get("total").and_then(Value::as_u64).unwrap_or(0) as usize;
        Self {
            origin,
            policy,
            category,
            stage,
            total,
            mem_bytes,
        }
    }
}

/// Config/Data 仕様の差分エントリ。
#[derive(Debug, Clone, Serialize)]
pub struct ConfigChange {
    pub kind: ChangeKind,
    pub key: Value,
    pub previous: Option<Value>,
    pub current: Option<Value>,
    pub type_tag: Option<String>,
    pub mem_bytes: usize,
}

impl ConfigChange {
    fn from_value(value: &Value, mem_bytes: usize) -> Result<Self, AuditBridgeError> {
        let kind = value
            .get("kind")
            .and_then(Value::as_str)
            .and_then(ChangeKind::from_label)
            .ok_or_else(|| AuditBridgeError::new("change missing kind"))?;
        let key = value
            .get("key")
            .cloned()
            .ok_or_else(|| AuditBridgeError::new("change missing key"))?;
        let previous = value.get("previous").cloned();
        let current = value.get("current").cloned();
        let sample = current
            .as_ref()
            .or_else(|| previous.as_ref())
            .unwrap_or(&Value::Null);
        let type_tag = Some(value_type_tag(sample).to_string());
        Ok(Self {
            kind,
            key,
            previous,
            current,
            type_tag,
            mem_bytes,
        })
    }

    fn into_value(&self) -> Value {
        let mut map = serde_json::Map::new();
        map.insert("kind".into(), Value::String(self.kind.label().into()));
        map.insert("key".into(), self.key.clone());
        match self.kind {
            ChangeKind::Added => {
                if let Some(current) = &self.current {
                    map.insert("current".into(), current.clone());
                }
            }
            ChangeKind::Removed => {
                if let Some(previous) = &self.previous {
                    map.insert("previous".into(), previous.clone());
                }
            }
            ChangeKind::Updated => {
                if let Some(previous) = &self.previous {
                    map.insert("previous".into(), previous.clone());
                }
                if let Some(current) = &self.current {
                    map.insert("current".into(), current.clone());
                }
            }
        }
        map.insert(
            "mem_bytes".into(),
            Value::Number(Number::from(self.mem_bytes as u64)),
        );
        if let Some(tag) = &self.type_tag {
            map.insert("type_tag".into(), Value::String(tag.clone()));
        }
        Value::Object(map)
    }
}

/// `ConfigChange` の種類を表す。
#[derive(Debug, Clone, Copy, Serialize)]
pub enum ChangeKind {
    Added,
    Removed,
    Updated,
}

impl ChangeKind {
    fn label(self) -> &'static str {
        match self {
            Self::Added => "collections.diff.added",
            Self::Removed => "collections.diff.removed",
            Self::Updated => "collections.diff.updated",
        }
    }

    fn from_label(label: &str) -> Option<Self> {
        match label {
            "collections.diff.added" => Some(Self::Added),
            "collections.diff.removed" => Some(Self::Removed),
            "collections.diff.updated" => Some(Self::Updated),
            _ => None,
        }
    }
}

fn value_type_tag(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "list",
        Value::Object(_) => "map",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collections::persistent::btree::PersistentMap;

    #[test]
    fn schema_diff_roundtrips_change_set() {
        let base = PersistentMap::new().insert("alpha", 1).insert("beta", 2);
        let delta = base.insert("beta", 3).insert("gamma", 4);
        let change_set = base
            .diff_change_set(&delta)
            .expect("diff change set should be available");
        let schema_diff = SchemaDiff::from_change_set(&change_set, None);
        let round_trip = schema_diff.to_change_set().expect("rebuild change set");
        assert_eq!(change_set.to_value(), round_trip.to_value());
    }

    #[test]
    fn mem_bytes_propagate_into_schema_diff() {
        let base = PersistentMap::new().insert("alpha", 1);
        let delta = base.insert("alpha", 2);
        let change_set = base.diff_change_set(&delta).unwrap();
        let labels = EffectLabels {
            mem: false,
            mutating: false,
            debug: false,
            async_pending: false,
            audit: false,
            cell: false,
            rc: false,
            unicode: false,
            io: false,
            io_blocking: false,
            io_async: false,
            security: false,
            transfer: false,
            fs_sync: false,
            mem_bytes: 123,
            predicate_calls: 0,
            rc_ops: 0,
            time: false,
            time_calls: 0,
            io_blocking_calls: 0,
            io_async_calls: 0,
            fs_sync_calls: 0,
            security_events: 0,
        };
        let schema_diff = SchemaDiff::from_change_set(&change_set, Some(labels));
        assert_eq!(schema_diff.metadata.mem_bytes, 123);
        assert!(schema_diff
            .changes
            .iter()
            .all(|change| change.mem_bytes == 123));
        assert!(schema_diff
            .changes
            .iter()
            .all(|change| change.type_tag.is_some()));
    }
}
