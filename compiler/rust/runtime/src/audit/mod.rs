//! Core Diagnostics で共有する `AuditEnvelope`/`AuditEvent` 型と
//! 監査メタデータのバリデーションヘルパ。
//! 仕様: `docs/spec/3-6-core-diagnostics-audit.md` §1.1.

use crate::anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use uuid::Uuid;

const EVENT_KIND_KEY: &str = "event.kind";
const EFFECT_STAGE_KEYS: &[&str] = &[
    "effect.stage.required",
    "effect.stage.actual",
    "effect.capability",
];
const BRIDGE_STAGE_KEYS: &[&str] = &["bridge.id", "bridge.stage.required", "bridge.stage.actual"];
const BRIDGE_RELOAD_KEYS: &[&str] =
    &["bridge.reload", "bridge.id", "bridge.stage.required", "bridge.stage.actual"];

const PIPELINE_STARTED_KEYS: &[&str] =
    &["pipeline.id", "pipeline.dsl_id", "pipeline.node", "timestamp"];
const PIPELINE_COMPLETED_KEYS: &[&str] = &[
    "pipeline.id",
    "pipeline.dsl_id",
    "pipeline.node",
    "timestamp",
    "pipeline.outcome",
    "pipeline.count",
];
const PIPELINE_FAILED_KEYS: &[&str] = &[
    "pipeline.id",
    "pipeline.dsl_id",
    "pipeline.node",
    "timestamp",
    "error.code",
    "error.message",
    "error.severity",
];
const CAPABILITY_MISMATCH_KEYS: &[&str] = &[
    "capability.id",
    "capability.expected_stage",
    "capability.actual_stage",
    "dsl.node",
];
const ASYNC_SUPERVISOR_RESTARTED_KEYS: &[&str] = &[
    "async.supervisor.id",
    "async.supervisor.actor",
    "async.supervisor.restart_count",
];
const ASYNC_SUPERVISOR_EXHAUSTED_KEYS: &[&str] = &[
    "async.supervisor.id",
    "async.supervisor.actor",
    "async.supervisor.restart_count",
    "async.supervisor.budget",
    "async.supervisor.outcome",
];
const CONFIG_COMPAT_CHANGED_KEYS: &[&str] =
    &["config.source", "config.format", "config.profile", "config.compatibility"];
const ENV_MUTATION_KEYS: &[&str] = &["env.operation", "env.key", "env.scope", "requested_by"];

/// `AuditEnvelope` は監査メタデータを保持するコンテナ。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEnvelope {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audit_id: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub change_set: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capability: Option<String>,
    #[serde(default)]
    pub metadata: Map<String, Value>,
}

impl AuditEnvelope {
    /// 空の Envelope を生成する。
    pub fn new() -> Self {
        Self::default()
    }

    /// 監査イベント種別（`metadata["event.kind"]`）を返す。
    pub fn event_kind(&self) -> Option<&str> {
        self.metadata
            .get(EVENT_KIND_KEY)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
    }

    /// `metadata` へキーと値を追加する。
    pub fn insert_metadata(&mut self, key: impl Into<String>, value: Value) {
        self.metadata.insert(key.into(), value);
    }

    /// 監査メタデータが仕様上必要な項目を揃えているか検証する。
    pub fn validate(&self) -> Result<()> {
        let mut missing = Vec::new();
        let metadata = &self.metadata;

        if metadata.is_empty() {
            missing.push("metadata".to_string());
        }

        if let Some(kind) = self.event_kind() {
            if let Some(required) = required_fields_for_event(kind) {
                missing.extend(missing_keys(metadata, required));
            }
            if kind.starts_with("bridge.") {
                missing.extend(missing_keys(metadata, BRIDGE_STAGE_KEYS));
            }
        }

        if contains_any(metadata, EFFECT_STAGE_KEYS) {
            missing.extend(missing_keys(metadata, EFFECT_STAGE_KEYS));
        }
        if contains_any(metadata, BRIDGE_STAGE_KEYS) {
            missing.extend(missing_keys(metadata, BRIDGE_STAGE_KEYS));
        }
        if expects_bridge_reload(metadata) {
            missing.extend(missing_keys(metadata, BRIDGE_RELOAD_KEYS));
        }

        if missing.is_empty() {
            Ok(())
        } else {
            missing.sort();
            missing.dedup();
            Err(anyhow(format!(
                "audit metadata validation failed: missing keys [{}]",
                missing.join(", ")
            )))
        }
    }
}

impl Default for AuditEnvelope {
    fn default() -> Self {
        Self {
            audit_id: None,
            change_set: None,
            capability: None,
            metadata: Map::new(),
        }
    }
}

/// `AuditEvent` はタイムスタンプとカテゴリを持つ監査ログの単位。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub timestamp: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    pub envelope: AuditEnvelope,
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub extensions: Map<String, Value>,
}

impl AuditEvent {
    /// 監査イベントを生成するヘルパ。
    pub fn new(timestamp: impl Into<String>, envelope: AuditEnvelope) -> Self {
        Self {
            timestamp: timestamp.into(),
            category: None,
            envelope,
            extensions: Map::new(),
        }
    }

    /// メタデータ上で宣言されたイベント種別を返す。
    pub fn event_kind(&self) -> Option<&str> {
        self.envelope.event_kind()
    }

    /// タイムスタンプと Envelope を検証する。
    pub fn validate(&self) -> Result<()> {
        if self.timestamp.trim().is_empty() {
            return Err(anyhow("audit event missing timestamp"));
        }
        self.envelope.validate()
    }
}

fn required_fields_for_event(kind: &str) -> Option<&'static [&'static str]> {
    match kind {
        "pipeline_started" => Some(PIPELINE_STARTED_KEYS),
        "pipeline_completed" => Some(PIPELINE_COMPLETED_KEYS),
        "pipeline_failed" => Some(PIPELINE_FAILED_KEYS),
        "capability_mismatch" => Some(CAPABILITY_MISMATCH_KEYS),
        "async_supervisor_restarted" => Some(ASYNC_SUPERVISOR_RESTARTED_KEYS),
        "async_supervisor_exhausted" => Some(ASYNC_SUPERVISOR_EXHAUSTED_KEYS),
        "config_compat_changed" => Some(CONFIG_COMPAT_CHANGED_KEYS),
        "env_mutation" => Some(ENV_MUTATION_KEYS),
        "bridge.reload" => Some(BRIDGE_RELOAD_KEYS),
        "bridge.rollback" => Some(BRIDGE_RELOAD_KEYS),
        _ => None,
    }
}

fn missing_keys(metadata: &Map<String, Value>, required: &[&str]) -> Vec<String> {
    required
        .iter()
        .filter(|key| !metadata.contains_key(**key))
        .map(|key| (*key).to_string())
        .collect()
}

fn contains_any(metadata: &Map<String, Value>, keys: &[&str]) -> bool {
    keys.iter().any(|key| metadata.contains_key(*key))
}

fn expects_bridge_reload(metadata: &Map<String, Value>) -> bool {
    metadata.contains_key("bridge.reload")
        || metadata
            .get(EVENT_KIND_KEY)
            .and_then(Value::as_str)
            .map(|kind| kind == "bridge.reload" || kind == "bridge.rollback")
            .unwrap_or(false)
}
