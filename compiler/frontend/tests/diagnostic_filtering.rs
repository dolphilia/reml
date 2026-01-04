use reml_frontend::diagnostic::filter::{
    apply_experimental_stage_policy, should_downgrade_experimental,
};
use reml_frontend::diagnostic::{DiagnosticSeverity, FrontendDiagnostic};
use serde_json::{Map, Value};

#[test]
fn severity_is_downgraded_for_unacknowledged_experimental_stage() {
    let cases = [
        ("experimental", false, DiagnosticSeverity::Warning),
        ("experimental", true, DiagnosticSeverity::Error),
        ("beta", false, DiagnosticSeverity::Error),
    ];
    for (label, ack, expected) in cases {
        let mut diag = FrontendDiagnostic::new("case").with_severity(DiagnosticSeverity::Error);
        let mut stage = Map::new();
        stage.insert("required".to_string(), Value::String(label.to_string()));
        let mut effects = Map::new();
        effects.insert("stage".to_string(), Value::Object(stage));
        let mut extensions = Map::new();
        extensions.insert("effects".to_string(), Value::Object(effects));
        apply_experimental_stage_policy(&mut diag, &extensions, ack);
        assert_eq!(diag.severity, Some(expected), "stage={label}, ack={ack}");
    }
}

#[test]
fn flattened_keys_trigger_detection() {
    let mut extensions = Map::new();
    extensions.insert(
        "effect.stage.actual".to_string(),
        Value::String("at_least:experimental".to_string()),
    );
    assert!(should_downgrade_experimental(false, &extensions));
    assert!(!should_downgrade_experimental(true, &extensions));
}
