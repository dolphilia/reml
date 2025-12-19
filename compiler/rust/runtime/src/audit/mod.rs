//! Core Diagnostics で共有する `AuditEnvelope`/`AuditEvent` 型と
//! 監査メタデータのバリデーションヘルパ。
//! 仕様: `docs/spec/3-6-core-diagnostics-audit.md` §1.1.

use crate::anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::borrow::Cow;
use uuid::Uuid;

const EVENT_KIND_KEY: &str = "event.kind";
const EFFECT_STAGE_KEYS: &[&str] = &[
    "effect.stage.required",
    "effect.stage.actual",
    "effect.capability",
];
const BRIDGE_STAGE_KEYS: &[&str] = &["bridge.id", "bridge.stage.required", "bridge.stage.actual"];
const BRIDGE_RELOAD_KEYS: &[&str] = &[
    "bridge.reload",
    "bridge.id",
    "bridge.stage.required",
    "bridge.stage.actual",
];

const PIPELINE_STARTED_KEYS: &[&str] = &[
    "pipeline.id",
    "pipeline.dsl_id",
    "pipeline.node",
    "timestamp",
];
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
const CAPABILITY_CHECK_KEYS: &[&str] = &[
    "capability.id",
    "capability.result",
    "effect.capability",
    "effect.stage.required",
    "effect.stage.actual",
    "capability.ids",
    "effect.required_capabilities",
    "effect.actual_capabilities",
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
const CONFIG_COMPAT_CHANGED_KEYS: &[&str] = &[
    "config.source",
    "config.format",
    "config.profile",
    "config.compatibility",
];
const ENV_MUTATION_KEYS: &[&str] = &["env.operation", "env.key", "env.scope", "requested_by"];
const SNAPSHOT_UPDATED_KEYS: &[&str] = &["snapshot.name", "snapshot.hash"];

/// Core Diagnostics で利用する監査イベント種別。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuditEventKind {
    PipelineStarted,
    PipelineCompleted,
    PipelineFailed,
    CapabilityMismatch,
    AsyncSupervisorRestarted,
    AsyncSupervisorExhausted,
    ConfigCompatChanged,
    EnvMutation,
    CapabilityCheck,
    BridgeReload,
    BridgeRollback,
    SnapshotUpdated,
    DocTest,
    Custom(String),
}

impl AuditEventKind {
    /// `snake_case` のラベルを返す。
    pub fn as_str(&self) -> Cow<'_, str> {
        match self {
            AuditEventKind::PipelineStarted => Cow::Borrowed("pipeline_started"),
            AuditEventKind::PipelineCompleted => Cow::Borrowed("pipeline_completed"),
            AuditEventKind::PipelineFailed => Cow::Borrowed("pipeline_failed"),
            AuditEventKind::CapabilityMismatch => Cow::Borrowed("capability_mismatch"),
            AuditEventKind::AsyncSupervisorRestarted => Cow::Borrowed("async_supervisor_restarted"),
            AuditEventKind::AsyncSupervisorExhausted => Cow::Borrowed("async_supervisor_exhausted"),
            AuditEventKind::ConfigCompatChanged => Cow::Borrowed("config_compat_changed"),
            AuditEventKind::EnvMutation => Cow::Borrowed("env_mutation"),
            AuditEventKind::CapabilityCheck => Cow::Borrowed("capability_check"),
            AuditEventKind::BridgeReload => Cow::Borrowed("bridge.reload"),
            AuditEventKind::BridgeRollback => Cow::Borrowed("bridge.rollback"),
            AuditEventKind::SnapshotUpdated => Cow::Borrowed("snapshot.updated"),
            AuditEventKind::DocTest => Cow::Borrowed("doc.doctest"),
            AuditEventKind::Custom(value) => Cow::Owned(value.clone()),
        }
    }

    /// 文字列から種別を復元する。
    pub fn from_str(value: &str) -> Self {
        match value {
            "pipeline_started" => AuditEventKind::PipelineStarted,
            "pipeline_completed" => AuditEventKind::PipelineCompleted,
            "pipeline_failed" => AuditEventKind::PipelineFailed,
            "capability_mismatch" => AuditEventKind::CapabilityMismatch,
            "capability_check" => AuditEventKind::CapabilityCheck,
            "async_supervisor_restarted" => AuditEventKind::AsyncSupervisorRestarted,
            "async_supervisor_exhausted" => AuditEventKind::AsyncSupervisorExhausted,
            "config_compat_changed" => AuditEventKind::ConfigCompatChanged,
            "env_mutation" => AuditEventKind::EnvMutation,
            "bridge.reload" => AuditEventKind::BridgeReload,
            "bridge.rollback" => AuditEventKind::BridgeRollback,
            "snapshot.updated" => AuditEventKind::SnapshotUpdated,
            "doc.doctest" => AuditEventKind::DocTest,
            other => AuditEventKind::Custom(other.to_string()),
        }
    }

    /// 必須メタデータキーの一覧を返す。
    pub fn required_metadata_keys(&self) -> Option<&'static [&'static str]> {
        match self {
            AuditEventKind::PipelineStarted => Some(PIPELINE_STARTED_KEYS),
            AuditEventKind::PipelineCompleted => Some(PIPELINE_COMPLETED_KEYS),
            AuditEventKind::PipelineFailed => Some(PIPELINE_FAILED_KEYS),
            AuditEventKind::CapabilityMismatch => Some(CAPABILITY_MISMATCH_KEYS),
            AuditEventKind::AsyncSupervisorRestarted => Some(ASYNC_SUPERVISOR_RESTARTED_KEYS),
            AuditEventKind::AsyncSupervisorExhausted => Some(ASYNC_SUPERVISOR_EXHAUSTED_KEYS),
            AuditEventKind::ConfigCompatChanged => Some(CONFIG_COMPAT_CHANGED_KEYS),
            AuditEventKind::EnvMutation => Some(ENV_MUTATION_KEYS),
            AuditEventKind::CapabilityCheck => Some(CAPABILITY_CHECK_KEYS),
            AuditEventKind::BridgeReload | AuditEventKind::BridgeRollback => {
                Some(BRIDGE_RELOAD_KEYS)
            }
            AuditEventKind::SnapshotUpdated => Some(SNAPSHOT_UPDATED_KEYS),
            AuditEventKind::DocTest => None,
            AuditEventKind::Custom(_) => None,
        }
    }
}

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

    /// フィールドを直接指定して Envelope を生成する。
    pub fn from_parts(
        metadata: Map<String, Value>,
        audit_id: Option<Uuid>,
        change_set: Option<Value>,
        capability: Option<String>,
    ) -> Self {
        Self {
            metadata,
            audit_id,
            change_set,
            capability,
        }
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
    AuditEventKind::from_str(kind).required_metadata_keys()
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
