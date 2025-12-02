use reml_frontend::diagnostic::{
    DiagnosticBuilder, DiagnosticBuilderError, DiagnosticDomain, DiagnosticSeverity,
    FrontendDiagnostic,
};
use test_case::test_case;

fn baseline_diagnostic() -> FrontendDiagnostic {
    FrontendDiagnostic::new("parser failure")
        .with_severity(DiagnosticSeverity::Error)
        .with_domain(DiagnosticDomain::Parser)
        .with_code("parser.syntax.expected_tokens")
}

#[derive(Clone, Copy)]
enum MissingField {
    Severity,
    Domain,
    Code,
}

#[test_case(MissingField::Severity; "missing severity")]
#[test_case(MissingField::Domain; "missing domain")]
#[test_case(MissingField::Code; "missing code")]
fn builder_rejects_missing_fields(field: MissingField) {
    let mut diagnostic = baseline_diagnostic();
    match field {
        MissingField::Severity => diagnostic.severity = None,
        MissingField::Domain => diagnostic.domain = None,
        MissingField::Code => {
            diagnostic.code = None;
            diagnostic.codes.clear();
        }
    }

    let mut builder = DiagnosticBuilder::new();
    let err = builder
        .push(diagnostic)
        .expect_err("missing field must error");
    match field {
        MissingField::Severity => assert!(matches!(err, DiagnosticBuilderError::MissingSeverity)),
        MissingField::Domain => assert!(matches!(err, DiagnosticBuilderError::MissingDomain)),
        MissingField::Code => assert!(matches!(err, DiagnosticBuilderError::MissingCode)),
    }
}

#[test]
fn builder_accepts_complete_diagnostic() {
    let mut builder = DiagnosticBuilder::new();
    builder
        .push(baseline_diagnostic())
        .expect("complete diagnostic");
    assert_eq!(builder.into_vec().len(), 1);
}
