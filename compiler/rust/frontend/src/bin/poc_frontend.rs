//! logos × chumsky フロントエンド PoC。入力ファイルを解析し JSON を出力する。

use serde_json::{json, Value};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use reml_frontend::diagnostic::{DiagnosticNote, FrontendDiagnostic};
use reml_frontend::error::Recoverability;
use reml_frontend::parser::{ParserDriver, ParserOptions};
use reml_frontend::span::Span;
use reml_frontend::streaming::{StreamFlowConfig, StreamFlowState, StreamingStateConfig};
use reml_frontend::typeck::{
    self, DualWriteGuards, InstallConfigError, RecoverConfig, StageContext, StageId,
    StageRequirement, TypeRowMode, TypecheckConfig, TypecheckDriver, TypecheckMetrics,
    TypecheckReport, TypedFunctionSummary,
};
use serde::Serialize;

const PARSER_NAMESPACE: &str = "rust.poc";
const PARSER_NAME: &str = "compilation_unit";
const PARSER_ORIGIN: &str = "poc_frontend";
const PARSER_FINGERPRINT: &str = "rust-poc-0001";
const SCHEMA_VERSION: &str = "2.0.0-draft";
const AUDIT_POLICY_VERSION: &str = "rust.poc.audit.v1";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = parse_args()?;
    install_typecheck_config(&args.typecheck_config)?;
    let input_path = args.input.clone();
    let source = fs::read_to_string(&input_path)?;
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
    let parser_options = ParserOptions {
        streaming: args.streaming_state_config(),
        merge_parse_expected: args.run_config.merge_warnings,
        streaming_enabled: args.stream_config.enabled,
        stream_flow: Some(stream_flow_state.clone()),
    };
    let result = ParserDriver::parse_with_options(&source, parser_options);
    trace_log(&args, "parsing", "finish");
    let typeck_report = result
        .ast
        .as_ref()
        .map(|module| TypecheckDriver::infer_module(module, &args.typecheck_config))
        .unwrap_or_else(|| {
            TypecheckDriver::infer_fallback_from_source(&source, &args.typecheck_config)
        });
    let artifacts = TypeckArtifacts::new(&input_path, &typeck_report, &args.typecheck_config);
    let parse_result = serde_json::json!({
        "packrat_stats": result.packrat_stats,
        "span_trace": result.span_trace,
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
        }
    });

    let runconfig_summary = build_runconfig_summary(&args, &stream_flow_state);
    let runconfig_top_level = build_runconfig_top_level(&args, &stream_flow_state);
    let diagnostics_entries = build_diagnostics(
        &result.diagnostics,
        &args,
        &input_path,
        &source,
        &runconfig_summary,
    );
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
        let payload = TypecheckMetricsPayload {
            metrics: &typeck_report.metrics,
            typed_functions: &typeck_report.functions,
        };
        write_json_file(path, &payload)?;
    }

    if let Some(guards) = dualwrite {
        write_dualwrite_typeck_payload(
            &guards,
            &typeck_report,
            &args.typecheck_config,
            &artifacts,
        )?;
    }

    Ok(())
}

fn install_typecheck_config(config: &TypecheckConfig) -> Result<(), InstallConfigError> {
    match typeck::install_config(config.clone()) {
        Ok(()) => Ok(()),
        Err(InstallConfigError::AlreadyInstalled) => Ok(()),
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
    emit_constraints: Option<PathBuf>,
    emit_typeck_debug: Option<PathBuf>,
    emit_effects_metrics: Option<PathBuf>,
    run_config: RunSettings,
    stream_config: StreamSettings,
    runtime_capabilities: Vec<String>,
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
    packrat: bool,
    left_recursion: String,
    trace: bool,
    merge_warnings: bool,
    require_eof: bool,
    legacy_result: bool,
    experimental_effects: bool,
}

impl Default for RunSettings {
    fn default() -> Self {
        Self {
            packrat: true,
            left_recursion: "off".to_string(),
            trace: false,
            merge_warnings: true,
            require_eof: false,
            legacy_result: true,
            experimental_effects: false,
        }
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
    runtime_capabilities: &mut Vec<String>,
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
            run_config.left_recursion = left_recursion.to_string();
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

fn extend_capabilities(target: &mut Vec<String>, value: &Value) {
    match value {
        Value::String(name) => {
            if !name.is_empty() && !target.iter().any(|existing| existing == name) {
                target.push(name.to_string());
            }
        }
        Value::Array(entries) => {
            for entry in entries {
                extend_capabilities(target, entry);
            }
        }
        _ => {}
    }
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
    let mut recover_expected_tokens = None;
    let mut recover_context = None;
    let mut recover_max_suggestions = None;
    let mut dualwrite_run_label = None;
    let mut dualwrite_case_label = None;
    let mut dualwrite_root = None;
    let mut emit_typed_ast = None;
    let mut emit_constraints = None;
    let mut emit_typeck_debug = None;
    let mut emit_effects_metrics = None;
    let mut run_config = RunSettings::default();
    let mut stream_config = StreamSettings::default();
    let mut runtime_capabilities: Vec<String> = Vec::new();
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
                run_config.left_recursion = value;
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
                    if !trimmed.is_empty()
                        && !runtime_capabilities
                            .iter()
                            .any(|existing| existing == trimmed)
                    {
                        runtime_capabilities.push(trimmed.to_string());
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

    let effect_context = StageContext {
        runtime: runtime_stage.unwrap_or(StageRequirement::AtLeast(StageId::stable())),
        capability: capability_stage.unwrap_or(StageRequirement::AtLeast(StageId::beta())),
    };
    let recover = RecoverConfig {
        emit_expected_tokens: recover_expected_tokens.unwrap_or(true),
        emit_context: recover_context.unwrap_or(true),
        max_suggestions: recover_max_suggestions.unwrap_or(3),
    };
    let mut builder = TypecheckConfig::builder()
        .effect_context(effect_context)
        .recover(recover)
        .experimental_effects(run_config.experimental_effects)
        .runtime_capabilities(runtime_capabilities.clone())
        .trace_enabled(run_config.trace);
    if let Some(mode) = row_mode {
        builder = builder.type_row_mode(mode);
    }

    let dualwrite = dualwrite_run_label.map(|run_label| DualwriteCliOpts {
        run_label,
        case_label: dualwrite_case_label.expect("validated together"),
        root: dualwrite_root,
    });

    Ok(CliArgs {
        program_name,
        raw_args: raw_cli_args,
        input,
        parse_debug_output: parse_debug,
        typecheck_config: builder.build(),
        dualwrite,
        emit_typed_ast,
        emit_constraints,
        emit_typeck_debug,
        emit_effects_metrics,
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
) -> Result<(), Box<dyn std::error::Error>> {
    guards.write_json("typeck/config.json", config)?;
    let payload = TypecheckMetricsPayload {
        metrics: &report.metrics,
        typed_functions: &report.functions,
    };
    guards.write_json("typeck/metrics.json", &payload)?;
    guards.write_json("typeck/typed-ast.rust.json", &artifacts.typed_ast)?;
    guards.write_json("typeck/constraints.rust.json", &artifacts.constraints)?;
    guards.write_json("typeck/typeck-debug.rust.json", &artifacts.debug)?;
    Ok(())
}

fn build_runconfig_summary(args: &CliArgs, flow: &StreamFlowState) -> Value {
    let flow_metrics = flow.metrics();
    json!({
        "packrat": args.run_config.packrat,
        "left_recursion": args.run_config.left_recursion,
        "trace": args.run_config.trace,
        "merge_warnings": args.run_config.merge_warnings,
        "require_eof": args.run_config.require_eof,
        "legacy_result": args.run_config.legacy_result,
        "experimental_effects": args.run_config.experimental_effects,
        "extensions": {
            "lex": {
                "profile": "strict_json",
                "identifier_profile": "unicode",
            },
            "recover": {
                "sync_tokens": [],
                "notes": false,
            },
            "stream": build_stream_extension(&args.stream_config, &flow_metrics),
            "config": build_config_extension(args),
        },
    })
}

fn build_runconfig_top_level(args: &CliArgs, flow: &StreamFlowState) -> Value {
    let flow_metrics = flow.metrics();
    json!({
        "switches": {
            "packrat": args.run_config.packrat,
            "left_recursion": args.run_config.left_recursion,
            "trace": args.run_config.trace,
            "merge_warnings": args.run_config.merge_warnings,
            "require_eof": args.run_config.require_eof,
            "legacy_result": args.run_config.legacy_result,
            "experimental_effects": args.run_config.experimental_effects,
        },
        "extensions": {
            "lex": {
                "profile": "strict_json",
                "identifier_profile": "unicode",
            },
            "recover": {
                "sync_tokens": [],
                "notes": false,
            },
            "stream": build_stream_extension(&args.stream_config, &flow_metrics),
            "effects": {
                "type_row_mode": type_row_mode_label(args.typecheck_config.type_row_mode),
            },
            "config": build_config_extension(args),
        },
        "runtime_capabilities": args.runtime_capabilities.clone(),
    })
}

fn build_config_extension(args: &CliArgs) -> Value {
    let mut config = serde_json::Map::new();
    config.insert("source".to_string(), json!("cli"));
    config.insert("packrat".to_string(), json!(args.run_config.packrat));
    config.insert(
        "left_recursion".to_string(),
        json!(args.run_config.left_recursion),
    );
    config.insert("trace".to_string(), json!(args.run_config.trace));
    config.insert(
        "merge_warnings".to_string(),
        json!(args.run_config.merge_warnings),
    );
    config.insert(
        "require_eof".to_string(),
        json!(args.run_config.require_eof),
    );
    config.insert(
        "legacy_result".to_string(),
        json!(args.run_config.legacy_result),
    );
    config.insert(
        "experimental_effects".to_string(),
        json!(args.run_config.experimental_effects),
    );
    if let Some(path) = args.config_path.as_ref() {
        config.insert(
            "path".to_string(),
            json!(path.display().to_string()),
        );
    }
    Value::Object(config)
}

fn build_stream_extension(stream: &StreamSettings) -> Value {
    let checkpoint = stream
        .checkpoint
        .clone()
        .unwrap_or_else(|| "unspecified".to_string());
    let resume_hint = stream
        .resume_hint
        .clone()
        .unwrap_or_else(|| "unspecified".to_string());
    let demand_min_bytes = stream.demand_min_bytes.unwrap_or(0);
    let demand_preferred_bytes = stream.demand_preferred_bytes.unwrap_or(0);
    let chunk_size = stream.chunk_size.unwrap_or(0);
    let flow_policy = stream
        .flow_policy
        .clone()
        .unwrap_or_else(|| "auto".to_string());
    let flow_max_lag = stream.flow_max_lag.unwrap_or(0);
    json!({
        "enabled": stream.enabled,
        "checkpoint": checkpoint,
        "resume_hint": resume_hint,
        "demand_min_bytes": demand_min_bytes,
        "demand_preferred_bytes": demand_preferred_bytes,
        "chunk_size": chunk_size,
        "flow_policy": flow_policy.clone(),
        "flow_max_lag": flow_max_lag,
        "flow": {
            "policy": flow_policy,
            "backpressure": {
                "max_lag_bytes": flow_max_lag,
            }
        }
    })
}

const STREAMING_PLACEHOLDER_TOKEN: &str = "解析継続トークン";

#[derive(Clone, Debug)]
struct StageAuditPayload {
    required_stage: Option<String>,
    actual_stage: Option<String>,
    capability_ids: Vec<String>,
}

impl StageAuditPayload {
    fn new(context: &StageContext, capability_ids: &[String]) -> Self {
        Self {
            required_stage: Some(stage_requirement_label(&context.capability)),
            actual_stage: Some(stage_requirement_label(&context.runtime)),
            capability_ids: capability_ids.to_vec(),
        }
    }

    fn primary_capability(&self) -> Option<&str> {
        self.capability_ids.first().map(|s| s.as_str())
    }

    fn capability_details(&self) -> Vec<Value> {
        if self.capability_ids.is_empty() {
            return Vec::new();
        }
        self.capability_ids
            .iter()
            .map(|cap| {
                let mut entry = serde_json::Map::new();
                entry.insert("capability".to_string(), json!(cap));
                if let Some(actual) = &self.actual_stage {
                    entry.insert("stage".to_string(), json!(actual));
                }
                Value::Object(entry)
            })
            .collect()
    }

    fn capability_ids_value(&self) -> Value {
        Value::Array(
            self.capability_ids
                .iter()
                .map(|cap| Value::String(cap.clone()))
                .collect(),
        )
    }

    fn apply_extensions(&self, extensions: &mut serde_json::Map<String, Value>) {
        let ids_value = self.capability_ids_value();
        let capability_details = Value::Array(self.capability_details());
        extensions.insert("effect.capabilities".to_string(), ids_value.clone());
        extensions.insert(
            "effect.required_capabilities".to_string(),
            ids_value.clone(),
        );
        extensions.insert(
            "effect.stage.required_capabilities".to_string(),
            ids_value.clone(),
        );
        extensions.insert(
            "effect.actual_capabilities".to_string(),
            capability_details.clone(),
        );
        extensions.insert(
            "effect.stage.actual_capabilities".to_string(),
            capability_details.clone(),
        );
        if let Some(required) = &self.required_stage {
            extensions.insert("effect.stage.required".to_string(), json!(required));
        }
        if let Some(actual) = &self.actual_stage {
            extensions.insert("effect.stage.actual".to_string(), json!(actual));
        }
        if let Some(primary) = self.primary_capability() {
            extensions.insert("effect.capability".to_string(), json!(primary));
        }
        let mut capability_ext = serde_json::Map::new();
        capability_ext.insert("ids".to_string(), ids_value.clone());
        if let Some(primary) = self.primary_capability() {
            capability_ext.insert("primary".to_string(), json!(primary));
        }
        capability_ext.insert(
            "stage".to_string(),
            json!({
                "required": self.required_stage.as_deref(),
                "actual": self.actual_stage.as_deref(),
            }),
        );
        capability_ext.insert("detail".to_string(), capability_details);
        capability_ext.insert("required_capabilities".to_string(), ids_value.clone());
        extensions.insert("capability".to_string(), Value::Object(capability_ext));
    }

    fn apply_audit_metadata(&self, metadata: &mut serde_json::Map<String, Value>) {
        if let Some(required) = &self.required_stage {
            metadata.insert("effect.stage.required".to_string(), json!(required));
        }
        if let Some(actual) = &self.actual_stage {
            metadata.insert("effect.stage.actual".to_string(), json!(actual));
        }
        let ids_value = self.capability_ids_value();
        let capability_details = Value::Array(self.capability_details());
        metadata.insert("capability.ids".to_string(), ids_value.clone());
        metadata.insert(
            "effect.required_capabilities".to_string(),
            ids_value.clone(),
        );
        metadata.insert(
            "effect.stage.required_capabilities".to_string(),
            ids_value.clone(),
        );
        metadata.insert(
            "effect.actual_capabilities".to_string(),
            capability_details.clone(),
        );
        metadata.insert(
            "effect.stage.actual_capabilities".to_string(),
            capability_details.clone(),
        );
        metadata.insert(
            "bridge.stage.required_capabilities".to_string(),
            ids_value.clone(),
        );
        metadata.insert(
            "bridge.stage.actual_capabilities".to_string(),
            capability_details.clone(),
        );
        if let Some(primary) = self.primary_capability() {
            metadata.insert("bridge.stage.capability".to_string(), json!(primary));
            metadata.insert("effect.capability".to_string(), json!(primary));
        }
        let mut stage_trace = Vec::new();
        if let Some(required) = &self.required_stage {
            stage_trace.push(json!({
                "source": "cli_option",
                "stage": required,
                "note": "--effect-stage",
            }));
        }
        if let Some(actual) = &self.actual_stage {
            stage_trace.push(json!({
                "source": "runtime",
                "stage": actual,
            }));
        }
        if !stage_trace.is_empty() {
            let trace_value = Value::Array(stage_trace.clone());
            metadata.insert("stage.trace".to_string(), trace_value.clone());
            metadata.insert("effect.stage.trace".to_string(), trace_value);
        }
    }
}

fn stage_requirement_label(requirement: &StageRequirement) -> String {
    match requirement {
        StageRequirement::Exact(stage) => stage.as_str().to_string(),
        StageRequirement::AtLeast(stage) => format!("at_least:{}", stage.as_str()),
    }
}

fn build_diagnostics(
    diagnostics: &[FrontendDiagnostic],
    args: &CliArgs,
    input_path: &Path,
    source: &str,
    runconfig_summary: &Value,
) -> Vec<Value> {
    let line_index = LineIndex::new(source);
    let streaming_enabled = args.stream_config.enabled;
    let has_streaming_recover = streaming_enabled && diagnostics.iter().any(has_recover_note);
    let mut placeholder_emitted = false;
    let stage_payload = StageAuditPayload::new(
        &args.typecheck_config.effect_context,
        &args.runtime_capabilities,
    );

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
            if streaming_enabled && adjusted.expected_tokens.is_empty() {
                adjusted = adjusted.ensure_streaming_expected();
            }
            Some(adjusted)
        })
        .map(|diag| {
            let timestamp = current_timestamp();
            let location_value = diag
                .span
                .map(|span| span_to_location(span, &line_index, input_path))
                .unwrap_or(Value::Null);
            let notes = diag
                .notes
                .iter()
                .map(|note| note_to_json(note, &line_index, input_path))
                .collect::<Vec<_>>();
            let recover_extension = build_recover_extension(&diag);
            let mut extensions = serde_json::Map::new();
            extensions.insert(
                "diagnostic.v2".to_string(),
                json!({ "timestamp": timestamp }),
            );
            if let Some(recover) = recover_extension.clone() {
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

            let audit_metadata = build_audit_metadata(&timestamp, args, input_path, &stage_payload);
            let audit = json!({
                "metadata": audit_metadata.clone(),
                "audit_id": audit_metadata
                    .get("cli.audit_id")
                    .cloned()
                    .unwrap_or_else(|| json!(format!("cli/{}#0", timestamp))),
                "change_set": audit_metadata
                    .get("cli.change_set")
                    .cloned()
                    .unwrap_or_else(|| json!({})),
            });

            json!({
                "severity": "error",
                "message": diag.message,
                "schema_version": SCHEMA_VERSION,
                "location": location_value,
                "domain": "parser",
                "timestamp": timestamp,
                "extensions": Value::Object(extensions),
                "audit_metadata": Value::Object(audit_metadata),
                "audit": audit,
                "notes": notes,
                "recoverability": recoverability_label(diag.recoverability),
                "code": diag.code,
                "expected": build_expected_field(&diag),
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

fn expected_token_object(token: &str) -> Value {
    let hint = classify_expected_token(token);
    json!({
        "token": token,
        "label": token,
        "hint": hint,
        "kind": hint,
    })
}

fn build_recover_extension(diag: &FrontendDiagnostic) -> Option<Value> {
    if diag.has_expected_tokens() {
        let message = diag
            .expected_humanized
            .clone()
            .unwrap_or_else(|| default_expected_message(&diag.expected_tokens));
        let tokens: Vec<Value> = diag
            .expected_tokens
            .iter()
            .map(|token| expected_token_object(token))
            .collect();
        Some(json!({
            "message": message,
            "expected_tokens": tokens,
        }))
    } else {
        diag.notes.iter().find_map(|note| {
            if note.label == "recover.expected_tokens" {
                Some(json!({
                    "message": note.message,
                    "expected_tokens": [],
                }))
            } else {
                None
            }
        })
    }
}

fn build_expected_field(diag: &FrontendDiagnostic) -> Value {
    if !diag.has_expected_tokens() {
        return Value::Null;
    }
    let message_key = diag
        .expected_message_key
        .clone()
        .unwrap_or_else(|| "parse.expected".to_string());
    let alternatives: Vec<Value> = diag
        .expected_tokens
        .iter()
        .map(|token| expected_token_object(token))
        .collect();
    let humanized = diag
        .expected_humanized
        .clone()
        .unwrap_or_else(|| default_expected_message(&diag.expected_tokens));
    let locale_args = if diag.expected_locale_args.is_empty() {
        diag.expected_tokens.clone()
    } else {
        diag.expected_locale_args.clone()
    };
    json!({
        "message_key": message_key,
        "humanized": humanized,
        "locale_args": locale_args,
        "alternatives": alternatives,
    })
}

fn classify_expected_token(token: &str) -> &'static str {
    let trimmed = token.trim();
    if trimmed.is_empty() {
        "token"
    } else if trimmed.contains("identifier")
        || trimmed.ends_with("literal")
        || trimmed.ends_with("-literal")
    {
        "class"
    } else if trimmed
        .chars()
        .all(|ch| ch.is_ascii_alphabetic() && ch.is_lowercase())
    {
        "keyword"
    } else if trimmed.chars().all(|ch| ch.is_ascii_uppercase()) {
        "class"
    } else {
        "token"
    }
}

fn default_expected_message(tokens: &[String]) -> String {
    if tokens.is_empty() {
        return "ここで解釈可能な構文が見つかりません".to_string();
    }
    let formatted = tokens
        .iter()
        .map(|token| format!("`{}`", token))
        .collect::<Vec<_>>()
        .join("、");
    format!("ここで{}のいずれかが必要です", formatted)
}

fn build_audit_metadata(
    timestamp: &str,
    args: &CliArgs,
    input_path: &Path,
    stage_payload: &StageAuditPayload,
) -> serde_json::Map<String, Value> {
    let mut metadata = serde_json::Map::new();
    metadata.insert("event.domain".to_string(), json!("parser"));
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
        json!(AUDIT_POLICY_VERSION),
    );
    metadata.insert("audit.channel".to_string(), json!("cli"));
    metadata.insert("audit.timestamp".to_string(), json!(timestamp));
    let cli_audit_id = format!("cli/{}#0", timestamp.replace(':', "").replace('-', ""));
    metadata.insert("cli.audit_id".to_string(), json!(cli_audit_id));
    let change_set = json!({
        "policy": AUDIT_POLICY_VERSION,
        "origin": "cli",
        "source": {
            "command": &args.program_name,
            "args": &args.raw_args,
            "workspace": ".",
        },
        "items": [
            {
                "kind": "cli-command",
                "command": &args.program_name,
                "args": &args.raw_args,
            },
            {
                "kind": "input",
                "path": input_path,
                "target": "rust-poc",
            }
        ],
    });
    metadata.insert("cli.change_set".to_string(), change_set);
    metadata.insert(
        "parser.runconfig.switches.packrat".to_string(),
        json!(args.run_config.packrat),
    );
    metadata.insert(
        "parser.runconfig.switches.left_recursion".to_string(),
        json!(args.run_config.left_recursion),
    );
    metadata.insert(
        "parser.runconfig.switches.trace".to_string(),
        json!(args.run_config.trace),
    );
    metadata.insert(
        "parser.runconfig.switches.merge_warnings".to_string(),
        json!(args.run_config.merge_warnings),
    );
    metadata.insert(
        "parser.runconfig.switches.require_eof".to_string(),
        json!(args.run_config.require_eof),
    );
    metadata.insert(
        "parser.runconfig.switches.legacy_result".to_string(),
        json!(args.run_config.legacy_result),
    );
    let runconfig_value = build_runconfig_top_level(args, flow);
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

struct LineIndex {
    starts: Vec<usize>,
    len: usize,
}

impl LineIndex {
    fn new(source: &str) -> Self {
        let mut starts = vec![0];
        for (idx, ch) in source.char_indices() {
            if ch == '\n' {
                starts.push(idx + ch.len_utf8());
            }
        }
        Self {
            starts,
            len: source.len(),
        }
    }

    fn line_col(&self, offset: usize) -> (u32, u32) {
        let clamped = offset.min(self.len);
        let idx = match self.starts.binary_search(&clamped) {
            Ok(pos) => pos,
            Err(pos) => pos.saturating_sub(1),
        };
        let line_start = self.starts[idx];
        (
            idx as u32 + 1,
            (clamped.saturating_sub(line_start)) as u32 + 1,
        )
    }
}

fn span_to_location(span: Span, index: &LineIndex, input_path: &Path) -> Value {
    let (line, column) = index.line_col(span.start as usize);
    let (end_line, end_column) = index.line_col(span.end as usize);
    json!({
        "file": input_path,
        "line": line,
        "column": column,
        "endLine": end_line,
        "endColumn": end_column,
    })
}

fn note_to_json(note: &DiagnosticNote, index: &LineIndex, input_path: &Path) -> Value {
    let span_value = note
        .span
        .map(|span| span_to_location(span, index, input_path))
        .unwrap_or(Value::Null);
    json!({
        "label": note.label,
        "message": note.message,
        "span": span_value,
    })
}

#[derive(Serialize)]
struct TypecheckMetricsPayload<'a> {
    metrics: &'a TypecheckMetrics,
    typed_functions: &'a [TypedFunctionSummary],
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
    functions: Vec<TypedFunctionSummary>,
}

#[derive(Clone, Serialize)]
struct ConstraintFile {
    total_constraints: usize,
    breakdown: Vec<ConstraintBucket>,
    functions: Vec<FunctionConstraintSummary>,
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
    runtime_capabilities: Vec<String>,
    trace_enabled: bool,
    metrics: TypecheckMetrics,
}

impl TypeckArtifacts {
    fn new(input: &Path, report: &TypecheckReport, config: &TypecheckConfig) -> Self {
        let typed_ast = TypedAstFile {
            input: input.display().to_string(),
            functions: report.functions.clone(),
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
            total_constraints: report.metrics.constraints_total,
            breakdown,
            functions,
        };
        let debug = TypeckDebugFile {
            effect_context: config.effect_context.clone(),
            type_row_mode: config.type_row_mode,
            recover: config.recover.clone(),
            runtime_capabilities: config.runtime_capabilities.clone(),
            trace_enabled: config.trace_enabled,
            metrics: report.metrics.clone(),
        };
        Self {
            typed_ast,
            constraints,
            debug,
        }
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

fn current_timestamp() -> String {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0));
    let seconds = duration.as_secs() as i64;
    let (year, month, day, hour, minute, second) = unix_seconds_to_components(seconds);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

fn unix_seconds_to_components(seconds: i64) -> (i32, u32, u32, u32, u32, u32) {
    const SECONDS_PER_DAY: i64 = 86_400;
    let days = seconds.div_euclid(SECONDS_PER_DAY);
    let mut rem = seconds.rem_euclid(SECONDS_PER_DAY);
    if rem < 0 {
        rem += SECONDS_PER_DAY;
    }
    let hour = (rem / 3_600) as u32;
    rem %= 3_600;
    let minute = (rem / 60) as u32;
    let second = (rem % 60) as u32;
    let (year, month, day) = civil_from_days(days);
    (year, month, day, hour, minute, second)
}

fn civil_from_days(days: i64) -> (i32, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 {
        z / 146_097
    } else {
        (z - 146_096) / 146_097
    };
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let mut year = (yoe + era * 400) as i32;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    year += ((mp + 2) / 12) as i32;
    let month = ((mp + 2) % 12 + 1) as u32;
    (year, month, day)
}
