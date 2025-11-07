//! logos × chumsky フロントエンド PoC。入力ファイルを解析し JSON を出力する。

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use reml_frontend::diagnostic::{DiagnosticNote, FrontendDiagnostic};
use reml_frontend::error::Recoverability;
use reml_frontend::parser::ParserDriver;
use reml_frontend::span::Span;
use reml_frontend::typeck::{
    self, DualWriteGuards, InstallConfigError, RecoverConfig, StageContext, StageId,
    StageRequirement, TypeRowMode, TypecheckConfig, TypecheckDriver, TypecheckMetrics,
    TypecheckReport, TypedFunctionSummary,
};
use serde::Serialize;

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

    let result = ParserDriver::parse(&source);
    let typeck_report = result
        .ast
        .as_ref()
        .map(|module| TypecheckDriver::infer_module(module, &args.typecheck_config))
        .unwrap_or_default();
    let artifacts = TypeckArtifacts::new(&input_path, &typeck_report, &args.typecheck_config);
    let diagnostics = result
        .diagnostics
        .iter()
        .map(DiagnosticJson::from)
        .collect::<Vec<_>>();

    let parse_result = serde_json::json!({
        "packrat_stats": result.packrat_stats,
        "span_trace": result.span_trace,
    });

    let stream_meta = serde_json::json!({
        "packrat": result.stream_metrics.packrat,
        "span_trace": result.stream_metrics.span_trace,
    });

    let payload = serde_json::json!({
        "input": input_path,
        "ast_render": result.ast_render(),
        "parse_result": parse_result.clone(),
        "stream_meta": stream_meta.clone(),
        "diagnostics": diagnostics,
        "typecheck": serde_json::json!({
            "metrics": typeck_report.metrics,
            "functions": typeck_report.functions,
        }),
        "tokens": result.tokens.iter().map(|token| serde_json::json!({
            "kind": format!("{:?}", token.kind),
            "span": token.span,
            "lexeme": token.lexeme,
        })).collect::<Vec<_>>(),
    });

    println!("{}", serde_json::to_string_pretty(&payload)?);

    if let Some(path) = args.parse_debug_output {
        let diagnostics_json = payload
            .get("diagnostics")
            .cloned()
            .unwrap_or_else(|| serde_json::Value::Array(vec![]));
        let parse_debug = serde_json::json!({
            "run_config": {
                "switches": {
                    "require_eof": true,
                    "packrat": true,
                    "left_recursion": "auto",
                    "trace": false,
                    "merge_warnings": true,
                    "legacy_result": false,
                },
                "extensions": {
                    "stream": {
                        "enabled": true,
                        "checkpoint": "poc_frontend",
                        "resume_hint": "n/a",
                        "chunk_size": 0,
                    }
                }
            },
            "input": input_path,
            "diagnostics": diagnostics_json,
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
    input: PathBuf,
    parse_debug_output: Option<PathBuf>,
    typecheck_config: TypecheckConfig,
    dualwrite: Option<DualwriteCliOpts>,
    emit_typed_ast: Option<PathBuf>,
    emit_constraints: Option<PathBuf>,
    emit_typeck_debug: Option<PathBuf>,
}

#[derive(Clone)]
struct DualwriteCliOpts {
    run_label: String,
    case_label: String,
    root: Option<PathBuf>,
}

fn parse_args() -> Result<CliArgs, Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
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
        .recover(recover);
    if let Some(mode) = row_mode {
        builder = builder.type_row_mode(mode);
    }

    let dualwrite = dualwrite_run_label.map(|run_label| DualwriteCliOpts {
        run_label,
        case_label: dualwrite_case_label.expect("validated together"),
        root: dualwrite_root,
    });

    Ok(CliArgs {
        input,
        parse_debug_output: parse_debug,
        typecheck_config: builder.build(),
        dualwrite,
        emit_typed_ast,
        emit_constraints,
        emit_typeck_debug,
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

#[derive(Debug, Serialize)]
struct DiagnosticJson {
    message: String,
    code: Option<String>,
    recoverability: String,
    span: Option<Span>,
    notes: Vec<NoteJson>,
}

impl From<&FrontendDiagnostic> for DiagnosticJson {
    fn from(value: &FrontendDiagnostic) -> Self {
        Self {
            message: value.message.clone(),
            code: value.code.clone(),
            recoverability: recoverability_label(value.recoverability).to_string(),
            span: value.span,
            notes: value.notes.iter().map(NoteJson::from).collect(),
        }
    }
}

#[derive(Debug, Serialize)]
struct NoteJson {
    label: String,
    message: String,
    span: Option<Span>,
}

impl From<&DiagnosticNote> for NoteJson {
    fn from(value: &DiagnosticNote) -> Self {
        Self {
            label: value.label.clone(),
            message: value.message.clone(),
            span: value.span,
        }
    }
}

fn recoverability_label(value: Recoverability) -> &'static str {
    match value {
        Recoverability::Recoverable => "recoverable",
        Recoverability::Fatal => "fatal",
    }
}
