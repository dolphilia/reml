//! logos × chumsky フロントエンド PoC。入力ファイルを解析し JSON を出力する。

use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::str::FromStr;

use reml_adapter::target::{self, TargetInference};
use reml_frontend::diagnostic::{
    effects,
    formatter::{self, FormatterContext},
    json as diag_json, DiagnosticDomain, FrontendDiagnostic,
};
use reml_frontend::error::Recoverability;
use reml_frontend::lexer::{lex_source_with_options, IdentifierProfile, LexerOptions};
use reml_frontend::parser::ast::Module;
use reml_frontend::parser::{
    LeftRecursionMode, ParseResult, ParserDriver, ParserOptions, RunConfig, StreamOutcome,
    StreamingRunner,
};
use reml_frontend::semantics::typed;
use reml_frontend::span::Span;
use reml_frontend::streaming::{
    StreamFlowConfig, StreamFlowMetrics, StreamFlowState, StreamingStateConfig,
};
use reml_frontend::typeck::{
    self, Constraint, DualWriteGuards, InstallConfigError, RecoverConfig, RuntimeCapability,
    StageContext, StageTraceStep, StageId, StageRequirement, TypeRowMode, TypecheckConfig,
    TypecheckDriver, TypecheckMetrics, TypecheckReport, TypecheckViolation,
    TypecheckViolationKind, TypedFunctionSummary,
};
use serde::Serialize;

const PARSER_NAMESPACE: &str = "rust.poc";
const PARSER_NAME: &str = "compilation_unit";
const PARSER_ORIGIN: &str = "poc_frontend";
const PARSER_FINGERPRINT: &str = "rust-poc-0001";
const SCHEMA_VERSION: &str = "2.0.0-draft";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = parse_args()?;
    install_typecheck_config(&args.typecheck_config)?;
    let input_path = args.input.clone();
    let source = fs::read_to_string(&input_path)?;
    if let Some(path) = &args.emit_tokens {
        let options = LexerOptions {
            identifier_profile: args.run_config.lex_identifier_profile,
        };
        let lex_output = lex_source_with_options(&source, options);
        write_json_file(path, &lex_output.tokens)?;
    }
    let dualwrite = if let Some(opts) = args.dualwrite.clone() {
        Some(if let Some(root) = opts.root {
            DualWriteGuards::with_root(root, &opts.run_label, &opts.case_label)?
        } else {
            DualWriteGuards::new(&opts.run_label, &opts.case_label)?
        })
    } else {
        None
    };

    trace_log(&args, "parsing", "start");
    let stream_flow_state =
        StreamFlowState::new(args.stream_config.to_flow_config(args.run_config.packrat));
    let mut run_config = args.run_config.to_run_config();
    let packrat_enabled = run_config.packrat;
    run_config = run_config.with_extension("stream", |existing| {
        merge_stream_extension(existing, &args.stream_config, packrat_enabled)
    });
    let mut parser_options = ParserOptions::from_run_config(&run_config);
    parser_options.streaming = args.streaming_state_config();
    parser_options.streaming_enabled = args.stream_config.enabled || run_config.trace;
    parser_options.stream_flow = Some(stream_flow_state.clone());
    let result = if args.stream_config.enabled {
        let runner = StreamingRunner::new(
            source.clone(),
            parser_options.clone(),
            run_config.clone(),
            stream_flow_state.clone(),
        );
        resolve_completed_stream_outcome(runner.run_stream())
    } else {
        ParserDriver::parse_with_options_and_run_config(&source, parser_options, run_config)
    };
    trace_log(&args, "parsing", "finish");
    let typeck_report = result
        .value
        .as_ref()
        .map(|module| TypecheckDriver::infer_module(module, &args.typecheck_config))
        .unwrap_or_else(|| {
            TypecheckDriver::infer_fallback_from_source(&source, &args.typecheck_config)
        });
    let artifacts = TypeckArtifacts::new(&input_path, &typeck_report, &args.typecheck_config);
    let parse_result = serde_json::json!({
        "packrat_stats": result.packrat_stats,
        "packrat_snapshot": result.packrat_snapshot,
        "span_trace": result.span_trace,
        "packrat_cache": result.packrat_cache,
        "recovered": result.recovered,
        "farthest_error_offset": result.farthest_error_offset,
    });

    let flow_metrics = result
        .stream_flow_state
        .as_ref()
        .map(|state| state.metrics().checkpoints_closed)
        .unwrap_or(0);
    let stream_meta = serde_json::json!({
        "packrat": result.stream_metrics.packrat,
        "span_trace": result.stream_metrics.span_trace,
        "flow": {
            "checkpoints_closed": flow_metrics,
        },
        "packrat_enabled": result.run_config.packrat,
    });

    let runconfig_summary = build_runconfig_summary(&result.run_config, &args, &stream_flow_state);
    let runconfig_top_level =
        build_runconfig_top_level(&result.run_config, &args, &stream_flow_state);
    let stage_payload = StageAuditPayload::new(
        &args.typecheck_config.effect_context,
        &args.runtime_capabilities,
    );
    let mut diagnostics_entries = build_parser_diagnostics(
        &result.diagnostics,
        &args,
        &input_path,
        &source,
        &result.run_config,
        &runconfig_summary,
        &stream_flow_state,
        &stage_payload,
    );
    let mut type_diagnostics = build_type_diagnostics(
        &typeck_report,
        &args,
        &input_path,
        &source,
        &result.run_config,
        &runconfig_summary,
        &stream_flow_state,
        &stage_payload,
    );
    diagnostics_entries.append(&mut type_diagnostics);
    let diagnostics_json = Value::Array(diagnostics_entries.clone());
    let diag_document = json!({
        "input": input_path,
        "diagnostics": diagnostics_json.clone(),
        "run_config": runconfig_top_level.clone(),
        "parse_result": parse_result.clone(),
        "stream_meta": stream_meta.clone(),
    });

    println!("{}", serde_json::to_string_pretty(&diag_document)?);

    if let Some(path) = args.parse_debug_output {
        let parse_debug = json!({
            "parser_run_config": runconfig_top_level.clone(),
            "input": input_path,
            "diagnostics": diagnostics_json.clone(),
            "parse_result": parse_result,
            "stream_meta": stream_meta,
        });
        fs::write(path, serde_json::to_string_pretty(&parse_debug)?)?;
    }

    if let Some(path) = &args.emit_ast {
        if let Some(ast) = &result.value {
            write_json_file(path, ast)?;
        }
    }

    if let Some(path) = &args.emit_typed_ast {
        write_json_file(path, &artifacts.typed_ast)?;
    }
    if let Some(path) = &args.emit_constraints {
        write_json_file(path, &artifacts.constraints)?;
    }
    if let Some(path) = &args.emit_typeck_debug {
        write_json_file(path, &artifacts.debug)?;
    }
    if let Some(path) = &args.emit_effects_metrics {
        let payload = TypecheckMetricsPayload::from_report(&typeck_report, &stage_payload);
        write_json_file(path, &payload)?;
    }
    if let Some(path) = &args.emit_impl_registry {
        let payload = build_impl_registry_payload(&typeck_report, None, None);
        write_json_file(path, &payload)?;
    }

    if let Some(guards) = dualwrite {
        write_dualwrite_typeck_payload(
            &guards,
            &typeck_report,
            &args.typecheck_config,
            &artifacts,
            &stage_payload,
        )?;
        write_dualwrite_parse_payload(&guards, &result, &runconfig_top_level)?;
    }

    Ok(())
}

fn install_typecheck_config(config: &TypecheckConfig) -> Result<(), InstallConfigError> {
    match typeck::install_config(config.clone()) {
        Ok(()) => Ok(()),
        Err(InstallConfigError::AlreadyInstalled) => Ok(()),
    }
}

fn resolve_completed_stream_outcome(outcome: StreamOutcome) -> ParseResult<Module> {
    match outcome {
        StreamOutcome::Completed { result, .. } => result,
        StreamOutcome::Pending { continuation, .. } => {
            let runner = StreamingRunner::from_continuation(continuation);
            resolve_completed_stream_outcome(runner.run_stream())
        }
    }
}

struct CliArgs {
    program_name: String,
    raw_args: Vec<String>,
    input: PathBuf,
    parse_debug_output: Option<PathBuf>,
    typecheck_config: TypecheckConfig,
    dualwrite: Option<DualwriteCliOpts>,
    emit_typed_ast: Option<PathBuf>,
    emit_ast: Option<PathBuf>,
    emit_constraints: Option<PathBuf>,
    emit_typeck_debug: Option<PathBuf>,
    emit_effects_metrics: Option<PathBuf>,
    emit_impl_registry: Option<PathBuf>,
    emit_tokens: Option<PathBuf>,
    #[allow(dead_code)]
    emit_effects: bool,
    #[allow(dead_code)]
    emit_diagnostics: bool,
    #[allow(dead_code)]
    emit_audit: bool,
    #[allow(dead_code)]
    show_stage_context: bool,
    #[allow(dead_code)]
    diagnostics_stream: bool,
    target_cfg_extension: Value,
    run_config: RunSettings,
    stream_config: StreamSettings,
    runtime_capabilities: Vec<RuntimeCapability>,
    config_path: Option<PathBuf>,
}

#[derive(Clone)]
struct DualwriteCliOpts {
    run_label: String,
    case_label: String,
    root: Option<PathBuf>,
}

#[derive(Clone)]
struct RunSettings {
    config: RunConfig,
    experimental_effects: bool,
    lex_identifier_profile: IdentifierProfile,
}

impl Default for RunSettings {
    fn default() -> Self {
        let mut config = RunConfig::default();
        config.trace = false;
        config.legacy_result = true;
        config.left_recursion = LeftRecursionMode::Off;
        Self {
            config,
            experimental_effects: false,
            lex_identifier_profile: IdentifierProfile::Unicode,
        }
    }
}

impl Deref for RunSettings {
    type Target = RunConfig;

    fn deref(&self) -> &Self::Target {
        &self.config
    }
}

impl DerefMut for RunSettings {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.config
    }
}

impl RunSettings {
    fn to_run_config(&self) -> RunConfig {
        let mut config = self.config.clone();
        config = config.with_extension("lex", |existing| self.lex_extension(existing));
        if self.experimental_effects {
            config = config.with_extension("effects", |existing| {
                let mut payload = existing
                    .and_then(|value| value.as_object().cloned())
                    .unwrap_or_default();
                payload.insert("experimental_effects".to_string(), json!(true));
                Value::Object(payload)
            });
        }
        config
    }

    fn lex_extension(&self, existing: Option<&Value>) -> Value {
        let mut payload = existing
            .and_then(|value| value.as_object().cloned())
            .unwrap_or_default();
        payload.insert(
            "identifier_profile".to_string(),
            json!(self.lex_identifier_profile.as_str()),
        );
        payload
            .entry("profile".to_string())
            .or_insert_with(|| json!("strict_json"));
        Value::Object(payload)
    }
}

#[derive(Clone, Default)]
struct StreamSettings {
    enabled: bool,
    resume_hint: Option<String>,
    flow_policy: Option<String>,
    flow_max_lag: Option<u64>,
    demand_min_bytes: Option<u64>,
    demand_preferred_bytes: Option<u64>,
    checkpoint: Option<String>,
    chunk_size: Option<u64>,
}

impl StreamSettings {
    fn to_flow_config(&self, packrat_enabled: bool) -> StreamFlowConfig {
        StreamFlowConfig {
            enabled: self.enabled,
            packrat_enabled,
            resume_hint: self.resume_hint.clone(),
            checkpoint: self.checkpoint.clone(),
            flow_policy: self.flow_policy.clone(),
            flow_max_lag: self.flow_max_lag,
            demand_min_bytes: self.demand_min_bytes,
            demand_preferred_bytes: self.demand_preferred_bytes,
        }
    }
}

impl CliArgs {
    fn streaming_state_config(&self) -> StreamingStateConfig {
        let mut config = StreamingStateConfig::default();
        config.packrat_enabled = self.run_config.packrat;
        config.trace_enabled = self.stream_config.enabled || self.run_config.trace;
        config
    }

    fn cli_command(&self) -> String {
        std::iter::once(self.program_name.clone())
            .chain(self.raw_args.iter().cloned())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

fn apply_workspace_config(
    path: &Path,
    run_config: &mut RunSettings,
    stream_config: &mut StreamSettings,
    runtime_capabilities: &mut Vec<RuntimeCapability>,
    row_mode: &mut Option<TypeRowMode>,
    emit_typeck_debug: &mut Option<PathBuf>,
    trace_overridden: bool,
    merge_overridden: bool,
) -> Result<(), String> {
    let raw = std::fs::read_to_string(path)
        .map_err(|err| format!("{} の読み込みに失敗しました: {err}", path.display()))?;
    let value: Value = serde_json::from_str(&raw)
        .map_err(|err| format!("{} を JSON として解析できません: {err}", path.display()))?;

    if let Some(parser) = value.get("parser") {
        if !trace_overridden {
            if let Some(trace) = parser.get("trace").and_then(|v| v.as_bool()) {
                run_config.trace = trace;
            }
        }
        if !merge_overridden {
            if let Some(merge) = parser.get("merge_warnings").and_then(|v| v.as_bool()) {
                run_config.merge_warnings = merge;
            }
        }
        if let Some(packrat) = parser.get("packrat").and_then(|v| v.as_bool()) {
            run_config.packrat = packrat;
        }
        if let Some(require_eof) = parser.get("require_eof").and_then(|v| v.as_bool()) {
            run_config.require_eof = require_eof;
        }
        if let Some(left_recursion) = parser.get("left_recursion").and_then(|v| v.as_str()) {
            match LeftRecursionMode::from_str(left_recursion) {
                Ok(mode) => run_config.left_recursion = mode,
                Err(_) => eprintln!(
                    "[CONFIG] left_recursion `{}` は未サポートの値なので無視します",
                    left_recursion
                ),
            }
        }
        if let Some(stream) = parser.get("stream").and_then(|v| v.as_object()) {
            if let Some(enabled) = stream.get("enabled").and_then(|v| v.as_bool()) {
                stream_config.enabled = enabled;
            }
            if let Some(resume) = stream.get("resume_hint").and_then(|v| v.as_str()) {
                stream_config.resume_hint = Some(resume.to_string());
            }
            if let Some(checkpoint) = stream.get("checkpoint").and_then(|v| v.as_str()) {
                stream_config.checkpoint = Some(checkpoint.to_string());
            }
            if let Some(min_bytes) = stream.get("demand_min_bytes").and_then(|v| v.as_u64()) {
                stream_config.demand_min_bytes = Some(min_bytes);
            }
            if let Some(pref_bytes) = stream
                .get("demand_preferred_bytes")
                .and_then(|v| v.as_u64())
            {
                stream_config.demand_preferred_bytes = Some(pref_bytes);
            }
            if let Some(chunk_size) = stream.get("chunk_size").and_then(|v| v.as_u64()) {
                stream_config.chunk_size = Some(chunk_size);
            }
            if let Some(policy) = stream.get("flow_policy").and_then(|v| v.as_str()) {
                stream_config.flow_policy = Some(policy.to_string());
            }
            if let Some(max_lag) = stream.get("flow_max_lag").and_then(|v| v.as_u64()) {
                stream_config.flow_max_lag = Some(max_lag);
            }
        }

        if let Some(extensions) = parser.get("extensions").and_then(|v| v.as_object()) {
            if let Some(lex) = extensions.get("lex").and_then(|v| v.as_object()) {
                run_config.config = run_config
                    .config
                    .with_extension("lex", |existing| merge_extension(existing, lex));
                if let Some(profile) = lex.get("identifier_profile").and_then(|v| v.as_str()) {
                    match IdentifierProfile::from_str(profile) {
                        Ok(parsed) => run_config.lex_identifier_profile = parsed,
                        Err(_) => eprintln!(
                            "[CONFIG] lex.identifier_profile `{profile}` は ascii/unicode 以外の値なので無視されました"
                        ),
                    }
                }
            }
        }
    }

    if let Some(effects) = value.get("effects") {
        if let Some(exp) = effects
            .get("experimental_effects")
            .and_then(|v| v.as_bool())
        {
            run_config.experimental_effects = exp;
        }
        if row_mode.is_none() {
            if let Some(mode) = effects.get("type_row_mode").and_then(|v| v.as_str()) {
                if let Ok(parsed) = TypeRowMode::from_str(mode) {
                    *row_mode = Some(parsed);
                }
            }
        }
        if let Some(list) = effects.get("runtime_capabilities") {
            extend_capabilities(runtime_capabilities, list);
        }
    }

    if let Some(list) = value.get("runtime_capabilities") {
        extend_capabilities(runtime_capabilities, list);
    }

    if emit_typeck_debug.is_none() {
        if let Some(debug_path) = value
            .get("typecheck")
            .and_then(|section| {
                section
                    .get("emit_typeck_debug")
                    .or_else(|| section.get("emit_debug_path"))
            })
            .and_then(|v| v.as_str())
        {
            if !debug_path.trim().is_empty() {
                let resolved = if Path::new(debug_path).is_absolute() {
                    PathBuf::from(debug_path)
                } else {
                    path.parent()
                        .unwrap_or_else(|| Path::new("."))
                        .join(debug_path)
                };
                *emit_typeck_debug = Some(resolved);
            }
        }
    }

    Ok(())
}

fn merge_extension(existing: Option<&Value>, overrides: &serde_json::Map<String, Value>) -> Value {
    let mut payload = existing
        .and_then(|value| value.as_object().cloned())
        .unwrap_or_default();
    for (key, value) in overrides {
        payload.insert(key.clone(), value.clone());
    }
    Value::Object(payload)
}

fn extend_capabilities(target: &mut Vec<RuntimeCapability>, value: &Value) {
    match value {
        Value::String(name) => {
            add_runtime_capability_from_str(target, name);
        }
        Value::Array(entries) => {
            for entry in entries {
                extend_capabilities(target, entry);
            }
        }
        Value::Object(map) => {
            if let Some(id) = map.get("id").and_then(|v| v.as_str()) {
                let stage_str = map
                    .get("stage")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .trim()
                    .to_string();
                let stage = if stage_str.is_empty() {
                    StageId::stable()
                } else {
                    StageId::from_str(&stage_str).unwrap_or_else(|_| {
                        eprintln!(
                            "[CONFIG] runtime_capability `{id}` の stage `{stage_str}` は解釈できないため stable を使用します"
                        );
                        StageId::stable()
                    })
                };
                push_runtime_capability(target, RuntimeCapability::new(id, stage));
            }
        }
        _ => {}
    }
}

fn add_runtime_capability_from_str(target: &mut Vec<RuntimeCapability>, entry: &str) {
    if let Some(capability) = RuntimeCapability::parse(entry) {
        push_runtime_capability(target, capability);
    } else if !entry.trim().is_empty() {
        eprintln!("[CONFIG] runtime_capability `{entry}` は Capability として解析できませんでした");
    }
}

fn push_runtime_capability(target: &mut Vec<RuntimeCapability>, capability: RuntimeCapability) {
    target.retain(|existing| existing.id() != capability.id());
    target.push(capability);
}

fn parse_args() -> Result<CliArgs, Box<dyn std::error::Error>> {
    let mut argv = env::args();
    let program_name = argv.next().unwrap_or_else(|| "poc_frontend".to_string());
    let remaining: Vec<String> = argv.collect();
    let raw_cli_args = remaining.clone();
    let mut args = remaining.into_iter();
    let mut input = None;
    let mut parse_debug = None;
    let mut row_mode = None;
    let mut runtime_stage = None;
    let mut capability_stage = None;
    let mut cli_stage_override = None;
    let mut recover_expected_tokens = None;
    let mut recover_context = None;
    let mut recover_max_suggestions = None;
    let mut dualwrite_run_label = None;
    let mut dualwrite_case_label = None;
    let mut dualwrite_root = None;
    let mut emit_ast = None;
    let mut emit_typed_ast = None;
    let mut emit_constraints = None;
    let mut emit_typeck_debug = None;
    let mut emit_effects_metrics = None;
    let mut emit_impl_registry = None;
    let mut emit_tokens = None;
    let mut emit_effects = false;
    let mut emit_diagnostics = false;
    let mut emit_audit = false;
    let mut show_stage_context = false;
    let mut diagnostics_stream = false;
    let mut run_config = RunSettings::default();
    let mut stream_config = StreamSettings::default();
    let mut runtime_capabilities: Vec<RuntimeCapability> = Vec::new();
    let mut config_path: Option<PathBuf> = None;
    let mut trace_overridden = false;
    let mut merge_warnings_overridden = false;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--emit-parse-debug" => {
                let path = args
                    .next()
                    .ok_or_else(|| "--emit-parse-debug は出力パスを伴う必要があります")?;
                parse_debug = Some(PathBuf::from(path));
            }
            "--emit-effects" => emit_effects = true,
            "--emit-diagnostics" => emit_diagnostics = true,
            "--emit-audit" => emit_audit = true,
            "--show-stage-context" => show_stage_context = true,
            "--diagnostics-stream" => diagnostics_stream = true,
            "--emit-ast" => {
                let path = args
                    .next()
                    .ok_or_else(|| "--emit-ast は出力パスを伴う必要があります")?;
                emit_ast = Some(PathBuf::from(path));
            }
            "--type-row-mode" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--type-row-mode は値を伴う必要があります")?;
                row_mode =
                    Some(TypeRowMode::from_str(&value).map_err(|err| {
                        format!("--type-row-mode の値 `{value}` が不正です: {err}")
                    })?);
            }
            "--effect-stage-runtime" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--effect-stage-runtime は stage 名を伴う必要があります")?;
                runtime_stage = Some(StageId::from_str(&value).map(StageRequirement::AtLeast)?);
            }
            "--effect-stage-capability" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--effect-stage-capability は stage 名を伴う必要があります")?;
                capability_stage = Some(StageId::from_str(&value).map(StageRequirement::AtLeast)?);
            }
            "--effect-stage" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--effect-stage は stage 名を伴う必要があります")?;
                let stage = StageId::from_str(&value)?;
                cli_stage_override = Some(stage.clone());
                let requirement = StageRequirement::Exact(stage);
                runtime_stage = Some(requirement.clone());
                capability_stage = Some(requirement);
            }
            "--recover-expected-tokens" => {
                let value = args.next().ok_or_else(|| {
                    "--recover-expected-tokens は on/off の値を伴う必要があります"
                })?;
                recover_expected_tokens = Some(parse_on_off(&value)?);
            }
            "--recover-context" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--recover-context は on/off の値を伴う必要があります")?;
                recover_context = Some(parse_on_off(&value)?);
            }
            "--recover-max-suggestions" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--recover-max-suggestions は数値を伴う必要があります")?;
                recover_max_suggestions = Some(value.parse::<usize>().map_err(|_| {
                    format!("--recover-max-suggestions の値 `{value}` は整数ではありません")
                })?);
            }
            "--dualwrite-run-label" => {
                dualwrite_run_label = Some(
                    args.next()
                        .ok_or_else(|| "--dualwrite-run-label は値を伴う必要があります")?,
                );
            }
            "--dualwrite-case-label" => {
                dualwrite_case_label = Some(
                    args.next()
                        .ok_or_else(|| "--dualwrite-case-label は値を伴う必要があります")?,
                );
            }
            "--dualwrite-root" => {
                dualwrite_root =
                    Some(PathBuf::from(args.next().ok_or_else(|| {
                        "--dualwrite-root はパスを伴う必要があります"
                    })?));
            }
            "--emit-typed-ast" => {
                let path = args
                    .next()
                    .ok_or_else(|| "--emit-typed-ast は出力パスを伴う必要があります")?;
                emit_typed_ast = Some(PathBuf::from(path));
            }
            "--emit-constraints" => {
                let path = args
                    .next()
                    .ok_or_else(|| "--emit-constraints は出力パスを伴う必要があります")?;
                emit_constraints = Some(PathBuf::from(path));
            }
            "--emit-typeck-debug" => {
                let path = args
                    .next()
                    .ok_or_else(|| "--emit-typeck-debug は出力パスを伴う必要があります")?;
                emit_typeck_debug = Some(PathBuf::from(path));
            }
            "--emit-effects-metrics" => {
                let path = args
                    .next()
                    .ok_or_else(|| "--emit-effects-metrics は出力パスを伴う必要があります")?;
                emit_effects_metrics = Some(PathBuf::from(path));
            }
            "--emit-impl-registry" => {
                let path = args
                    .next()
                    .ok_or_else(|| "--emit-impl-registry は出力パスを伴う必要があります")?;
                emit_impl_registry = Some(PathBuf::from(path));
            }
            "--emit-tokens" => {
                let path = args
                    .next()
                    .ok_or_else(|| "--emit-tokens は出力パスを伴う必要があります")?;
                emit_tokens = Some(PathBuf::from(path));
            }
            "--packrat" => run_config.packrat = true,
            "--no-packrat" => run_config.packrat = false,
            "--trace" => {
                run_config.trace = true;
                trace_overridden = true;
            }
            "--no-trace" => {
                run_config.trace = false;
                trace_overridden = true;
            }
            "--merge-warnings" => {
                run_config.merge_warnings = true;
                merge_warnings_overridden = true;
            }
            "--no-merge-warnings" => {
                run_config.merge_warnings = false;
                merge_warnings_overridden = true;
            }
            "--require-eof" => run_config.require_eof = true,
            "--no-require-eof" => run_config.require_eof = false,
            "--legacy-result" => run_config.legacy_result = true,
            "--no-legacy-result" => run_config.legacy_result = false,
            "--experimental-effects" => run_config.experimental_effects = true,
            "--no-experimental-effects" => run_config.experimental_effects = false,
            "--left-recursion" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--left-recursion は値を伴う必要があります")?;
                run_config.left_recursion = LeftRecursionMode::from_str(&value)
                    .map_err(|_| format!("--left-recursion の値 `{value}` は無効です"))?;
            }
            "--lex-profile" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--lex-profile は ascii|unicode の値を伴う必要があります")?;
                run_config.lex_identifier_profile = value
                    .parse::<IdentifierProfile>()
                    .map_err(|_| {
                        format!(
                            "--lex-profile の値 `{value}` は `ascii` または `unicode` である必要があります"
                        )
                    })?;
            }
            "--streaming" => stream_config.enabled = true,
            "--no-streaming" => stream_config.enabled = false,
            "--stream-resume-hint" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--stream-resume-hint は値を伴う必要があります")?;
                stream_config.resume_hint = Some(value);
            }
            "--stream-flow-policy" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--stream-flow-policy は値を伴う必要があります")?;
                stream_config.flow_policy = Some(value);
            }
            "--stream-flow-max-lag" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--stream-flow-max-lag は値を伴う必要があります")?;
                stream_config.flow_max_lag = Some(value.parse::<u64>().map_err(|_| {
                    format!("--stream-flow-max-lag の値 `{value}` は整数ではありません")
                })?);
            }
            "--stream-demand-min-bytes" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--stream-demand-min-bytes は値を伴う必要があります")?;
                stream_config.demand_min_bytes = Some(value.parse::<u64>().map_err(|_| {
                    format!("--stream-demand-min-bytes の値 `{value}` は整数ではありません")
                })?);
            }
            "--stream-demand-preferred-bytes" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--stream-demand-preferred-bytes は値を伴う必要があります")?;
                stream_config.demand_preferred_bytes =
                    Some(value.parse::<u64>().map_err(|_| {
                        format!(
                            "--stream-demand-preferred-bytes の値 `{value}` は整数ではありません"
                        )
                    })?);
            }
            "--stream-checkpoint" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--stream-checkpoint は値を伴う必要があります")?;
                stream_config.checkpoint = Some(value);
            }
            "--runtime-capabilities" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--runtime-capabilities は値を伴う必要があります")?;
                for entry in value.split(',') {
                    let trimmed = entry.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    if let Some(capability) = RuntimeCapability::parse(trimmed) {
                        push_runtime_capability(&mut runtime_capabilities, capability);
                    } else {
                        eprintln!(
                            "[CONFIG] `--runtime-capabilities {trimmed}` が Capability として解析できませんでした"
                        );
                    }
                }
            }
            "--config" => {
                let path = args
                    .next()
                    .ok_or_else(|| "--config はパスを伴う必要があります")?;
                config_path = Some(PathBuf::from(&path));
            }
            _ if arg.starts_with("--") => {
                return Err(format!("未知のオプション: {arg}").into());
            }
            _ => {
                if input.is_some() {
                    return Err("入力ファイルは 1 つのみ指定できます".into());
                }
                input = Some(PathBuf::from(arg));
            }
        }
    }

    let input = match input {
        Some(path) => path,
        None => {
            eprintln!("使用方法: poc_frontend [options] <input.reml>");
            std::process::exit(1);
        }
    };

    if dualwrite_run_label.is_some() ^ dualwrite_case_label.is_some() {
        return Err("dual-write の run/case ラベルはセットで指定してください".into());
    }

    if let Some(additional) = config_path.clone() {
        if let Err(error) = apply_workspace_config(
            &additional,
            &mut run_config,
            &mut stream_config,
            &mut runtime_capabilities,
            &mut row_mode,
            &mut emit_typeck_debug,
            trace_overridden,
            merge_warnings_overridden,
        ) {
            eprintln!("[CONFIG] {}", error);
        }
    }

    let dualwrite = dualwrite_run_label.map(|run_label| DualwriteCliOpts {
        run_label,
        case_label: dualwrite_case_label.expect("validated together"),
        root: dualwrite_root,
    });

    let (target_inference, target_errors) = match target::infer_target_from_env() {
        Ok(inference) => (inference, 0),
        Err(err) => {
            eprintln!("[TARGET] 環境からのターゲット推論に失敗しました: {err}");
            (TargetInference::host_default(), 1)
        }
    };
    let target_extension_value = target_inference.inferred_payload();
    run_config.config = run_config
        .config
        .with_extension("target", |_| target_extension_value.clone());
    let target_cfg_extension = target_inference.cfg_extension(target_errors);

    let stage_context = StageContext::resolve(
        cli_stage_override.clone(),
        runtime_stage.clone(),
        capability_stage.clone(),
        &runtime_capabilities,
        target_inference.profile.triple.as_deref(),
    );
    let recover = RecoverConfig {
        emit_expected_tokens: recover_expected_tokens.unwrap_or(true),
        emit_context: recover_context.unwrap_or(true),
        max_suggestions: recover_max_suggestions.unwrap_or(3),
    };
    let mut builder = TypecheckConfig::builder()
        .effect_context(stage_context)
        .recover(recover)
        .experimental_effects(run_config.experimental_effects)
        .runtime_capabilities(runtime_capabilities.clone())
        .trace_enabled(run_config.trace);
    if let Some(mode) = row_mode {
        builder = builder.type_row_mode(mode);
    }

    Ok(CliArgs {
        program_name,
        raw_args: raw_cli_args,
        input,
        parse_debug_output: parse_debug,
        typecheck_config: builder.build(),
        dualwrite,
        emit_ast,
        emit_typed_ast,
        emit_constraints,
        emit_typeck_debug,
        emit_effects_metrics,
        emit_impl_registry,
        emit_tokens,
        emit_effects,
        emit_diagnostics,
        emit_audit,
        show_stage_context,
        diagnostics_stream,
        target_cfg_extension,
        run_config,
        stream_config,
        runtime_capabilities,
        config_path,
    })
}

fn parse_on_off(value: &str) -> Result<bool, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "on" | "true" | "1" => Ok(true),
        "off" | "false" | "0" => Ok(false),
        other => Err(format!("値 `{other}` は on/off ではありません")),
    }
}

fn write_dualwrite_typeck_payload(
    guards: &DualWriteGuards,
    report: &TypecheckReport,
    config: &TypecheckConfig,
    artifacts: &TypeckArtifacts,
    stage_payload: &StageAuditPayload,
) -> Result<(), Box<dyn std::error::Error>> {
    guards.write_json("typeck/config.json", config)?;
    let payload = TypecheckMetricsPayload::from_report(report, stage_payload);
    guards.write_json("typeck/metrics.json", &payload)?;
    guards.write_json("typeck/typed-ast.rust.json", &artifacts.typed_ast)?;
    guards.write_json("typeck/constraints.rust.json", &artifacts.constraints)?;
    guards.write_json("typeck/typeck-debug.rust.json", &artifacts.debug)?;
    let (run_label, case_label) = guards.labels();
    let impl_registry =
        build_impl_registry_payload(report, Some(run_label.as_ref()), Some(case_label.as_ref()));
    guards.write_json("typeck/impl-registry.rust.json", &impl_registry)?;
    Ok(())
}

fn write_dualwrite_parse_payload(
    guards: &DualWriteGuards,
    result: &ParseResult<Module>,
    run_config_value: &Value,
) -> Result<(), Box<dyn std::error::Error>> {
    guards.write_json("parse/ast.rust.json", &result.value)?;
    let cache_payload = json!({
        "packrat_stats": result.packrat_stats,
        "packrat_snapshot": result.packrat_snapshot,
        "packrat_cache": result.packrat_cache,
    });
    guards.write_json("parse/packrat_cache.json", &cache_payload)?;
    guards.write_json("parse/parser_run_config.rust.json", run_config_value)?;
    Ok(())
}

fn left_recursion_label(mode: LeftRecursionMode) -> &'static str {
    match mode {
        LeftRecursionMode::Off => "off",
        LeftRecursionMode::On => "on",
        LeftRecursionMode::Auto => "auto",
    }
}

fn build_runconfig_summary(
    run_config: &RunConfig,
    args: &CliArgs,
    flow: &StreamFlowState,
) -> Value {
    let flow_metrics = flow.metrics();
    let lex_extension = lex_extension_payload(run_config);
    let mut extensions = Map::new();
    extensions.insert("lex".to_string(), lex_extension.clone());
    extensions.insert(
        "recover".to_string(),
        json!({ "sync_tokens": [], "notes": false }),
    );
    extensions.insert(
        "stream".to_string(),
        build_stream_extension(&args.stream_config, &flow_metrics, run_config.packrat),
    );
    extensions.insert(
        "config".to_string(),
        build_config_extension(run_config, args),
    );
    if let Some(target_extension) = run_config.extension("target") {
        extensions.insert("target".to_string(), target_extension.clone());
    }
    json!({
        "packrat": run_config.packrat,
        "left_recursion": left_recursion_label(run_config.left_recursion),
        "trace": run_config.trace,
        "merge_warnings": run_config.merge_warnings,
        "require_eof": run_config.require_eof,
        "legacy_result": run_config.legacy_result,
        "experimental_effects": args.run_config.experimental_effects,
        "extensions": Value::Object(extensions),
    })
}

fn build_runconfig_top_level(
    run_config: &RunConfig,
    args: &CliArgs,
    flow: &StreamFlowState,
) -> Value {
    let flow_metrics = flow.metrics();
    let lex_extension = lex_extension_payload(run_config);
    let mut extensions = Map::new();
    extensions.insert("lex".to_string(), lex_extension.clone());
    extensions.insert(
        "recover".to_string(),
        json!({ "sync_tokens": [], "notes": false }),
    );
    extensions.insert(
        "stream".to_string(),
        build_stream_extension(&args.stream_config, &flow_metrics, run_config.packrat),
    );
    extensions.insert(
        "effects".to_string(),
        json!({
            "type_row_mode": type_row_mode_label(args.typecheck_config.type_row_mode),
        }),
    );
    extensions.insert(
        "config".to_string(),
        build_config_extension(run_config, args),
    );
    if let Some(target_extension) = run_config.extension("target") {
        extensions.insert("target".to_string(), target_extension.clone());
    }
    json!({
        "switches": {
            "packrat": run_config.packrat,
            "left_recursion": left_recursion_label(run_config.left_recursion),
            "trace": run_config.trace,
            "merge_warnings": run_config.merge_warnings,
            "require_eof": run_config.require_eof,
            "legacy_result": run_config.legacy_result,
            "experimental_effects": args.run_config.experimental_effects,
        },
        "extensions": Value::Object(extensions),
        "runtime_capabilities": args
            .runtime_capabilities
            .iter()
            .map(|cap| cap.to_string())
            .collect::<Vec<_>>(),
    })
}

fn lex_extension_payload(run_config: &RunConfig) -> Value {
    run_config.extension("lex").cloned().unwrap_or_else(|| {
        json!({
            "profile": "strict_json",
            "identifier_profile": IdentifierProfile::Unicode.as_str(),
        })
    })
}

fn build_config_extension(run_config: &RunConfig, args: &CliArgs) -> Value {
    let mut config = serde_json::Map::new();
    config.insert("source".to_string(), json!("cli"));
    config.insert("packrat".to_string(), json!(run_config.packrat));
    config.insert(
        "left_recursion".to_string(),
        json!(left_recursion_label(run_config.left_recursion)),
    );
    config.insert("trace".to_string(), json!(run_config.trace));
    config.insert(
        "merge_warnings".to_string(),
        json!(run_config.merge_warnings),
    );
    config.insert("require_eof".to_string(), json!(run_config.require_eof));
    config.insert("legacy_result".to_string(), json!(run_config.legacy_result));
    config.insert(
        "experimental_effects".to_string(),
        json!(args.run_config.experimental_effects),
    );
    if let Some(path) = args.config_path.as_ref() {
        config.insert("path".to_string(), json!(path.display().to_string()));
    }
    Value::Object(config)
}

fn build_stream_extension(
    stream: &StreamSettings,
    flow: &StreamFlowMetrics,
    packrat_enabled: bool,
) -> Value {
    let mut payload = stream_config_payload(stream, packrat_enabled);
    let flow_policy = stream
        .flow_policy
        .clone()
        .unwrap_or_else(|| "auto".to_string());
    let flow_max_lag = stream.flow_max_lag.unwrap_or(0);
    payload.insert(
        "flow".to_string(),
        json!({
            "policy": flow_policy,
            "backpressure": {
                "max_lag_bytes": flow_max_lag,
            },
            "checkpoints_closed": flow.checkpoints_closed,
        }),
    );
    Value::Object(payload)
}

fn stream_config_payload(stream: &StreamSettings, packrat_enabled: bool) -> Map<String, Value> {
    let mut payload = Map::new();
    payload.insert("enabled".to_string(), json!(stream.enabled));
    payload.insert("packrat_enabled".to_string(), json!(packrat_enabled));
    payload.insert(
        "checkpoint".to_string(),
        json!(stream
            .checkpoint
            .clone()
            .unwrap_or_else(|| "unspecified".to_string())),
    );
    payload.insert(
        "resume_hint".to_string(),
        json!(stream
            .resume_hint
            .clone()
            .unwrap_or_else(|| "unspecified".to_string())),
    );
    payload.insert(
        "demand_min_bytes".to_string(),
        json!(stream.demand_min_bytes.unwrap_or(0)),
    );
    payload.insert(
        "demand_preferred_bytes".to_string(),
        json!(stream.demand_preferred_bytes.unwrap_or(0)),
    );
    payload.insert(
        "chunk_size".to_string(),
        json!(stream.chunk_size.unwrap_or(0)),
    );
    payload.insert(
        "flow_policy".to_string(),
        json!(stream
            .flow_policy
            .clone()
            .unwrap_or_else(|| "auto".to_string())),
    );
    payload.insert(
        "flow_max_lag".to_string(),
        json!(stream.flow_max_lag.unwrap_or(0)),
    );
    payload
}

fn merge_stream_extension(
    existing: Option<&Value>,
    stream: &StreamSettings,
    packrat_enabled: bool,
) -> Value {
    let mut payload = existing
        .and_then(|value| value.as_object().cloned())
        .unwrap_or_default();
    for (key, value) in stream_config_payload(stream, packrat_enabled) {
        payload.insert(key, value);
    }
    Value::Object(payload)
}

const STREAMING_PLACEHOLDER_TOKEN: &str = "解析継続トークン";

#[derive(Clone, Debug)]
struct StageAuditPayload {
    required_stage: Option<String>,
    actual_stage: Option<String>,
    runtime_capabilities: Vec<RuntimeCapability>,
    stage_trace: Vec<StageTraceStep>,
}

impl StageAuditPayload {
    fn new(context: &StageContext, capabilities: &[RuntimeCapability]) -> Self {
        Self {
            required_stage: Some(stage_requirement_label(&context.capability)),
            actual_stage: Some(stage_requirement_label(&context.runtime)),
            runtime_capabilities: capabilities.to_vec(),
            stage_trace: context.stage_trace.clone(),
        }
    }

    fn primary_capability(&self) -> Option<&str> {
        self.runtime_capabilities
            .first()
            .map(|cap| cap.id().as_str())
    }

    fn effect_context(&self) -> effects::EffectAuditContext {
        effects::EffectAuditContext::new(
            self.required_stage.clone(),
            self.actual_stage.clone(),
            self.runtime_capabilities.clone(),
            self.stage_trace.clone(),
        )
    }

    fn apply_extensions(&self, extensions: &mut serde_json::Map<String, Value>) {
        effects::apply_extensions(&self.effect_context(), extensions);
    }

    fn apply_audit_metadata(&self, metadata: &mut serde_json::Map<String, Value>) {
        effects::apply_audit_metadata(&self.effect_context(), metadata);
    }
}

fn stage_requirement_label(requirement: &StageRequirement) -> String {
    requirement.label()
}

fn build_parser_diagnostics(
    diagnostics: &[FrontendDiagnostic],
    args: &CliArgs,
    input_path: &Path,
    source: &str,
    run_config: &RunConfig,
    runconfig_summary: &Value,
    flow: &StreamFlowState,
    stage_payload: &StageAuditPayload,
) -> Vec<Value> {
    let line_index = diag_json::LineIndex::new(source);
    let streaming_enabled = args.stream_config.enabled;
    let has_streaming_recover = streaming_enabled && diagnostics.iter().any(has_recover_note);
    let mut placeholder_emitted = false;
    diagnostics
        .iter()
        .filter_map(|diag| {
            if streaming_enabled && is_streaming_placeholder_lexer(diag) {
                if has_streaming_recover {
                    return None;
                }
                if placeholder_emitted {
                    return None;
                }
                placeholder_emitted = true;
            }

            let mut adjusted = diag.clone();
            if streaming_enabled && has_streaming_recover && has_recover_note(&adjusted) {
                adjusted = adjusted.force_streaming_expected();
            } else if streaming_enabled && adjusted.expected_tokens.is_empty() {
                adjusted = adjusted.ensure_streaming_expected();
            }
            Some(adjusted)
        })
        .map(|diag| {
            let timestamp = formatter::current_timestamp();
            let mut diag = diag.clone();
            if diag.domain.is_none() {
                diag.domain = Some(DiagnosticDomain::Parser);
            }
            diag.timestamp = Some(timestamp.clone());
            let recover_extension = diag_json::build_recover_extension(&diag);
            let mut extensions = serde_json::Map::new();
            extensions.insert(
                "diagnostic.v2".to_string(),
                json!({ "timestamp": timestamp }),
            );
            if let Some(recover) = recover_extension {
                extensions.insert("recover".to_string(), recover);
            }
            extensions.insert(
                "parse".to_string(),
                json!({
                    "parser_id": {
                        "namespace": PARSER_NAMESPACE,
                        "name": PARSER_NAME,
                        "ordinal": 0,
                        "origin": PARSER_ORIGIN,
                        "fingerprint": PARSER_FINGERPRINT,
                    }
                }),
            );
            stage_payload.apply_extensions(&mut extensions);
            extensions.insert("runconfig".to_string(), runconfig_summary.clone());
            extensions.insert("cfg".to_string(), args.target_cfg_extension.clone());

            let domain_label = diag
                .domain
                .as_ref()
                .map(|domain| domain.label().into_owned())
                .unwrap_or_else(|| "parser".to_string());
            let mut metadata = build_audit_metadata(
                &timestamp,
                args,
                run_config,
                &stage_payload,
                flow,
                domain_label.as_str(),
            );
            let context = FormatterContext {
                program_name: &args.program_name,
                raw_args: &args.raw_args,
                input_path,
            };
            let audit_envelope = formatter::finalize_audit_metadata(
                &mut metadata,
                &mut diag,
                &timestamp,
                &context,
                stage_payload.primary_capability(),
            );
            let payload_metadata = metadata.clone();
            let mut audit_object = serde_json::Map::new();
            audit_object.insert(
                "metadata".to_string(),
                Value::Object(payload_metadata.clone()),
            );
            if let Some(audit_id) = audit_envelope.audit_id {
                audit_object.insert("audit_id".to_string(), json!(audit_id));
            }
            if let Some(change_set) = audit_envelope.change_set {
                audit_object.insert("change_set".to_string(), change_set);
            }
            if let Some(capability) = audit_envelope.capability {
                audit_object.insert("capability".to_string(), json!(capability));
            }

            let expected_value = diag_json::build_expected_field(&diag);
            diag_json::build_frontend_diagnostic(diag_json::FrontendDiagnosticPayload {
                diag: &diag,
                timestamp: &timestamp,
                domain_label: &domain_label,
                line_index: &line_index,
                input_path,
                extensions,
                audit_metadata: payload_metadata,
                audit: Value::Object(audit_object),
                recoverability: recoverability_label(diag.recoverability),
                expected: expected_value,
                schema_version: SCHEMA_VERSION,
            })
        })
        .collect()
}

fn build_type_diagnostics(
    report: &TypecheckReport,
    args: &CliArgs,
    input_path: &Path,
    source: &str,
    run_config: &RunConfig,
    runconfig_summary: &Value,
    flow: &StreamFlowState,
    stage_payload: &StageAuditPayload,
) -> Vec<Value> {
    if report.violations.is_empty() {
        return Vec::new();
    }
    let line_index = diag_json::LineIndex::new(source);
    report
        .violations
        .iter()
        .map(|violation| {
            let timestamp = formatter::current_timestamp();
            let mut extensions = serde_json::Map::new();
            extensions.insert(
                "diagnostic.v2".to_string(),
                json!({
                    "timestamp": timestamp,
                    "codes": [violation.code],
                }),
            );
            stage_payload.apply_extensions(&mut extensions);
            let mut expected_value = Value::Null;
            if let Some(summary) = violation.expected_summary() {
                let payload = diag_json::expected_payload_from_summary(summary);
                expected_value = payload.clone();
                extensions.insert(
                    "recover".to_string(),
                    diag_json::recover_extension_payload_from_summary(summary),
                );
            }
            extensions.insert("runconfig".to_string(), runconfig_summary.clone());
            extensions.insert("cfg".to_string(), args.target_cfg_extension.clone());
            let mut metadata = build_audit_metadata(
                &timestamp,
                args,
                run_config,
                stage_payload,
                flow,
                violation.domain(),
            );
            let context = FormatterContext {
                program_name: &args.program_name,
                raw_args: &args.raw_args,
                input_path,
            };
            let audit_envelope = formatter::complete_audit_metadata(
                &mut metadata,
                &timestamp,
                &context,
                stage_payload.primary_capability(),
            );
            let payload_metadata = metadata.clone();
            let mut audit_object = serde_json::Map::new();
            audit_object.insert(
                "metadata".to_string(),
                Value::Object(payload_metadata.clone()),
            );
            if let Some(audit_id) = audit_envelope.audit_id {
                audit_object.insert("audit_id".to_string(), json!(audit_id));
            }
            if let Some(change_set) = audit_envelope.change_set {
                audit_object.insert("change_set".to_string(), change_set);
            }
            if let Some(capability) = audit_envelope.capability {
                audit_object.insert("capability".to_string(), json!(capability));
            }
            let notes = violation
                .notes
                .iter()
                .map(|note| {
                    json!({
                        "label": note.label.clone(),
                        "message": note.message.clone(),
                        "span": Value::Null,
                    })
                })
                .collect::<Vec<_>>();
            let primary = diag_json::span_to_primary_value(violation.span, &line_index, input_path);
            let location = diag_json::span_to_location_opt(violation.span, &line_index, input_path);
            json!({
                "schema_version": SCHEMA_VERSION,
                "timestamp": timestamp,
                "message": violation.message,
                "severity": "error",
                "severity_hint": Value::Null,
                "domain": violation.domain(),
                "primary": primary,
                "location": location,
                "extensions": Value::Object(extensions),
                "audit_metadata": Value::Object(payload_metadata),
                "audit": Value::Object(audit_object),
                "notes": notes,
                "secondary": Value::Array(vec![]),
                "hints": Value::Array(vec![]),
                "fixits": Value::Array(vec![]),
                "recoverability": recoverability_label(Recoverability::Fatal),
                "code": violation.code,
                "codes": [violation.code],
                "expected": expected_value,
            })
        })
        .collect()
}

fn has_recover_note(diag: &FrontendDiagnostic) -> bool {
    diag.notes
        .iter()
        .any(|note| note.label == "recover.expected_tokens")
}

fn is_streaming_placeholder_lexer(diag: &FrontendDiagnostic) -> bool {
    (diag.expected_tokens.is_empty()
        || (diag.expected_tokens.len() == 1
            && diag.expected_tokens[0] == STREAMING_PLACEHOLDER_TOKEN))
        && !diag.notes.is_empty()
        && diag.notes.iter().all(|note| note.label == "lexer")
}

fn build_audit_metadata(
    timestamp: &str,
    args: &CliArgs,
    run_config: &RunConfig,
    stage_payload: &StageAuditPayload,
    flow: &StreamFlowState,
    domain: &str,
) -> serde_json::Map<String, Value> {
    let mut metadata = serde_json::Map::new();
    metadata.insert("event.domain".to_string(), json!(domain));
    metadata.insert("event.kind".to_string(), json!("diagnostic"));
    metadata.insert("schema.version".to_string(), json!(SCHEMA_VERSION));
    metadata.insert(
        "parser.core.rule.namespace".to_string(),
        json!(PARSER_NAMESPACE),
    );
    metadata.insert("parser.core.rule.name".to_string(), json!(PARSER_NAME));
    metadata.insert("parser.core.rule.ordinal".to_string(), json!(0));
    metadata.insert("parser.core.rule.origin".to_string(), json!(PARSER_ORIGIN));
    metadata.insert(
        "parser.core.rule.fingerprint".to_string(),
        json!(PARSER_FINGERPRINT),
    );
    metadata.insert("namespace".to_string(), json!(PARSER_NAMESPACE));
    metadata.insert("name".to_string(), json!(PARSER_NAME));
    metadata.insert("origin".to_string(), json!(PARSER_ORIGIN));
    metadata.insert("fingerprint".to_string(), json!(PARSER_FINGERPRINT));
    metadata.insert(
        "audit.policy.version".to_string(),
        json!(formatter::AUDIT_POLICY_VERSION),
    );
    metadata.insert("audit.channel".to_string(), json!("cli"));
    metadata.insert("audit.timestamp".to_string(), json!(timestamp));
    metadata.insert(
        "parser.runconfig.switches.packrat".to_string(),
        json!(run_config.packrat),
    );
    metadata.insert(
        "parser.runconfig.switches.left_recursion".to_string(),
        json!(left_recursion_label(run_config.left_recursion)),
    );
    metadata.insert(
        "parser.runconfig.switches.trace".to_string(),
        json!(run_config.trace),
    );
    metadata.insert(
        "parser.runconfig.switches.merge_warnings".to_string(),
        json!(run_config.merge_warnings),
    );
    metadata.insert(
        "parser.runconfig.switches.require_eof".to_string(),
        json!(run_config.require_eof),
    );
    metadata.insert(
        "parser.runconfig.switches.legacy_result".to_string(),
        json!(run_config.legacy_result),
    );
    metadata.insert(
        "parser.runconfig.switches.experimental_effects".to_string(),
        json!(args.run_config.experimental_effects),
    );
    let runconfig_value = build_runconfig_top_level(run_config, args, flow);
    metadata.insert("parser.runconfig".to_string(), runconfig_value.clone());
    if let Some(extensions) = runconfig_value
        .get("extensions")
        .and_then(|value| value.as_object())
    {
        if let Some(lex) = extensions.get("lex") {
            metadata.insert("parser.runconfig.extensions.lex".to_string(), lex.clone());
            if let Some(profile) = lex.get("profile") {
                metadata.insert(
                    "parser.runconfig.extensions.lex.profile".to_string(),
                    profile.clone(),
                );
            }
            if let Some(identifier_profile) = lex.get("identifier_profile") {
                metadata.insert(
                    "parser.runconfig.extensions.lex.identifier_profile".to_string(),
                    identifier_profile.clone(),
                );
            }
        }
        if let Some(recover) = extensions.get("recover") {
            metadata.insert(
                "parser.runconfig.extensions.recover".to_string(),
                recover.clone(),
            );
            if let Some(notes) = recover.get("notes") {
                metadata.insert(
                    "parser.runconfig.extensions.recover.notes".to_string(),
                    notes.clone(),
                );
            }
            if let Some(tokens) = recover.get("sync_tokens") {
                metadata.insert(
                    "parser.runconfig.extensions.recover.sync_tokens".to_string(),
                    tokens.clone(),
                );
            }
        }
        if let Some(stream) = extensions.get("stream") {
            metadata.insert(
                "parser.runconfig.extensions.stream".to_string(),
                stream.clone(),
            );
            if let Some(checkpoint) = stream.get("checkpoint") {
                metadata.insert(
                    "parser.runconfig.extensions.stream.checkpoint".to_string(),
                    checkpoint.clone(),
                );
            }
            if let Some(resume_hint) = stream.get("resume_hint") {
                metadata.insert(
                    "parser.runconfig.extensions.stream.resume_hint".to_string(),
                    resume_hint.clone(),
                );
            }
            if let Some(min_bytes) = stream.get("demand_min_bytes") {
                metadata.insert(
                    "parser.runconfig.extensions.stream.demand_min_bytes".to_string(),
                    min_bytes.clone(),
                );
            }
            if let Some(pref_bytes) = stream.get("demand_preferred_bytes") {
                metadata.insert(
                    "parser.runconfig.extensions.stream.demand_preferred_bytes".to_string(),
                    pref_bytes.clone(),
                );
            }
            if let Some(chunk) = stream.get("chunk_size") {
                metadata.insert(
                    "parser.runconfig.extensions.stream.chunk_size".to_string(),
                    chunk.clone(),
                );
            }
            if let Some(flow) = stream.get("flow") {
                if let Some(policy) = flow.get("policy") {
                    metadata.insert(
                        "parser.runconfig.extensions.stream.flow.policy".to_string(),
                        policy.clone(),
                    );
                }
                if let Some(backpressure) = flow.get("backpressure") {
                    if let Some(max_lag) = backpressure.get("max_lag_bytes") {
                        metadata.insert(
                            "parser.runconfig.extensions.stream.flow.backpressure.max_lag_bytes"
                                .to_string(),
                            max_lag.clone(),
                        );
                    }
                }
            }
        }
        if let Some(config_extension) = extensions.get("config") {
            metadata.insert(
                "parser.runconfig.extensions.config".to_string(),
                config_extension.clone(),
            );
            if let Some(path) = config_extension.get("path") {
                metadata.insert(
                    "parser.runconfig.extensions.config.path".to_string(),
                    path.clone(),
                );
            }
            if let Some(source) = config_extension.get("source") {
                metadata.insert(
                    "parser.runconfig.extensions.config.source".to_string(),
                    source.clone(),
                );
            }
        }
    }
    metadata.insert(
        "parser.runconfig.extensions.stream.enabled".to_string(),
        json!(args.stream_config.enabled),
    );
    metadata.insert(
        "parser.stream.resume_hint".to_string(),
        json!(args
            .stream_config
            .resume_hint
            .clone()
            .unwrap_or_else(|| "unspecified".to_string())),
    );
    metadata.insert(
        "parser.stream.demand_min_bytes".to_string(),
        json!(args.stream_config.demand_min_bytes.unwrap_or(0)),
    );
    metadata.insert(
        "parser.stream.demand_preferred_bytes".to_string(),
        json!(args.stream_config.demand_preferred_bytes.unwrap_or(0)),
    );
    metadata.insert(
        "parser.stream.flow_policy".to_string(),
        json!(args
            .stream_config
            .flow_policy
            .clone()
            .unwrap_or_else(|| "auto".to_string())),
    );
    metadata.insert(
        "parser.stream.flow_max_lag".to_string(),
        json!(args.stream_config.flow_max_lag.unwrap_or(0)),
    );
    metadata.insert(
        "parser.stream.checkpoint".to_string(),
        json!(args
            .stream_config
            .checkpoint
            .clone()
            .unwrap_or_else(|| "unspecified".to_string())),
    );
    let flow_metrics = flow.metrics();
    let flow_config = flow.config();
    let packrat_enabled = flow_config
        .as_ref()
        .map(|cfg| cfg.packrat_enabled)
        .unwrap_or(run_config.packrat);
    metadata.insert(
        "parser.stream.packrat_enabled".to_string(),
        json!(packrat_enabled),
    );
    metadata.insert(
        "parser.stream.flow.checkpoints_closed".to_string(),
        json!(flow_metrics.checkpoints_closed),
    );
    if let Some(config) = flow_config.as_ref() {
        metadata.insert(
            "parser.stream.flow.enabled".to_string(),
            json!(config.enabled),
        );
        if let Some(resume_hint) = config.resume_hint.as_ref() {
            metadata.insert(
                "parser.stream.flow.resume_source".to_string(),
                json!(resume_hint),
            );
        }
    }
    stage_payload.apply_audit_metadata(&mut metadata);
    metadata
}

fn type_row_mode_label(mode: TypeRowMode) -> &'static str {
    match mode {
        TypeRowMode::MetadataOnly => "ty-metadata-only",
        TypeRowMode::DualWrite => "ty-dual-write",
        TypeRowMode::Integrated => "ty-integrated",
    }
}

#[derive(Serialize)]
struct TypecheckMetricsPayload<'a> {
    metrics: &'a TypecheckMetrics,
    typed_functions: &'a [TypedFunctionSummary],
    extensions: serde_json::Map<String, Value>,
    audit_metadata: serde_json::Map<String, Value>,
}

impl<'a> TypecheckMetricsPayload<'a> {
    fn from_report(report: &'a TypecheckReport, stage_payload: &StageAuditPayload) -> Self {
        let mut extensions = serde_json::Map::new();
        stage_payload.apply_extensions(&mut extensions);
        apply_residual_extension(&mut extensions, report);
        let mut audit_metadata = serde_json::Map::new();
        stage_payload.apply_audit_metadata(&mut audit_metadata);
        Self {
            metrics: &report.metrics,
            typed_functions: &report.functions,
            extensions,
            audit_metadata,
        }
    }
}

fn apply_residual_extension(
    extensions: &mut serde_json::Map<String, Value>,
    report: &TypecheckReport,
) {
    let residuals: Vec<String> = report
        .violations
        .iter()
        .filter_map(|violation| match violation.kind {
            TypecheckViolationKind::ResidualLeak => Some(
                violation
                    .capability
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string()),
            ),
            _ => None,
        })
        .collect();
    if residuals.is_empty() {
        return;
    }
    let effects_entry = extensions
        .entry("effects".to_string())
        .or_insert_with(|| Value::Object(serde_json::Map::new()));
    let effects_obj = effects::ensure_object(effects_entry);
    effects_obj.insert(
        "residual".to_string(),
        json!({
            "leaks": residuals,
            "count": residuals.len(),
        }),
    );
}

#[derive(Clone, Serialize)]
struct TypeckArtifacts {
    typed_ast: TypedAstFile,
    constraints: ConstraintFile,
    debug: TypeckDebugFile,
}

#[derive(Clone, Serialize)]
struct TypedAstFile {
    input: String,
    function_summaries: Vec<FunctionSummaryExport>,
    rendered: String,
    module: typed::TypedModule,
}

#[derive(Clone, Serialize)]
struct ConstraintFile {
    function_summaries: Vec<FunctionSummaryExport>,
    stats: ConstraintStats,
    total_constraints: usize,
    breakdown: Vec<ConstraintBucket>,
    functions: Vec<FunctionConstraintSummary>,
    constraints: Vec<Constraint>,
    used_impls: Vec<String>,
}

#[derive(Clone, Serialize)]
struct FunctionSummaryExport {
    name: String,
    param_count: usize,
    return_type: String,
    effect_row: String,
    span: Span,
    dict_refs: usize,
}

#[derive(Clone, Serialize)]
struct ConstraintStats {
    unify_calls: usize,
    ast_nodes: usize,
    token_count: usize,
}

#[derive(Clone, Serialize)]
struct ImplRegistryEntry {
    index: usize,
    impl_id: String,
    span: Span,
    requirements: Vec<String>,
    ty: String,
    functions: Vec<String>,
}

#[derive(Clone, Serialize)]
struct ImplRegistryFile {
    schema_version: &'static str,
    frontend: &'static str,
    run_label: Option<String>,
    case_label: Option<String>,
    used_impls: Vec<String>,
    entries: Vec<ImplRegistryEntry>,
}

impl Default for ConstraintStats {
    fn default() -> Self {
        Self {
            unify_calls: 0,
            ast_nodes: 0,
            token_count: 0,
        }
    }
}

#[derive(Clone, Serialize)]
struct ConstraintBucket {
    metric: String,
    count: usize,
}

#[derive(Clone, Serialize)]
struct FunctionConstraintSummary {
    name: String,
    constraints: usize,
    typed_exprs: usize,
    unresolved_identifiers: usize,
}

#[derive(Clone, Serialize)]
struct TypeckDebugFile {
    effect_context: StageContext,
    type_row_mode: TypeRowMode,
    recover: RecoverConfig,
    runtime_capabilities: Vec<RuntimeCapability>,
    trace_enabled: bool,
    metrics: TypecheckMetrics,
    violations: Vec<TypecheckViolation>,
}

fn build_function_summaries(
    module: &typed::TypedModule,
    metrics: &[TypedFunctionSummary],
) -> Vec<FunctionSummaryExport> {
    let mut summaries = Vec::with_capacity(module.functions.len().max(metrics.len()));
    for (typed_fn, stats) in module.functions.iter().zip(metrics.iter()) {
        summaries.push(FunctionSummaryExport {
            name: typed_fn.name.clone(),
            param_count: typed_fn.params.len(),
            return_type: stats.return_type.clone(),
            effect_row: String::new(),
            span: typed_fn.span,
            dict_refs: typed_fn.dict_ref_ids.len(),
        });
    }
    if summaries.len() < metrics.len() {
        for stats in &metrics[summaries.len()..] {
            summaries.push(FunctionSummaryExport {
                name: stats.name.clone(),
                param_count: stats.param_types.len(),
                return_type: stats.return_type.clone(),
                effect_row: String::new(),
                span: Span::default(),
                dict_refs: 0,
            });
        }
    }
    summaries
}

fn render_typed_module(module: &typed::TypedModule) -> String {
    if module.functions.is_empty() {
        return "=== Typed AST ===\n\n<empty>".to_string();
    }
    let mut lines = vec!["=== Typed AST ===".to_string()];
    for function in &module.functions {
        let params = function
            .params
            .iter()
            .map(|param| format!("{}: {}", param.name, param.ty))
            .collect::<Vec<_>>()
            .join(", ");
        let line = format!(
            "fn {}({}) : {}",
            function.name, params, function.return_type
        );
        lines.push(line);
    }
    lines.join("\n\n")
}

impl TypeckArtifacts {
    fn new(input: &Path, report: &TypecheckReport, config: &TypecheckConfig) -> Self {
        let typed_module = report.typed_module.clone();
        let function_summaries = build_function_summaries(&typed_module, &report.functions);
        let rendered = render_typed_module(&typed_module);
        let typed_ast = TypedAstFile {
            input: input.display().to_string(),
            function_summaries: function_summaries.clone(),
            rendered,
            module: typed_module,
        };
        let functions = report
            .functions
            .iter()
            .map(|function| FunctionConstraintSummary {
                name: function.name.clone(),
                constraints: function.constraints,
                typed_exprs: function.typed_exprs,
                unresolved_identifiers: function.unresolved_identifiers,
            })
            .collect();
        let breakdown = report
            .metrics
            .constraint_breakdown
            .iter()
            .map(|(metric, count)| ConstraintBucket {
                metric: metric.clone(),
                count: *count,
            })
            .collect();
        let constraints = ConstraintFile {
            function_summaries: function_summaries.clone(),
            stats: ConstraintStats {
                unify_calls: report.metrics.unify_calls,
                ast_nodes: report.metrics.ast_nodes,
                token_count: report.metrics.token_count,
            },
            total_constraints: report.metrics.constraints_total,
            breakdown,
            functions,
            constraints: report.constraints.clone(),
            used_impls: report.used_impls.clone(),
        };
        let debug = TypeckDebugFile {
            effect_context: config.effect_context.clone(),
            type_row_mode: config.type_row_mode,
            recover: config.recover.clone(),
            runtime_capabilities: config.runtime_capabilities.clone(),
            trace_enabled: config.trace_enabled,
            metrics: report.metrics.clone(),
            violations: report.violations.clone(),
        };
        Self {
            typed_ast,
            constraints,
            debug,
        }
    }
}

fn build_impl_registry_payload(
    report: &TypecheckReport,
    run_label: Option<&str>,
    case_label: Option<&str>,
) -> ImplRegistryFile {
    let mut owners: HashMap<typed::DictRefId, Vec<String>> = HashMap::new();
    for function in &report.typed_module.functions {
        for &dict_ref_id in &function.dict_ref_ids {
            owners
                .entry(dict_ref_id)
                .or_insert_with(Vec::new)
                .push(function.name.clone());
        }
    }
    let entries = report
        .typed_module
        .dict_refs
        .iter()
        .enumerate()
        .map(|(index, dict_ref)| ImplRegistryEntry {
            index,
            impl_id: dict_ref.impl_id.clone(),
            span: dict_ref.span,
            requirements: dict_ref.requirements.clone(),
            ty: dict_ref.ty.clone(),
            functions: owners.get(&index).cloned().unwrap_or_else(Vec::new),
        })
        .collect();
    ImplRegistryFile {
        schema_version: "w3-typeck-impl-registry/0.1",
        frontend: "rust",
        run_label: run_label.map(|value| value.to_string()),
        case_label: case_label.map(|value| value.to_string()),
        used_impls: report.used_impls.clone(),
        entries,
    }
}

fn write_json_file(path: &Path, value: &impl Serialize) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_vec_pretty(value)?;
    std::fs::write(path, json)?;
    Ok(())
}

fn trace_log(args: &CliArgs, phase: &str, status: &str) {
    if args.run_config.trace {
        eprintln!("[TRACE] {phase}.{status}");
    }
}

fn recoverability_label(value: Recoverability) -> &'static str {
    match value {
        Recoverability::Recoverable => "recoverable",
        Recoverability::Fatal => "fatal",
    }
}
