use reml_frontend::pipeline::{AuditEmitter, PipelineDescriptor, PipelineFailure, PipelineOutcome};
use serde_json::Value;
use std::path::Path;
use uuid::Uuid;

fn descriptor() -> PipelineDescriptor {
    PipelineDescriptor::new(
        Path::new("examples/demo.reml"),
        Uuid::nil(),
        "Check",
        "Reporting",
        "reml_frontend",
        "reml_frontend demo.reml",
        "3.0.0-alpha",
    )
}

#[test]
fn pipeline_audit_emits_expected_events() {
    let mut emitter = AuditEmitter::new(Vec::<u8>::new(), true);
    let desc = descriptor();
    emitter.pipeline_started(&desc, None).expect("start event");
    let outcome = PipelineOutcome::success(1, 0, "success");
    emitter
        .pipeline_completed(&desc, &outcome, None)
        .expect("completion event");
    let failure = PipelineFailure::new("cli.pipeline.failure", "boom", "error");
    emitter
        .pipeline_failed(&desc, &failure, None)
        .expect("failure event");
    let output = String::from_utf8(emitter.into_inner().unwrap()).expect("utf8");
    let lines: Vec<_> = output.lines().collect();
    assert_eq!(lines.len(), 3);

    let started: Value = serde_json::from_str(lines[0]).expect("json");
    assert_eq!(
        started["envelope"]["metadata"]["event.kind"],
        "pipeline_started"
    );

    let completed: Value = serde_json::from_str(lines[1]).expect("json");
    assert_eq!(
        completed["envelope"]["metadata"]["pipeline.outcome"],
        "success"
    );
    assert_eq!(completed["envelope"]["metadata"]["pipeline.count"], 1);

    let failed: Value = serde_json::from_str(lines[2]).expect("json");
    assert_eq!(
        failed["envelope"]["metadata"]["error.code"],
        "cli.pipeline.failure"
    );
}
