use insta::assert_snapshot;
use reml_frontend::output::cli::{
    render_human_output_to_string, CliCommandKind, CliDiagnosticEnvelope, CliExitCode,
    CliPhaseKind, CliSummary,
};
use reml_frontend::parser::ParserDriver;
use serde_json::{json, Map, Value};
use std::fs;
use std::path::PathBuf;
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

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root must resolve")
}

fn assert_parses_without_diagnostics(path: PathBuf) {
    let source =
        fs::read_to_string(&path).unwrap_or_else(|err| panic!("{path:?} の読み込みに失敗: {err}"));
    let result = ParserDriver::parse(&source);
    assert!(
        result.diagnostics.is_empty(),
        "{path:?} で診断が発生しました: {:?}",
        result
            .diagnostics
            .iter()
            .map(|diag| &diag.message)
            .collect::<Vec<_>>()
    );
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

#[test]
fn streaming_examples_parse_without_diagnostics() {
    let root = workspace_root();
    let samples = [
        "examples/docs-examples/spec/2-7-core-parse-streaming/sec_a_1-b.reml",
        "examples/docs-examples/spec/2-7-core-parse-streaming/sec_a_2.reml",
        "examples/docs-examples/spec/2-7-core-parse-streaming/sec_b_1.reml",
        "examples/docs-examples/spec/2-7-core-parse-streaming/sec_b_2.reml",
        "examples/docs-examples/spec/2-7-core-parse-streaming/sec_d.reml",
        "examples/docs-examples/spec/2-7-core-parse-streaming/sec_e.reml",
    ];

    for sample in samples {
        assert_parses_without_diagnostics(root.join(sample));
    }
}
