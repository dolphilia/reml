use crate::{
    audit::{AuditContext, AuditError},
    capability_metadata::{CapabilityDescriptor, StageRequirement},
    record_bridge_with_metadata, BridgeAuditMetadata, CapabilityError, CapabilityRegistry,
    Ownership,
};
use serde_json::{json, Map, Value};

/// FFI 契約の違反種別。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContractViolation {
    /// リンクすべきシンボルが特定できない。
    SymbolMissing,
    /// 所有権の指定が矛盾している。
    OwnershipMismatch {
        actual: Ownership,
        expected: Ownership,
    },
    /// 期待される ABI と実際の ABI が一致しない。
    UnsupportedAbi { actual: String, expected: String },
}

impl ContractViolation {
    fn event_name(&self) -> &'static str {
        match self {
            ContractViolation::SymbolMissing => "ffi.contract.symbol_missing",
            ContractViolation::OwnershipMismatch { .. } => "ffi.contract.ownership_mismatch",
            ContractViolation::UnsupportedAbi { .. } => "ffi.contract.unsupported_abi",
        }
    }

    fn payload(&self, metadata: &BridgeAuditMetadata<'_>) -> Value {
        match self {
            ContractViolation::SymbolMissing => json!({
                "symbol": metadata.symbol,
                "link_name": metadata.link_name,
                "extern_symbol": metadata.extern_symbol,
                "extern_name": metadata.extern_name,
                "target": metadata.target,
                "status": metadata.status.as_str(),
            }),
            ContractViolation::OwnershipMismatch { actual, expected } => json!({
                "ownership.actual": actual.as_str(),
                "ownership.expected": expected.as_str(),
                "bridge.ownership": metadata.ownership.as_str(),
                "bridge.return.ownership": metadata.return_info.ownership.as_str(),
            }),
            ContractViolation::UnsupportedAbi { actual, expected } => json!({
                "target": metadata.target,
                "metadata.abi": metadata.abi,
                "metadata.expected_abi": metadata.expected_abi,
                "abi.actual": actual,
                "abi.expected": expected,
            }),
        }
    }
}

/// `BridgeAuditMetadata` を検証し、契約違反があれば `ContractViolation` を返す。
pub fn check_contract(metadata: &BridgeAuditMetadata<'_>) -> Option<ContractViolation> {
    let symbol_missing = metadata.link_name.trim().is_empty()
        && metadata.extern_symbol.trim().is_empty()
        && metadata.extern_name.trim().is_empty();
    if symbol_missing {
        return Some(ContractViolation::SymbolMissing);
    }

    if metadata.ownership != metadata.return_info.ownership {
        return Some(ContractViolation::OwnershipMismatch {
            actual: metadata.ownership,
            expected: metadata.return_info.ownership,
        });
    }

    let expected_abi = metadata.expected_abi.trim();
    if expected_abi.is_empty() || expected_abi != metadata.abi {
        return Some(ContractViolation::UnsupportedAbi {
            actual: metadata.abi.to_string(),
            expected: metadata.expected_abi.to_string(),
        });
    }

    None
}

/// 診断イベントを記録する。
pub fn emit_contract_violation(
    ctx: &AuditContext,
    metadata: &BridgeAuditMetadata<'_>,
    violation: ContractViolation,
) -> Result<(), AuditError> {
    let event = violation.event_name();
    let payload = violation.payload(metadata);
    record_bridge_with_metadata(ctx, event, metadata, payload)
}

/// Stage/Effect 逸脱のときに `effects.contract.stage_mismatch` 診断を追記する。
pub fn maybe_log_stage_mismatch(
    ctx: &AuditContext,
    metadata: &BridgeAuditMetadata<'_>,
    registry: &CapabilityRegistry,
    capability_id: &str,
    requirement: StageRequirement,
    required_effects: &[String],
    violation: &CapabilityError,
) -> Result<(), AuditError> {
    match violation {
        CapabilityError::StageViolation { .. } | CapabilityError::EffectViolation { .. } => {
            let key = capability_id.to_string();
            if let Some(descriptor) = registry.describe(&key) {
                let payload =
                    build_stage_payload(descriptor, requirement, required_effects, violation);
                record_bridge_with_metadata(
                    ctx,
                    "effects.contract.stage_mismatch",
                    metadata,
                    payload,
                )
            } else {
                Ok(())
            }
        }
        _ => Ok(()),
    }
}

fn build_stage_payload(
    descriptor: CapabilityDescriptor,
    requirement: StageRequirement,
    required_effects: &[String],
    violation: &CapabilityError,
) -> Value {
    let mut map = Map::new();
    map.insert(
        "effect.capability".into(),
        Value::String(descriptor.id.clone()),
    );
    map.insert(
        "effect.stage.required".into(),
        Value::String(requirement.to_string()),
    );
    map.insert(
        "effect.stage.actual".into(),
        Value::String(descriptor.stage.to_string()),
    );
    map.insert(
        "effect.stage.required_effects".into(),
        Value::Array(
            required_effects
                .iter()
                .cloned()
                .map(Value::String)
                .collect(),
        ),
    );
    map.insert(
        "effect.scope".into(),
        Value::Array(
            descriptor
                .effect_scope
                .iter()
                .cloned()
                .map(Value::String)
                .collect(),
        ),
    );

    if let CapabilityError::EffectViolation { missing, .. } = violation {
        map.insert(
            "effect.stage.missing_effects".into(),
            Value::Array(missing.iter().cloned().map(Value::String).collect()),
        );
    }

    map.insert("audit.reason".into(), Value::String(violation.to_string()));
    Value::Object(map)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AuditContext, AuditSink, BridgeReturnAuditMetadata, BridgeStatus, Span};

    fn make_metadata(
        link_name: &'static str,
        extern_symbol: &'static str,
        extern_name: &'static str,
        expected_abi: &'static str,
        abi: &'static str,
        ownership: Ownership,
        return_ownership: Ownership,
    ) -> BridgeAuditMetadata<'static> {
        BridgeAuditMetadata {
            status: BridgeStatus::Ok,
            ownership,
            span: Span::new(0, 1),
            target: "test-target",
            arch: "x86_64",
            platform: "linux-x64",
            abi,
            expected_abi,
            symbol: "symbol",
            extern_symbol,
            extern_name,
            link_name,
            return_info: BridgeReturnAuditMetadata::pending(return_ownership),
        }
    }

    #[test]
    fn detects_symbol_missing() {
        let metadata = make_metadata(
            "",
            "",
            "",
            "system_v",
            "system_v",
            Ownership::Borrowed,
            Ownership::Borrowed,
        );
        assert_eq!(
            check_contract(&metadata),
            Some(ContractViolation::SymbolMissing)
        );
    }

    #[test]
    fn detects_ownership_mismatch() {
        let metadata = make_metadata(
            "foo",
            "foo",
            "extern",
            "system_v",
            "system_v",
            Ownership::Borrowed,
            Ownership::Transferred,
        );
        assert_eq!(
            check_contract(&metadata),
            Some(ContractViolation::OwnershipMismatch {
                actual: Ownership::Borrowed,
                expected: Ownership::Transferred,
            })
        );
    }

    #[test]
    fn detects_unsupported_abi() {
        let metadata = make_metadata(
            "foo",
            "foo",
            "extern",
            "msvc",
            "system_v",
            Ownership::Borrowed,
            Ownership::Borrowed,
        );
        assert_eq!(
            check_contract(&metadata),
            Some(ContractViolation::UnsupportedAbi {
                actual: "system_v".into(),
                expected: "msvc".into(),
            })
        );
    }

    #[test]
    fn emit_records_event() {
        let sink = AuditSink::new();
        let ctx = AuditContext::new("ffi", "symbol", sink.clone()).unwrap();
        let metadata = make_metadata(
            "",
            "",
            "",
            "system_v",
            "system_v",
            Ownership::Borrowed,
            Ownership::Borrowed,
        );
        let violation = ContractViolation::SymbolMissing;
        emit_contract_violation(&ctx, &metadata, violation.clone()).unwrap();

        let entries = sink.entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].event, "ffi.contract.symbol_missing");
        assert_eq!(entries[0].metadata["bridge.link_name"], "");
        assert_eq!(entries[0].metadata["bridge.symbol"], "symbol");
    }
}
