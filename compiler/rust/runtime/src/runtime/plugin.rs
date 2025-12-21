use std::{path::Path, sync::Mutex};

use once_cell::sync::Lazy;
use serde_json::{Map as JsonMap, Value};
use thiserror::Error;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use crate::{
    audit::{AuditEnvelope, AuditEvent},
    capability::{CapabilityError, CapabilityRegistry, PluginCapabilityMetadata},
    config::manifest::{
        load_manifest, Manifest, ManifestCapabilities, ManifestCapabilityError, ProjectKind,
    },
    stage::{StageId, StageRequirement},
};

static PLUGIN_AUDIT_EVENTS: Lazy<Mutex<Vec<AuditEvent>>> = Lazy::new(|| Mutex::new(Vec::new()));
const PLUGIN_DOMAIN: &str = "plugin";
const PLUGIN_EVENT_INSTALL: &str = "plugin.install";
const PLUGIN_EVENT_VERIFY_SIGNATURE: &str = "plugin.verify_signature";

/// 署名検証の方針。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerificationPolicy {
    Strict,
    Permissive,
}

/// プラグイン署名のアルゴリズム。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignatureAlgorithm {
    Ed25519,
    Unknown(String),
}

impl SignatureAlgorithm {
    pub fn as_str(&self) -> &str {
        match self {
            SignatureAlgorithm::Ed25519 => "ed25519",
            SignatureAlgorithm::Unknown(value) => value.as_str(),
        }
    }
}

/// プラグイン署名情報（最小）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginSignature {
    pub algorithm: SignatureAlgorithm,
    pub certificate: Option<String>,
    pub issued_to: Option<String>,
    pub valid_until: Option<String>,
}

/// 署名検証の結果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignatureStatus {
    Verified,
    Skipped,
}

/// プラグインバンドルのメタデータ。
#[derive(Debug, Clone)]
pub struct PluginBundleManifest {
    pub bundle_id: String,
    pub bundle_version: String,
    pub plugins: Vec<Manifest>,
    pub signature: Option<PluginSignature>,
}

/// プラグイン登録結果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginRegistration {
    pub plugin_id: String,
    pub capabilities: Vec<String>,
}

/// バンドル登録結果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginBundleRegistration {
    pub bundle_id: String,
    pub bundle_version: String,
    pub plugins: Vec<PluginRegistration>,
    pub signature_status: SignatureStatus,
}

/// プラグインローダのエラー。
#[derive(Debug, Error)]
pub enum PluginLoadError {
    #[error("manifest project.kind が plugin ではありません: {kind}")]
    NotPluginProject { kind: String },
    #[error("manifest 読み込みに失敗しました: {message}")]
    ManifestLoad { message: String },
    #[error("manifest capability 解析に失敗しました: {0}")]
    ManifestCapability(#[from] ManifestCapabilityError),
    #[error("capability 登録に失敗しました: {0}")]
    CapabilityRegistration(#[from] CapabilityError),
    #[error("plugin signature が見つかりません (Strict モード)")]
    SignatureMissing,
    #[error("plugin signature の検証に失敗しました: {reason}")]
    SignatureInvalid { reason: String },
}

/// プラグイン登録のローダ。
#[derive(Debug, Clone)]
pub struct PluginLoader {
    registry: &'static CapabilityRegistry,
}

impl PluginLoader {
    pub fn new() -> Self {
        Self {
            registry: CapabilityRegistry::registry(),
        }
    }

    /// バンドル単位でプラグインを登録する（署名検証含む）。
    pub fn register_bundle(
        &self,
        bundle: PluginBundleManifest,
        policy: VerificationPolicy,
    ) -> Result<PluginBundleRegistration, PluginLoadError> {
        let signature_status = verify_plugin_signature(bundle.signature.as_ref(), policy)?;
        record_signature_audit(&bundle, &signature_status);
        let mut registered = Vec::new();
        let context = BundleContext::new(&bundle, signature_status.clone());
        for manifest in bundle.plugins {
            registered.push(self.register_manifest_with_context(&manifest, Some(&context))?);
        }
        Ok(PluginBundleRegistration {
            bundle_id: bundle.bundle_id,
            bundle_version: bundle.bundle_version,
            plugins: registered,
            signature_status,
        })
    }

    /// マニフェストファイルからプラグイン Capability を登録する。
    pub fn register_manifest_path(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<PluginRegistration, PluginLoadError> {
        let manifest = load_manifest(path).map_err(|diagnostic| PluginLoadError::ManifestLoad {
            message: diagnostic.message,
        })?;
        self.register_manifest_with_context(&manifest, None)
    }

    /// 既に読み込まれたマニフェストからプラグイン Capability を登録する。
    pub fn register_manifest(
        &self,
        manifest: &Manifest,
    ) -> Result<PluginRegistration, PluginLoadError> {
        self.register_manifest_with_context(manifest, None)
    }

    fn register_manifest_with_context(
        &self,
        manifest: &Manifest,
        context: Option<&BundleContext>,
    ) -> Result<PluginRegistration, PluginLoadError> {
        if !matches!(manifest.project.kind, ProjectKind::Plugin) {
            return Err(PluginLoadError::NotPluginProject {
                kind: manifest.project.kind.as_str().to_string(),
            });
        }

        let capabilities = ManifestCapabilities::from_manifest(manifest)?;
        let capability_ids = capabilities.ids();
        let package = manifest.project.name.0.clone();
        let version = normalize_version(&manifest.project.version.0);
        let metadata = PluginCapabilityMetadata::new(
            package.clone(),
            version.clone(),
            capability_ids.clone(),
        );

        for capability_id in &capability_ids {
            if let Some(record) = capabilities.get(capability_id) {
                let stage = stage_from_requirement(record.stage);
                let effects: Vec<&str> =
                    record.declared_effects.iter().map(String::as_str).collect();
                self.registry.register_plugin_capability(
                    capability_id,
                    stage,
                    &effects,
                    metadata.clone(),
                )?;
            }
        }

        let registration = PluginRegistration {
            plugin_id: package,
            capabilities: capability_ids,
        };
        record_install_audit(&registration, context);
        Ok(registration)
    }
}

fn normalize_version(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn stage_from_requirement(requirement: StageRequirement) -> StageId {
    match requirement {
        StageRequirement::Exact(stage) | StageRequirement::AtLeast(stage) => stage,
    }
}

fn verify_plugin_signature(
    signature: Option<&PluginSignature>,
    policy: VerificationPolicy,
) -> Result<SignatureStatus, PluginLoadError> {
    let signature = match signature {
        Some(signature) => signature,
        None => {
            return match policy {
                VerificationPolicy::Strict => Err(PluginLoadError::SignatureMissing),
                VerificationPolicy::Permissive => Ok(SignatureStatus::Skipped),
            };
        }
    };

    if signature.algorithm.as_str().trim().is_empty() {
        return Err(PluginLoadError::SignatureInvalid {
            reason: "algorithm が空です".to_string(),
        });
    }

    if matches!(policy, VerificationPolicy::Strict)
        && matches!(signature.algorithm, SignatureAlgorithm::Unknown(_))
    {
        return Err(PluginLoadError::SignatureInvalid {
            reason: format!("未知の署名アルゴリズム: {}", signature.algorithm.as_str()),
        });
    }

    Ok(SignatureStatus::Verified)
}

fn record_signature_audit(bundle: &PluginBundleManifest, status: &SignatureStatus) {
    let timestamp = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".into());
    let mut metadata = JsonMap::new();
    metadata.insert(
        "event.kind".into(),
        Value::String(PLUGIN_EVENT_VERIFY_SIGNATURE.to_string()),
    );
    metadata.insert(
        "event.domain".into(),
        Value::String(PLUGIN_DOMAIN.to_string()),
    );
    metadata.insert(
        "plugin.bundle_id".into(),
        Value::String(bundle.bundle_id.clone()),
    );
    metadata.insert(
        "plugin.bundle_version".into(),
        Value::String(bundle.bundle_version.clone()),
    );
    metadata.insert(
        "plugin.signature.status".into(),
        Value::String(match status {
            SignatureStatus::Verified => "verified",
            SignatureStatus::Skipped => "skipped",
        }
        .to_string()),
    );
    if let Some(signature) = &bundle.signature {
        metadata.insert(
            "plugin.signature.algorithm".into(),
            Value::String(signature.algorithm.as_str().to_string()),
        );
        if let Some(issued_to) = &signature.issued_to {
            metadata.insert(
                "plugin.signature.issued_to".into(),
                Value::String(issued_to.clone()),
            );
        }
        if let Some(valid_until) = &signature.valid_until {
            metadata.insert(
                "plugin.signature.valid_until".into(),
                Value::String(valid_until.clone()),
            );
        }
    }
    let envelope = AuditEnvelope::from_parts(metadata, None, None, Some("plugin.bundle".into()));
    let event = AuditEvent::new(timestamp, envelope);
    PLUGIN_AUDIT_EVENTS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .push(event);
}

fn record_install_audit(registration: &PluginRegistration, context: Option<&BundleContext>) {
    let timestamp = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".into());
    let mut metadata = JsonMap::new();
    metadata.insert(
        "event.kind".into(),
        Value::String(PLUGIN_EVENT_INSTALL.to_string()),
    );
    metadata.insert(
        "event.domain".into(),
        Value::String(PLUGIN_DOMAIN.to_string()),
    );
    metadata.insert(
        "plugin.id".into(),
        Value::String(registration.plugin_id.clone()),
    );
    metadata.insert(
        "plugin.capabilities".into(),
        Value::Array(
            registration
                .capabilities
                .iter()
                .cloned()
                .map(Value::String)
                .collect(),
        ),
    );
    if let Some(context) = context {
        metadata.insert(
            "plugin.bundle_id".into(),
            Value::String(context.bundle_id.clone()),
        );
        metadata.insert(
            "plugin.bundle_version".into(),
            Value::String(context.bundle_version.clone()),
        );
        metadata.insert(
            "plugin.signature.status".into(),
            Value::String(context.signature_status_label().to_string()),
        );
    }
    let envelope = AuditEnvelope::from_parts(metadata, None, None, Some("plugin.install".into()));
    let event = AuditEvent::new(timestamp, envelope);
    PLUGIN_AUDIT_EVENTS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .push(event);
}

#[derive(Debug, Clone)]
struct BundleContext {
    bundle_id: String,
    bundle_version: String,
    signature_status: SignatureStatus,
}

impl BundleContext {
    fn new(bundle: &PluginBundleManifest, signature_status: SignatureStatus) -> Self {
        Self {
            bundle_id: bundle.bundle_id.clone(),
            bundle_version: bundle.bundle_version.clone(),
            signature_status,
        }
    }

    fn signature_status_label(&self) -> &str {
        match self.signature_status {
            SignatureStatus::Verified => "verified",
            SignatureStatus::Skipped => "skipped",
        }
    }
}

pub fn take_plugin_audit_events() -> Vec<AuditEvent> {
    PLUGIN_AUDIT_EVENTS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .drain(..)
        .collect()
}
