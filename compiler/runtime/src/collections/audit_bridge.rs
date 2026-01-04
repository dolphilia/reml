//! `Core.Collections` で発生した差分を `AuditEnvelope.change_set` へ
//! 転写するためのユーティリティ。
//!
//! `docs/spec/3-7-core-config-data.md` で定義されている `ChangeSet` の
//! 最小要素（`added` / `removed` / `updated` と Stage メタデータ）を
//! Rust 実装から JSON へ整形する。

use std::{error, fmt};

use serde::Serialize;
use serde_json::{json, Value};

use super::persistent::btree::{PersistentMap, PersistentSet};

const DEFAULT_ORIGIN: &str = "core.collections";
const DEFAULT_POLICY: &str = "core.collections.audit.v1";
const DEFAULT_STAGE: &str = "stable";
const DEFAULT_CATEGORY: &str = "collections.diff";

/// 監査ログへ書き込むための差分スナップショット。
#[derive(Debug, Clone)]
pub struct ChangeSet {
    kind: ChangeSetKind,
    items: Vec<ChangeItem>,
    origin: &'static str,
    policy: &'static str,
    stage: &'static str,
    category: &'static str,
}

impl ChangeSet {
    fn new(kind: ChangeSetKind, items: Vec<ChangeItem>) -> Self {
        Self {
            kind,
            items,
            origin: DEFAULT_ORIGIN,
            policy: DEFAULT_POLICY,
            stage: DEFAULT_STAGE,
            category: DEFAULT_CATEGORY,
        }
    }

    pub fn kind(&self) -> ChangeSetKind {
        self.kind
    }

    pub fn from_value(value: Value) -> Result<Self, AuditBridgeError> {
        let kind = value
            .get("kind")
            .and_then(Value::as_str)
            .and_then(ChangeSetKind::from_label)
            .ok_or_else(|| AuditBridgeError::new("change set missing valid kind"))?;
        let items = value
            .get("items")
            .and_then(Value::as_array)
            .ok_or_else(|| AuditBridgeError::new("change set missing items array"))?
            .iter()
            .map(ChangeItem::from_value)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(ChangeSet::new(kind, items))
    }

    /// 差分の内訳を返す。
    pub fn summary(&self) -> ChangeSummary {
        let mut summary = ChangeSummary::default();
        for item in &self.items {
            summary.register(item);
        }
        summary
    }

    /// 差分総数を返す。
    pub fn total(&self) -> usize {
        self.items.len()
    }

    /// 監査ログ向け JSON を値として取得する。
    pub fn to_value(&self) -> Value {
        self.clone().into_value()
    }

    /// 監査ログへ埋め込む JSON を生成する。
    pub fn into_value(self) -> Value {
        let summary = self.summary();
        let Self {
            kind,
            items,
            origin,
            policy,
            stage,
            category,
        } = self;
        let items = items
            .into_iter()
            .map(ChangeItem::into_value)
            .collect::<Vec<_>>();
        json!({
            "origin": origin,
            "policy": policy,
            "category": category,
            "stage": stage,
            "kind": kind.label(),
            "summary": {
                "added": summary.added,
                "removed": summary.removed,
                "updated": summary.updated,
                "total": summary.total(),
            },
            "metadata": {
                "stage": stage,
                "category": category,
                "kind": kind.label(),
                "origin": origin,
            },
            "items": items,
            "total": summary.total(),
        })
    }
}

/// 差分の種類。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ChangeSetKind {
    MapDiff,
    SetDiff,
}

impl ChangeSetKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::MapDiff => "collections.diff.map",
            Self::SetDiff => "collections.diff.set",
        }
    }

    pub fn from_label(label: &str) -> Option<Self> {
        match label {
            "collections.diff.map" => Some(Self::MapDiff),
            "collections.diff.set" => Some(Self::SetDiff),
            _ => None,
        }
    }
}

/// 差分サマリ。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ChangeSummary {
    pub added: usize,
    pub removed: usize,
    pub updated: usize,
}

impl ChangeSummary {
    fn register(&mut self, item: &ChangeItem) {
        match item {
            ChangeItem::Added { .. } => self.added = self.added.saturating_add(1),
            ChangeItem::Removed { .. } => self.removed = self.removed.saturating_add(1),
            ChangeItem::Updated { .. } => self.updated = self.updated.saturating_add(1),
        }
    }

    pub fn total(self) -> usize {
        self.added + self.removed + self.updated
    }
}

/// 差分エントリ。
#[derive(Debug, Clone)]
pub enum ChangeItem {
    Added {
        key: Value,
        value: Value,
    },
    Removed {
        key: Value,
        value: Value,
    },
    Updated {
        key: Value,
        previous: Value,
        current: Value,
    },
}

impl ChangeItem {
    fn added(key: Value, value: Value) -> Self {
        Self::Added { key, value }
    }

    fn removed(key: Value, value: Value) -> Self {
        Self::Removed { key, value }
    }

    fn updated(key: Value, previous: Value, current: Value) -> Self {
        Self::Updated {
            key,
            previous,
            current,
        }
    }

    fn into_value(self) -> Value {
        match self {
            Self::Added { key, value } => json!({
                "kind": "collections.diff.added",
                "key": key,
                "current": value,
            }),
            Self::Removed { key, value } => json!({
                "kind": "collections.diff.removed",
                "key": key,
                "previous": value,
            }),
            Self::Updated {
                key,
                previous,
                current,
            } => json!({
                "kind": "collections.diff.updated",
                "key": key,
                "previous": previous,
                "current": current,
            }),
        }
    }

    fn from_value(value: &Value) -> Result<Self, AuditBridgeError> {
        let kind = value
            .get("kind")
            .and_then(Value::as_str)
            .ok_or_else(|| AuditBridgeError::new("change item missing kind"))?;
        let key = value
            .get("key")
            .cloned()
            .ok_or_else(|| AuditBridgeError::new("change item missing key"))?;
        match kind {
            "collections.diff.added" => {
                let current = value
                    .get("current")
                    .cloned()
                    .ok_or_else(|| AuditBridgeError::new("added item missing current value"))?;
                Ok(Self::added(key, current))
            }
            "collections.diff.removed" => {
                let previous = value
                    .get("previous")
                    .cloned()
                    .ok_or_else(|| AuditBridgeError::new("removed item missing previous value"))?;
                Ok(Self::removed(key, previous))
            }
            "collections.diff.updated" => {
                let previous = value
                    .get("previous")
                    .cloned()
                    .ok_or_else(|| AuditBridgeError::new("updated item missing previous value"))?;
                let current = value
                    .get("current")
                    .cloned()
                    .ok_or_else(|| AuditBridgeError::new("updated item missing current value"))?;
                Ok(Self::updated(key, previous, current))
            }
            other => Err(AuditBridgeError::new(format!(
                "unknown change item kind: {other}"
            ))),
        }
    }
}

/// `serde_json` 変換時のエラー。
#[derive(Debug)]
pub struct AuditBridgeError {
    message: String,
}

impl AuditBridgeError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for AuditBridgeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Collections.audit_bridge error: {}", self.message)
    }
}

impl error::Error for AuditBridgeError {}

impl From<serde_json::Error> for AuditBridgeError {
    fn from(err: serde_json::Error) -> Self {
        Self::new(err.to_string())
    }
}

/// Map 差分を ChangeSet へ変換する。
pub fn map_diff_to_changes<K, V>(
    base: &PersistentMap<K, V>,
    delta: &PersistentMap<K, V>,
) -> Result<ChangeSet, AuditBridgeError>
where
    K: Ord + Clone + Serialize,
    V: Clone + Serialize,
{
    let mut items = Vec::new();
    let base_map = base.clone().into_map();
    let delta_map = delta.clone().into_map();

    for (key, value) in base_map.iter() {
        match delta_map.get(key) {
            Some(next_value) => {
                let previous = to_value(value)?;
                let current = to_value(next_value)?;
                if previous != current {
                    items.push(ChangeItem::updated(to_value(key)?, previous, current));
                }
            }
            None => items.push(ChangeItem::removed(to_value(key)?, to_value(value)?)),
        }
    }

    for (key, value) in delta_map.iter() {
        if !base_map.contains_key(key) {
            items.push(ChangeItem::added(to_value(key)?, to_value(value)?));
        }
    }

    Ok(ChangeSet::new(ChangeSetKind::MapDiff, items))
}

/// Set 差分を ChangeSet へ変換する。
pub fn set_diff_to_changes<T>(
    base: &PersistentSet<T>,
    delta: &PersistentSet<T>,
) -> Result<ChangeSet, AuditBridgeError>
where
    T: Ord + Clone + Serialize,
{
    let mut items = Vec::new();
    let base_set = base.clone().into_set();
    let delta_set = delta.clone().into_set();

    for value in base_set.iter() {
        if !delta_set.contains(value) {
            items.push(ChangeItem::removed(to_value(value)?, json!(true)));
        }
    }

    for value in delta_set.iter() {
        if !base_set.contains(value) {
            items.push(ChangeItem::added(to_value(value)?, json!(true)));
        }
    }

    Ok(ChangeSet::new(ChangeSetKind::SetDiff, items))
}

fn to_value<T: Serialize>(value: &T) -> Result<Value, AuditBridgeError> {
    serde_json::to_value(value).map_err(AuditBridgeError::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collections::persistent::btree::{PersistentMap, PersistentSet};

    #[test]
    fn map_diff_detects_changes() {
        let base = PersistentMap::new().insert("alpha", 1).insert("beta", 2);
        let updated = base.insert("beta", 3).insert("gamma", 4);

        let change_set = map_diff_to_changes(&base, &updated).expect("diff");
        let summary = change_set.summary();
        assert_eq!(summary.added, 1);
        assert_eq!(summary.updated, 1);
        assert_eq!(summary.removed, 0);

        let json = change_set.into_value();
        assert_eq!(json["summary"]["total"], 2);
        assert_eq!(
            json["items"][0]["kind"],
            Value::String("collections.diff.updated".into())
        );
    }

    #[test]
    fn set_diff_detects_changes() {
        let base = PersistentSet::new().insert("alpha").insert("beta");
        let updated = base.insert("gamma");
        let change_set = set_diff_to_changes(&base, &updated).expect("diff");
        let summary = change_set.summary();
        assert_eq!(summary.added, 1);
        assert_eq!(summary.removed, 0);
        assert_eq!(summary.updated, 0);
        let json = change_set.into_value();
        assert_eq!(json["summary"]["added"], 1);
    }
}
