use std::{
    collections::HashMap,
    sync::{RwLock, RwLockReadGuard},
};

use once_cell::sync::Lazy;
use thiserror::Error;

use super::{
    descriptor::{CapabilityDescriptor, CapabilityId},
    handle::CapabilityHandle,
};
use crate::stage::{StageId, StageRequirement};

static REGISTRY: Lazy<RwLock<Option<&'static CapabilityRegistry>>> =
    Lazy::new(|| RwLock::new(None));

/// Capability を検証するためのレジストリ。
#[derive(Debug)]
pub struct CapabilityRegistry {
    entries: RwLock<CapabilityEntries>,
}

impl CapabilityRegistry {
    /// シングルトンのレジストリを取得する。
    pub fn registry() -> &'static Self {
        if let Some(instance) = Self::try_get_cached(REGISTRY.read().unwrap()) {
            return instance;
        }
        let mut guard = REGISTRY.write().unwrap();
        if let Some(instance) = *guard {
            return instance;
        }
        let leaked: &'static CapabilityRegistry = Box::leak(Box::new(Self::new()));
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
        entries.entries.insert(
            capability_id,
            CapabilityEntry {
                descriptor,
                handle,
            },
        );
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
    pub fn describe_all(&self) -> Vec<CapabilityDescriptor> {
        let entries = self.entries.read().unwrap();
        entries
            .ordered_keys
            .iter()
            .filter_map(|id| entries.entries.get(id))
            .map(|entry| entry.descriptor.clone())
            .collect()
    }

    pub fn verify_capability_stage(
        &self,
        capability: &str,
        requirement: StageRequirement,
        _required_effects: &[String],
    ) -> Result<StageId, CapabilityError> {
        // 将来の 3.2 タスクで effect_scope 検証と未登録 Capability の扱いを拡張する。
        // 現段階では既存挙動との互換性を優先し、登録済みなら Descriptor を使用し、
        // 未登録の場合は Stable 相当として扱う。
        let descriptor = self.descriptor_for(capability);
        let actual = descriptor
            .as_ref()
            .map(|descriptor| descriptor.stage())
            .unwrap_or(StageId::Stable);
        if requirement.matches(actual) {
            Ok(actual)
        } else {
            Err(CapabilityError::stage_violation(
                capability,
                requirement,
                actual,
                descriptor,
            ))
        }
    }

    fn descriptor_for(&self, capability: &str) -> Option<CapabilityDescriptor> {
        let entries = self.entries.read().unwrap();
        entries
            .entries
            .get(capability)
            .map(|entry| entry.descriptor.clone())
    }

    /// Core.IO アダプタ向けの Stage 検証ヘルパ。
    pub fn verify_stage_for_io(
        &self,
        capability: &'static str,
        requirement: StageRequirement,
    ) -> Result<StageId, CapabilityError> {
        self.verify_capability_stage(capability, requirement, &[])
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

    pub fn code(&self) -> &'static str {
        match self {
            CapabilityError::AlreadyRegistered { .. } => "runtime.capability.already_registered",
            CapabilityError::NotRegistered { .. } => "runtime.capability.unknown",
            CapabilityError::StageViolation { .. } => "capability.stage.mismatch",
        }
    }

    pub fn detail(&self) -> &str {
        match self {
            CapabilityError::AlreadyRegistered { message, .. } => message,
            CapabilityError::NotRegistered { message, .. } => message,
            CapabilityError::StageViolation { message, .. } => message,
        }
    }

    pub fn actual_stage(&self) -> Option<StageId> {
        match self {
            CapabilityError::StageViolation { actual, .. } => Some(*actual),
            _ => None,
        }
    }

    pub fn descriptor(&self) -> Option<&CapabilityDescriptor> {
        match self {
            // 3-6 Core Diagnostics の `effects.contract.stage_mismatch` で Capability 情報を転写する。
            CapabilityError::StageViolation { descriptor, .. } => descriptor.as_ref(),
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

pub fn reset_for_tests() {
    if let Some(instance) = REGISTRY.write().unwrap().take() {
        unsafe {
            drop(Box::from_raw(
                instance as *const CapabilityRegistry as *mut CapabilityRegistry,
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_returns_same_instance() {
        let first = CapabilityRegistry::registry() as *const CapabilityRegistry;
        let second = CapabilityRegistry::registry() as *const CapabilityRegistry;
        assert_eq!(first, second);
    }

    #[test]
    fn reset_for_tests_clears_instance() {
        let first = CapabilityRegistry::registry() as *const CapabilityRegistry;
        reset_for_tests();
        let second = CapabilityRegistry::registry() as *const CapabilityRegistry;
        assert_ne!(first, second);
    }
}
