use std::{
    collections::{HashMap, HashSet},
    fmt,
    path::Path,
    sync::{Arc, Mutex},
    time::SystemTime,
};

use once_cell::sync::OnceCell;
use serde::Serialize;
use serde_json::{json, Map, Value};

use crate::{
    audit::{AuditError, AuditSink},
    capability_handle::CapabilityHandle,
    capability_metadata::{CapabilityDescriptor, CapabilityId, StageId, StageRequirement},
    manifest_contract::{
        ConductorCapabilityContract, ConductorCapabilityRequirement, ManifestCapabilities,
        ManifestCapabilityEntry, ManifestError,
    },
};

/// Bridge/Streaming から渡される Stage trace の 1 フレーム。
#[derive(Debug, Clone, Serialize)]
pub struct BridgeStageTraceStep {
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capability: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
}

/// Streaming parser / Runtime Bridge の意図を表す。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum BridgeIntent {
    Await,
    Resume,
    Backpressure,
}

impl BridgeIntent {
    pub fn as_str(&self) -> &'static str {
        match self {
            BridgeIntent::Await => "await",
            BridgeIntent::Resume => "resume",
            BridgeIntent::Backpressure => "backpressure",
        }
    }
}

/// Runtime から収集する Stage mismatch/backpressure 診断のメタデータ。
#[derive(Debug, Clone, Serialize)]
pub struct RuntimeBridgeStreamSignal {
    pub bridge_id: String,
    pub required_stage: String,
    pub actual_stage: String,
    pub intent: BridgeIntent,
    pub reason: String,
    pub await_count: u32,
    pub resume_count: u32,
    pub backpressure_count: u32,
    pub parser_offset: Option<u32>,
    pub stream_sequence: Option<u64>,
    pub stage_trace: Vec<BridgeStageTraceStep>,
    pub timestamp: SystemTime,
}

impl RuntimeBridgeStreamSignal {
    pub fn normalized_reason(&self) -> String {
        self.reason.clone()
    }
}

/// Runtime 側で Stage/backpressure 信号をキャッシュするレジストリ。
pub struct RuntimeBridgeRegistry {
    signals: Mutex<HashMap<String, RuntimeBridgeStreamSignal>>,
}

impl RuntimeBridgeRegistry {
    pub fn registry() -> &'static RuntimeBridgeRegistry {
        static INSTANCE: OnceCell<RuntimeBridgeRegistry> = OnceCell::new();
        INSTANCE.get_or_init(|| RuntimeBridgeRegistry {
            signals: Mutex::new(HashMap::new()),
        })
    }

    pub fn stream_signal(&self, signal: RuntimeBridgeStreamSignal) {
        let mut lock = self
            .signals
            .lock()
            .expect("RuntimeBridgeRegistry mutex がロックできません");
        lock.insert(signal.bridge_id.clone(), signal);
    }

    pub fn latest_signal(&self, bridge_id: &str) -> Option<RuntimeBridgeStreamSignal> {
        if let Ok(lock) = self.signals.lock() {
            lock.get(bridge_id).cloned()
        } else {
            None
        }
    }
}

/// Registry 内の Capability を格納するシングルトン。
#[derive(Clone)]
pub struct CapabilityRegistry {
    handles: Arc<Mutex<HashMap<CapabilityId, CapabilityHandle>>>,
}

impl CapabilityRegistry {
    /// グローバルインスタンスを取得する。
    pub fn registry() -> CapabilityRegistry {
        static INSTANCE: OnceCell<Arc<Mutex<HashMap<CapabilityId, CapabilityHandle>>>> =
            OnceCell::new();
        CapabilityRegistry {
            handles: INSTANCE
                .get_or_init(|| Arc::new(Mutex::new(HashMap::new())))
                .clone(),
        }
    }

    /// Descriptor を登録する。
    pub fn register(&self, handle: CapabilityHandle) -> Result<(), CapabilityError> {
        let id = handle.descriptor().id.clone();
        let mut lock = self
            .handles
            .lock()
            .expect("CapabilityRegistry mutex がロックできません");
        if lock.contains_key(&id) {
            return Err(CapabilityError::AlreadyRegistered { id });
        }
        lock.insert(id, handle);
        Ok(())
    }

    /// 登録済み Capability を取得する（クローン）。
    pub fn get(&self, id: &CapabilityId) -> Option<CapabilityDescriptor> {
        let lock = self
            .handles
            .lock()
            .expect("CapabilityRegistry mutex がロックできません");
        lock.get(id).map(|handle| handle.descriptor().clone())
    }

    /// Descriptor を返す。
    pub fn describe(&self, id: &CapabilityId) -> Option<CapabilityDescriptor> {
        self.get(id)
    }

    /// Stage 要件と効果スコープを検証し、必要な Stage を返す。
    pub fn verify_capability_stage(
        &self,
        id: impl AsRef<str>,
        requirement: StageRequirement,
        required_effects: &[String],
    ) -> Result<StageId, CapabilityError> {
        self.verify_capability_handle(id, requirement, required_effects)
            .map(|handle| handle.descriptor().stage)
    }

    /// Stage 要件と効果スコープを検証し、ハンドルを返す。
    pub fn verify_capability_handle(
        &self,
        id: impl AsRef<str>,
        requirement: StageRequirement,
        required_effects: &[String],
    ) -> Result<CapabilityHandle, CapabilityError> {
        let mut lock = self
            .handles
            .lock()
            .expect("CapabilityRegistry mutex がロックできません");

        let key = id.as_ref();
        let requested_id = key.to_string();
        let handle = lock
            .get_mut(key)
            .ok_or_else(|| CapabilityError::MissingCapability {
                id: requested_id.clone(),
            })?;

        {
            let descriptor = handle.descriptor();
            if !requirement.matches(descriptor.stage) {
                return Err(CapabilityError::StageViolation {
                    id: descriptor.id.clone(),
                    required: requirement,
                    actual: descriptor.stage,
                });
            }

            let actual_scope = descriptor.effect_scope.clone();
            let missing_effects: Vec<String> = required_effects
                .iter()
                .filter(|effect| !actual_scope.contains(effect))
                .cloned()
                .collect();
            if !missing_effects.is_empty() {
                return Err(CapabilityError::EffectViolation {
                    id: descriptor.id.clone(),
                    required: required_effects.to_vec(),
                    missing: missing_effects,
                    actual_scope,
                });
            }
        }

        handle.descriptor_mut().last_verified_at = Some(SystemTime::now());
        Ok(handle.clone())
    }

    /// Core.IO アダプタ向けに Stage 情報だけを返すヘルパ。
    pub fn verify_stage_for_io(
        &self,
        capability: &'static str,
        requirement: StageRequirement,
    ) -> Result<StageId, CapabilityError> {
        self.verify_capability_stage(capability, requirement, &[])
    }

    /// Conductor/DSL から渡された契約集合を検査する。
    pub fn verify_conductor_contract(
        &self,
        contract: ConductorCapabilityContract,
        audit_sink: Option<&AuditSink>,
    ) -> Result<(), CapabilityError> {
        let manifest_path_buf = contract.manifest_path.clone();
        let manifest_path_label = manifest_path_buf
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned());
        let manifest_data = if let Some(path) = manifest_path_buf.as_deref() {
            Some(ManifestCapabilities::load(path).map_err(|source| {
                CapabilityError::ManifestLoadFailure {
                    manifest_path: manifest_path_label.clone(),
                    source,
                }
            })?)
        } else {
            None
        };

        for requirement in &contract.requirements {
            let handle = self.verify_capability_handle(
                &requirement.id,
                requirement.stage,
                &requirement.declared_effects,
            )?;
            log_contract_audit(
                audit_sink,
                requirement,
                handle.descriptor(),
                manifest_path_buf.as_deref(),
            )?;

            if let Some(manifest) = manifest_data.as_ref() {
                if let Some(entry) = manifest.get(&requirement.id) {
                    if entry.stage != requirement.stage {
                        log_manifest_mismatch_event(
                            audit_sink,
                            manifest_path_buf.as_deref(),
                            requirement,
                            Some(entry),
                            "stage mismatch",
                        )?;
                        return Err(CapabilityError::ManifestMismatch {
                            id: requirement.id.clone(),
                            manifest_path: manifest_path_label.clone(),
                            reason: format!(
                                "manifest stage={} vs contract stage={}",
                                entry.stage, requirement.stage
                            ),
                        });
                    }

                    let requirement_effects: HashSet<_> =
                        requirement.declared_effects.iter().cloned().collect();
                    let manifest_effects: HashSet<_> =
                        entry.declared_effects.iter().cloned().collect();
                    if requirement_effects != manifest_effects {
                        log_manifest_mismatch_event(
                            audit_sink,
                            manifest_path_buf.as_deref(),
                            requirement,
                            Some(entry),
                            "declared_effects mismatch",
                        )?;
                        return Err(CapabilityError::ManifestMismatch {
                            id: requirement.id.clone(),
                            manifest_path: manifest_path_label.clone(),
                            reason: format!(
                                "manifest effects {:?} vs contract {:?}",
                                manifest_effects, requirement_effects
                            ),
                        });
                    }

                    if entry.source_span != requirement.source_span {
                        log_manifest_mismatch_event(
                            audit_sink,
                            manifest_path_buf.as_deref(),
                            requirement,
                            Some(entry),
                            "source span mismatch",
                        )?;
                        return Err(CapabilityError::ManifestMismatch {
                            id: requirement.id.clone(),
                            manifest_path: manifest_path_label.clone(),
                            reason: "manifest と source_span が一致しません".into(),
                        });
                    }
                } else {
                    log_manifest_mismatch_event(
                        audit_sink,
                        manifest_path_buf.as_deref(),
                        requirement,
                        None,
                        "missing manifest entry",
                    )?;
                    return Err(CapabilityError::ManifestMismatch {
                        id: requirement.id.clone(),
                        manifest_path: manifest_path_label.clone(),
                        reason: "manifest entry が見つかりません".into(),
                    });
                }
            }
        }

        Ok(())
    }
}

/// Capability 検証エラー。
#[derive(Debug, Clone)]
pub enum CapabilityError {
    AlreadyRegistered {
        id: CapabilityId,
    },
    MissingCapability {
        id: CapabilityId,
    },
    StageViolation {
        id: CapabilityId,
        required: StageRequirement,
        actual: StageId,
    },
    EffectViolation {
        id: CapabilityId,
        required: Vec<String>,
        missing: Vec<String>,
        actual_scope: Vec<String>,
    },
    ManifestLoadFailure {
        manifest_path: Option<String>,
        source: ManifestError,
    },
    ManifestMismatch {
        id: CapabilityId,
        manifest_path: Option<String>,
        reason: String,
    },
    AuditFailure {
        source: AuditError,
    },
}

impl fmt::Display for CapabilityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CapabilityError::AlreadyRegistered { id } => {
                write!(f, "Capability '{}' はすでに登録済みです", id)
            }
            CapabilityError::MissingCapability { id } => {
                write!(f, "Capability '{}' は未登録です", id)
            }
            CapabilityError::StageViolation {
                id,
                required,
                actual,
            } => write!(
                f,
                "Capability '{}' の stage が一致しません (required={}, actual={})",
                id, required, actual
            ),
            CapabilityError::EffectViolation {
                id,
                missing,
                actual_scope,
                ..
            } => write!(
                f,
                "Capability '{}' に required effects {} が含まれていません (available={})",
                id,
                missing.join(", "),
                actual_scope.join(", ")
            ),
            CapabilityError::ManifestLoadFailure {
                manifest_path,
                source,
            } => {
                if let Some(path) = manifest_path {
                    write!(f, "manifest '{}' の読み込みに失敗: {}", path, source)
                } else {
                    write!(f, "manifest の読み込みに失敗しました: {}", source)
                }
            }
            CapabilityError::ManifestMismatch {
                id,
                manifest_path,
                reason,
            } => {
                if let Some(path) = manifest_path {
                    write!(
                        f,
                        "Capability '{}': manifest '{}' と不一致 ({})",
                        id, path, reason
                    )
                } else {
                    write!(f, "Capability '{}': manifest と不一致 ({})", id, reason)
                }
            }
            CapabilityError::AuditFailure { source } => {
                write!(f, "監査ログへの記録に失敗しました: {}", source)
            }
        }
    }
}

impl std::error::Error for CapabilityError {}

fn log_contract_audit(
    sink: Option<&AuditSink>,
    requirement: &ConductorCapabilityRequirement,
    descriptor: &CapabilityDescriptor,
    manifest_path: Option<&Path>,
) -> Result<(), CapabilityError> {
    if let Some(sink) = sink {
        let mut metadata = Map::new();
        metadata.insert(
            "effect.capability".into(),
            Value::String(requirement.id.clone()),
        );
        metadata.insert(
            "effect.stage.required".into(),
            Value::String(requirement.stage.to_string()),
        );
        metadata.insert(
            "effect.stage.actual".into(),
            Value::String(descriptor.stage.to_string()),
        );
        metadata.insert(
            "effect.stage.required_effects".into(),
            Value::Array(
                requirement
                    .declared_effects
                    .iter()
                    .map(|effect| Value::String(effect.clone()))
                    .collect(),
            ),
        );
        metadata.insert(
            "effect.scope".into(),
            Value::Array(
                descriptor
                    .effect_scope
                    .iter()
                    .map(|scope| Value::String(scope.clone()))
                    .collect(),
            ),
        );
        if let Some(path) = manifest_path {
            metadata.insert(
                "effect.manifest_path".into(),
                Value::String(path.to_string_lossy().into_owned()),
            );
        }
        if let Some(span) = requirement.source_span {
            metadata.insert(
                "effect.stage.source".into(),
                json!({
                    "start": span.start,
                    "end": span.end,
                    "length": span.len(),
                }),
            );
        }
        sink.log("effect.stage.contract", Value::Null, metadata)
            .map_err(|source| CapabilityError::AuditFailure { source })?;
    }
    Ok(())
}

fn log_manifest_mismatch_event(
    sink: Option<&AuditSink>,
    manifest_path: Option<&Path>,
    requirement: &ConductorCapabilityRequirement,
    entry: Option<&ManifestCapabilityEntry>,
    reason: &str,
) -> Result<(), CapabilityError> {
    if let Some(sink) = sink {
        let mut metadata = Map::new();
        metadata.insert(
            "effect.capability".into(),
            Value::String(requirement.id.clone()),
        );
        metadata.insert(
            "effect.stage.required".into(),
            Value::String(requirement.stage.to_string()),
        );
        metadata.insert("audit.reason".into(), Value::String(reason.to_string()));
        metadata.insert(
            "effect.stage.required_effects".into(),
            Value::Array(
                requirement
                    .declared_effects
                    .iter()
                    .map(|effect| Value::String(effect.clone()))
                    .collect(),
            ),
        );
        if let Some(entry) = entry {
            metadata.insert(
                "effect.stage.manifest".into(),
                Value::String(entry.stage.to_string()),
            );
            metadata.insert(
                "effect.stage.manifest_effects".into(),
                Value::Array(
                    entry
                        .declared_effects
                        .iter()
                        .map(|effect| Value::String(effect.clone()))
                        .collect(),
                ),
            );
            if let Some(span) = entry.source_span {
                metadata.insert(
                    "effect.stage.manifest_source".into(),
                    json!({
                        "start": span.start,
                        "end": span.end,
                        "length": span.len(),
                    }),
                );
            }
        }
        if let Some(path) = manifest_path {
            metadata.insert(
                "effect.manifest_path".into(),
                Value::String(path.to_string_lossy().into_owned()),
            );
        }
        sink.log("effect.stage.capability_mismatch", Value::Null, metadata)
            .map_err(|source| CapabilityError::AuditFailure { source })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability_handle::CapabilityHandle;
    use crate::capability_metadata::{CapabilityDescriptor, CapabilityProvider, StageId};
    use crate::{
        AuditSink, CapabilityContractSpan, ConductorCapabilityContract,
        ConductorCapabilityRequirement,
    };
    use std::{fs, time::SystemTime};

    fn new_gc_handle(id: &str, stage: StageId) -> CapabilityHandle {
        CapabilityHandle::gc(CapabilityDescriptor::new(
            id,
            stage,
            vec!["ffi".into()],
            CapabilityProvider::Core,
        ))
    }

    #[test]
    fn stage_requirement_matches() {
        assert!(StageRequirement::Exact(StageId::Beta).matches(StageId::Beta));
        assert!(!StageRequirement::Exact(StageId::Beta).matches(StageId::Stable));
        assert!(StageRequirement::AtLeast(StageId::Beta).matches(StageId::Stable));
        assert!(!StageRequirement::AtLeast(StageId::Stable).matches(StageId::Beta));
    }

    #[test]
    fn register_and_verify_capability() {
        let registry = CapabilityRegistry::registry();
        let handle = new_gc_handle("ffi.capability", StageId::Beta);
        let id = handle.descriptor().id.clone();
        let _ = registry.handles.lock().expect("lock").remove(&id);

        registry
            .register(handle.clone())
            .expect("登録に失敗しました");

        let required_effects = vec!["ffi".into()];
        let verified = registry
            .verify_capability_handle(
                &id,
                StageRequirement::Exact(StageId::Beta),
                &required_effects,
            )
            .expect("stage 検証に失敗");
        assert_eq!(verified.descriptor().stage, StageId::Beta);
        assert!(verified.as_gc().is_some());
    }

    #[test]
    fn stage_violation_error() {
        let registry = CapabilityRegistry::registry();
        let handle = new_gc_handle("ffi.stage-test", StageId::Experimental);
        let id = handle.descriptor().id.clone();
        let _ = registry.handles.lock().expect("lock").remove(&id);
        registry.register(handle).expect("登録失敗");

        let required_effects: &[String] = &[];
        let result = registry.verify_capability_stage(
            &id,
            StageRequirement::Exact(StageId::Beta),
            required_effects,
        );
        assert!(matches!(
            result,
            Err(CapabilityError::StageViolation { .. })
        ));
    }
    #[test]
    fn effect_violation_error() {
        let registry = CapabilityRegistry::registry();
        let handle = new_gc_handle("ffi.effect-test", StageId::Beta);
        let id = handle.descriptor().id.clone();
        let _ = registry.handles.lock().expect("lock").remove(&id);
        registry.register(handle).expect("登録失敗");

        let required_effects = vec!["audit".into()];
        let result = registry.verify_capability_stage(
            &id,
            StageRequirement::Exact(StageId::Beta),
            &required_effects,
        );
        assert!(matches!(
            result,
            Err(CapabilityError::EffectViolation { .. })
        ));
    }

    #[test]
    fn conductor_contract_logs_stage_metadata() {
        let registry = CapabilityRegistry::registry();
        let handle = new_gc_handle("ffi.contract-test", StageId::Beta);
        let id = handle.descriptor().id.clone();
        let _ = registry.handles.lock().expect("lock").remove(&id);
        registry.register(handle).expect("登録失敗");

        let span = CapabilityContractSpan::new(4, 11);
        let contract = ConductorCapabilityContract {
            requirements: vec![ConductorCapabilityRequirement {
                id: id.clone(),
                stage: StageRequirement::Exact(StageId::Beta),
                declared_effects: vec!["ffi".into()],
                source_span: Some(span),
            }],
            manifest_path: None,
        };

        let sink = AuditSink::new();
        registry
            .verify_conductor_contract(contract, Some(&sink))
            .expect("契約検証に失敗");

        let entries = sink.entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].event, "effect.stage.contract");
        assert_eq!(
            entries[0].metadata["effect.capability"].as_str(),
            Some(id.as_str())
        );
        assert_eq!(
            entries[0].metadata["effect.stage.source"]["length"],
            span.len()
        );
    }

    #[test]
    fn manifest_mismatch_error_logs_audit() {
        let registry = CapabilityRegistry::registry();
        let handle = new_gc_handle("ffi.manifest-mismatch", StageId::Beta);
        let id = handle.descriptor().id.clone();
        let _ = registry.handles.lock().expect("lock").remove(&id);
        registry.register(handle).expect("登録失敗");

        let manifest_path = std::env::temp_dir().join(format!(
            "reml_manifest_contract_{}.toml",
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_file(&manifest_path);
        fs::write(
            &manifest_path,
            r#"
[run.target]
[[run.target.capabilities]]
id = "ffi.manifest-mismatch"
stage = "stable"
declared_effects = ["ffi"]
"#,
        )
        .expect("manifest 書き込み失敗");

        let contract = ConductorCapabilityContract {
            requirements: vec![ConductorCapabilityRequirement {
                id: id.clone(),
                stage: StageRequirement::Exact(StageId::Beta),
                declared_effects: vec!["ffi".into()],
                source_span: None,
            }],
            manifest_path: Some(manifest_path.clone()),
        };
        let sink = AuditSink::new();

        let err = registry
            .verify_conductor_contract(contract, Some(&sink))
            .expect_err("manifest mismatch で失敗するはず");
        assert!(matches!(err, CapabilityError::ManifestMismatch { .. }));
        let entries = sink.entries();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[1].event, "effect.stage.capability_mismatch");
        let _ = fs::remove_file(&manifest_path);
    }
}
