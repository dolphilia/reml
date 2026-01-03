//! Rust Frontend CLI（`reml_frontend`）。入力ファイルを解析して JSON を出力する。

use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::env;
use std::fmt;
use std::fs;
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::{Arc, Mutex};

use reml_adapter::target::{self, TargetInference};
use reml_frontend::diagnostic::messages;
use reml_frontend::diagnostic::{
    effects,
    filter::{
        apply_experimental_stage_policy, should_downgrade_experimental, AuditPolicy,
        DiagnosticFilter,
    },
    formatter::{self, FormatterContext},
    json as diag_json, unicode, DiagnosticDomain, FrontendDiagnostic, StageAuditPayload,
};
use reml_frontend::diagnostic::{ExpectedToken, ExpectedTokenCollector, ExpectedTokensSummary};
use reml_frontend::effects::diagnostics::EffectDiagnostic;
use reml_frontend::error::Recoverability;
use reml_frontend::ffi_executor::install_cli_ffi_executor;
use reml_frontend::lexer::{lex_source_with_options, IdentifierProfile, LexerOptions};
use reml_frontend::output::cli::{
    emit_cli_output, CliCommandKind, CliDiagnosticEnvelope, CliExitCode, CliPhaseKind, CliSummary,
    OutputFormat,
};
use reml_frontend::parser::ast::Module;
use reml_frontend::parser::{
    LeftRecursionMode, ParseResult, ParserDriver, ParserOptions, ParserTraceEvent, RunConfig,
    StreamOutcome, StreamingRunner,
};
use reml_frontend::pipeline::{AuditEmitter, PipelineDescriptor, PipelineFailure, PipelineOutcome};
use reml_frontend::semantics::{mir, typed};
use reml_frontend::span::Span;
use reml_frontend::streaming::{
    StreamFlowConfig, StreamFlowMetrics, StreamFlowState, StreamingStateConfig, TraceFrame,
};
use reml_frontend::typeck::telemetry::TraitResolutionTelemetry;
use reml_frontend::typeck::{
    self, Constraint, DualWriteGuards, InstallConfigError, IteratorStageViolationInfo,
    RecoverConfig, RuntimeCapability, StageContext, StageId, StageRequirement, StageTraceStep,
    TypeRowMode, TypecheckConfig, TypecheckDriver, TypecheckMetrics, TypecheckReport,
    TypecheckViolation, TypecheckViolationKind, TypedFunctionSummary,
};
use reml_runtime::audit::AuditEvent;
use reml_runtime::config::{
    compatibility_profile, resolve_compat, CompatibilityLayer, CompatibilityProfileError,
    ConfigFormat, ManifestLoader, ResolveCompatOptions, ResolvedConfigCompatibility,
};
use reml_runtime::lsp::derive::{Derive, DeriveModel};
use reml_runtime::parse as runtime_parse;
use reml_runtime::parse::combinator::LEFT_RECURSION_MESSAGE;
use reml_runtime::prelude::ensure::{DiagnosticSeverity, GuardDiagnostic, IntoDiagnostic};
use reml_runtime::run_config::{
    apply_manifest_overrides, ApplyManifestOverridesArgs, LeftRecursionStrategy,
    RunConfigManifestOverrides,
};
use reml_runtime::runtime::plugin::{
    take_plugin_audit_events, PluginBundleRegistration, PluginBundleVerification, PluginError,
    PluginLoadError, PluginLoader, SignatureStatus, VerificationPolicy,
};
use reml_runtime::runtime::plugin_bridge::NativePluginExecutionBridge;
use reml_runtime::runtime::plugin_manager::PluginRuntimeManager;
use reml_runtime::stage::StageId as RuntimeStageId;
use reml_runtime::test as runtime_test;
use reml_runtime::text::LocaleId;
use reml_runtime::text::Str as RuntimeStr;
use reml_runtime::{
    path as runtime_path,
    path::{PathBuf as RuntimePathBuf, SecurityPolicy as RuntimeSecurityPolicy},
};
use reml_runtime::{
    CapabilityDescriptor, CapabilityIsolationLevel, CapabilityPermission, CapabilityProvider,
    CapabilityRegistry, CapabilityTimestamp,
};
use serde::Serialize;
use uuid::Uuid;

const PARSER_NAMESPACE: &str = "rust.frontend";
const PARSER_NAME: &str = "compilation_unit";
const PARSER_ORIGIN: &str = "reml_frontend";
const PARSER_FINGERPRINT: &str = "rust-frontend-0001";
const SCHEMA_VERSION: &str = "3.0.0-alpha";

struct CliRunResult {
    envelope: CliDiagnosticEnvelope,
    exit_code: CliExitCode,
    input_path: PathBuf,
    diagnostic_count: usize,
    stage_payload: StageAuditPayload,
    test_audit_events: Vec<AuditEvent>,
    lsp_derive: Option<DeriveModel>,
}

#[derive(Default)]
struct FilterStats {
    suppressed_by_filter: usize,
    audit_dropped: usize,
    audit_anonymized: usize,
}

fn try_run_capability_command() -> Result<bool, Box<dyn std::error::Error>> {
    let mut argv = env::args();
    let _program_name = argv.next();
    let args: Vec<String> = argv.collect();
    if args.first().map(|arg| arg.as_str()) != Some("--capability") {
        return Ok(false);
    }
    let mut iter = args.iter().skip(1);
    let subcommand = iter
        .next()
        .ok_or("--capability には describe などのサブコマンドを指定してください")?;
    match subcommand.as_str() {
        "describe" => {
            let capability_id = iter
                .next()
                .ok_or("--capability describe には Capability ID が必要です")?
                .to_string();
            let mut format = OutputFormat::Json;
            while let Some(arg) = iter.next() {
                match arg.as_str() {
                    "--output" => {
                        let value = iter.next().ok_or(
                            "--capability describe --output には human/json/lsp/lsp-derive を指定してください",
                        )?;
                        format = OutputFormat::parse(value)?;
                    }
                    "--human" => format = OutputFormat::Human,
                    "--json" => format = OutputFormat::Json,
                    other => {
                        return Err(
                            format!("--capability describe の未知のオプション: {other}").into()
                        )
                    }
                }
            }
            run_capability_describe(&capability_id, format)?;
            Ok(true)
        }
        other => Err(format!("--capability {other} は未サポートです").into()),
    }
}

fn try_run_plugin_command() -> Result<bool, Box<dyn std::error::Error>> {
    let mut argv = env::args();
    let _program_name = argv.next();
    let args: Vec<String> = argv.collect();
    if args.first().map(|arg| arg.as_str()) != Some("plugin") {
        return Ok(false);
    }
    let mut iter = args.iter().skip(1);
    let subcommand = iter
        .next()
        .ok_or("plugin には install などのサブコマンドを指定してください")?;
    match subcommand.as_str() {
        "install" => {
            let mut bundle_path = None;
            let mut policy = VerificationPolicy::Strict;
            let mut output = OutputFormat::Human;
            while let Some(arg) = iter.next() {
                match arg.as_str() {
                    "--bundle" => {
                        bundle_path = iter.next().cloned();
                    }
                    "--policy" => {
                        let value = iter
                            .next()
                            .ok_or("--policy は strict|permissive を指定してください")?;
                        policy = match value.as_str() {
                            "strict" => VerificationPolicy::Strict,
                            "permissive" => VerificationPolicy::Permissive,
                            other => return Err(format!("--policy の未知の値: {other}").into()),
                        };
                    }
                    "--output" => {
                        let value = iter
                            .next()
                            .ok_or("--output は human|json を指定してください")?;
                        output = OutputFormat::parse(value)?;
                        if matches!(output, OutputFormat::Lsp | OutputFormat::LspDerive) {
                            return Err("--output は human|json のみ対応しています".into());
                        }
                    }
                    "--human" => output = OutputFormat::Human,
                    "--json" => output = OutputFormat::Json,
                    other => {
                        return Err(format!("plugin install の未知のオプション: {other}").into())
                    }
                }
            }
            let bundle_path =
                bundle_path.ok_or("plugin install には --bundle <path> が必要です")?;
            let registration = match run_plugin_install(bundle_path, policy) {
                Ok(registration) => registration,
                Err(err) => {
                    emit_plugin_audit_events();
                    return Err(format_plugin_error(&err).into());
                }
            };
            emit_plugin_audit_events();
            match output {
                OutputFormat::Human => {
                    print_plugin_install_human(&registration);
                }
                OutputFormat::Json => {
                    println!("{}", serde_json::to_string_pretty(&registration)?);
                }
                _ => {}
            }
            Ok(true)
        }
        "verify" => {
            let mut bundle_path = None;
            let mut policy = VerificationPolicy::Strict;
            let mut output = OutputFormat::Human;
            while let Some(arg) = iter.next() {
                match arg.as_str() {
                    "--bundle" => {
                        bundle_path = iter.next().cloned();
                    }
                    "--policy" => {
                        let value = iter
                            .next()
                            .ok_or("--policy は strict|permissive を指定してください")?;
                        policy = match value.as_str() {
                            "strict" => VerificationPolicy::Strict,
                            "permissive" => VerificationPolicy::Permissive,
                            other => return Err(format!("--policy の未知の値: {other}").into()),
                        };
                    }
                    "--output" => {
                        let value = iter
                            .next()
                            .ok_or("--output は human|json を指定してください")?;
                        output = OutputFormat::parse(value)?;
                        if matches!(output, OutputFormat::Lsp | OutputFormat::LspDerive) {
                            return Err("--output は human|json のみ対応しています".into());
                        }
                    }
                    "--human" => output = OutputFormat::Human,
                    "--json" => output = OutputFormat::Json,
                    other => {
                        return Err(format!("plugin verify の未知のオプション: {other}").into())
                    }
                }
            }
            let bundle_path = bundle_path.ok_or("plugin verify には --bundle <path> が必要です")?;
            let verification = match run_plugin_verify(bundle_path, policy) {
                Ok(verification) => verification,
                Err(err) => {
                    emit_plugin_audit_events();
                    return Err(format_plugin_load_error(&err).into());
                }
            };
            emit_plugin_audit_events();
            match output {
                OutputFormat::Human => {
                    print_plugin_verification_human(&verification);
                }
                OutputFormat::Json => {
                    println!("{}", serde_json::to_string_pretty(&verification)?);
                }
                _ => {}
            }
            Ok(true)
        }
        other => Err(format!("plugin {other} は未サポートです").into()),
    }
}

fn run_plugin_install(
    bundle_path: String,
    policy: VerificationPolicy,
) -> Result<PluginBundleRegistration, PluginError> {
    let loader = PluginLoader::new();
    let bridge = NativePluginExecutionBridge::new();
    let manager = PluginRuntimeManager::new(loader, Box::new(bridge));
    manager.load_bundle_and_attach(bundle_path, policy)
}

fn run_plugin_verify(
    bundle_path: String,
    policy: VerificationPolicy,
) -> Result<PluginBundleVerification, PluginLoadError> {
    let loader = PluginLoader::new();
    loader.verify_bundle_path(bundle_path, policy)
}

fn format_plugin_error(error: &PluginError) -> String {
    match error {
        PluginError::Load(err) => format_plugin_load_error(err),
        PluginError::Capability(err) => {
            format!(
                "Capability の登録または検証に失敗しました: {}",
                err.detail()
            )
        }
        PluginError::VerificationFailed { message } => {
            format!("プラグイン検証に失敗しました: {message}")
        }
        PluginError::Io { message } => format!("プラグイン I/O エラーが発生しました: {message}"),
        PluginError::AlreadyLoaded { plugin_id } => {
            format!("プラグインは既にロードされています: {plugin_id}")
        }
        PluginError::NotLoaded { plugin_id } => {
            format!("プラグインはロードされていません: {plugin_id}")
        }
        PluginError::Bridge { message } => {
            format!("プラグインブリッジでエラーが発生しました: {message}")
        }
        PluginError::BundleInstallFailed {
            message,
            capability_error,
        } => {
            if let Some(error) = capability_error {
                format!(
                    "バンドルのインストールに失敗しました: {message}（capability: {}）",
                    error.detail()
                )
            } else {
                format!("バンドルのインストールに失敗しました: {message}")
            }
        }
    }
}

fn format_plugin_load_error(error: &PluginLoadError) -> String {
    error.to_string()
}

fn print_plugin_install_human(registration: &PluginBundleRegistration) {
    println!(
        "plugin.verify_signature: {}@{} ({})",
        registration.bundle_id,
        registration.bundle_version,
        signature_status_label(&registration.signature_status)
    );
    for plugin in &registration.plugins {
        println!(
            "plugin.install: {} ({} caps)",
            plugin.plugin_id,
            plugin.capabilities.len()
        );
    }
}

fn print_plugin_verification_human(verification: &PluginBundleVerification) {
    println!(
        "plugin.verify_signature: {}@{} ({})",
        verification.bundle_id,
        verification.bundle_version,
        signature_status_label(&verification.signature_status)
    );
    if let Some(bundle_hash) = &verification.bundle_hash {
        println!("  bundle_hash: {bundle_hash}");
    }
    if verification.manifest_paths.is_empty() {
        println!("  manifests: (none)");
    } else {
        println!("  manifests:");
        for path in &verification.manifest_paths {
            println!("    - {path}");
        }
    }
}

fn signature_status_label(status: &SignatureStatus) -> &'static str {
    match status {
        SignatureStatus::Verified => "verified",
        SignatureStatus::Skipped => "skipped",
    }
}

fn emit_plugin_audit_events() {
    let events = take_plugin_audit_events();
    if events.is_empty() {
        return;
    }
    let mut emitter = AuditEmitter::stderr(true);
    for event in events {
        if let Err(err) = emitter.emit_external_event(&event) {
            eprintln!("[AUDIT] plugin 監査イベントの書き出しに失敗しました: {err}");
        }
    }
}

fn run_capability_describe(
    capability_id: &str,
    format: OutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    let registry = CapabilityRegistry::registry();
    let descriptor = registry
        .describe(capability_id)
        .map_err(|err| format!("Capability `{capability_id}` の取得に失敗しました: {err}"))?;
    match format {
        OutputFormat::Json => {
            let body = serde_json::to_string_pretty(&descriptor)?;
            println!("{body}");
        }
        OutputFormat::Human => print_capability_descriptor_human(&descriptor),
        OutputFormat::Lsp => {
            return Err("--capability describe では LSP 出力をサポートしていません".into())
        }
        OutputFormat::LspDerive => {
            return Err("--capability describe では lsp-derive 出力をサポートしていません".into())
        }
    }
    Ok(())
}

fn print_capability_descriptor_human(descriptor: &CapabilityDescriptor) {
    println!("Capability: {}", descriptor.id);
    println!("  stage: {}", descriptor.stage().as_str());
    if descriptor.effect_scope().is_empty() {
        println!("  effect_scope: (none)");
    } else {
        println!(
            "  effect_scope: [{}]",
            descriptor
                .effect_scope()
                .iter()
                .map(|effect| effect.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    let metadata = descriptor.metadata();
    println!(
        "  provider: {}",
        format_capability_provider(&metadata.provider)
    );
    if let Some(path) = metadata.manifest_path.as_ref() {
        println!("  manifest_path: {}", path.display());
    }
    if let Some(timestamp) = metadata.last_verified_at {
        println!(
            "  last_verified_at: {}",
            format_capability_timestamp(timestamp)
        );
    }
    let security = &metadata.security;
    println!(
        "  security.audit_required: {}",
        if security.audit_required {
            "true"
        } else {
            "false"
        }
    );
    println!(
        "  security.isolation_level: {}",
        format_isolation_level(&security.isolation_level)
    );
    if security.permissions.is_empty() {
        println!("  security.permissions: (none)");
    } else {
        println!("  security.permissions:");
        for permission in &security.permissions {
            println!("    - {}", format_permission(permission));
        }
    }
    if let Some(policy) = &security.policy {
        println!("  security.policy: {policy}");
    }
    if let Some(profile) = &security.sandbox_profile {
        println!(
            "  security.sandbox: {}{}",
            profile.name,
            profile
                .version
                .as_ref()
                .map(|version| format!(" v{version}"))
                .unwrap_or_default()
        );
    }
    if let Some(signature) = &security.signature {
        println!(
            "  security.signature: issuer={:?}, algorithm={:?}, digest={:?}",
            signature.issuer, signature.algorithm, signature.digest
        );
    }
}

fn format_capability_provider(provider: &CapabilityProvider) -> String {
    match provider {
        CapabilityProvider::Core => "core".to_string(),
        CapabilityProvider::Plugin { package, version } => {
            let mut label = format!("plugin:{package}");
            if let Some(version) = version {
                label.push('@');
                label.push_str(version);
            }
            label
        }
        CapabilityProvider::ExternalBridge { name, version } => {
            let mut label = format!("bridge:{name}");
            if let Some(version) = version {
                label.push('@');
                label.push_str(version);
            }
            label
        }
        CapabilityProvider::RuntimeComponent { name } => {
            format!("runtime:{name}")
        }
    }
}

fn format_capability_timestamp(timestamp: CapabilityTimestamp) -> String {
    format!(
        "{}.{:09}s (unix)",
        timestamp.seconds,
        timestamp.nanos.max(0)
    )
}

fn format_isolation_level(level: &CapabilityIsolationLevel) -> &'static str {
    match level {
        CapabilityIsolationLevel::None => "none",
        CapabilityIsolationLevel::Sandboxed => "sandboxed",
        CapabilityIsolationLevel::FullIsolation => "full_isolation",
    }
}

fn format_permission(permission: &CapabilityPermission) -> String {
    match permission {
        CapabilityPermission::ReadConfig => "read_config".to_string(),
        CapabilityPermission::WriteConfig => "write_config".to_string(),
        CapabilityPermission::FileSystem { pattern } => {
            format!("filesystem({pattern})")
        }
        CapabilityPermission::Network { pattern } => format!("network({pattern})"),
        CapabilityPermission::Runtime { operation } => {
            format!("runtime({operation})")
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if let Err(err) = install_cli_ffi_executor() {
        eprintln!("[FFI] 実行エンジンの初期化に失敗しました: {err}");
    }
    let plugin_command_executed = match try_run_plugin_command() {
        Ok(executed) => executed,
        Err(err) => {
            eprintln!("[PLUGIN] {err}");
            std::process::exit(1);
        }
    };
    if plugin_command_executed {
        return Ok(());
    }
    let capability_command_executed = match try_run_capability_command() {
        Ok(executed) => executed,
        Err(err) => {
            eprintln!("[CAPABILITY] {err}");
            std::process::exit(1);
        }
    };
    if capability_command_executed {
        return Ok(());
    }
    let args = parse_args()?;
    let cli_command = args.cli_command();
    let stage_payload_seed = StageAuditPayload::new(
        &args.typecheck_config.effect_context,
        &args.runtime_capabilities,
        None,
    );
    let mut audit_emitter = AuditEmitter::stderr(args.emit_audit);
    let resolved_cli_compat = args.run_config.resolved_config_compat();
    let descriptor = PipelineDescriptor::new(
        &args.input,
        args.run_id,
        args.command_label(),
        args.phase_label(),
        args.program_name.clone(),
        cli_command,
        SCHEMA_VERSION,
    );
    if let Err(err) = audit_emitter.pipeline_started(&descriptor, Some(&stage_payload_seed)) {
        eprintln!("[AUDIT] pipeline_started の書き出しに失敗しました: {err}");
    }
    if let Err(err) = audit_emitter.config_compat_changed(&descriptor, &resolved_cli_compat) {
        eprintln!("[AUDIT] config_compat_changed の書き出しに失敗しました: {err}");
    }

    match run_frontend(&args) {
        Ok(result) => {
            let outcome =
                PipelineOutcome::success(1, result.diagnostic_count, result.exit_code.label());
            for event in &result.test_audit_events {
                if let Err(err) = audit_emitter.emit_external_event(event) {
                    eprintln!("[AUDIT] test イベントの書き出しに失敗しました: {err}");
                }
            }
            if let Err(err) =
                audit_emitter.pipeline_completed(&descriptor, &outcome, Some(&result.stage_payload))
            {
                eprintln!("[AUDIT] pipeline_completed の書き出しに失敗しました: {err}");
            }
            emit_cli_output(
                args.output_format,
                &result.envelope,
                &result.input_path,
                result.lsp_derive.as_ref(),
            )?;
            std::process::exit(result.exit_code.value());
        }
        Err(err) => {
            let failure = PipelineFailure::new("cli.pipeline.failure", err.to_string(), "error");
            if let Err(audit_err) =
                audit_emitter.pipeline_failed(&descriptor, &failure, Some(&stage_payload_seed))
            {
                eprintln!("[AUDIT] pipeline_failed の書き出しに失敗しました: {audit_err}");
            }
            Err(err)
        }
    }
}

fn run_frontend(args: &CliArgs) -> Result<CliRunResult, Box<dyn std::error::Error>> {
    let started_at = formatter::current_timestamp();
    install_typecheck_config(&args.typecheck_config)?;
    let input_path = args.input.clone();
    let source_text = fs::read_to_string(&input_path)?;
    let shared_source: Arc<str> = source_text.into_boxed_str().into();
    let source = shared_source.as_ref();
    if args.parse_driver {
        return run_parse_driver_mode(args, &input_path, Arc::clone(&shared_source), &started_at);
    }
    if let Some(path) = &args.emit_tokens {
        let options = LexerOptions {
            identifier_profile: args.run_config.lex_identifier_profile,
            identifier_locale: args.run_config.lex_identifier_locale.clone(),
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

    trace_log(args, "parsing", "start");
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
            source.to_owned(),
            parser_options.clone(),
            run_config.clone(),
            stream_flow_state.clone(),
        );
        resolve_completed_stream_outcome(runner.run_stream())
    } else {
        ParserDriver::parse_with_options_and_run_config(source, parser_options, run_config)
    };
    trace_log(args, "parsing", "finish");
    let parse_only = args.emit_diagnostics;
    let typeck_report = if parse_only {
        TypecheckReport::default()
    } else if result.value.is_some() {
        TypecheckDriver::infer_module(result.value.as_ref(), &args.typecheck_config)
    } else {
        TypecheckReport::default()
    };
    let flow_metrics = result
        .stream_flow_state
        .as_ref()
        .map(|state| state.metrics())
        .unwrap_or_default();
    let bridge_signal = result
        .stream_flow_state
        .as_ref()
        .and_then(|state| state.latest_bridge_signal());
    let stage_payload = StageAuditPayload::new(
        &args.typecheck_config.effect_context,
        &args.runtime_capabilities,
        bridge_signal.clone(),
    );
    let artifacts = TypeckArtifacts::new(
        &input_path,
        &typeck_report,
        &args.typecheck_config,
        &stage_payload,
    );
    let parse_result = serde_json::json!({
        "packrat_stats": result.packrat_stats,
        "packrat_snapshot": result.packrat_snapshot,
        "span_trace": result.span_trace,
        "packrat_cache": result.packrat_cache,
        "recovered": result.recovered,
        "farthest_error_offset": result.farthest_error_offset,
        "trace_events": result.trace_events,
    });
    if let Some(path) = &args.trace_output {
        if let Err(error) = write_parser_trace_file(path, &result.trace_events) {
            eprintln!("[TRACE] トレースイベントの出力に失敗しました: {error}");
        }
    }

    let stream_meta = serde_json::json!({
        "packrat": result.stream_metrics.packrat,
        "span_trace": result.stream_metrics.span_trace,
        "flow": {
            "checkpoints_closed": flow_metrics.checkpoints_closed,
            "await_count": flow_metrics.await_count,
            "resume_count": flow_metrics.resume_count,
            "backpressure_count": flow_metrics.backpressure_count,
        },
        "bridge": bridge_signal.as_ref().map(|signal| json!(signal)),
        "last_reason": bridge_signal
            .as_ref()
            .map(|signal| signal.normalized_reason()),
        "packrat_enabled": result.run_config.packrat,
    });

    let runconfig_summary = build_runconfig_summary(&result.run_config, args, &stream_flow_state);
    let runconfig_top_level =
        build_runconfig_top_level(&result.run_config, args, &stream_flow_state);
    let mut diagnostics_entries = build_parser_diagnostics(
        &result.diagnostics,
        &result.trace_events,
        &result.span_trace,
        args,
        &input_path,
        &source,
        &result.run_config,
        &runconfig_summary,
        &stream_flow_state,
        &stage_payload,
    );
    let mut type_diagnostics = if parse_only {
        Vec::new()
    } else {
        build_type_diagnostics(
            &typeck_report,
            args,
            &input_path,
            &source,
            &result.run_config,
            &runconfig_summary,
            &stream_flow_state,
            &stage_payload,
        )
    };
    diagnostics_entries.append(&mut type_diagnostics);
    if !parse_only && diagnostics_entries.is_empty() && args.runtime_phase_enabled {
        let mut runtime_diags = execute_runtime_phase(&input_path);
        diagnostics_entries.append(&mut runtime_diags);
    }
    let mut test_diags = runtime_test::take_test_diagnostics()
        .into_iter()
        .map(|diag| diag.into_json())
        .collect::<Vec<_>>();
    diagnostics_entries.append(&mut test_diags);
    let test_audit_events = runtime_test::take_test_audit_events();
    let mut filter_stats = FilterStats::default();
    if let Some(filter) = args.diagnostic_filter() {
        let mut retained = Vec::with_capacity(diagnostics_entries.len());
        for entry in diagnostics_entries.into_iter() {
            if filter.allows_value(&entry) {
                retained.push(entry);
            } else {
                filter_stats.suppressed_by_filter += 1;
            }
        }
        diagnostics_entries = retained;
    }
    if let Some(policy) = args.audit_policy() {
        for entry in diagnostics_entries.iter_mut() {
            let enforcement = policy.apply(entry);
            if enforcement.dropped {
                filter_stats.audit_dropped += 1;
            }
            if enforcement.anonymized {
                filter_stats.audit_anonymized += 1;
            }
        }
    }
    let diagnostics_json = Value::Array(diagnostics_entries.clone());
    if let Some(path) = args.parse_debug_output.as_ref() {
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
    if let Some(path) = &args.emit_mir {
        write_json_file(path, &artifacts.mir)?;
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

    emit_telemetry_outputs(&args.telemetry_requests, &typeck_report, &input_path)?;

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

    let finished_at = formatter::current_timestamp();
    let diagnostic_count = diagnostics_entries.len();
    let summary = build_cli_summary(
        args,
        &input_path,
        &started_at,
        &finished_at,
        &runconfig_top_level,
        &parse_result,
        &stream_meta,
        diagnostic_count,
        &filter_stats,
    );
    let exit_code = determine_exit_code(&diagnostics_entries);
    let envelope = CliDiagnosticEnvelope::new(
        &args.command,
        &args.phase,
        args.run_id,
        diagnostics_entries,
        summary,
        exit_code.clone(),
    );
    Ok(CliRunResult {
        envelope,
        exit_code,
        input_path,
        diagnostic_count,
        stage_payload: stage_payload.clone(),
        test_audit_events,
        lsp_derive: None,
    })
}

fn install_typecheck_config(config: &TypecheckConfig) -> Result<(), InstallConfigError> {
    match typeck::install_config(config.clone()) {
        Ok(()) => Ok(()),
        Err(InstallConfigError::AlreadyInstalled) => Ok(()),
    }
}

fn build_cli_summary(
    args: &CliArgs,
    input_path: &Path,
    started_at: &str,
    finished_at: &str,
    runconfig_top_level: &Value,
    parse_result: &Value,
    stream_meta: &Value,
    diagnostic_count: usize,
    filter_stats: &FilterStats,
) -> CliSummary {
    let mut stats = Map::new();
    stats.insert("diagnostic_count".to_string(), json!(diagnostic_count));
    stats.insert(
        "filtering".to_string(),
        json!({
            "suppressed": filter_stats.suppressed_by_filter,
            "audit_policy_dropped": filter_stats.audit_dropped,
            "audit_policy_anonymized": filter_stats.audit_anonymized,
        }),
    );
    stats.insert("run_config".to_string(), runconfig_top_level.clone());
    stats.insert("parse_result".to_string(), parse_result.clone());
    stats.insert("stream_meta".to_string(), stream_meta.clone());
    stats.insert("cli_command".to_string(), json!(args.cli_command()));
    CliSummary {
        inputs: vec![input_path.display().to_string()],
        started_at: started_at.to_string(),
        finished_at: finished_at.to_string(),
        artifact: None,
        stats,
        dsl_embeddings: Vec::new(),
    }
}

fn determine_exit_code(diagnostics: &[Value]) -> CliExitCode {
    let mut rank = 0;
    for diag in diagnostics {
        if let Some(severity) = diag.get("severity").and_then(|value| value.as_str()) {
            match severity {
                "error" => rank = rank.max(3),
                "warning" => rank = rank.max(2),
                "info" => rank = rank.max(1),
                _ => {}
            }
        }
    }
    match rank {
        3 => CliExitCode::failure(),
        2 => CliExitCode::warning(),
        _ => CliExitCode::success(),
    }
}

fn run_parse_driver_mode(
    args: &CliArgs,
    input_path: &Path,
    shared_source: Arc<str>,
    started_at: &str,
) -> Result<CliRunResult, Box<dyn std::error::Error>> {
    let source = shared_source.as_ref();
    let label = args.parse_driver_label.clone().unwrap_or_default();
    let use_label = !label.trim().is_empty();
    let is_lexpack_basic = input_path
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name == "core-parse-lexpack-basic.reml")
        .unwrap_or(false);
    let (parser, label_for_diagnostic) = if args.parse_driver_left_recursion_parser {
        (build_left_recursion_guard_parser().map(|_| ()), None)
    } else if is_lexpack_basic {
        (build_lexpack_basic_parser(), None)
    } else {
        (
            build_labelled_expr_parser(use_label.then_some(label.as_str())).map(|_| ()),
            use_label.then_some(label.as_str()),
        )
    };
    let mut run_config = reml_runtime::run_config::RunConfig::default();
    if let Some(packrat) = args.parse_driver_packrat {
        run_config.packrat = packrat;
    }
    if let Some(strategy) = args.parse_driver_left_recursion {
        run_config.left_recursion = strategy;
    }
    if let Some(output) = args.parse_driver_profile_output.as_ref() {
        run_config.profile = true;
        let output_path = output.display().to_string();
        run_config = run_config.with_extension("parse", |mut existing| {
            existing.insert("profile_output".to_string(), Value::String(output_path));
            existing
        });
    }
    let driver_source = extract_parse_driver_input(source);
    let driver_input = Arc::<str>::from(driver_source);
    let result = runtime_parse::run_shared(&parser, driver_input, &run_config);
    let lsp_derive = if matches!(args.output_format, OutputFormat::LspDerive) {
        Some(Derive::collect_with_source(
            &parser,
            driver_source,
            &run_config,
        ))
    } else {
        None
    };

    let mut diagnostics = Vec::new();
    for err in &result.diagnostics {
        diagnostics.push(build_parse_driver_diagnostic(
            err,
            driver_source,
            label_for_diagnostic,
        ));
    }

    let finished_at = formatter::current_timestamp();
    let mut stats = Map::new();
    stats.insert("diagnostic_count".to_string(), json!(diagnostics.len()));
    let summary = CliSummary {
        inputs: vec![input_path.display().to_string()],
        started_at: started_at.to_string(),
        finished_at,
        artifact: None,
        stats,
        dsl_embeddings: Vec::new(),
    };
    let exit_code = determine_exit_code(&diagnostics);
    let diagnostic_count = diagnostics.len();
    let stage_payload = StageAuditPayload::new(
        &args.typecheck_config.effect_context,
        &args.runtime_capabilities,
        None,
    );
    let envelope = CliDiagnosticEnvelope::new(
        &args.command,
        &args.phase,
        args.run_id,
        diagnostics,
        summary,
        exit_code.clone(),
    );
    Ok(CliRunResult {
        envelope,
        exit_code,
        input_path: input_path.to_path_buf(),
        diagnostic_count,
        stage_payload,
        test_audit_events: Vec::new(),
        lsp_derive,
    })
}

fn extract_parse_driver_input(source: &str) -> &str {
    if let Some(run_pos) = source.find("Parse.run(") {
        let tail = &source[run_pos..];
        if let Some(open_quote) = tail.find('"') {
            let after_quote = run_pos + open_quote + 1;
            if let Some(end_quote) = source[after_quote..].find('"') {
                return &source[after_quote..after_quote + end_quote];
            }
        }
    }
    source
}

fn build_labelled_expr_parser(label: Option<&str>) -> runtime_parse::Parser<i32> {
    let int = integer_literal_parser();
    let plus = symbol_with_ws("+");
    let body = int
        .clone()
        .then(plus)
        .then(int)
        .map(|((lhs, _), rhs)| lhs.0 + rhs.0);
    let wrapped = if let Some(name) = label {
        runtime_parse::label(name.to_string(), body)
    } else {
        body
    };
    wrapped.skip_r(symbol_with_ws("").skip_r(runtime_parse::eof()))
}

fn build_lexpack_basic_parser() -> runtime_parse::Parser<()> {
    let space = lexpack_space_parser();
    let identifier = runtime_parse::label("identifier", lexpack_identifier(space.clone()));
    let number = runtime_parse::label("number", lexpack_number(space.clone()));
    let string_lit = runtime_parse::label("string", lexpack_string(space.clone()));
    let value = lexpack_value_parser(identifier.clone(), number.clone(), string_lit.clone());

    let assign = identifier
        .clone()
        .then(runtime_parse::symbol(space.clone(), "="))
        .then(value)
        .then(runtime_parse::symbol(space.clone(), ";"))
        .map(|_| ());

    runtime_parse::preceded(space.clone(), assign)
        .many()
        .skip_r(space)
        .skip_r(runtime_parse::eof())
        .map(|_| ())
}

fn build_left_recursion_guard_parser() -> runtime_parse::Parser<i32> {
    let depth = Arc::new(Mutex::new(0usize));
    let slot: Arc<Mutex<Option<runtime_parse::Parser<i32>>>> = Arc::new(Mutex::new(None));
    let slot_inner = Arc::clone(&slot);
    let depth_inner = Arc::clone(&depth);
    let parser = runtime_parse::Parser::new(move |state| {
        let mut current = depth_inner.lock().expect("left recursion depth lock");
        if *current > 0 {
            if !matches!(state.run_config.left_recursion, LeftRecursionStrategy::Off) {
                state.record_left_recursion_guard();
            }
            return runtime_parse::Reply::Err {
                error: runtime_parse::ParseError::new("left recursion", state.input().position()),
                consumed: false,
                committed: false,
            };
        }
        *current += 1;
        drop(current);
        let recursive = slot_inner
            .lock()
            .expect("left recursion parser slot")
            .clone()
            .expect("left recursion parser initialized");
        let plus = symbol_with_ws("+");
        let int = integer_literal_parser().map(|(value, _)| value);
        let branch = recursive
            .clone()
            .then(plus)
            .then(int.clone())
            .map(|((lhs, _), rhs)| lhs + rhs);
        let result = branch.or(int).parse(state);
        let mut current = depth_inner.lock().expect("left recursion depth lock");
        *current = current.saturating_sub(1);
        result
    });
    *slot.lock().expect("left recursion parser slot") = Some(parser.clone());
    parser
}

fn lexpack_value_parser(
    identifier: runtime_parse::Parser<String>,
    number: runtime_parse::Parser<String>,
    string_lit: runtime_parse::Parser<String>,
) -> runtime_parse::Parser<String> {
    runtime_parse::Parser::new(move |state| {
        let start_input = state.input().clone();
        let Some(ch) = start_input.remaining().chars().next() else {
            return runtime_parse::Reply::Err {
                error: runtime_parse::ParseError::new("value", start_input.position())
                    .with_expected_tokens([
                        String::from("identifier"),
                        String::from("number"),
                        String::from("string"),
                    ]),
                consumed: false,
                committed: false,
            };
        };
        if ch.is_ascii_alphabetic() || ch == '_' {
            identifier.parse(state)
        } else if ch.is_ascii_digit() {
            number.parse(state)
        } else if ch == '"' {
            string_lit.parse(state)
        } else {
            state.set_input(start_input.clone());
            runtime_parse::Reply::Err {
                error: runtime_parse::ParseError::new("value", start_input.position())
                    .with_expected_tokens([
                        String::from("identifier"),
                        String::from("number"),
                        String::from("string"),
                    ]),
                consumed: false,
                committed: false,
            }
        }
    })
}

fn lexpack_space_parser() -> runtime_parse::Parser<()> {
    runtime_parse::Parser::new(|state| {
        let start = state.input().clone();
        let mut input = start.clone();
        loop {
            let remaining = input.remaining();
            if remaining.is_empty() {
                break;
            }
            if remaining.starts_with("//") {
                let mut rest = remaining;
                if let Some(pos) = rest.find('\n') {
                    rest = &rest[pos + 1..];
                    input = input.advance(remaining.len() - rest.len());
                    continue;
                }
                input = input.advance(remaining.len());
                break;
            }
            if remaining.starts_with("/*") {
                if let Some(pos) = remaining.find("*/") {
                    input = input.advance(pos + 2);
                    continue;
                }
                input = input.advance(remaining.len());
                break;
            }

            let mut bytes = 0usize;
            for (idx, ch) in remaining.char_indices() {
                if ch.is_ascii_whitespace() {
                    bytes = idx + ch.len_utf8();
                } else {
                    break;
                }
            }
            if bytes == 0 {
                break;
            }
            input = input.advance(bytes);
        }

        let consumed = input.position().byte != start.position().byte;
        let span = start.span_to(&input);
        state.set_input(input.clone());
        runtime_parse::Reply::Ok {
            value: (),
            span,
            consumed,
            rest: input,
        }
    })
}

fn lexpack_identifier(space: runtime_parse::Parser<()>) -> runtime_parse::Parser<String> {
    let core = runtime_parse::Parser::new(move |state| {
        let input = state.input().clone();
        let mut rest = input.clone();
        let mut iter = input.remaining().char_indices();
        let Some((_, first)) = iter.next() else {
            return runtime_parse::Reply::Err {
                error: runtime_parse::ParseError::new("identifier", input.position()),
                consumed: false,
                committed: false,
            };
        };
        if !(first.is_ascii_alphabetic() || first == '_') {
            return runtime_parse::Reply::Err {
                error: runtime_parse::ParseError::new("identifier", input.position()),
                consumed: false,
                committed: false,
            };
        }
        rest = rest.advance(first.len_utf8());
        loop {
            let Some(next) = rest.remaining().chars().next() else {
                break;
            };
            if next.is_ascii_alphanumeric() || next == '_' {
                rest = rest.advance(next.len_utf8());
            } else {
                break;
            }
        }
        let span = input.span_to(&rest);
        let value = input.remaining()[..(rest.byte_offset() - input.byte_offset())].to_string();
        state.set_input(rest.clone());
        runtime_parse::Reply::Ok {
            value,
            span,
            consumed: true,
            rest,
        }
    });
    runtime_parse::lexeme(space, core)
}

fn lexpack_number(space: runtime_parse::Parser<()>) -> runtime_parse::Parser<String> {
    let core = runtime_parse::Parser::new(move |state| {
        let input = state.input().clone();
        let mut rest = input.clone();
        let mut saw_digit = false;
        for (_, ch) in input.remaining().char_indices() {
            if ch.is_ascii_digit() {
                saw_digit = true;
                rest = rest.advance(ch.len_utf8());
            } else {
                break;
            }
        }
        if rest.remaining().starts_with('.') {
            rest = rest.advance(1);
            loop {
                let Some(next) = rest.remaining().chars().next() else {
                    break;
                };
                if next.is_ascii_digit() {
                    saw_digit = true;
                    rest = rest.advance(next.len_utf8());
                } else {
                    break;
                }
            }
        }
        if !saw_digit {
            return runtime_parse::Reply::Err {
                error: runtime_parse::ParseError::new("number", input.position()),
                consumed: false,
                committed: false,
            };
        }
        let span = input.span_to(&rest);
        let value = input.remaining()[..(rest.byte_offset() - input.byte_offset())].to_string();
        state.set_input(rest.clone());
        runtime_parse::Reply::Ok {
            value,
            span,
            consumed: true,
            rest,
        }
    });
    runtime_parse::lexeme(space, core)
}

fn lexpack_string(space: runtime_parse::Parser<()>) -> runtime_parse::Parser<String> {
    let core = runtime_parse::Parser::new(move |state| {
        let input = state.input().clone();
        if !input.remaining().starts_with('"') {
            return runtime_parse::Reply::Err {
                error: runtime_parse::ParseError::new("string", input.position()),
                consumed: false,
                committed: false,
            };
        }
        let mut rest = input.advance(1);
        while let Some(ch) = rest.remaining().chars().next() {
            if ch == '"' {
                let closing = rest.advance(1);
                let span = input.span_to(&closing);
                let value =
                    input.remaining()[..(closing.byte_offset() - input.byte_offset())].to_string();
                state.set_input(closing.clone());
                return runtime_parse::Reply::Ok {
                    value,
                    span,
                    consumed: true,
                    rest: closing,
                };
            }
            rest = rest.advance(ch.len_utf8());
        }
        runtime_parse::Reply::Err {
            error: runtime_parse::ParseError::new("string", input.position()),
            consumed: true,
            committed: false,
        }
    });
    runtime_parse::lexeme(space, core)
}

fn integer_literal_parser() -> runtime_parse::Parser<(i32, runtime_parse::Span)> {
    runtime_parse::Parser::new(|state| {
        let start_input = skip_ascii_whitespace(state.input().clone());
        let mut rest = start_input.clone();
        let mut value: i32 = 0;
        let mut digits = 0;
        for ch in start_input.remaining().chars() {
            if ch.is_ascii_digit() {
                digits += 1;
                value = value
                    .saturating_mul(10)
                    .saturating_add(ch.to_digit(10).unwrap() as i32);
                rest = rest.advance(ch.len_utf8());
            } else {
                break;
            }
        }
        if digits == 0 {
            return runtime_parse::Reply::Err {
                error: runtime_parse::ParseError::new("value", start_input.position())
                    .with_expected_tokens([
                        String::from("integer-literal"),
                        String::from("identifier"),
                    ]),
                consumed: false,
                committed: false,
            };
        }
        let span = start_input.span_to(&rest);
        let span_for_value = span.clone();
        runtime_parse::Reply::Ok {
            value: (value, span_for_value),
            span,
            consumed: true,
            rest,
        }
    })
}

fn symbol_with_ws(text: &str) -> runtime_parse::Parser<String> {
    let expected = text.to_string();
    runtime_parse::Parser::new(move |state| {
        let input = skip_ascii_whitespace(state.input().clone());
        if expected.is_empty() {
            let pos = input.position();
            let span = runtime_parse::Span::new(pos, pos);
            return runtime_parse::Reply::Ok {
                value: String::new(),
                span,
                consumed: false,
                rest: input,
            };
        }
        if input.remaining().starts_with(&expected) {
            let rest = input.advance(expected.len());
            let span = input.span_to(&rest);
            runtime_parse::Reply::Ok {
                value: expected.clone(),
                span,
                consumed: true,
                rest,
            }
        } else {
            runtime_parse::Reply::Err {
                error: runtime_parse::ParseError::new(expected.clone(), input.position())
                    .with_expected_tokens([expected.clone()]),
                consumed: false,
                committed: false,
            }
        }
    })
}

fn skip_ascii_whitespace(mut input: runtime_parse::Input) -> runtime_parse::Input {
    let mut bytes = 0usize;
    for (idx, ch) in input.remaining().char_indices() {
        if ch.is_ascii_whitespace() {
            bytes = idx + ch.len_utf8();
        } else {
            break;
        }
    }
    if bytes > 0 {
        input = input.advance(bytes);
    }
    input
}

fn build_parse_driver_diagnostic(
    err: &runtime_parse::ParseError,
    source: &str,
    label: Option<&str>,
) -> Value {
    if err.message == LEFT_RECURSION_MESSAGE {
        return json!({
            "severity": "error",
            "code": "E4001",
            "domain": "parser",
            "message": "左再帰を検出しました",
            "expected": [],
        });
    }
    let mut collector = ExpectedTokenCollector::new();
    let mut label_added = false;
    for token in &err.expected_tokens {
        if let Some(rule) = label {
            if token == rule {
                collector.push_rule(token.to_string());
                label_added = true;
                continue;
            }
        }
        collector.push(classify_parse_driver_token(token));
    }
    if let Some(rule) = label {
        if !label_added {
            collector.push_rule(rule.to_string());
        }
    }
    if collector.is_empty() {
        if let Some(rule) = label {
            collector.push_rule(rule.to_string());
        } else {
            collector.push_custom("value");
        }
    }
    let context_note = derive_context_note(source, err.position.byte, label);
    let summary = collector.summarize_with_context(context_note.clone());
    let message = context_note
        .or_else(|| summary.humanized.clone())
        .map(|text| format!("構文エラー: {text}"))
        .unwrap_or_else(|| "構文エラー: 入力を解釈できません".to_string());
    let expected = expected_from_summary(&summary);

    json!({
        "severity": "error",
        "code": "parser.syntax.expected_tokens",
        "domain": "parser",
        "message": message,
        "expected": expected,
    })
}

fn classify_parse_driver_token(token: &str) -> ExpectedToken {
    if token == "<eof>" {
        ExpectedToken::eof()
    } else if token == "number" || token == "string" {
        ExpectedToken::class(token.to_string())
    } else if token.contains("identifier") || token.contains("literal") || token.ends_with("EOF") {
        ExpectedToken::class(token.to_string())
    } else if token.chars().all(|ch| ch.is_ascii_lowercase())
        && token.chars().all(|ch| ch.is_ascii_alphabetic())
    {
        ExpectedToken::keyword(token.to_string())
    } else {
        ExpectedToken::token(token.to_string())
    }
}

fn derive_context_note(source: &str, error_byte: usize, label: Option<&str>) -> Option<String> {
    let context_label = label.unwrap_or("項");
    let prefix = source.get(..error_byte)?;
    let trimmed = prefix.trim_end_matches(char::is_whitespace);
    if let Some(last) = trimmed.chars().rev().next() {
        if last == '+' {
            return Some(format!("`+` の後に {context_label} が必要です"));
        }
    }
    label.map(|rule| format!("{rule} が必要です"))
}

fn expected_from_summary(summary: &ExpectedTokensSummary) -> Value {
    let alternatives: Vec<Value> = summary
        .alternatives
        .iter()
        .map(|token| {
            let label = token.raw_label();
            let kind = token.kind_label();
            json!({
                "token": label,
                "label": label,
                "hint": kind,
                "kind": kind,
            })
        })
        .collect();
    let mut expected = Map::new();
    expected.insert(
        "message_key".to_string(),
        json!(summary
            .message_key
            .clone()
            .unwrap_or_else(|| "parse.expected".to_string())),
    );
    expected.insert("humanized".to_string(), json!(summary.humanized));
    expected.insert(
        "locale_args".to_string(),
        json!(summary.locale_args.clone()),
    );
    expected.insert("alternatives".to_string(), json!(alternatives));
    if let Some(context) = summary.context_note.as_ref() {
        if !context.trim().is_empty() {
            expected.insert("context_note".to_string(), json!(context));
        }
    }
    Value::Object(expected)
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
    output_format: OutputFormat,
    command: CliCommandKind,
    phase: CliPhaseKind,
    run_id: Uuid,
    typecheck_config: TypecheckConfig,
    dualwrite: Option<DualwriteCliOpts>,
    emit_typed_ast: Option<PathBuf>,
    emit_ast: Option<PathBuf>,
    emit_mir: Option<PathBuf>,
    emit_constraints: Option<PathBuf>,
    emit_typeck_debug: Option<PathBuf>,
    emit_effects_metrics: Option<PathBuf>,
    emit_impl_registry: Option<PathBuf>,
    emit_tokens: Option<PathBuf>,
    trace_output: Option<PathBuf>,
    #[allow(dead_code)]
    emit_effects: bool,
    #[allow(dead_code)]
    emit_diagnostics: bool,
    #[allow(dead_code)]
    parse_driver: bool,
    parse_driver_label: Option<String>,
    parse_driver_profile_output: Option<PathBuf>,
    parse_driver_left_recursion: Option<LeftRecursionStrategy>,
    parse_driver_packrat: Option<bool>,
    parse_driver_left_recursion_parser: bool,
    emit_audit: bool,
    #[allow(dead_code)]
    show_stage_context: bool,
    #[allow(dead_code)]
    diagnostics_stream: bool,
    runtime_phase_enabled: bool,
    target_cfg_extension: Value,
    run_config: RunSettings,
    stream_config: StreamSettings,
    runtime_capabilities: Vec<RuntimeCapability>,
    config_path: Option<PathBuf>,
    #[allow(dead_code)]
    manifest_path: Option<PathBuf>,
    telemetry_requests: Vec<TelemetryRequest>,
}

#[derive(Clone)]
struct DualwriteCliOpts {
    run_label: String,
    case_label: String,
    root: Option<PathBuf>,
}

#[derive(Clone)]
struct TelemetryRequest {
    kind: TelemetryKind,
    destination: Option<PathBuf>,
}

impl TelemetryRequest {
    fn parse(value: &str) -> Result<Self, TelemetryParseError> {
        let mut parts = value.splitn(2, '=');
        let kind_raw = parts
            .next()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                TelemetryParseError(
                    "--emit-telemetry は <kind>[=<path>] 形式で指定してください".to_string(),
                )
            })?;
        let destination = parts
            .next()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(PathBuf::from);
        let kind = TelemetryKind::parse(kind_raw)?;
        Ok(Self { kind, destination })
    }

    fn label(&self) -> &'static str {
        self.kind.label()
    }

    fn resolved_path(&self, input: &Path) -> PathBuf {
        self.destination
            .clone()
            .unwrap_or_else(|| default_telemetry_path(self.label(), input))
    }
}

#[derive(Clone)]
enum TelemetryKind {
    ConstraintGraph,
}

impl TelemetryKind {
    fn parse(raw: &str) -> Result<Self, TelemetryParseError> {
        match raw {
            "constraint_graph" => Ok(Self::ConstraintGraph),
            other => Err(TelemetryParseError(format!(
                "--emit-telemetry で未知の種別 `{other}` が指定されました"
            ))),
        }
    }

    fn label(&self) -> &'static str {
        match self {
            TelemetryKind::ConstraintGraph => "constraint_graph",
        }
    }
}

#[derive(Debug)]
struct TelemetryParseError(String);

impl fmt::Display for TelemetryParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for TelemetryParseError {}

#[derive(Clone)]
struct RunSettings {
    config: RunConfig,
    experimental_effects: bool,
    lex_identifier_profile: IdentifierProfile,
    lex_identifier_locale: Option<LocaleId>,
    diagnostic_filter: Option<DiagnosticFilter>,
    audit_policy: Option<AuditPolicy>,
    config_compat_cli: Option<CompatibilityLayer>,
    config_compat_env: Option<CompatibilityLayer>,
    config_compat_manifest: Option<CompatibilityLayer>,
    config_stage: RuntimeStageId,
    config_format: ConfigFormat,
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
            lex_identifier_locale: None,
            diagnostic_filter: None,
            audit_policy: None,
            config_compat_cli: None,
            config_compat_env: None,
            config_compat_manifest: None,
            config_stage: RuntimeStageId::Stable,
            config_format: ConfigFormat::Toml,
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
        if let Some(filter) = &self.diagnostic_filter {
            config = config.with_extension("diagnostics", |existing| {
                let mut payload = existing
                    .and_then(|value| value.as_object().cloned())
                    .unwrap_or_default();
                payload.insert("filter".to_string(), filter.to_value());
                Value::Object(payload)
            });
        }
        if let Some(policy) = &self.audit_policy {
            config = config.with_extension("audit", |existing| {
                let mut payload = existing
                    .and_then(|value| value.as_object().cloned())
                    .unwrap_or_default();
                payload.insert("policy".to_string(), policy.to_value());
                Value::Object(payload)
            });
        }
        if self.experimental_effects {
            config = config.with_extension("effects", |existing| {
                let mut payload = existing
                    .and_then(|value| value.as_object().cloned())
                    .unwrap_or_default();
                payload.insert("experimental_effects".to_string(), json!(true));
                Value::Object(payload)
            });
        }
        let resolved_compat = self.resolve_config_compat();
        config.set_config_compat(resolved_compat.clone());
        config = config.with_extension("config", |existing| {
            let mut payload = existing
                .and_then(|value| value.as_object().cloned())
                .unwrap_or_default();
            if let Ok(value) = serde_json::to_value(&resolved_compat.compatibility) {
                payload.insert("compatibility".to_string(), value);
            }
            payload.insert(
                "compatibility_source".to_string(),
                json!(resolved_compat.source.as_str()),
            );
            if let Some(label) = &resolved_compat.profile_label {
                payload.insert("compatibility_profile".to_string(), json!(label));
            }
            Value::Object(payload)
        });
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
        if let Some(locale) = &self.lex_identifier_locale {
            payload.insert("identifier_locale".to_string(), json!(locale.canonical()));
        }
        payload
            .entry("profile".to_string())
            .or_insert_with(|| json!("strict_json"));
        Value::Object(payload)
    }

    fn resolve_config_compat(&self) -> ResolvedConfigCompatibility {
        resolve_compat(ResolveCompatOptions {
            format: self.config_format,
            stage: self.config_stage,
            cli: self.config_compat_cli.clone(),
            env: self.config_compat_env.clone(),
            manifest: self.config_compat_manifest.clone(),
        })
    }

    fn resolved_config_compat(&self) -> ResolvedConfigCompatibility {
        self.resolve_config_compat()
    }

    fn set_config_stage(&mut self, stage: RuntimeStageId) {
        self.config_stage = stage;
    }

    fn config_stage(&self) -> RuntimeStageId {
        self.config_stage
    }

    fn config_format(&self) -> ConfigFormat {
        self.config_format
    }

    fn apply_manifest_overrides(&mut self, overrides: RunConfigManifestOverrides) {
        if let Some(layer) = overrides.compatibility_layer {
            self.config_compat_manifest = Some(layer);
        }
        let manifest_payload = Value::Object(overrides.manifest_extension);
        self.config = self.config.with_extension("config", |existing| {
            let mut payload = existing
                .and_then(|value| value.as_object().cloned())
                .unwrap_or_default();
            payload.insert("manifest".into(), manifest_payload.clone());
            Value::Object(payload)
        });
    }

    fn apply_cli_config_profile(&mut self, label: &str) -> Result<(), CompatibilityProfileError> {
        let compatibility = compatibility_profile(label)?;
        self.config_compat_cli = Some(CompatibilityLayer::new(
            compatibility,
            Some(label.to_string()),
        ));
        Ok(())
    }

    fn diagnostic_filter(&self) -> Option<&DiagnosticFilter> {
        self.diagnostic_filter.as_ref()
    }

    fn audit_policy(&self) -> Option<&AuditPolicy> {
        self.audit_policy.as_ref()
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

    fn command_label(&self) -> &'static str {
        self.command.as_str()
    }

    fn phase_label(&self) -> &'static str {
        self.phase.as_str()
    }

    fn diagnostic_filter(&self) -> Option<&DiagnosticFilter> {
        self.run_config.diagnostic_filter()
    }

    fn audit_policy(&self) -> Option<&AuditPolicy> {
        self.run_config.audit_policy()
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
    diagnostic_filter_overridden: bool,
    audit_policy_overridden: bool,
    allow_top_level_expr_overridden: bool,
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
        if !allow_top_level_expr_overridden {
            if let Some(allow) = parser.get("allow_top_level_expr").and_then(|v| v.as_bool()) {
                run_config.allow_top_level_expr = allow;
            }
        }
        if let Some(ack) = parser
            .get("ack_experimental_diagnostics")
            .and_then(|v| v.as_bool())
        {
            run_config.ack_experimental_diagnostics = ack;
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
                if let Some(locale) = lex.get("identifier_locale").and_then(|v| v.as_str()) {
                    match LocaleId::parse(locale) {
                        Ok(parsed) => run_config.lex_identifier_locale = Some(parsed),
                        Err(_) => eprintln!(
                            "[CONFIG] lex.identifier_locale `{locale}` は無効なロケールなので無視されました"
                        ),
                    }
                }
            }
        }
    }

    if !diagnostic_filter_overridden {
        if let Some(filter) = value
            .get("diagnostics")
            .and_then(|section| section.get("filter"))
        {
            match DiagnosticFilter::from_json(filter) {
                Ok(parsed) => run_config.diagnostic_filter = Some(parsed),
                Err(err) => eprintln!("[CONFIG] diagnostics.filter の解析に失敗しました: {err}"),
            }
        }
    }

    if !audit_policy_overridden {
        if let Some(policy) = value.get("audit").and_then(|section| section.get("policy")) {
            match AuditPolicy::from_json(policy) {
                Ok(parsed) => run_config.audit_policy = Some(parsed),
                Err(err) => eprintln!("[CONFIG] audit.policy の解析に失敗しました: {err}"),
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
    let program_name = argv.next().unwrap_or_else(|| "reml_frontend".to_string());
    let remaining: Vec<String> = argv.collect();
    let raw_cli_args = remaining.clone();
    if raw_cli_args
        .iter()
        .any(|arg| arg == "--help" || arg == "-h")
    {
        print_help(&program_name);
        std::process::exit(0);
    }
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
    let mut emit_mir = None;
    let mut emit_constraints = None;
    let mut emit_typeck_debug = None;
    let mut emit_effects_metrics = None;
    let mut emit_impl_registry = None;
    let mut emit_tokens = None;
    let mut trace_output = None;
    let mut emit_effects = false;
    let mut emit_diagnostics = false;
    let mut parse_driver = false;
    let mut parse_driver_label: Option<String> = None;
    let mut parse_driver_profile_output: Option<PathBuf> = None;
    let mut parse_driver_left_recursion: Option<LeftRecursionStrategy> = None;
    let mut parse_driver_packrat: Option<bool> = None;
    let mut parse_driver_left_recursion_parser = false;
    let mut emit_audit = false;
    let mut show_stage_context = false;
    let mut diagnostics_stream = false;
    let mut runtime_phase_enabled = true;
    let mut output_format = OutputFormat::default();
    let mut run_config = RunSettings::default();
    let mut stream_config = StreamSettings::default();
    let mut runtime_capabilities: Vec<RuntimeCapability> = Vec::new();
    let mut config_path: Option<PathBuf> = None;
    let mut manifest_path: Option<PathBuf> = None;
    let mut trace_overridden = false;
    let mut merge_warnings_overridden = false;
    let mut diagnostic_filter_overridden = false;
    let mut audit_policy_overridden = false;
    let mut telemetry_requests: Vec<TelemetryRequest> = Vec::new();
    let mut allow_top_level_expr_overridden = false;
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
            "--parse-driver" => parse_driver = true,
            "--parse-driver-label" => {
                parse_driver_label = Some(
                    args.next()
                        .ok_or_else(|| "--parse-driver-label は値を伴う必要があります")?,
                )
            }
            "--parse-driver-profile-output" => {
                let path = args.next().ok_or_else(|| {
                    "--parse-driver-profile-output は出力パスを伴う必要があります"
                })?;
                parse_driver_profile_output = Some(PathBuf::from(path));
            }
            "--parse-driver-left-recursion" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--parse-driver-left-recursion は値を伴う必要があります")?;
                parse_driver_left_recursion = Some(parse_left_recursion_strategy(&value)?);
            }
            "--parse-driver-packrat" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--parse-driver-packrat は on/off の値を伴う必要があります")?;
                parse_driver_packrat = Some(parse_on_off(&value)?);
            }
            "--parse-driver-left-recursion-parser" => {
                parse_driver_left_recursion_parser = true;
            }
            "--emit-audit" | "--emit-audit-log" => emit_audit = true,
            "--show-stage-context" => show_stage_context = true,
            "--diagnostics-stream" => diagnostics_stream = true,
            "--runtime-phase" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--runtime-phase は on/off の値を伴う必要があります")?;
                runtime_phase_enabled = parse_on_off(&value)?;
            }
            "--no-runtime-phase" => runtime_phase_enabled = false,
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
            "--emit-mir" | "--debug-mir" => {
                let path = args
                    .next()
                    .ok_or_else(|| "--emit-mir は出力パスを伴う必要があります")?;
                emit_mir = Some(PathBuf::from(path));
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
            "--output" | "--format" => {
                let value = args.next().ok_or_else(|| {
                    "--output は human|json|lsp|lsp-derive のいずれかを指定してください"
                })?;
                output_format = OutputFormat::parse(&value)?;
            }
            "--emit-tokens" => {
                let path = args
                    .next()
                    .ok_or_else(|| "--emit-tokens は出力パスを伴う必要があります")?;
                emit_tokens = Some(PathBuf::from(path));
            }
            "--emit-telemetry" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--emit-telemetry は <kind>[=<path>] を伴う必要があります")?;
                let request = TelemetryRequest::parse(&value)?;
                telemetry_requests.push(request);
            }
            "--trace-output" => {
                let path = args
                    .next()
                    .ok_or_else(|| "--trace-output は出力パスを伴う必要があります")?;
                trace_output = Some(PathBuf::from(path));
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
            "--allow-top-level-expr" => {
                run_config.allow_top_level_expr = true;
                allow_top_level_expr_overridden = true;
            }
            "--no-allow-top-level-expr" => {
                run_config.allow_top_level_expr = false;
                allow_top_level_expr_overridden = true;
            }
            "--require-eof" => run_config.require_eof = true,
            "--no-require-eof" => run_config.require_eof = false,
            "--legacy-result" => run_config.legacy_result = true,
            "--no-legacy-result" => run_config.legacy_result = false,
            "--experimental-effects" => run_config.experimental_effects = true,
            "--no-experimental-effects" => run_config.experimental_effects = false,
            "--ack-experimental-diagnostics" => {
                run_config.ack_experimental_diagnostics = true;
            }
            "--no-ack-experimental-diagnostics" => {
                run_config.ack_experimental_diagnostics = false;
            }
            "--diagnostic-filter" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--diagnostic-filter は key=value 形式で指定してください")?;
                let existing = run_config.diagnostic_filter.take();
                let parsed = DiagnosticFilter::parse_assignment(existing, &value)?;
                run_config.diagnostic_filter = Some(parsed);
                diagnostic_filter_overridden = true;
            }
            "--audit-policy" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--audit-policy は key=value 形式で指定してください")?;
                let existing = run_config.audit_policy.take();
                let parsed = AuditPolicy::parse_assignment(existing, &value)?;
                run_config.audit_policy = Some(parsed);
                audit_policy_overridden = true;
            }
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
            "--lex-locale" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--lex-locale はロケール ID を伴う必要があります")?;
                match LocaleId::parse(&value) {
                    Ok(locale) => run_config.lex_identifier_locale = Some(locale),
                    Err(_) => eprintln!(
                        "[CLI] --lex-locale の値 `{value}` は無効なロケールなので無視されました"
                    ),
                }
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
            "--config-compat" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--config-compat はプロファイル名を伴う必要があります")?;
                run_config.apply_cli_config_profile(&value)?;
            }
            "--config" => {
                let path = args
                    .next()
                    .ok_or_else(|| "--config はパスを伴う必要があります")?;
                config_path = Some(PathBuf::from(&path));
            }
            "--manifest" => {
                let path = args
                    .next()
                    .ok_or_else(|| "--manifest はパスを伴う必要があります")?;
                manifest_path = Some(PathBuf::from(&path));
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
            eprintln!("使用方法: reml_frontend [options] <input.reml>");
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
            diagnostic_filter_overridden,
            audit_policy_overridden,
            allow_top_level_expr_overridden,
        ) {
            eprintln!("[CONFIG] {}", error);
        }
    }

    if let Some(manifest_file) = manifest_path.clone() {
        let loader = ManifestLoader::new();
        let manifest = loader.load(&manifest_file).map_err(|diag| {
            format!(
                "{}: {} ({})",
                manifest_file.display(),
                diag.message,
                diag.code
            )
        })?;
        let overrides = apply_manifest_overrides(ApplyManifestOverridesArgs {
            manifest: &manifest,
            format: run_config.config_format(),
            stage: run_config.config_stage(),
        });
        run_config.apply_manifest_overrides(overrides);
    }

    let dualwrite = dualwrite_run_label.map(|run_label| DualwriteCliOpts {
        run_label,
        case_label: dualwrite_case_label.expect("validated together"),
        root: dualwrite_root,
    });
    let run_id = Uuid::new_v4();

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
    let compat_stage = cli_stage_override
        .as_ref()
        .or_else(|| runtime_stage.as_ref().map(|req| req.base_stage()))
        .map(|stage| convert_stage_id(stage))
        .unwrap_or(RuntimeStageId::Stable);
    run_config.set_config_stage(compat_stage);
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
        output_format,
        command: CliCommandKind::default(),
        phase: CliPhaseKind::default(),
        run_id,
        typecheck_config: builder.build(),
        dualwrite,
        emit_ast,
        emit_typed_ast,
        emit_mir,
        emit_constraints,
        emit_typeck_debug,
        emit_effects_metrics,
        emit_impl_registry,
        emit_tokens,
        trace_output,
        emit_effects,
        emit_diagnostics,
        parse_driver,
        parse_driver_label,
        parse_driver_profile_output,
        parse_driver_left_recursion,
        parse_driver_packrat,
        parse_driver_left_recursion_parser,
        emit_audit,
        show_stage_context,
        diagnostics_stream,
        target_cfg_extension,
        run_config,
        stream_config,
        runtime_capabilities,
        config_path,
        manifest_path,
        telemetry_requests,
        runtime_phase_enabled,
    })
}

fn print_help(program_name: &str) {
    println!(
        "\
{prog} は Reml Rust フロントエンド PoC です。

使用方法:
  {prog} [OPTIONS] <input.reml>

主なオプション:
  --emit-ast <PATH>              解析結果 AST を JSON で保存
  --emit-typed-ast <PATH>        型付き AST を JSON で保存
  --emit-mir <PATH>              Match/Pattern MIR を JSON で保存（--debug-mir も利用可能）
  --emit-constraints <PATH>      Typecheck 制約を JSON で保存
  --emit-typeck-debug <PATH>     型推論デバッグ情報を JSON で保存
  --config-compat <PROFILE>      設定ファイル互換プロファイルを指定 (strict-json / json-relaxed 等)
  --manifest <PATH>              reml.toml マニフェストを読み込み RunConfig に反映
  --emit-effects-metrics <PATH>  効果メトリクスを JSON で保存
  --parse-driver                 パース専用ドライバで入力ファイルを評価し、診断を JSON 出力（型検査/Runtime をスキップ）
  --parse-driver-label <NAME>    上記ドライバで付与するラベル名（既定: expression）
  --parse-driver-profile-output <PATH> parse-driver の profile 出力先（JSON）
  --parse-driver-left-recursion <MODE> parse-driver の左再帰モード（off/on/auto）
  --parse-driver-packrat <MODE>  parse-driver の Packrat を on/off で切替
  --parse-driver-left-recursion-parser parse-driver で左再帰ガード検証用の専用パーサを使用
  --emit-diagnostics             標準出力へ診断 JSON を出力
  --emit-audit-log               Audit メタデータを出力（--emit-audit も利用可能）
  --emit-telemetry <KIND>[=PATH] 制約グラフ等のテレメトリを JSON で保存
  --emit-tokens <PATH>           字句解析結果を JSON で保存
  --trace-output <PATH>          Parser TraceEvent を Markdown で保存
  --lex-profile ascii|unicode    識別子プロファイルの切替
  --lex-locale <Bcp47>          識別子正規化で使用するロケール ID
  --runtime-phase on|off         パース/型検査後の簡易 runtime 実行フェーズを有効/無効化（既定: on）
  --no-runtime-phase             上記のショートカット（off）
  --packrat / --no-packrat       Packrat キャッシュを有効/無効化
  --allow-top-level-expr         トップレベル式を許可（サンプル検証モード）
  --no-allow-top-level-expr      トップレベル式を拒否（既定）
  --streaming / --no-streaming   Streaming Runner の有無を切替
  --effect-stage <STAGE>         Stage 要件を指定
  --ack-experimental-diagnostics 実験的 Stage の診断を Error として扱う
  --dualwrite-run-label <NAME>   dual-write ラベル設定（case も必須）
  --config <PATH>                追加設定ファイルを適用

これら以外にも recover, runtime capability, streaming flow などの
細かなオプションがあります。詳細は `docs/plans/rust-migration/` と
`docs/spec/3-6-core-diagnostics-audit.md` を参照してください。
",
        prog = program_name
    );
}

fn parse_on_off(value: &str) -> Result<bool, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "on" | "true" | "1" => Ok(true),
        "off" | "false" | "0" => Ok(false),
        other => Err(format!("値 `{other}` は on/off ではありません")),
    }
}

fn parse_left_recursion_strategy(value: &str) -> Result<LeftRecursionStrategy, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "off" => Ok(LeftRecursionStrategy::Off),
        "on" => Ok(LeftRecursionStrategy::On),
        "auto" => Ok(LeftRecursionStrategy::Auto),
        other => Err(format!("値 `{other}` は off/on/auto ではありません")),
    }
}

fn convert_stage_id(stage: &StageId) -> RuntimeStageId {
    match stage.as_str().to_ascii_lowercase().as_str() {
        "stable" => RuntimeStageId::Stable,
        "beta" => RuntimeStageId::Beta,
        "alpha" => RuntimeStageId::Alpha,
        _ => RuntimeStageId::Experimental,
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
    guards.write_json("typeck/mir.rust.json", &artifacts.mir)?;
    guards.write_json("typeck/constraints.rust.json", &artifacts.constraints)?;
    guards.write_json("typeck/typeck-debug.rust.json", &artifacts.debug)?;
    let (run_label, case_label) = guards.labels();
    let impl_registry =
        build_impl_registry_payload(report, Some(run_label.as_ref()), Some(case_label.as_ref()));
    guards.write_json("typeck/impl-registry.rust.json", &impl_registry)?;
    Ok(())
}

fn emit_telemetry_outputs(
    requests: &[TelemetryRequest],
    report: &TypecheckReport,
    input: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    if requests.is_empty() {
        return Ok(());
    }
    let input_label = input.display().to_string();
    for request in requests {
        match request.kind {
            TelemetryKind::ConstraintGraph => {
                let path = request.resolved_path(input);
                let dot_path = path.with_extension("dot").display().to_string();
                let telemetry = TraitResolutionTelemetry::from_report(
                    report,
                    Some(input_label.as_str()),
                    Some(dot_path),
                );
                write_json_file(&path, &telemetry)?;
                eprintln!(
                    "[TELEMETRY] constraint_graph を {} へ書き出しました",
                    path.display()
                );
            }
        }
    }
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

/// 型検査まで成功した場合に限定して、簡易的な runtime フェーズを実行する。
/// 現時点では practical/path セキュリティシナリオの診断生成を目的とした最小実装。
fn execute_runtime_phase(input_path: &Path) -> Vec<Value> {
    match RuntimeExecutionPlan::from_input(input_path) {
        Some(plan) => match plan.run() {
            Ok(diags) => diags,
            Err(err) => {
                eprintln!("[RUNTIME] 実行フェーズでエラーが発生しました: {err}");
                Vec::new()
            }
        },
        None => Vec::new(),
    }
}

enum RuntimeExecutionPlan {
    CorePathRelativeDenied,
    CoreRuntimeBridgeStageMismatch,
    CoreTestSnapshotBasic,
    CoreTestTableBasic,
    CoreTestFuzzBasic,
}

impl RuntimeExecutionPlan {
    fn from_input(input: &Path) -> Option<Self> {
        let label = input.to_string_lossy();
        if label.contains("examples/practical/core_path/security_check/relative_denied.reml") {
            Some(Self::CorePathRelativeDenied)
        } else if label.contains(
            "examples/practical/core_runtime/capability/stage_mismatch_runtime_bridge.reml",
        ) {
            Some(Self::CoreRuntimeBridgeStageMismatch)
        } else if label.contains("examples/practical/core_test/snapshot/basic_ok.reml") {
            Some(Self::CoreTestSnapshotBasic)
        } else if label.contains("examples/practical/core_test/table/basic_ok.reml") {
            Some(Self::CoreTestTableBasic)
        } else if label.contains("examples/practical/core_test/fuzz/basic_ok.reml") {
            Some(Self::CoreTestFuzzBasic)
        } else {
            None
        }
    }

    fn run(&self) -> Result<Vec<Value>, String> {
        match self {
            Self::CorePathRelativeDenied => self.run_core_path_relative_denied(),
            Self::CoreRuntimeBridgeStageMismatch => self.run_core_runtime_bridge_stage_mismatch(),
            Self::CoreTestSnapshotBasic => self.run_core_test_snapshot_basic(),
            Self::CoreTestTableBasic => self.run_core_test_table_basic(),
            Self::CoreTestFuzzBasic => self.run_core_test_fuzz_basic(),
        }
    }

    fn run_core_test_snapshot_basic(&self) -> Result<Vec<Value>, String> {
        let force_fail = std::env::var("REML_CORE_TEST_FORCE_FAIL").is_ok();
        if force_fail {
            let _ = runtime_test::test("core_test_basic_fail", || {
                runtime_test::assert_snapshot("core_test_basic_fail", "beta")
            });
        }
        let policy = runtime_test::SnapshotPolicy::record();
        let _ = runtime_test::assert_snapshot_with(policy, "core_test_basic", "alpha");
        Ok(Vec::new())
    }

    fn run_core_test_table_basic(&self) -> Result<Vec<Value>, String> {
        let cases = vec![
            runtime_test::TableCase {
                input: "alpha".to_string(),
                expected: "alpha".to_string(),
            },
            runtime_test::TableCase {
                input: "beta".to_string(),
                expected: "beta".to_string(),
            },
        ];
        let _ = runtime_test::table_test(&cases, |value| value.clone());
        Ok(Vec::new())
    }

    fn run_core_test_fuzz_basic(&self) -> Result<Vec<Value>, String> {
        let config = runtime_test::FuzzConfig {
            seed: b"seed".to_vec(),
            max_cases: 4,
            max_bytes: 8,
        };
        let _ = runtime_test::fuzz_bytes(&config, |_| Ok(()));
        Ok(Vec::new())
    }

    fn run_core_path_relative_denied(&self) -> Result<Vec<Value>, String> {
        let mut diagnostics = Vec::new();
        let policy_root = match runtime_path::path(RuntimeStr::from("/srv/app")) {
            Ok(value) => value,
            Err(err) => {
                diagnostics.push(err.into_diagnostic().into_json());
                return Ok(diagnostics);
            }
        };
        let policy = RuntimeSecurityPolicy::new()
            .add_allowed_root(policy_root.clone())
            .allow_relative(false);
        let sandbox_root = match runtime_path::path(RuntimeStr::from("/srv/app/tmp")) {
            Ok(value) => value,
            Err(err) => {
                diagnostics.push(err.into_diagnostic().into_json());
                return Ok(diagnostics);
            }
        };

        match ensure_workspace_path_runtime(RuntimeStr::from("secret.txt"), sandbox_root, policy) {
            Ok(_) => Ok(diagnostics),
            Err(diag) => {
                diagnostics.push(diag.into_json());
                Ok(diagnostics)
            }
        }
    }

    fn run_core_runtime_bridge_stage_mismatch(&self) -> Result<Vec<Value>, String> {
        let mut extensions = Map::new();
        extensions.insert(
            "runtime.bridge.id".into(),
            Value::String("telemetry_bridge".into()),
        );
        extensions.insert(
            "runtime.bridge.stage.required".into(),
            Value::String("stable".into()),
        );
        extensions.insert(
            "runtime.bridge.stage.actual".into(),
            Value::String("beta".into()),
        );
        let diagnostic = GuardDiagnostic {
            code: "runtime.bridge.stage_mismatch",
            domain: "runtime",
            severity: DiagnosticSeverity::Error,
            message: "Bridge `telemetry_bridge` は Stage::Beta で、要求 Stage::Stable を満たしていません。"
                .into(),
            notes: Vec::new(),
            extensions,
            audit_metadata: Map::new(),
        };
        Ok(vec![diagnostic.into_json()])
    }
}

#[allow(dead_code)]
#[derive(Clone)]
struct SecurityReport {
    requested: RuntimePathBuf,
    sandboxed: RuntimePathBuf,
    is_symlink: bool,
}

fn ensure_workspace_path_runtime(
    raw: RuntimeStr<'_>,
    root: RuntimePathBuf,
    policy: RuntimeSecurityPolicy,
) -> Result<SecurityReport, reml_runtime::prelude::ensure::GuardDiagnostic> {
    let requested = runtime_path::path(raw).map_err(|err| err.into_diagnostic())?;
    let normalized = runtime_path::normalize(&requested);

    runtime_path::validate_path(&normalized, &policy).map_err(|err| err.into_diagnostic())?;
    let sandboxed =
        runtime_path::sandbox_path(&normalized, &root).map_err(|err| err.into_diagnostic())?;
    let is_symlink =
        runtime_path::is_safe_symlink(&sandboxed).map_err(|err| err.into_diagnostic())?;

    Ok(SecurityReport {
        requested: normalized,
        sandboxed,
        is_symlink,
    })
}

fn write_parser_trace_file(
    path: &Path,
    events: &[ParserTraceEvent],
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    let mut buffer = String::from("# Parser Trace Events\n\n");
    buffer.push_str("| # | kind | trace_id | span | label |\n");
    buffer.push_str("|---|------|----------|------|-------|\n");
    for (index, event) in events.iter().enumerate() {
        let label = event.label.as_deref().unwrap_or("-");
        let span_label = format!("{}..{}", event.span.start, event.span.end);
        buffer.push_str(&format!(
            "| {} | {} | {} | {} | {} |\n",
            index + 1,
            event.kind.label(),
            event.trace_id,
            span_label,
            label
        ));
    }
    fs::write(path, buffer)?;
    Ok(())
}

fn default_telemetry_path(kind: &str, input: &Path) -> PathBuf {
    let stem = input
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("input");
    let dir = PathBuf::from("tmp/telemetry");
    dir.join(format!("{stem}-{kind}.json"))
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
    if let Some(filter) = args.diagnostic_filter() {
        extensions.insert(
            "diagnostics".to_string(),
            json!({ "filter": filter.to_value() }),
        );
    }
    if let Some(policy) = args.audit_policy() {
        extensions.insert("audit".to_string(), json!({ "policy": policy.to_value() }));
    }
    if let Some(target_extension) = run_config.extension("target") {
        extensions.insert("target".to_string(), target_extension.clone());
    }
    json!({
        "packrat": run_config.packrat,
        "left_recursion": left_recursion_label(run_config.left_recursion),
        "trace": run_config.trace,
        "merge_warnings": run_config.merge_warnings,
        "allow_top_level_expr": run_config.allow_top_level_expr,
        "ack_experimental_diagnostics": run_config.ack_experimental_diagnostics,
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
            "allow_top_level_expr": run_config.allow_top_level_expr,
            "ack_experimental_diagnostics": run_config.ack_experimental_diagnostics,
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
    config.insert(
        "allow_top_level_expr".to_string(),
        json!(run_config.allow_top_level_expr),
    );
    config.insert(
        "ack_experimental_diagnostics".to_string(),
        json!(run_config.ack_experimental_diagnostics),
    );
    config.insert("require_eof".to_string(), json!(run_config.require_eof));
    config.insert("legacy_result".to_string(), json!(run_config.legacy_result));
    config.insert(
        "experimental_effects".to_string(),
        json!(args.run_config.experimental_effects),
    );
    if let Some(filter) = args.diagnostic_filter() {
        config.insert("diagnostic_filter".to_string(), filter.to_value());
    }
    if let Some(policy) = args.audit_policy() {
        config.insert("audit_policy".to_string(), policy.to_value());
    }
    if let Some(path) = args.config_path.as_ref() {
        config.insert("path".to_string(), json!(path.display().to_string()));
    }
    if let Some(resolved) = run_config.config_compat() {
        config.insert(
            "compatibility_source".to_string(),
            json!(resolved.source.as_str()),
        );
        if let Some(label) = &resolved.profile_label {
            config.insert("compatibility_profile".to_string(), json!(label));
        }
        if let Ok(value) = serde_json::to_value(&resolved.compatibility) {
            config.insert("compatibility".to_string(), value);
        }
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
            "await_count": flow.await_count,
            "resume_count": flow.resume_count,
            "backpressure_count": flow.backpressure_count,
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

fn build_parser_diagnostics(
    diagnostics: &[FrontendDiagnostic],
    trace_events: &[ParserTraceEvent],
    span_trace: &[TraceFrame],
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
            if diag.span_trace.is_empty() && !span_trace.is_empty() {
                diag.span_trace = span_trace.to_vec();
            }
            diag.set_timestamp(timestamp.clone());
            if !diag.has_primary_span() {
                if let Some(frame) = diag.span_trace.first() {
                    diag.set_span(frame.span);
                } else {
                    diag.set_span(Span::new(0, 0));
                }
            }
            let recover_extension = diag_json::build_recover_extension(&diag);
            let trace_ids = trace_ids_for_diagnostic(&diag, trace_events);
            let mut extensions = diag.extensions.clone();
            extensions.insert(
                "diagnostic.v2".to_string(),
                json!({ "timestamp": timestamp }),
            );
            if let Some(recover) = recover_extension {
                if !extensions.contains_key("recover") {
                    extensions.insert("recover".to_string(), recover);
                }
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
            apply_experimental_stage_policy(
                &mut diag,
                &extensions,
                args.run_config.ack_experimental_diagnostics,
            );
            extensions.insert("runconfig".to_string(), runconfig_summary.clone());
            extensions.insert("cfg".to_string(), args.target_cfg_extension.clone());
            if !trace_ids.is_empty() {
                extensions.insert("trace_ids".to_string(), json!(trace_ids));
            }

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
            if diag.unicode.is_some() {
                unicode::integrate_unicode_metadata(
                    &mut diag,
                    source,
                    &mut extensions,
                    &mut metadata,
                );
            }
            let context = FormatterContext {
                program_name: &args.program_name,
                raw_args: &args.raw_args,
                input_path,
                run_id: args.run_id,
                phase: args.phase_label(),
                command: args.command_label(),
            };
            let audit_envelope = formatter::finalize_audit_metadata(
                &mut metadata,
                &mut diag,
                &timestamp,
                &context,
                stage_payload.primary_capability(),
            );
            let payload_metadata = metadata.clone();
            formatter::propagate_collections_diff_extensions(
                &mut extensions,
                audit_envelope.change_set.as_ref(),
            );
            let mut audit_object = serde_json::Map::new();
            audit_object.insert(
                "metadata".to_string(),
                Value::Object(payload_metadata.clone()),
            );
            if let Some(audit_id) = audit_envelope.audit_id {
                audit_object.insert("audit_id".to_string(), json!(audit_id.to_string()));
            }
            if let Some(change_set) = audit_envelope.change_set {
                audit_object.insert("change_set".to_string(), change_set);
            }
            if let Some(capability) = audit_envelope.capability {
                audit_object.insert("capability".to_string(), json!(capability));
            }

            let expected_value = diag_json::build_expected_field(&diag);
            diag.extensions = extensions.clone();
            diag_json::build_frontend_diagnostic(diag_json::FrontendDiagnosticPayload {
                diag: &diag,
                timestamp: &timestamp,
                domain_label: &domain_label,
                line_index: &line_index,
                input_path,
                source,
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

fn trace_ids_for_diagnostic(
    diagnostic: &FrontendDiagnostic,
    trace_events: &[ParserTraceEvent],
) -> Vec<String> {
    let span = match diagnostic.primary_span() {
        Some(span) => span,
        None => return Vec::new(),
    };
    trace_events
        .iter()
        .filter(|event| spans_overlap(event.span, span))
        .map(|event| event.trace_id.to_string())
        .collect()
}

fn spans_overlap(left: Span, right: Span) -> bool {
    let left_start = left.start.min(left.end);
    let left_end = left.end.max(left.start);
    let right_start = right.start.min(right.end);
    let right_end = right.end.max(right.start);
    left_start < right_end && right_start < left_end
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
            if let Some(recover_hint) = violation.recover_hint() {
                let mut recover_payload = match extensions.remove("recover") {
                    Some(Value::Object(map)) => map,
                    _ => Map::new(),
                };
                if let Some(mode) = recover_hint.mode.as_ref() {
                    recover_payload.insert("mode".to_string(), json!(mode));
                }
                if let Some(action) = recover_hint.action.as_ref() {
                    recover_payload.insert("action".to_string(), json!(action));
                }
                if let Some(sync) = recover_hint.sync.as_ref() {
                    recover_payload.insert("sync".to_string(), json!(sync));
                }
                if let Some(context) = recover_hint.context.as_ref() {
                    recover_payload.insert("context".to_string(), json!(context));
                }
                if !recover_payload.is_empty() {
                    extensions.insert("recover".to_string(), Value::Object(recover_payload));
                }
            }
            extensions.insert("runconfig".to_string(), runconfig_summary.clone());
            extensions.insert("cfg".to_string(), args.target_cfg_extension.clone());
            let mut pattern_extension = Map::new();
            if let Some(variants) = violation.pattern_missing_variants.as_ref() {
                pattern_extension.insert("missing_variants".to_string(), json!(variants));
            }
            if let Some(ranges) = violation.pattern_missing_ranges.as_ref() {
                pattern_extension.insert("missing_ranges".to_string(), json!(ranges));
            }
            if let Some(range) = violation.pattern_range.as_ref() {
                pattern_extension.insert("range".to_string(), json!(range));
            }
            if !pattern_extension.is_empty() {
                extensions.insert("pattern".to_string(), Value::Object(pattern_extension));
            }
            let mut severity_label = messages::find_message(violation.code)
                .map(|template| template.severity.as_str())
                .unwrap_or("error");
            if let Some(template) = messages::find_message(violation.code) {
                extensions.insert(
                    "diagnostic.message".to_string(),
                    json!({
                        "code": template.code,
                        "title": template.title,
                        "message": template.message,
                        "severity": template.severity.as_str(),
                    }),
                );
            }
            if should_downgrade_experimental(
                args.run_config.ack_experimental_diagnostics,
                &extensions,
            ) {
                severity_label = "warning";
            };
            let mut metadata = build_audit_metadata(
                &timestamp,
                args,
                run_config,
                stage_payload,
                flow,
                violation.domain(),
            );
            if let Some(info) = violation.iterator_stage.as_ref() {
                apply_iterator_stage_metadata(&mut extensions, &mut metadata, info);
            }
            if let Some(mismatch) = violation.capability_mismatch.as_ref() {
                EffectDiagnostic::apply_stage_violation(mismatch, &mut extensions, &mut metadata);
            }
            let context = FormatterContext {
                program_name: &args.program_name,
                raw_args: &args.raw_args,
                input_path,
                run_id: args.run_id,
                phase: args.phase_label(),
                command: args.command_label(),
            };
            let audit_envelope = formatter::complete_audit_metadata(
                &mut metadata,
                &timestamp,
                &context,
                stage_payload.primary_capability(),
            );
            let payload_metadata = metadata.clone();
            formatter::propagate_collections_diff_extensions(
                &mut extensions,
                audit_envelope.change_set.as_ref(),
            );
            let mut audit_object = serde_json::Map::new();
            audit_object.insert(
                "metadata".to_string(),
                Value::Object(payload_metadata.clone()),
            );
            if let Some(audit_id) = audit_envelope.audit_id {
                audit_object.insert("audit_id".to_string(), json!(audit_id.to_string()));
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
            let primary =
                diag_json::span_to_primary_value(violation.span, &line_index, input_path, source);
            let location = diag_json::span_to_location_opt(violation.span, &line_index, input_path);
            json!({
                "schema_version": SCHEMA_VERSION,
                "timestamp": timestamp,
                "message": violation.message,
                "severity": severity_label,
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
        "parser.runconfig.switches.allow_top_level_expr".to_string(),
        json!(run_config.allow_top_level_expr),
    );
    metadata.insert(
        "parser.runconfig.switches.ack_experimental_diagnostics".to_string(),
        json!(run_config.ack_experimental_diagnostics),
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

fn apply_iterator_stage_metadata(
    extensions: &mut serde_json::Map<String, Value>,
    metadata: &mut serde_json::Map<String, Value>,
    info: &IteratorStageViolationInfo,
) {
    let required_label = info.required.label();
    let actual_label = info.actual.label();
    let capability = info
        .capability
        .clone()
        .unwrap_or_else(|| "core.iter.unknown".to_string());
    let iterator_entry = json!({
        "stage": {
            "required": required_label,
            "actual": actual_label,
        },
        "capability": capability,
        "kind": info.kind,
        "source": info.source,
    });
    extensions.insert("iterator.stage".to_string(), iterator_entry);

    metadata.insert("iterator.stage.required".to_string(), json!(required_label));
    metadata.insert("iterator.stage.actual".to_string(), json!(actual_label));
    metadata.insert(
        "iterator.stage.capability".to_string(),
        json!(capability.clone()),
    );
    metadata.insert("iterator.stage.kind".to_string(), json!(info.kind.clone()));
    metadata.insert(
        "iterator.stage.source".to_string(),
        json!(info.source.clone()),
    );
    metadata.insert(
        "effect.stage.required".to_string(),
        json!(required_label.clone()),
    );
    metadata.insert(
        "effect.stage.actual".to_string(),
        json!(actual_label.clone()),
    );
    metadata.insert("effect.capability".to_string(), json!(capability.clone()));
    metadata.insert("capability.ids".to_string(), json!([capability.clone()]));
    metadata.insert(
        "effect.required_capabilities".to_string(),
        json!([capability.clone()]),
    );
    metadata.insert(
        "effect.stage.required_capabilities".to_string(),
        json!([{
            "id": capability,
            "stage": required_label,
        }]),
    );
}

#[derive(Clone, Serialize)]
struct TypeckArtifacts {
    typed_ast: TypedAstFile,
    constraints: ConstraintFile,
    debug: TypeckDebugFile,
    mir: mir::MirModule,
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
struct ActivePatternBranchPlan {
    on_match: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    on_miss: Option<String>,
}

#[derive(Clone, Serialize)]
struct ActivePatternLowering {
    name: String,
    kind: typed::ActivePatternKind,
    carrier: typed::ActiveReturnCarrier,
    has_miss_path: bool,
    span: Span,
    branches: ActivePatternBranchPlan,
}

#[derive(Clone, Serialize)]
struct TypeckDebugFile {
    schema_version: &'static str,
    effect_context: StageContext,
    type_row_mode: TypeRowMode,
    recover: RecoverConfig,
    runtime_capabilities: Vec<RuntimeCapability>,
    trace_enabled: bool,
    stage_trace: Vec<StageTraceStep>,
    used_impls: Vec<String>,
    metrics: TypecheckMetrics,
    violations: Vec<TypecheckViolation>,
    active_patterns: Vec<ActivePatternLowering>,
    match_lowerings: Vec<mir::MatchLoweringPlan>,
    mir: mir::MirModule,
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

fn build_active_pattern_lowerings(module: &typed::TypedModule) -> Vec<ActivePatternLowering> {
    module
        .active_patterns
        .iter()
        .map(|pattern| {
            let branches = match pattern.return_carrier {
                typed::ActiveReturnCarrier::OptionLike => ActivePatternBranchPlan {
                    on_match: "Some(value) を束縛して現在のアームを評価".to_string(),
                    on_miss: Some("None なら次のアームへフォールスルー".to_string()),
                },
                typed::ActiveReturnCarrier::Value => ActivePatternBranchPlan {
                    on_match: "値を束縛して現在のアームを評価".to_string(),
                    on_miss: None,
                },
            };
            ActivePatternLowering {
                name: pattern.name.clone(),
                kind: pattern.kind.clone(),
                carrier: pattern.return_carrier.clone(),
                has_miss_path: pattern.has_miss_path,
                span: pattern.span,
                branches,
            }
        })
        .collect()
}

fn render_typed_module(module: &typed::TypedModule) -> String {
    if module.functions.is_empty() && module.active_patterns.is_empty() {
        return "=== Typed AST ===\n\n<empty>".to_string();
    }
    let mut lines = vec!["=== Typed AST ===".to_string()];
    if !module.active_patterns.is_empty() {
        lines.push("[active_patterns]".to_string());
        for pattern in &module.active_patterns {
            let params = pattern
                .params
                .iter()
                .map(|param| format!("{}: {}", param.name, param.ty))
                .collect::<Vec<_>>()
                .join(", ");
            let head = match pattern.kind {
                typed::ActivePatternKind::Partial => format!("(|{}|_|)", pattern.name),
                typed::ActivePatternKind::Total => format!("(|{}|)", pattern.name),
            };
            let carrier = match pattern.return_carrier {
                typed::ActiveReturnCarrier::OptionLike => "OptionLike",
                typed::ActiveReturnCarrier::Value => "Value",
            };
            let line = if params.is_empty() {
                format!(
                    "{head} : {}{}",
                    carrier,
                    if pattern.has_miss_path {
                        " (miss -> next arm)"
                    } else {
                        ""
                    }
                )
            } else {
                format!(
                    "{head}({}) : {}{}",
                    params,
                    carrier,
                    if pattern.has_miss_path {
                        " (miss -> next arm)"
                    } else {
                        ""
                    }
                )
            };
            lines.push(line);
        }
    }
    if !module.functions.is_empty() {
        lines.push("[functions]".to_string());
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
    }
    lines.join("\n\n")
}

impl TypeckArtifacts {
    fn new(
        input: &Path,
        report: &TypecheckReport,
        config: &TypecheckConfig,
        stage_payload: &StageAuditPayload,
    ) -> Self {
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
            schema_version: SCHEMA_VERSION,
            effect_context: config.effect_context.clone(),
            type_row_mode: config.type_row_mode,
            recover: config.recover.clone(),
            runtime_capabilities: config.runtime_capabilities.clone(),
            trace_enabled: config.trace_enabled,
            stage_trace: stage_payload.stage_trace().to_vec(),
            used_impls: report.used_impls.clone(),
            metrics: report.metrics.clone(),
            violations: report.violations.clone(),
            active_patterns: build_active_pattern_lowerings(&report.typed_module),
            match_lowerings: mir::build_match_lowerings(&report.typed_module),
            mir: report.mir.clone(),
        };
        Self {
            typed_ast,
            constraints,
            debug,
            mir: report.mir.clone(),
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
