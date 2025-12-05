use insta::assert_snapshot;
use reml_frontend::output::cli::{
    render_human_output_to_string, CliCommandKind, CliDiagnosticEnvelope, CliExitCode,
    CliPhaseKind, CliSummary,
};
use serde_json::{json, Map, Value};
use uuid::Uuid;

fn load_fixture_diagnostics() -> Vec<Value> {
    serde_json::from_str(include_str!("../fixtures/diagnostics_roundtrip.json"))
        .expect("diagnostic fixture must be valid json")
}

fn sample_summary() -> CliSummary {
    let mut stats = Map::new();
    stats.insert(
        "parser.expected_tokens".to_string(),
        json!({
            "total": 2,
            "notes": 1
        }),
    );
    stats.insert(
        "effects.stage.audit_presence".to_string(),
        json!({
            "required": "beta",
            "actual": "beta"
        }),
    );
    CliSummary {
        inputs: vec!["tests/fixtures/sample.reml".to_string()],
        started_at: "2025-01-01T00:00:00Z".to_string(),
        finished_at: "2025-01-01T00:00:01Z".to_string(),
        artifact: Some("diagnostics.json".to_string()),
        stats,
    }
}

fn sample_envelope() -> CliDiagnosticEnvelope {
    CliDiagnosticEnvelope::new(
        &CliCommandKind::Check,
        &CliPhaseKind::Reporting,
        Uuid::nil(),
        load_fixture_diagnostics(),
        sample_summary(),
        CliExitCode::warning(),
    )
}

#[test]
fn cli_json_output_snapshot() {
    let envelope = sample_envelope();
    let json_line = serde_json::to_string(&envelope).expect("cli envelope must serialize to json");
    assert_snapshot!("cli_json_output", json_line);
}

#[test]
fn cli_human_output_snapshot() {
    let envelope = sample_envelope();
    let human = render_human_output_to_string(&envelope)
        .expect("human output must render without io error");
    assert_snapshot!("cli_human_output", human);
}
