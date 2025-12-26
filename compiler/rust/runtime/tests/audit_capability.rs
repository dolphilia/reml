use once_cell::sync::Lazy;
use reml_runtime::{
    audit::AuditEvent,
    capability::registry::{self, CapabilityRegistry},
    stage::{StageId, StageRequirement},
};
use serde_json::Value;
use std::{fs, path::PathBuf, sync::Mutex};

static CAPABILITY_TEST_GUARD: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

fn setup_registry() -> &'static CapabilityRegistry {
    registry::reset_for_tests();
    CapabilityRegistry::registry()
}

#[test]
fn capability_check_success_event_contains_required_metadata() {
    let _guard = CAPABILITY_TEST_GUARD.lock().unwrap();
    let registry = setup_registry();
    let required_effects = vec!["io".to_string()];
    registry
        .verify_capability(
            "io.fs.read",
            StageRequirement::AtLeast(StageId::Stable),
            &required_effects,
        )
        .expect("builtin capability should satisfy stable stage");
    let event = latest_capability_event(registry);
    assert_eq!(event.event_kind(), Some("capability_check"));
    event.validate().expect("metadata must satisfy schema");
    let metadata = &event.envelope.metadata;
    assert_eq!(
        metadata.get("capability.result").and_then(Value::as_str),
        Some("success")
    );
    assert_eq!(
        metadata
            .get("effect.required_effects")
            .and_then(Value::as_array)
            .map(|values| values.first().and_then(Value::as_str)),
        Some(Some("io"))
    );
    assert_matches_golden(&event, "capability_check_success.jsonl");
}

#[test]
fn capability_check_stage_violation_is_recorded() {
    let _guard = CAPABILITY_TEST_GUARD.lock().unwrap();
    let registry = setup_registry();
    let error = registry
        .verify_capability(
            "io.fs.read",
            StageRequirement::Exact(StageId::Experimental),
            &[],
        )
        .expect_err("stage mismatch must fail");
    assert_eq!(error.code(), "capability.stage.mismatch");
    let event = latest_capability_event(registry);
    assert_eq!(event.event_kind(), Some("capability_check"));
    event.validate().expect("metadata must satisfy schema");
    let metadata = &event.envelope.metadata;
    assert_eq!(
        metadata.get("capability.result").and_then(Value::as_str),
        Some("error")
    );
    assert_eq!(
        metadata
            .get("capability.error.code")
            .and_then(Value::as_str),
        Some("capability.stage.mismatch")
    );
    assert_matches_golden(&event, "capability_check_stage_violation.jsonl");
}

fn latest_capability_event(registry: &CapabilityRegistry) -> AuditEvent {
    let mut events = registry.capability_checks();
    events
        .pop()
        .expect("a capability check event must be recorded")
}

fn assert_matches_golden(event: &AuditEvent, golden_name: &str) {
    let mut normalized = event.clone();
    normalized.timestamp = "1970-01-01T00:00:00Z".into();
    let json = serde_json::to_string(&normalized).expect("serialize capability audit log");
    let path = golden_path(golden_name);
    let expected = fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("missing golden {:?}: {err}", path));
    let expected = expected.trim_end_matches(|c| c == '\n' || c == '\r');
    assert_eq!(
        json, expected,
        "capability audit event differed from golden {:?}",
        path
    );
}

fn golden_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden/audit")
        .join(name)
}
