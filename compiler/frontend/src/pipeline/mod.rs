use crate::diagnostic::{
    effects::StageAuditPayload,
    formatter::{self, AUDIT_POLICY_VERSION},
};
use reml_runtime::{
    audit::{AuditEnvelope, AuditEvent, AuditEventKind},
    config::{ConfigCompatibilitySource, ResolvedConfigCompatibility},
};
use serde_json::{json, Map, Value};
use std::io::{self, ErrorKind, LineWriter, Write};
use std::path::Path;
use uuid::Uuid;

const DEFAULT_CAPABILITY: &str = "core.diagnostics";

/// CLI 実行 1 回分の識別情報を保持する。
pub struct PipelineDescriptor {
    pipeline_id: String,
    pipeline_dsl_id: String,
    pipeline_node: String,
    input_label: String,
    run_id: Uuid,
    command: String,
    phase: String,
    program_name: String,
    cli_command: String,
    schema_version: &'static str,
}

impl PipelineDescriptor {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        input: &Path,
        run_id: Uuid,
        command_label: impl Into<String>,
        phase_label: impl Into<String>,
        program_name: impl Into<String>,
        cli_command: impl Into<String>,
        schema_version: &'static str,
    ) -> Self {
        let (pipeline_id, pipeline_dsl_id, pipeline_node) = pipeline_identifiers(input);
        Self {
            pipeline_id,
            pipeline_dsl_id,
            pipeline_node,
            input_label: input.display().to_string(),
            run_id,
            command: command_label.into(),
            phase: phase_label.into(),
            program_name: program_name.into(),
            cli_command: cli_command.into(),
            schema_version,
        }
    }

    fn base_metadata(
        &self,
        timestamp: &str,
        stage: Option<&StageAuditPayload>,
    ) -> Map<String, Value> {
        let mut metadata = Map::new();
        metadata.insert("schema.version".to_string(), json!(self.schema_version));
        metadata.insert("pipeline.id".to_string(), json!(self.pipeline_id()));
        metadata.insert("pipeline.dsl_id".to_string(), json!(self.pipeline_dsl_id()));
        metadata.insert("pipeline.node".to_string(), json!(self.pipeline_node()));
        metadata.insert("scenario.id".to_string(), json!(self.pipeline_dsl_id()));
        metadata.insert("timestamp".to_string(), json!(timestamp));
        metadata.insert("cli.input".to_string(), json!(self.input_label.clone()));
        metadata.insert("cli.run_id".to_string(), json!(self.run_id.to_string()));
        metadata.insert("cli.command".to_string(), json!(self.command.clone()));
        metadata.insert("cli.phase".to_string(), json!(self.phase.clone()));
        metadata.insert("cli.program".to_string(), json!(self.program_name.clone()));
        metadata.insert(
            "cli.command_line".to_string(),
            json!(self.cli_command.clone()),
        );
        metadata.insert("audit.channel".to_string(), json!("cli"));
        metadata.insert(
            "audit.policy.version".to_string(),
            json!(AUDIT_POLICY_VERSION),
        );
        if let Some(payload) = stage {
            if payload.primary_capability().is_some() {
                payload.apply_audit_metadata(&mut metadata);
            } else {
                let required = payload.required_stage_label().unwrap_or(DEFAULT_CAPABILITY);
                let actual = payload.actual_stage_label().unwrap_or(required);
                metadata.insert("effect.capability".to_string(), json!(DEFAULT_CAPABILITY));
                metadata.insert("effect.stage.required".to_string(), json!(required));
                metadata.insert("effect.stage.actual".to_string(), json!(actual));
            }
        }
        metadata
    }

    pub fn pipeline_id(&self) -> &str {
        &self.pipeline_id
    }

    pub fn pipeline_dsl_id(&self) -> &str {
        &self.pipeline_dsl_id
    }

    pub fn pipeline_node(&self) -> &str {
        &self.pipeline_node
    }
}

/// パイプライン成功時に記録する統計情報。
pub struct PipelineOutcome {
    pub processed_inputs: u64,
    pub diagnostic_count: usize,
    pub outcome_label: String,
    pub exit_status_label: String,
}

impl PipelineOutcome {
    pub fn success(
        processed_inputs: u64,
        diagnostic_count: usize,
        exit_status: impl Into<String>,
    ) -> Self {
        Self {
            processed_inputs,
            diagnostic_count,
            outcome_label: "success".to_string(),
            exit_status_label: exit_status.into(),
        }
    }
}

/// パイプライン失敗時の付帯情報。
pub struct PipelineFailure {
    pub code: String,
    pub message: String,
    pub severity: String,
}

impl PipelineFailure {
    pub fn new(
        code: impl Into<String>,
        message: impl Into<String>,
        severity: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            severity: severity.into(),
        }
    }
}

/// CLI 実行から監査イベントを生成するユーティリティ。
pub struct AuditEmitter<W: Write> {
    writer: Option<W>,
}

impl AuditEmitter<LineWriter<io::Stderr>> {
    pub fn stderr(enabled: bool) -> Self {
        if enabled {
            Self {
                writer: Some(LineWriter::new(io::stderr())),
            }
        } else {
            Self { writer: None }
        }
    }
}

impl<W: Write> AuditEmitter<W> {
    pub fn new(writer: W, enabled: bool) -> Self {
        if enabled {
            Self {
                writer: Some(writer),
            }
        } else {
            Self { writer: None }
        }
    }

    pub fn pipeline_started(
        &mut self,
        descriptor: &PipelineDescriptor,
        stage: Option<&StageAuditPayload>,
    ) -> io::Result<()> {
        let timestamp = formatter::current_timestamp();
        let metadata = descriptor.base_metadata(&timestamp, stage);
        self.emit_event(AuditEventKind::PipelineStarted, metadata, timestamp)
    }

    pub fn pipeline_completed(
        &mut self,
        descriptor: &PipelineDescriptor,
        outcome: &PipelineOutcome,
        stage: Option<&StageAuditPayload>,
    ) -> io::Result<()> {
        let timestamp = formatter::current_timestamp();
        let mut metadata = descriptor.base_metadata(&timestamp, stage);
        metadata.insert(
            "pipeline.outcome".to_string(),
            json!(outcome.outcome_label.clone()),
        );
        metadata.insert(
            "pipeline.count".to_string(),
            json!(outcome.processed_inputs),
        );
        metadata.insert(
            "pipeline.diagnostics".to_string(),
            json!(outcome.diagnostic_count),
        );
        metadata.insert(
            "pipeline.exit_code".to_string(),
            json!(outcome.exit_status_label.clone()),
        );
        self.emit_event(AuditEventKind::PipelineCompleted, metadata, timestamp)
    }

    pub fn pipeline_failed(
        &mut self,
        descriptor: &PipelineDescriptor,
        failure: &PipelineFailure,
        stage: Option<&StageAuditPayload>,
    ) -> io::Result<()> {
        let timestamp = formatter::current_timestamp();
        let mut metadata = descriptor.base_metadata(&timestamp, stage);
        metadata.insert("pipeline.outcome".to_string(), json!("failure"));
        metadata.insert("error.code".to_string(), json!(failure.code.clone()));
        metadata.insert("error.message".to_string(), json!(failure.message.clone()));
        metadata.insert(
            "error.severity".to_string(),
            json!(failure.severity.clone()),
        );
        self.emit_event(AuditEventKind::PipelineFailed, metadata, timestamp)
    }

    pub fn config_compat_changed(
        &mut self,
        descriptor: &PipelineDescriptor,
        resolved: &ResolvedConfigCompatibility,
    ) -> io::Result<()> {
        if resolved.source == ConfigCompatibilitySource::Default {
            return Ok(());
        }
        let timestamp = formatter::current_timestamp();
        let mut metadata = descriptor.base_metadata(&timestamp, None);
        metadata.insert("config.source".to_string(), json!(resolved.source.as_str()));
        metadata.insert("config.format".to_string(), json!(resolved.format.as_str()));
        let profile = resolved
            .profile_label
            .clone()
            .unwrap_or_else(|| resolved.source.as_str().to_string());
        metadata.insert("config.profile".to_string(), json!(profile));
        let compat_json =
            serde_json::to_value(&resolved.compatibility).unwrap_or_else(|_| json!({}));
        metadata.insert("config.compatibility".to_string(), compat_json);
        self.emit_event(AuditEventKind::ConfigCompatChanged, metadata, timestamp)
    }

    pub fn emit_external_event(&mut self, event: &AuditEvent) -> io::Result<()> {
        let writer = match self.writer.as_mut() {
            Some(writer) => writer,
            None => return Ok(()),
        };
        event
            .validate()
            .map_err(|err| io::Error::new(ErrorKind::InvalidData, err))?;
        let payload =
            serde_json::to_vec(event).map_err(|err| io::Error::new(ErrorKind::Other, err))?;
        writer.write_all(&payload)?;
        writer.write_all(b"\n")
    }

    pub fn into_inner(self) -> Option<W> {
        self.writer
    }

    fn emit_event(
        &mut self,
        kind: AuditEventKind,
        mut metadata: Map<String, Value>,
        timestamp: String,
    ) -> io::Result<()> {
        let writer = match self.writer.as_mut() {
            Some(writer) => writer,
            None => return Ok(()),
        };
        let label = kind.as_str().to_string();
        metadata.insert("event.kind".to_string(), json!(label));
        let envelope = AuditEnvelope::from_parts(
            metadata,
            Some(Uuid::new_v4()),
            None,
            Some(DEFAULT_CAPABILITY.to_string()),
        );
        envelope
            .validate()
            .map_err(|err| io::Error::new(ErrorKind::InvalidData, err))?;
        let event = AuditEvent::new(timestamp, envelope);
        let payload =
            serde_json::to_vec(&event).map_err(|err| io::Error::new(ErrorKind::Other, err))?;
        writer.write_all(&payload)?;
        writer.write_all(b"\n")
    }
}

fn pipeline_identifiers(input: &Path) -> (String, String, String) {
    let rendered = input.display().to_string();
    let dsl_id = input
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map(|stem| stem.to_string())
        .unwrap_or_else(|| rendered.clone());
    let node = input
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.to_string())
        .unwrap_or_else(|| "root".to_string());
    (format!("dsl://{rendered}"), dsl_id, node)
}
