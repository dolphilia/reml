use reml_runtime::audit::AuditEnvelope;
use serde_json;
use std::{fs, path::PathBuf};

#[test]
fn pipeline_started_fixture_is_valid() {
    let envelope = load_envelope("pipeline_started_valid.json");
    envelope.validate().expect("fixture should be valid");
}

#[test]
fn reports_missing_effect_stage_metadata() {
    let envelope = load_envelope("effect_stage_missing.json");
    let error = envelope.validate().expect_err("validation must fail");
    assert_eq!(error.to_string(), load_expected("effect_stage_missing.txt"));
}

#[test]
fn reports_missing_bridge_stage_metadata() {
    let envelope = load_envelope("bridge_reload_missing.json");
    let error = envelope.validate().expect_err("validation must fail");
    assert_eq!(
        error.to_string(),
        load_expected("bridge_reload_missing.txt")
    );
}

fn load_envelope(name: &str) -> AuditEnvelope {
    let path = fixture_path(name);
    let data = fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("failed to read fixture {path:?}: {err}"));
    serde_json::from_str(&data)
        .unwrap_or_else(|err| panic!("invalid AuditEnvelope fixture {path:?}: {err}"))
}

fn load_expected(name: &str) -> String {
    let path = expected_path(name);
    fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("failed to read expected output {path:?}: {err}"))
        .trim()
        .to_string()
}

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/audit")
        .join(name)
}

fn expected_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/expected/audit")
        .join(name)
}
