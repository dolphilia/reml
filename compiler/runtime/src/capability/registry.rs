use std::{
    collections::{HashMap, HashSet},
    ops::{Deref, DerefMut},
    path::PathBuf,
    sync::{Mutex, RwLock, RwLockReadGuard},
};

use once_cell::sync::Lazy;
use serde_json::{Map as JsonMap, Number, Value};
use thiserror::Error;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use super::{
    audit::{AuditCapability, AuditCapabilityMetadata},
    collections::{CollectionsCapability, CollectionsCapabilityMetadata},
    contract::{
        CapabilityContractSpan, ConductorCapabilityContract, ConductorCapabilityRequirement,
    },
    descriptor::{
        CapabilityDescriptor, CapabilityId, CapabilityProvider, CapabilityTimestamp, EffectTag,
    },
    handle::CapabilityHandle,
    io::{IoAdapterKind, IoCapability, IoCapabilityMetadata, IoOperationKind},
    memory::{MemoryCapability, MemoryCapabilityMetadata},
    metrics::{MetricsCapability, MetricsCapabilityMetadata, MetricsExporterKind},
    native::{NativeCapability, NativeCapabilityMetadata},
    plugin::{PluginCapability, PluginCapabilityMetadata},
    process::{ProcessCapability, ProcessCapabilityMetadata},
    realtime::{RealtimeCapability, RealtimeCapabilityMetadata, RealtimeClockSource},
    security::{SecurityCapability, SecurityCapabilityMetadata, SecurityPolicyKind},
    signal::{SignalCapability, SignalCapabilityMetadata},
    system::{SystemCapability, SystemCapabilityMetadata},
};
use crate::{
    audit::{AuditEnvelope, AuditEvent, AuditEventKind},
    config::manifest::{ManifestCapabilities, ManifestCapabilityError},
    stage::{StageId, StageRequirement},
};

static REGISTRY: Lazy<RwLock<Option<&'static CapabilityRegistry>>> =
    Lazy::new(|| RwLock::new(None));
#[cfg(test)]
static LEAKED_FOR_TESTS: Lazy<Mutex<Vec<Box<CapabilityRegistry>>>> =
    Lazy::new(|| Mutex::new(Vec::new()));
static REGISTRY_TEST_LOCK: Lazy<RwLock<()>> = Lazy::new(|| RwLock::new(()));
const CAPABILITY_SCHEMA_VERSION: &str = "3.0.0-alpha";

/// Capability を検証するためのレジストリ。
#[derive(Debug)]
pub struct CapabilityRegistry {
    entries: RwLock<CapabilityEntries>,
    audit_events: RwLock<Vec<AuditEvent>>,
}

impl CapabilityRegistry {
    /// シングルトンのレジストリを取得する。
    pub fn registry() -> &'static Self {
        let _test_guard = REGISTRY_TEST_LOCK.read().unwrap();
        if let Some(instance) = Self::try_get_cached(REGISTRY.read().unwrap()) {
            return instance;
        }
        let mut guard = REGISTRY.write().unwrap();
        if let Some(instance) = *guard {
            return instance;
        }
        let leaked: &'static CapabilityRegistry = Box::leak(Box::new(Self::new()));
        leaked.bootstrap_default_capabilities();
        *guard = Some(leaked);
        leaked
    }

    fn try_get_cached(
        guard: RwLockReadGuard<'_, Option<&'static CapabilityRegistry>>,
    ) -> Option<&'static Self> {
        *guard
    }

    fn new() -> Self {
        Self {
            entries: RwLock::new(CapabilityEntries::default()),
            audit_events: RwLock::new(Vec::new()),
        }
    }

    fn bootstrap_default_capabilities(&self) {
        for handle in builtin_capabilities() {
            let _ = self.register(handle);
        }
    }

    /// Capability を登録する。
    pub fn register(&self, handle: CapabilityHandle) -> Result<(), CapabilityError> {
        let descriptor = handle.descriptor().clone();
        let capability_id = descriptor.id.clone();
        let mut entries = self.entries.write().unwrap();
        if entries.entries.contains_key(&capability_id) {
            return Err(CapabilityError::already_registered(capability_id));
        }
        entries.ordered_keys.push(capability_id.clone());
        entries
            .entries
            .insert(capability_id, CapabilityEntry { descriptor, handle });
        Ok(())
    }

    /// Capability を登録解除する。
    pub fn unregister(&self, capability: &str) -> Result<(), CapabilityError> {
        let mut entries = self.entries.write().unwrap();
        let capability_id = capability.to_string();
        if entries.entries.remove(&capability_id).is_none() {
            return Err(CapabilityError::not_registered(capability));
        }
        entries.ordered_keys.retain(|id| id != &capability_id);
        Ok(())
    }

    /// Plugin Capability を登録する。
    pub fn register_plugin_capability(
        &self,
        id: &str,
        stage: StageId,
        effects: &[&str],
        metadata: PluginCapabilityMetadata,
    ) -> Result<(), CapabilityError> {
        let provider = CapabilityProvider::Plugin {
            package: metadata.package.clone(),
            version: metadata.version.clone(),
        };
        let descriptor = CapabilityDescriptor::new(id, stage, effects.iter().copied(), provider);
        let handle = CapabilityHandle::Plugin(PluginCapability::new(descriptor, metadata));
        self.register(handle)
    }

    fn ensure_manifest_alignment(
        &self,
        requirement: &ConductorCapabilityRequirement,
        descriptor: &CapabilityDescriptor,
        manifest: &ManifestCapabilities,
        manifest_path: Option<&PathBuf>,
    ) -> Result<(), CapabilityError> {
        let manifest_entry = match manifest.get(&requirement.id) {
            Some(entry) => entry,
            None => {
                let error = CapabilityError::contract_violation(
                    requirement.id.clone(),
                    manifest_path.cloned(),
                    requirement.source_span.clone(),
                    Some(descriptor.clone()),
                    format!(
                        "manifest entry for capability '{}' is missing",
                        requirement.id
                    ),
                );
                self.record_capability_check(
                    &requirement.id,
                    requirement.stage,
                    Some(descriptor.stage()),
                    Some(descriptor),
                    &requirement.declared_effects,
                    Err(&error),
                );
                return Err(error);
            }
        };

        if manifest_entry.stage != requirement.stage {
            let message = format!(
                "manifest stage {} does not match contract stage {} for '{}'",
                manifest_entry.stage, requirement.stage, requirement.id
            );
            let error = CapabilityError::contract_violation(
                requirement.id.clone(),
                manifest_path.cloned(),
                requirement.source_span.clone(),
                Some(descriptor.clone()),
                message,
            );
            self.record_capability_check(
                &requirement.id,
                requirement.stage,
                Some(descriptor.stage()),
                Some(descriptor),
                &requirement.declared_effects,
                Err(&error),
            );
            return Err(error);
        }

        let required_effects: HashSet<_> = requirement.declared_effects.iter().cloned().collect();
        let manifest_effects: HashSet<_> =
            manifest_entry.declared_effects.iter().cloned().collect();
        if required_effects != manifest_effects {
            let missing: Vec<_> = required_effects
                .difference(&manifest_effects)
                .cloned()
                .collect();
            let extra: Vec<_> = manifest_effects
                .difference(&required_effects)
                .cloned()
                .collect();
            let message = format!(
                "declared_effects mismatch for '{}': missing {:?}, unexpected {:?}",
                requirement.id, missing, extra
            );
            let error = CapabilityError::contract_violation(
                requirement.id.clone(),
                manifest_path.cloned(),
                requirement.source_span.clone(),
                Some(descriptor.clone()),
                message,
            );
            self.record_capability_check(
                &requirement.id,
                requirement.stage,
                Some(descriptor.stage()),
                Some(descriptor),
                &requirement.declared_effects,
                Err(&error),
            );
            return Err(error);
        }

        if manifest_entry.source_span != requirement.source_span {
            let message = format!(
                "source span mismatch for '{}': manifest={:?}, contract={:?}",
                requirement.id, manifest_entry.source_span, requirement.source_span
            );
            let error = CapabilityError::contract_violation(
                requirement.id.clone(),
                manifest_path.cloned(),
                requirement.source_span.clone(),
                Some(descriptor.clone()),
                message,
            );
            self.record_capability_check(
                &requirement.id,
                requirement.stage,
                Some(descriptor.stage()),
                Some(descriptor),
                &requirement.declared_effects,
                Err(&error),
            );
            return Err(error);
        }

        Ok(())
    }

    /// Capability ハンドルを取得する。
    pub fn get(&self, capability: &str) -> Result<CapabilityHandle, CapabilityError> {
        let entries = self.entries.read().unwrap();
        entries
            .entries
            .get(capability)
            .map(|entry| entry.handle.clone())
            .ok_or_else(|| CapabilityError::not_registered(capability))
    }

    /// CapabilityDescriptor を返す。
    pub fn describe(&self, capability: &str) -> Result<CapabilityDescriptor, CapabilityError> {
        let entries = self.entries.read().unwrap();
        entries
            .entries
            .get(capability)
            .map(|entry| entry.descriptor.clone())
            .ok_or_else(|| CapabilityError::not_registered(capability))
    }

    /// すべての CapabilityDescriptor を登録順に返す。
    pub fn describe_all(&self) -> CapabilityDescriptorList {
        let entries = self.entries.read().unwrap();
        let descriptors = entries
            .ordered_keys
            .iter()
            .filter_map(|id| entries.entries.get(id))
            .map(|entry| entry.descriptor.clone())
            .collect();
        CapabilityDescriptorList::new(descriptors)
    }

    /// すべての CapabilityHandle を登録順に返す。
    pub fn handles_all(&self) -> Vec<CapabilityHandle> {
        let entries = self.entries.read().unwrap();
        entries
            .ordered_keys
            .iter()
            .filter_map(|id| entries.entries.get(id))
            .map(|entry| entry.handle.clone())
            .collect()
    }

    /// Capability ハンドルを取得し Stage/効果スコープを検証する。
    pub fn verify_capability(
        &self,
        capability: &str,
        requirement: StageRequirement,
        required_effects: &[String],
    ) -> Result<CapabilityHandle, CapabilityError> {
        let (handle, descriptor) = {
            let entries = self.entries.read().unwrap();
            match entries.entries.get(capability) {
                Some(entry) => (entry.handle.clone(), entry.descriptor.clone()),
                None => {
                    let error = CapabilityError::not_registered(capability);
                    self.record_capability_check(
                        capability,
                        requirement,
                        None,
                        None,
                        required_effects,
                        Err(&error),
                    );
                    return Err(error);
                }
            }
        };
        let actual_stage = descriptor.stage();
        if !requirement.matches(actual_stage) {
            let error = CapabilityError::stage_violation(
                capability,
                requirement,
                actual_stage,
                Some(descriptor.clone()),
            );
            self.record_capability_check(
                capability,
                requirement,
                Some(actual_stage),
                Some(&descriptor),
                required_effects,
                Err(&error),
            );
            return Err(error);
        }
        if let Some(mismatch) =
            missing_effects(required_effects, descriptor.effect_scope().iter().cloned())
        {
            let error = CapabilityError::effect_scope_mismatch(
                capability,
                requirement,
                actual_stage,
                Some(descriptor.clone()),
                required_effects.to_vec(),
                mismatch,
            );
            self.record_capability_check(
                capability,
                requirement,
                Some(actual_stage),
                Some(&descriptor),
                required_effects,
                Err(&error),
            );
            return Err(error);
        }
        self.record_capability_check(
            capability,
            requirement,
            Some(actual_stage),
            Some(&descriptor),
            required_effects,
            Ok(()),
        );
        self.update_last_verified_at(capability);
        Ok(handle)
    }

    pub fn verify_capability_stage(
        &self,
        capability: &str,
        requirement: StageRequirement,
        _required_effects: &[String],
    ) -> Result<StageId, CapabilityError> {
        self.verify_capability(capability, requirement, _required_effects)
            .map(|handle| handle.descriptor().stage())
    }

    /// Conductor 契約全体を検証する。
    pub fn verify_conductor_contract(
        &self,
        contract: ConductorCapabilityContract,
    ) -> Result<(), CapabilityError> {
        let manifest_bundle = if let Some(path) = contract.manifest_path.as_ref() {
            let manifest_caps = ManifestCapabilities::load(path).map_err(|source| {
                CapabilityError::manifest_load_failure(Some(path.clone()), source)
            })?;
            Some((path.clone(), manifest_caps))
        } else {
            None
        };

        for requirement in contract.requirements {
            let handle = self.verify_capability(
                &requirement.id,
                requirement.stage,
                &requirement.declared_effects,
            )?;
            if let Some((manifest_path, manifest_caps)) = &manifest_bundle {
                self.ensure_manifest_alignment(
                    &requirement,
                    handle.descriptor(),
                    manifest_caps,
                    Some(manifest_path),
                )?;
            }
        }
        Ok(())
    }

    #[allow(dead_code)]
    fn descriptor_for(&self, capability: &str) -> Option<CapabilityDescriptor> {
        let entries = self.entries.read().unwrap();
        entries
            .entries
            .get(capability)
            .map(|entry| entry.descriptor.clone())
    }

    fn record_capability_check(
        &self,
        capability: &str,
        requirement: StageRequirement,
        actual_stage: Option<StageId>,
        descriptor: Option<&CapabilityDescriptor>,
        required_effects: &[String],
        outcome: Result<(), &CapabilityError>,
    ) {
        let mut metadata = JsonMap::new();
        metadata.insert(
            "schema.version".into(),
            Value::String(CAPABILITY_SCHEMA_VERSION.into()),
        );
        metadata.insert(
            "event.kind".into(),
            Value::String(AuditEventKind::CapabilityCheck.as_str().into_owned()),
        );
        metadata.insert(
            "event.domain".into(),
            Value::String("runtime.capability".into()),
        );
        metadata.insert(
            "capability.id".into(),
            Value::String(capability.to_string()),
        );
        metadata.insert(
            "capability.ids".into(),
            Value::Array(vec![Value::String(capability.to_string())]),
        );
        metadata.insert(
            "effect.capability".into(),
            Value::String(capability.to_string()),
        );
        metadata.insert(
            "effect.stage.required".into(),
            Value::String(stage_requirement_label(requirement)),
        );
        metadata.insert(
            "effect.stage.actual".into(),
            Value::String(
                actual_stage
                    .map(|stage| stage.as_str().to_string())
                    .unwrap_or_else(|| "unknown".into()),
            ),
        );
        let required_caps = Value::Array(vec![Value::String(capability.to_string())]);
        metadata.insert("effect.required_capabilities".into(), required_caps.clone());
        metadata.insert(
            "effect.stage.required_capabilities".into(),
            required_caps.clone(),
        );
        metadata.insert(
            "effect.actual_capabilities".into(),
            Value::Array(vec![Value::String(capability.to_string())]),
        );
        metadata.insert(
            "effect.stage.actual_capabilities".into(),
            Value::Array(vec![Value::String(capability.to_string())]),
        );
        if !required_effects.is_empty() {
            metadata.insert(
                "effect.required_effects".into(),
                Value::Array(
                    required_effects
                        .iter()
                        .cloned()
                        .map(Value::String)
                        .collect(),
                ),
            );
        }
        if let Some(desc) = descriptor {
            metadata.insert(
                "effect.capability_descriptor".into(),
                serde_json::to_value(desc).unwrap_or(Value::Null),
            );
            let provider = &desc.metadata().provider;
            metadata.insert(
                "capability.provider".into(),
                Value::String(format_provider(provider)),
            );
            metadata.insert(
                "capability.provider.kind".into(),
                Value::String(provider_kind(provider).into()),
            );
            if let CapabilityProvider::Plugin { package, version } = provider {
                metadata.insert("plugin.package".into(), Value::String(package.clone()));
                if let Some(version) = version {
                    metadata.insert("plugin.version".into(), Value::String(version.clone()));
                }
            }
            if !desc.effect_scope().is_empty() {
                metadata.insert(
                    "effect.actual_effects".into(),
                    Value::Array(
                        desc.effect_scope()
                            .iter()
                            .cloned()
                            .map(Value::String)
                            .collect(),
                    ),
                );
            }
        }
        match outcome {
            Ok(()) => {
                metadata.insert("capability.result".into(), Value::String("success".into()));
            }
            Err(error) => {
                metadata.insert("capability.result".into(), Value::String("error".into()));
                metadata.insert(
                    "capability.error.code".into(),
                    Value::String(error.code().into()),
                );
                metadata.insert(
                    "capability.error.message".into(),
                    Value::String(error.detail().into()),
                );
                if let CapabilityError::EffectScopeMismatch {
                    missing_effects, ..
                } = error
                {
                    if !missing_effects.is_empty() {
                        metadata.insert(
                            "effect.missing_effects".into(),
                            Value::Array(
                                missing_effects.iter().cloned().map(Value::String).collect(),
                            ),
                        );
                    }
                }
                if let CapabilityError::ContractViolation {
                    manifest_path,
                    source_span,
                    ..
                } = error
                {
                    if let Some(path) = manifest_path {
                        metadata.insert(
                            "config.manifest.path".into(),
                            Value::String(path.display().to_string()),
                        );
                    }
                    if let Some(span) = source_span {
                        metadata.insert(
                            "config.manifest.span.start".into(),
                            Value::Number(Number::from(span.start)),
                        );
                        metadata.insert(
                            "config.manifest.span.end".into(),
                            Value::Number(Number::from(span.end)),
                        );
                    }
                }
            }
        }
        let timestamp = OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .unwrap_or_else(|_| "1970-01-01T00:00:00Z".into());
        let envelope =
            AuditEnvelope::from_parts(metadata, None, None, Some(capability.to_string()));
        let event = AuditEvent::new(timestamp, envelope);
        let mut log = self.audit_events.write().unwrap();
        log.push(event);
    }

    /// Core.IO アダプタ向けの Stage 検証ヘルパ。
    pub fn verify_stage_for_io(
        &self,
        capability: &'static str,
        requirement: StageRequirement,
    ) -> Result<StageId, CapabilityError> {
        self.verify_capability_stage(capability, requirement, &[])
    }

    /// Capability 検証イベントの履歴を取得する。
    pub fn capability_checks(&self) -> Vec<AuditEvent> {
        self.audit_events.read().unwrap().clone()
    }

    fn update_last_verified_at(&self, capability: &str) {
        let mut entries = self.entries.write().unwrap();
        if let Some(entry) = entries.entries.get_mut(capability) {
            entry.descriptor.metadata_mut().last_verified_at = Some(now_capability_timestamp());
        }
    }
}

#[derive(Debug, Default)]
struct CapabilityEntries {
    entries: HashMap<CapabilityId, CapabilityEntry>,
    ordered_keys: Vec<CapabilityId>,
}

#[derive(Debug, Clone)]
struct CapabilityEntry {
    descriptor: CapabilityDescriptor,
    handle: CapabilityHandle,
}

/// CapabilityDescriptor の列挙結果。
#[derive(Debug, Clone, Default)]
pub struct CapabilityDescriptorList {
    descriptors: Vec<CapabilityDescriptor>,
}

impl CapabilityDescriptorList {
    pub fn new(descriptors: Vec<CapabilityDescriptor>) -> Self {
        Self { descriptors }
    }

    pub fn is_empty(&self) -> bool {
        self.descriptors.is_empty()
    }

    pub fn len(&self) -> usize {
        self.descriptors.len()
    }

    pub fn iter(&self) -> std::slice::Iter<'_, CapabilityDescriptor> {
        self.descriptors.iter()
    }
}

impl IntoIterator for CapabilityDescriptorList {
    type Item = CapabilityDescriptor;
    type IntoIter = std::vec::IntoIter<CapabilityDescriptor>;

    fn into_iter(self) -> Self::IntoIter {
        self.descriptors.into_iter()
    }
}

impl<'a> IntoIterator for &'a CapabilityDescriptorList {
    type Item = &'a CapabilityDescriptor;
    type IntoIter = std::slice::Iter<'a, CapabilityDescriptor>;

    fn into_iter(self) -> Self::IntoIter {
        self.descriptors.iter()
    }
}

impl Deref for CapabilityDescriptorList {
    type Target = [CapabilityDescriptor];

    fn deref(&self) -> &Self::Target {
        &self.descriptors
    }
}

impl DerefMut for CapabilityDescriptorList {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.descriptors
    }
}

fn builtin_capabilities() -> Vec<CapabilityHandle> {
    vec![
        io_capability(
            "io.fs.read",
            StageId::Stable,
            &["io", "fs.read"],
            vec![IoAdapterKind::FileSystem],
            vec![IoOperationKind::Read],
            false,
        ),
        io_capability(
            "io.fs.write",
            StageId::Stable,
            &["io", "fs.write", "mem"],
            vec![IoAdapterKind::FileSystem],
            vec![IoOperationKind::Write],
            false,
        ),
        io_capability(
            "fs.permissions.read",
            StageId::Stable,
            &["io", "security"],
            vec![IoAdapterKind::FileSystem],
            vec![IoOperationKind::Metadata],
            false,
        ),
        io_capability(
            "fs.permissions.modify",
            StageId::Stable,
            &["io", "security"],
            vec![IoAdapterKind::FileSystem],
            vec![IoOperationKind::Metadata],
            false,
        ),
        io_capability(
            "fs.symlink.query",
            StageId::Stable,
            &["io", "fs.symlink"],
            vec![IoAdapterKind::FileSystem],
            vec![IoOperationKind::Symlink],
            false,
        ),
        io_capability(
            "fs.symlink.modify",
            StageId::Stable,
            &["io", "fs.symlink", "security"],
            vec![IoAdapterKind::FileSystem],
            vec![IoOperationKind::Symlink],
            false,
        ),
        io_capability(
            "fs.watcher.native",
            StageId::Stable,
            &["io", "watcher"],
            vec![IoAdapterKind::Watcher],
            vec![IoOperationKind::Watcher],
            true,
        ),
        io_capability(
            "fs.watcher.recursive",
            StageId::Stable,
            &["io", "watcher"],
            vec![IoAdapterKind::Watcher],
            vec![IoOperationKind::Watcher],
            true,
        ),
        io_capability(
            "watcher.resource_limits",
            StageId::Stable,
            &["io", "watcher"],
            vec![IoAdapterKind::Watcher],
            vec![IoOperationKind::Watcher],
            true,
        ),
        memory_capability("memory.buffered_io", StageId::Stable, &["mem"]),
        security_capability("security.fs.policy", StageId::Stable, &["security"]),
        process_capability(
            "core.process",
            StageId::Experimental,
            &[
                "process",
                "thread",
                "io.blocking",
                "signal",
                "hardware",
                "security",
            ],
            ProcessCapabilityMetadata::default(),
        ),
        signal_capability(
            "core.signal",
            StageId::Experimental,
            &["signal", "process", "unsafe", "audit", "security", "io.blocking"],
            SignalCapabilityMetadata::default(),
        ),
        system_capability(
            "core.system",
            StageId::Experimental,
            &["syscall", "unsafe", "audit", "security", "memory"],
            SystemCapabilityMetadata::default(),
        ),
        realtime_capability("core.time.timezone.lookup", StageId::Beta, &["time"]),
        realtime_capability("core.time.timezone.local", StageId::Beta, &["time"]),
        collections_ref_capability(),
        audit_capability("core.collections.audit", StageId::Stable, &["audit", "mem"]),
        metrics_capability("metrics.emit", StageId::Stable, &["audit"]),
        native_capability(
            "native.inline_asm",
            StageId::Experimental,
            &["native", "audit", "unsafe"],
        ),
        native_capability(
            "native.llvm_ir",
            StageId::Experimental,
            &["native", "audit", "unsafe"],
        ),
    ]
}

fn now_capability_timestamp() -> CapabilityTimestamp {
    let now = OffsetDateTime::now_utc();
    CapabilityTimestamp::new(now.unix_timestamp(), now.nanosecond() as i32)
}

fn descriptor(id: &'static str, stage: StageId, effects: &[&str]) -> CapabilityDescriptor {
    CapabilityDescriptor::new(id, stage, effects.iter().copied(), CapabilityProvider::Core)
}

fn io_capability(
    id: &'static str,
    stage: StageId,
    effects: &[&str],
    adapters: Vec<IoAdapterKind>,
    operations: Vec<IoOperationKind>,
    supports_async: bool,
) -> CapabilityHandle {
    CapabilityHandle::Io(IoCapability::new(
        descriptor(id, stage, effects),
        IoCapabilityMetadata {
            adapters,
            operations,
            supports_async,
        },
    ))
}

fn memory_capability(id: &'static str, stage: StageId, effects: &[&str]) -> CapabilityHandle {
    CapabilityHandle::Memory(MemoryCapability::new(
        descriptor(id, stage, effects),
        MemoryCapabilityMetadata::default(),
    ))
}

fn security_capability(id: &'static str, stage: StageId, effects: &[&str]) -> CapabilityHandle {
    CapabilityHandle::Security(SecurityCapability::new(
        descriptor(id, stage, effects),
        SecurityCapabilityMetadata {
            policies: vec![
                SecurityPolicyKind::FsSandbox,
                SecurityPolicyKind::ManifestContract,
            ],
            enforces_path_sandbox: true,
            tracks_manifest: true,
        },
    ))
}

fn process_capability(
    id: &'static str,
    stage: StageId,
    effects: &[&str],
    metadata: ProcessCapabilityMetadata,
) -> CapabilityHandle {
    CapabilityHandle::Process(ProcessCapability::new(
        descriptor(id, stage, effects),
        metadata,
    ))
}

fn signal_capability(
    id: &'static str,
    stage: StageId,
    effects: &[&str],
    metadata: SignalCapabilityMetadata,
) -> CapabilityHandle {
    CapabilityHandle::Signal(SignalCapability::new(
        descriptor(id, stage, effects),
        metadata,
    ))
}

fn system_capability(
    id: &'static str,
    stage: StageId,
    effects: &[&str],
    metadata: SystemCapabilityMetadata,
) -> CapabilityHandle {
    CapabilityHandle::System(SystemCapability::new(
        descriptor(id, stage, effects),
        metadata,
    ))
}

fn realtime_capability(id: &'static str, stage: StageId, effects: &[&str]) -> CapabilityHandle {
    CapabilityHandle::Realtime(RealtimeCapability::new(
        descriptor(id, stage, effects),
        RealtimeCapabilityMetadata {
            latency_budget_ns: Some(1_000_000),
            supports_deadlines: true,
            clock_source: RealtimeClockSource::Monotonic,
        },
    ))
}

fn audit_capability(id: &'static str, stage: StageId, effects: &[&str]) -> CapabilityHandle {
    CapabilityHandle::Audit(AuditCapability::new(
        descriptor(id, stage, effects),
        AuditCapabilityMetadata::default(),
    ))
}

fn metrics_capability(id: &'static str, stage: StageId, effects: &[&str]) -> CapabilityHandle {
    CapabilityHandle::Metrics(MetricsCapability::new(
        descriptor(id, stage, effects),
        MetricsCapabilityMetadata {
            exporters: vec![
                MetricsExporterKind::Json,
                MetricsExporterKind::Prometheus,
                MetricsExporterKind::Otel,
            ],
            supports_histogram: true,
            supports_sampling: true,
        },
    ))
}

fn native_capability(id: &'static str, stage: StageId, effects: &[&str]) -> CapabilityHandle {
    CapabilityHandle::Native(NativeCapability::new(
        descriptor(id, stage, effects),
        NativeCapabilityMetadata::default(),
    ))
}

fn collections_ref_capability() -> CapabilityHandle {
    CapabilityHandle::Collections(CollectionsCapability::new(
        descriptor(
            "core.collections.ref",
            StageId::Stable,
            &["mem", "mut", "rc"],
        ),
        CollectionsCapabilityMetadata {
            collector_effects: vec!["collector.effect.rc".into(), "collector.effect.mut".into()],
            tracks_mutation: true,
            tracks_reference_count: true,
        },
    ))
}

fn provider_kind(provider: &CapabilityProvider) -> &'static str {
    match provider {
        CapabilityProvider::Core => "core",
        CapabilityProvider::Plugin { .. } => "plugin",
        CapabilityProvider::ExternalBridge { .. } => "bridge",
        CapabilityProvider::RuntimeComponent { .. } => "runtime",
    }
}

fn format_provider(provider: &CapabilityProvider) -> String {
    match provider {
        CapabilityProvider::Core => "core".to_string(),
        CapabilityProvider::Plugin { package, version } => match version {
            Some(version) => format!("plugin:{package}@{version}"),
            None => format!("plugin:{package}"),
        },
        CapabilityProvider::ExternalBridge { name, version } => match version {
            Some(version) => format!("bridge:{name}@{version}"),
            None => format!("bridge:{name}"),
        },
        CapabilityProvider::RuntimeComponent { name } => format!("runtime:{name}"),
    }
}

/// Capability 検証に失敗した場合のエラー。
#[derive(Debug, Clone, Error)]
pub enum CapabilityError {
    #[error("{message}")]
    AlreadyRegistered {
        capability_id: CapabilityId,
        message: String,
    },
    #[error("{message}")]
    NotRegistered {
        capability_id: CapabilityId,
        message: String,
    },
    #[error("{message}")]
    StageViolation {
        capability_id: CapabilityId,
        required: StageRequirement,
        actual: StageId,
        descriptor: Option<CapabilityDescriptor>,
        message: String,
    },
    #[error("{message}")]
    EffectScopeMismatch {
        capability_id: CapabilityId,
        required_stage: StageRequirement,
        actual_stage: StageId,
        descriptor: Option<CapabilityDescriptor>,
        required_effects: Vec<String>,
        missing_effects: Vec<String>,
        message: String,
    },
    #[error("{message}")]
    ContractViolation {
        capability_id: CapabilityId,
        manifest_path: Option<PathBuf>,
        source_span: Option<CapabilityContractSpan>,
        descriptor: Option<CapabilityDescriptor>,
        message: String,
    },
    #[error("{message}")]
    ManifestLoadFailure {
        manifest_path: Option<PathBuf>,
        #[source]
        source: ManifestCapabilityError,
        message: String,
    },
}

impl CapabilityError {
    fn already_registered(capability_id: impl Into<String>) -> Self {
        let capability_id = capability_id.into();
        let message = format!("capability '{capability_id}' is already registered");
        CapabilityError::AlreadyRegistered {
            capability_id,
            message,
        }
    }

    fn not_registered(capability_id: impl Into<String>) -> Self {
        let capability_id = capability_id.into();
        let message = format!("capability '{capability_id}' is not registered");
        CapabilityError::NotRegistered {
            capability_id,
            message,
        }
    }

    pub fn stage_violation(
        capability_id: impl Into<String>,
        required: StageRequirement,
        actual: StageId,
        descriptor: Option<CapabilityDescriptor>,
    ) -> Self {
        let capability_id = capability_id.into();
        let message = format!(
            "capability '{capability_id}' requires {} but runtime is {}",
            requirement_description(required),
            actual.as_str()
        );
        CapabilityError::StageViolation {
            capability_id,
            required,
            actual,
            descriptor,
            message,
        }
    }

    pub fn effect_scope_mismatch(
        capability_id: impl Into<String>,
        required_stage: StageRequirement,
        actual_stage: StageId,
        descriptor: Option<CapabilityDescriptor>,
        required_effects: Vec<String>,
        missing_effects: Vec<String>,
    ) -> Self {
        let capability_id = capability_id.into();
        let message = format!(
            "capability '{capability_id}' is missing required effects: {}",
            missing_effects.join(", ")
        );
        CapabilityError::EffectScopeMismatch {
            capability_id,
            required_stage,
            actual_stage,
            descriptor,
            required_effects,
            missing_effects,
            message,
        }
    }

    fn contract_violation(
        capability_id: impl Into<String>,
        manifest_path: Option<PathBuf>,
        source_span: Option<CapabilityContractSpan>,
        descriptor: Option<CapabilityDescriptor>,
        message: impl Into<String>,
    ) -> Self {
        CapabilityError::ContractViolation {
            capability_id: capability_id.into(),
            manifest_path,
            source_span,
            descriptor,
            message: message.into(),
        }
    }

    fn manifest_load_failure(
        manifest_path: Option<PathBuf>,
        source: ManifestCapabilityError,
    ) -> Self {
        let message = format!("unable to load manifest capabilities: {source}");
        CapabilityError::ManifestLoadFailure {
            manifest_path,
            source,
            message,
        }
    }

    pub fn code(&self) -> &'static str {
        match self {
            CapabilityError::AlreadyRegistered { .. } => "runtime.capability.already_registered",
            CapabilityError::NotRegistered { .. } => "runtime.capability.unknown",
            CapabilityError::StageViolation { .. } => "capability.stage.mismatch",
            CapabilityError::EffectScopeMismatch { .. } => "capability.effect_scope.mismatch",
            CapabilityError::ContractViolation { .. } => "config.manifest.capability_contract",
            CapabilityError::ManifestLoadFailure { .. } => "config.manifest.capability_contract",
        }
    }

    pub fn detail(&self) -> &str {
        match self {
            CapabilityError::AlreadyRegistered { message, .. } => message,
            CapabilityError::NotRegistered { message, .. } => message,
            CapabilityError::StageViolation { message, .. } => message,
            CapabilityError::EffectScopeMismatch { message, .. } => message,
            CapabilityError::ContractViolation { message, .. } => message,
            CapabilityError::ManifestLoadFailure { message, .. } => message,
        }
    }

    pub fn actual_stage(&self) -> Option<StageId> {
        match self {
            CapabilityError::StageViolation { actual, .. } => Some(*actual),
            CapabilityError::EffectScopeMismatch { actual_stage, .. } => Some(*actual_stage),
            _ => None,
        }
    }

    pub fn descriptor(&self) -> Option<&CapabilityDescriptor> {
        match self {
            // 3-6 Core Diagnostics の `effects.contract.stage_mismatch` で Capability 情報を転写する。
            CapabilityError::StageViolation { descriptor, .. } => descriptor.as_ref(),
            CapabilityError::EffectScopeMismatch { descriptor, .. } => descriptor.as_ref(),
            CapabilityError::ContractViolation { descriptor, .. } => descriptor.as_ref(),
            _ => None,
        }
    }

    pub fn missing_effects(&self) -> Option<&[String]> {
        match self {
            CapabilityError::EffectScopeMismatch {
                missing_effects, ..
            } => Some(missing_effects.as_slice()),
            _ => None,
        }
    }

    pub fn required_effects(&self) -> Option<&[String]> {
        match self {
            CapabilityError::EffectScopeMismatch {
                required_effects, ..
            } => Some(required_effects.as_slice()),
            _ => None,
        }
    }
}

fn requirement_description(requirement: StageRequirement) -> String {
    match requirement {
        StageRequirement::Exact(stage) => format!("exact {}", stage.as_str()),
        StageRequirement::AtLeast(stage) => format!("at least {}", stage.as_str()),
    }
}

fn stage_requirement_label(requirement: StageRequirement) -> String {
    match requirement {
        StageRequirement::Exact(stage) => stage.as_str().into(),
        StageRequirement::AtLeast(stage) => format!("at_least {}", stage.as_str()),
    }
}

fn missing_effects<I>(required: &[String], actual_scope: I) -> Option<Vec<String>>
where
    I: IntoIterator<Item = EffectTag>,
{
    if required.is_empty() {
        return None;
    }
    let actual: HashSet<_> = actual_scope.into_iter().collect();
    let missing: Vec<String> = required
        .iter()
        .filter(|effect| !actual.contains(*effect))
        .cloned()
        .collect();
    if missing.is_empty() {
        None
    } else {
        Some(missing)
    }
}

pub fn reset_for_tests() {
    let _test_guard = REGISTRY_TEST_LOCK.write().unwrap();
    reset_for_tests_inner();
}

fn reset_for_tests_inner() {
    if let Some(instance) = REGISTRY.write().unwrap().take() {
        unsafe {
            #[cfg(test)]
            {
                LEAKED_FOR_TESTS.lock().unwrap().push(Box::from_raw(
                    instance as *const CapabilityRegistry as *mut CapabilityRegistry,
                ));
            }
            #[cfg(not(test))]
            {
                drop(Box::from_raw(
                    instance as *const CapabilityRegistry as *mut CapabilityRegistry,
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_returns_same_instance() {
        let _test_guard = REGISTRY_TEST_LOCK.read().unwrap();
        let first = CapabilityRegistry::registry() as *const CapabilityRegistry;
        let second = CapabilityRegistry::registry() as *const CapabilityRegistry;
        assert_eq!(first, second);
    }

    #[test]
    fn reset_for_tests_clears_instance() {
        let first = CapabilityRegistry::registry() as *const CapabilityRegistry;
        {
            let _test_guard = REGISTRY_TEST_LOCK.write().unwrap();
            reset_for_tests_inner();
        }
        let second = CapabilityRegistry::registry() as *const CapabilityRegistry;
        assert_ne!(first, second);
    }
}
