use std::{
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
};

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::{Map as JsonMap, Value};
use sha2::{Digest, Sha256};
use thiserror::Error;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use crate::{
    audit::{AuditEnvelope, AuditEvent},
    capability::{CapabilityError, CapabilityRegistry, PluginCapabilityMetadata},
    config::manifest::{
        load_manifest, Manifest, ManifestCapabilities, ManifestCapabilityError, ProjectKind,
    },
    prelude::ensure::{DiagnosticSeverity, GuardDiagnostic, IntoDiagnostic},
    runtime::bridge::attach_bridge_stage_metadata,
    stage::{StageId, StageRequirement},
};

static PLUGIN_AUDIT_EVENTS: Lazy<Mutex<Vec<AuditEvent>>> = Lazy::new(|| Mutex::new(Vec::new()));
const PLUGIN_DOMAIN: &str = "plugin";
const PLUGIN_EVENT_INSTALL: &str = "plugin.install";
const PLUGIN_EVENT_REVOKE: &str = "plugin.revoke";
const PLUGIN_EVENT_VERIFY_SIGNATURE: &str = "plugin.verify_signature";
const PLUGIN_EVENT_SIGNATURE_FAILURE: &str = "plugin.signature.failure";

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
    pub bundle_hash: Option<String>,
}

/// 署名検証の結果。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
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
    pub bundle_hash: Option<String>,
    pub modules: Vec<PluginModuleInfo>,
    pub manifest_paths: Vec<PathBuf>,
}

/// プラグイン登録結果。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PluginRegistration {
    pub plugin_id: String,
    pub capabilities: Vec<String>,
}

/// バンドル登録結果。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PluginBundleRegistration {
    pub bundle_id: String,
    pub bundle_version: String,
    pub plugins: Vec<PluginRegistration>,
    pub signature_status: SignatureStatus,
}

/// バンドル検証結果。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PluginBundleVerification {
    pub bundle_id: String,
    pub bundle_version: String,
    pub signature_status: SignatureStatus,
    pub bundle_hash: Option<String>,
    pub manifest_paths: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct PluginModuleInfo {
    pub plugin_id: String,
    pub module_path: PathBuf,
    pub module_hash: String,
}

impl PluginModuleInfo {
    fn as_audit_value(&self) -> Value {
        let mut value = JsonMap::new();
        value.insert("plugin.id".into(), Value::String(self.plugin_id.clone()));
        value.insert(
            "plugin.module_path".into(),
            Value::String(self.module_path.to_string_lossy().to_string()),
        );
        value.insert(
            "plugin.module_hash".into(),
            Value::String(self.module_hash.clone()),
        );
        Value::Object(value)
    }
}

impl PluginBundleManifest {
    pub fn module_info_for(&self, plugin_id: &str) -> Option<&PluginModuleInfo> {
        self.modules
            .iter()
            .find(|module| module.plugin_id == plugin_id)
    }
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
    #[error("bundle 読み込みに失敗しました: {message}")]
    BundleLoad { message: String },
    #[error("plugin signature が見つかりません (Strict モード)")]
    SignatureMissing,
    #[error("plugin signature の検証に失敗しました: {reason}")]
    SignatureInvalid { reason: String },
}

/// プラグイン実行時のエラー。
#[derive(Debug, Error)]
pub enum PluginError {
    #[error("plugin load error: {0}")]
    Load(#[from] PluginLoadError),
    #[error("plugin capability error: {0}")]
    Capability(#[from] CapabilityError),
    #[error("plugin verification failed: {message}")]
    VerificationFailed { message: String },
    #[error("plugin io error: {message}")]
    Io { message: String },
    #[error("plugin already loaded: {plugin_id}")]
    AlreadyLoaded { plugin_id: String },
    #[error("plugin not loaded: {plugin_id}")]
    NotLoaded { plugin_id: String },
    #[error("plugin bridge error: {message}")]
    Bridge { message: String },
    #[error("plugin bundle install failed: {message}")]
    BundleInstallFailed {
        message: String,
        capability_error: Option<CapabilityError>,
    },
}

impl PluginError {
    pub fn into_diagnostic_with_bridge(
        self,
        bridge_id: Option<&str>,
        capability: Option<&str>,
    ) -> GuardDiagnostic {
        let (mut code, kind, mut message, capability_error) = match &self {
            PluginError::Load(error) => (
                "runtime.plugin.load_failed",
                "load",
                error.to_string(),
                None,
            ),
            PluginError::Capability(error) => (
                error.code(),
                "capability",
                error.detail().into(),
                Some(error),
            ),
            PluginError::VerificationFailed { message } => (
                "runtime.plugin.verify_failed",
                "verify",
                message.clone(),
                None,
            ),
            PluginError::Io { message } => ("runtime.plugin.io_error", "io", message.clone(), None),
            PluginError::AlreadyLoaded { plugin_id } => (
                "runtime.plugin.already_loaded",
                "already_loaded",
                format!("plugin already loaded: {plugin_id}"),
                None,
            ),
            PluginError::NotLoaded { plugin_id } => (
                "runtime.plugin.not_loaded",
                "not_loaded",
                format!("plugin not loaded: {plugin_id}"),
                None,
            ),
            PluginError::Bridge { message } => (
                "runtime.plugin.bridge_error",
                "bridge",
                message.clone(),
                None,
            ),
            PluginError::BundleInstallFailed {
                message,
                capability_error,
            } => (
                "runtime.plugin.bundle_install_failed",
                "bundle_install_failed",
                message.clone(),
                capability_error.as_ref(),
            ),
        };

        let stage_snapshot = capability_error.and_then(stage_mismatch_snapshot);
        if let Some(snapshot) = stage_snapshot.as_ref() {
            code = "effects.contract.stage_mismatch";
            message = snapshot.detail.clone();
        }

        let mut extensions = JsonMap::new();
        let mut plugin_meta = JsonMap::new();
        plugin_meta.insert("kind".into(), Value::String(kind.to_string()));
        plugin_meta.insert("message".into(), Value::String(message.clone()));
        if let Some(capability) = capability {
            plugin_meta.insert("capability".into(), Value::String(capability.to_string()));
        }
        if let Some(bridge_id) = bridge_id {
            plugin_meta.insert("bridge_id".into(), Value::String(bridge_id.to_string()));
        }
        extensions.insert("plugin".into(), Value::Object(plugin_meta));
        extensions.insert("message".into(), Value::String(message.clone()));

        let mut audit_metadata = JsonMap::new();
        audit_metadata.insert("plugin.error.kind".into(), Value::String(kind.to_string()));
        audit_metadata.insert(
            "plugin.error.message".into(),
            Value::String(message.clone()),
        );

        if let Some(snapshot) = stage_snapshot {
            extensions.insert(
                "effects.contract.capability".into(),
                Value::String(snapshot.capability_id.clone()),
            );
            extensions.insert(
                "effects.contract.stage.required".into(),
                Value::String(stage_requirement_label(snapshot.required)),
            );
            extensions.insert(
                "effects.contract.stage.actual".into(),
                Value::String(snapshot.actual.as_str().into()),
            );
            if !snapshot.required_effects.is_empty() {
                extensions.insert(
                    "effects.contract.required_effects".into(),
                    Value::Array(
                        snapshot
                            .required_effects
                            .iter()
                            .cloned()
                            .map(Value::String)
                            .collect(),
                    ),
                );
            }
            if !snapshot.missing_effects.is_empty() {
                extensions.insert(
                    "effects.contract.missing_effects".into(),
                    Value::Array(
                        snapshot
                            .missing_effects
                            .iter()
                            .cloned()
                            .map(Value::String)
                            .collect(),
                    ),
                );
            }
            extensions.insert(
                "effects.contract.detail".into(),
                Value::String(snapshot.detail.clone()),
            );

            audit_metadata.insert(
                "effect.capability".into(),
                Value::String(snapshot.capability_id.clone()),
            );
            audit_metadata.insert(
                "effect.stage.required".into(),
                Value::String(stage_requirement_label(snapshot.required)),
            );
            audit_metadata.insert(
                "effect.stage.actual".into(),
                Value::String(snapshot.actual.as_str().into()),
            );
            if !snapshot.required_effects.is_empty() {
                audit_metadata.insert(
                    "effect.required_effects".into(),
                    Value::Array(
                        snapshot
                            .required_effects
                            .iter()
                            .cloned()
                            .map(Value::String)
                            .collect(),
                    ),
                );
            }
            if !snapshot.missing_effects.is_empty() {
                audit_metadata.insert(
                    "effect.missing_effects".into(),
                    Value::Array(
                        snapshot
                            .missing_effects
                            .iter()
                            .cloned()
                            .map(Value::String)
                            .collect(),
                    ),
                );
            }
            audit_metadata.insert(
                "effects.contract.detail".into(),
                Value::String(snapshot.detail.clone()),
            );
        }
        if let Some(capability) = capability {
            let bridge_id = bridge_id
                .map(str::to_string)
                .unwrap_or_else(|| format!("plugin::{capability}"));
            attach_bridge_stage_metadata(&bridge_id, capability, &mut audit_metadata);
        }

        GuardDiagnostic {
            code,
            domain: "runtime",
            severity: DiagnosticSeverity::Error,
            message,
            notes: Vec::new(),
            extensions,
            audit_metadata,
        }
    }
}

impl IntoDiagnostic for PluginError {
    fn into_diagnostic(self) -> GuardDiagnostic {
        self.into_diagnostic_with_bridge(None, None)
    }
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

    pub(crate) fn verify_bundle_signature(
        &self,
        bundle: &PluginBundleManifest,
        policy: VerificationPolicy,
    ) -> Result<SignatureStatus, PluginLoadError> {
        let signature_status = verify_plugin_signature(bundle, policy)?;
        record_signature_audit(bundle, &signature_status);
        Ok(signature_status)
    }

    /// バンドル単位でプラグインを登録する（署名検証含む）。
    pub fn register_bundle(
        &self,
        bundle: PluginBundleManifest,
        policy: VerificationPolicy,
    ) -> Result<PluginBundleRegistration, PluginLoadError> {
        let signature_status = self.verify_bundle_signature(&bundle, policy)?;
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

    /// バンドルファイルから読み込み、登録する。
    pub fn register_bundle_path(
        &self,
        path: impl AsRef<Path>,
        policy: VerificationPolicy,
    ) -> Result<PluginBundleRegistration, PluginLoadError> {
        let bundle = load_bundle_from_path(path)?;
        self.register_bundle(bundle, policy)
    }

    /// バンドルファイルを読み込み、署名検証のみ行う。
    pub fn verify_bundle_path(
        &self,
        path: impl AsRef<Path>,
        policy: VerificationPolicy,
    ) -> Result<PluginBundleVerification, PluginLoadError> {
        let bundle = load_bundle_from_path(path)?;
        let signature_status = self.verify_bundle_signature(&bundle, policy)?;
        let manifest_paths = bundle
            .manifest_paths
            .iter()
            .map(|path| path.to_string_lossy().to_string())
            .collect::<Vec<_>>();
        Ok(PluginBundleVerification {
            bundle_id: bundle.bundle_id,
            bundle_version: bundle.bundle_version,
            signature_status,
            bundle_hash: bundle.bundle_hash,
            manifest_paths,
        })
    }

    /// バンドルファイルを読み込む（登録は行わない）。
    pub fn load_bundle_manifest(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<PluginBundleManifest, PluginLoadError> {
        load_bundle_from_path(path)
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

    pub(crate) fn register_manifest_with_context(
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
        let mut metadata =
            PluginCapabilityMetadata::new(package.clone(), version.clone(), capability_ids.clone());
        if let Some(context) = context {
            metadata.bundle_id = Some(context.bundle_id.clone());
            metadata.bundle_version = Some(context.bundle_version.clone());
        }

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

fn stage_requirement_label(requirement: StageRequirement) -> String {
    match requirement {
        StageRequirement::Exact(stage) => stage.as_str().into(),
        StageRequirement::AtLeast(stage) => format!("at_least {}", stage.as_str()),
    }
}

fn verify_plugin_signature(
    bundle: &PluginBundleManifest,
    policy: VerificationPolicy,
) -> Result<SignatureStatus, PluginLoadError> {
    let signature = match bundle.signature.as_ref() {
        Some(signature) => signature,
        None => {
            return match policy {
                VerificationPolicy::Strict => {
                    record_signature_failure_audit(bundle, "signature が見つかりません");
                    Err(PluginLoadError::SignatureMissing)
                }
                VerificationPolicy::Permissive => Ok(SignatureStatus::Skipped),
            };
        }
    };

    if signature.algorithm.as_str().trim().is_empty() {
        record_signature_failure_audit(bundle, "algorithm が空です");
        return Err(PluginLoadError::SignatureInvalid {
            reason: "algorithm が空です".to_string(),
        });
    }

    if matches!(policy, VerificationPolicy::Strict)
        && matches!(signature.algorithm, SignatureAlgorithm::Unknown(_))
    {
        record_signature_failure_audit(
            bundle,
            &format!("未知の署名アルゴリズム: {}", signature.algorithm.as_str()),
        );
        return Err(PluginLoadError::SignatureInvalid {
            reason: format!("未知の署名アルゴリズム: {}", signature.algorithm.as_str()),
        });
    }

    let signature_hash = signature.bundle_hash.as_deref();
    if signature_hash.is_none() || bundle.bundle_hash.is_none() {
        return match policy {
            VerificationPolicy::Strict => {
                record_signature_failure_audit(bundle, "bundle_hash が不足しています");
                Err(PluginLoadError::SignatureInvalid {
                    reason: "bundle_hash が不足しています".to_string(),
                })
            }
            VerificationPolicy::Permissive => Ok(SignatureStatus::Skipped),
        };
    }

    if signature_hash != bundle.bundle_hash.as_deref() {
        record_signature_failure_audit(bundle, "bundle_hash が一致しません");
        return Err(PluginLoadError::SignatureInvalid {
            reason: "bundle_hash が一致しません".to_string(),
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
    if let Some(bundle_hash) = &bundle.bundle_hash {
        metadata.insert(
            "plugin.bundle_hash".into(),
            Value::String(bundle_hash.clone()),
        );
    }
    if !bundle.modules.is_empty() {
        metadata.insert(
            "plugin.modules".into(),
            Value::Array(
                bundle
                    .modules
                    .iter()
                    .map(PluginModuleInfo::as_audit_value)
                    .collect(),
            ),
        );
    }
    metadata.insert(
        "plugin.signature.status".into(),
        Value::String(
            match status {
                SignatureStatus::Verified => "verified",
                SignatureStatus::Skipped => "skipped",
            }
            .to_string(),
        ),
    );
    if let Some(signature) = &bundle.signature {
        metadata.insert(
            "plugin.signature.algorithm".into(),
            Value::String(signature.algorithm.as_str().to_string()),
        );
        if let Some(bundle_hash) = &signature.bundle_hash {
            metadata.insert(
                "plugin.signature.bundle_hash".into(),
                Value::String(bundle_hash.clone()),
            );
        }
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

fn record_signature_failure_audit(bundle: &PluginBundleManifest, reason: &str) {
    let timestamp = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".into());
    let mut metadata = JsonMap::new();
    metadata.insert(
        "event.kind".into(),
        Value::String(PLUGIN_EVENT_SIGNATURE_FAILURE.to_string()),
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
    if let Some(bundle_hash) = &bundle.bundle_hash {
        metadata.insert(
            "plugin.bundle_hash".into(),
            Value::String(bundle_hash.clone()),
        );
    }
    metadata.insert(
        "plugin.signature.status".into(),
        Value::String("failed".to_string()),
    );
    metadata.insert(
        "plugin.signature.reason".into(),
        Value::String(reason.to_string()),
    );
    if let Some(signature) = &bundle.signature {
        metadata.insert(
            "plugin.signature.algorithm".into(),
            Value::String(signature.algorithm.as_str().to_string()),
        );
        if let Some(bundle_hash) = &signature.bundle_hash {
            metadata.insert(
                "plugin.signature.bundle_hash".into(),
                Value::String(bundle_hash.clone()),
            );
        }
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
    let envelope = AuditEnvelope::from_parts(metadata, None, None, Some("plugin.signature".into()));
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
        if let Some(bundle_hash) = context.bundle_hash.as_ref() {
            metadata.insert(
                "plugin.bundle_hash".into(),
                Value::String(bundle_hash.clone()),
            );
        }
        metadata.insert(
            "plugin.signature.status".into(),
            Value::String(context.signature_status_label().to_string()),
        );
        if let Some(module) = context.module_info_for(&registration.plugin_id) {
            metadata.insert(
                "plugin.module_path".into(),
                Value::String(module.module_path.to_string_lossy().to_string()),
            );
            metadata.insert(
                "plugin.module_hash".into(),
                Value::String(module.module_hash.clone()),
            );
        }
    }
    let envelope = AuditEnvelope::from_parts(metadata, None, None, Some("plugin.install".into()));
    let event = AuditEvent::new(timestamp, envelope);
    PLUGIN_AUDIT_EVENTS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .push(event);
}

pub(crate) fn record_revoke_audit(
    plugin_id: &str,
    capabilities: &[String],
    bundle_id: Option<&str>,
    bundle_version: Option<&str>,
    signature_status: Option<&SignatureStatus>,
) {
    let timestamp = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".into());
    let mut metadata = JsonMap::new();
    metadata.insert(
        "event.kind".into(),
        Value::String(PLUGIN_EVENT_REVOKE.to_string()),
    );
    metadata.insert(
        "event.domain".into(),
        Value::String(PLUGIN_DOMAIN.to_string()),
    );
    metadata.insert("plugin.id".into(), Value::String(plugin_id.to_string()));
    metadata.insert(
        "plugin.capabilities".into(),
        Value::Array(capabilities.iter().cloned().map(Value::String).collect()),
    );
    if let Some(bundle_id) = bundle_id {
        metadata.insert(
            "plugin.bundle_id".into(),
            Value::String(bundle_id.to_string()),
        );
    }
    if let Some(bundle_version) = bundle_version {
        metadata.insert(
            "plugin.bundle_version".into(),
            Value::String(bundle_version.to_string()),
        );
    }
    if let Some(signature_status) = signature_status {
        metadata.insert(
            "plugin.signature.status".into(),
            Value::String(signature_status_label(signature_status).to_string()),
        );
    }
    let envelope = AuditEnvelope::from_parts(metadata, None, None, Some("plugin.revoke".into()));
    let event = AuditEvent::new(timestamp, envelope);
    PLUGIN_AUDIT_EVENTS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .push(event);
}

#[derive(Debug, Clone)]
pub(crate) struct BundleContext {
    bundle_id: String,
    bundle_version: String,
    bundle_hash: Option<String>,
    signature_status: SignatureStatus,
    modules: Vec<PluginModuleInfo>,
}

impl BundleContext {
    pub(crate) fn new(bundle: &PluginBundleManifest, signature_status: SignatureStatus) -> Self {
        Self {
            bundle_id: bundle.bundle_id.clone(),
            bundle_version: bundle.bundle_version.clone(),
            bundle_hash: bundle.bundle_hash.clone(),
            signature_status,
            modules: bundle.modules.clone(),
        }
    }

    fn signature_status_label(&self) -> &str {
        signature_status_label(&self.signature_status)
    }

    fn module_info_for(&self, plugin_id: &str) -> Option<&PluginModuleInfo> {
        self.modules
            .iter()
            .find(|module| module.plugin_id == plugin_id)
    }
}

#[derive(Debug, Clone)]
struct StageMismatchSnapshot {
    capability_id: String,
    required: StageRequirement,
    actual: StageId,
    required_effects: Vec<String>,
    missing_effects: Vec<String>,
    detail: String,
}

fn stage_mismatch_snapshot(error: &CapabilityError) -> Option<StageMismatchSnapshot> {
    match error {
        CapabilityError::StageViolation {
            capability_id,
            required,
            actual,
            message,
            ..
        } => Some(StageMismatchSnapshot {
            capability_id: capability_id.clone(),
            required: *required,
            actual: *actual,
            required_effects: Vec::new(),
            missing_effects: Vec::new(),
            detail: message.clone(),
        }),
        CapabilityError::EffectScopeMismatch {
            capability_id,
            required_stage,
            actual_stage,
            required_effects,
            missing_effects,
            message,
            ..
        } => Some(StageMismatchSnapshot {
            capability_id: capability_id.clone(),
            required: *required_stage,
            actual: *actual_stage,
            required_effects: required_effects.clone(),
            missing_effects: missing_effects.clone(),
            detail: message.clone(),
        }),
        _ => None,
    }
}

fn signature_status_label(signature_status: &SignatureStatus) -> &'static str {
    match signature_status {
        SignatureStatus::Verified => "verified",
        SignatureStatus::Skipped => "skipped",
    }
}

pub fn take_plugin_audit_events() -> Vec<AuditEvent> {
    PLUGIN_AUDIT_EVENTS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .drain(..)
        .collect()
}

#[derive(Debug, Deserialize)]
struct PluginBundleFile {
    bundle_id: String,
    bundle_version: String,
    plugins: Vec<PluginBundleEntry>,
    signature: Option<PluginSignatureFile>,
}

#[derive(Debug, Deserialize)]
struct PluginBundleEntry {
    manifest_path: PathBuf,
    #[serde(default)]
    module_path: Option<PathBuf>,
}

#[derive(Debug, Deserialize)]
struct PluginSignatureFile {
    algorithm: Option<String>,
    certificate: Option<String>,
    issued_to: Option<String>,
    valid_until: Option<String>,
    bundle_hash: Option<String>,
}

fn load_bundle_from_path(path: impl AsRef<Path>) -> Result<PluginBundleManifest, PluginLoadError> {
    let bundle_path = path.as_ref();
    let body = fs::read_to_string(bundle_path).map_err(|err| PluginLoadError::BundleLoad {
        message: err.to_string(),
    })?;
    let bundle_file: PluginBundleFile =
        serde_json::from_str(&body).map_err(|err| PluginLoadError::BundleLoad {
            message: err.to_string(),
        })?;
    let base_dir = bundle_path.parent().unwrap_or_else(|| Path::new("."));
    let mut manifests = Vec::new();
    let mut hash_sources = Vec::new();
    let mut manifest_paths = Vec::new();
    let mut modules = Vec::new();

    for entry in &bundle_file.plugins {
        let manifest_path = base_dir.join(&entry.manifest_path);
        manifest_paths.push(entry.manifest_path.clone());
        let manifest_body =
            fs::read_to_string(&manifest_path).map_err(|err| PluginLoadError::BundleLoad {
                message: err.to_string(),
            })?;
        let manifest =
            load_manifest(&manifest_path).map_err(|diagnostic| PluginLoadError::BundleLoad {
                message: diagnostic.message,
            })?;
        if let Some(module_path) = entry.module_path.as_ref() {
            let resolved_path = base_dir.join(module_path);
            let module_bytes =
                fs::read(&resolved_path).map_err(|err| PluginLoadError::BundleLoad {
                    message: err.to_string(),
                })?;
            modules.push(PluginModuleInfo {
                plugin_id: manifest.project.name.0.clone(),
                module_path: resolved_path,
                module_hash: compute_module_hash(&module_bytes),
            });
        }
        manifests.push(manifest);
        hash_sources.push((manifest_path, manifest_body));
    }

    let bundle_hash = Some(compute_bundle_hash(
        &bundle_file.bundle_id,
        &bundle_file.bundle_version,
        &hash_sources,
    ));

    let signature = bundle_file.signature.map(|sig| PluginSignature {
        algorithm: parse_signature_algorithm(sig.algorithm),
        certificate: sig.certificate,
        issued_to: sig.issued_to,
        valid_until: sig.valid_until,
        bundle_hash: sig.bundle_hash,
    });

    Ok(PluginBundleManifest {
        bundle_id: bundle_file.bundle_id,
        bundle_version: bundle_file.bundle_version,
        plugins: manifests,
        signature,
        bundle_hash,
        modules,
        manifest_paths,
    })
}

fn parse_signature_algorithm(value: Option<String>) -> SignatureAlgorithm {
    match value.as_deref() {
        Some("ed25519") => SignatureAlgorithm::Ed25519,
        Some(other) => SignatureAlgorithm::Unknown(other.to_string()),
        None => SignatureAlgorithm::Unknown("unknown".to_string()),
    }
}

fn compute_bundle_hash(
    bundle_id: &str,
    bundle_version: &str,
    sources: &[(PathBuf, String)],
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bundle_id.as_bytes());
    hasher.update(b"\n");
    hasher.update(bundle_version.as_bytes());
    hasher.update(b"\n");
    for (path, body) in sources {
        hasher.update(path.to_string_lossy().as_bytes());
        hasher.update(b"\n");
        hasher.update(body.as_bytes());
        hasher.update(b"\n");
    }
    let digest = hasher.finalize();
    format!("sha256:{}", bytes_to_hex(digest.as_slice()))
}

fn compute_module_hash(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    format!("sha256:{}", bytes_to_hex(digest.as_slice()))
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for value in bytes {
        out.push(hex_nibble(value >> 4));
        out.push(hex_nibble(value & 0x0f));
    }
    out
}

fn hex_nibble(value: u8) -> char {
    match value {
        0..=9 => (b'0' + value) as char,
        10..=15 => (b'a' + (value - 10)) as char,
        _ => '0',
    }
}
