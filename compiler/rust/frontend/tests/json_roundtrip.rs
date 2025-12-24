use reml_frontend::output::cli::{
    render_human_output_to_string, CliCommandKind, CliDiagnosticEnvelope, CliExitCode,
    CliPhaseKind, CliSummary,
};
use serde_json::{json, Map, Value};
use uuid::Uuid;

fn load_fixture_diagnostics() -> Vec<Value> {
    serde_json::from_str(include_str!("fixtures/diagnostics_roundtrip.json"))
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
        dsl_embeddings: Vec::new(),
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
fn cli_envelope_roundtrips_via_serde_json() {
    let envelope = sample_envelope();
    let json_line = serde_json::to_string(&envelope).expect("cli envelope must serialize to json");
    let decoded: CliDiagnosticEnvelope =
        serde_json::from_str(&json_line).expect("cli envelope json must deserialize");
    assert_eq!(decoded, envelope);
}

#[test]
fn render_human_output_matches_fixture_shape() {
    let envelope = sample_envelope();
    let rendered = render_human_output_to_string(&envelope)
        .expect("human output must render without io error");
    assert!(
        rendered.contains("diagnostics=1"),
        "human output must contain summary line"
    );
    assert!(
        rendered.contains("parser.syntax.expected_tokens"),
        "human output must show diagnostic code"
    );
}
