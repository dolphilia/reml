use reml_frontend::ffi_executor::install_cli_ffi_executor;
use reml_runtime::collections::{
    audit_bridge::{AuditBridgeError, ChangeSet},
    persistent::btree::PersistentMap,
};
use reml_runtime::config::SchemaDiff as ConfigChangeSummary;
use reml_runtime::config::{
    ensure_schema_version_compatibility, load_manifest, validate_manifest, Manifest,
};
use reml_runtime::config::{ChangeKind, ConfigChange};
use reml_runtime::data::schema::Schema;
use reml_runtime::prelude::ensure::{DiagnosticSeverity, GuardDiagnostic};
use serde::{Deserialize, Serialize};
use serde_json::{self, Map, Value};
use std::env;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::{collections::BTreeMap, process};

fn main() {
    if let Err(err) = install_cli_ffi_executor() {
        eprintln!("[FFI] 実行エンジンの初期化に失敗しました: {err}");
    }
    match try_main() {
        Ok(code) => process::exit(code),
        Err(err) => {
            eprintln!("remlc: {err}");
            process::exit(1);
        }
    }
}

fn try_main() -> Result<i32, CliError> {
    let mut args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        print_help();
        return Ok(0);
    }
    match args.remove(0).as_str() {
        "new" => handle_new(args),
        "manifest" => handle_manifest(args),
        "config" => handle_config(args),
        "build" => handle_build(args),
        "--help" | "-h" => {
            print_help();
            Ok(0)
        }
        other => Err(CliError::Usage(format!(
            "未知のサブコマンド `{other}` が指定されました"
        ))),
    }
}

fn handle_manifest(mut args: Vec<String>) -> Result<i32, CliError> {
    if args.is_empty() {
        print_manifest_help();
        return Ok(0);
    }
    match args.remove(0).as_str() {
        "dump" => manifest_dump(args),
        "--help" | "-h" => {
            print_manifest_help();
            Ok(0)
        }
        other => Err(CliError::Usage(format!(
            "manifest コマンドに未知のサブコマンド `{other}` が指定されました"
        ))),
    }
}

fn manifest_dump(args: Vec<String>) -> Result<i32, CliError> {
    let opts = ManifestDumpOptions::parse(args)?;
    let manifest = read_manifest(&opts.manifest_path)?;
    match opts.format {
        OutputFormat::Json => {
            let body = serde_json::to_string_pretty(&manifest)?;
            if let Some(path) = opts.output {
                if let Some(parent) = path.parent() {
                    if !parent.as_os_str().is_empty() {
                        fs::create_dir_all(parent)?;
                    }
                }
                fs::write(path, format!("{body}\n"))?;
            } else {
                println!("{body}");
            }
            Ok(0)
        }
    }
}

fn handle_new(args: Vec<String>) -> Result<i32, CliError> {
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_new_help();
        return Ok(0);
    }
    let opts = NewOptions::parse(args)?;
    let template_root = template_root_path()?;
    let source = template_root.join(&opts.template);
    if !source.is_dir() {
        return Err(CliError::Usage(format!(
            "テンプレート `{}` が見つかりませんでした（探索先: {}）",
            opts.template,
            template_root.display()
        )));
    }
    if opts.output_path.exists() && !is_dir_empty(&opts.output_path)? {
        return Err(CliError::Usage(format!(
            "出力先 `{}` が空ではありません",
            opts.output_path.display()
        )));
    }
    if !opts.output_path.exists() {
        fs::create_dir_all(&opts.output_path)?;
    }
    copy_dir_all(&source, &opts.output_path)?;
    Ok(0)
}

fn handle_config(mut args: Vec<String>) -> Result<i32, CliError> {
    if args.is_empty() {
        print_config_help();
        return Ok(0);
    }
    match args.remove(0).as_str() {
        "lint" => config_lint(args),
        "diff" => config_diff(args),
        "--help" | "-h" => {
            print_config_help();
            Ok(0)
        }
        other => Err(CliError::Usage(format!(
            "config コマンドに未知のサブコマンド `{other}` が指定されました"
        ))),
    }
}

fn handle_build(args: Vec<String>) -> Result<i32, CliError> {
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_build_help();
        return Ok(0);
    }
    let opts = BuildLintOptions::parse(args)?;
    let mut diagnostics = Vec::new();
    let config = match load_build_config(&opts.config_path) {
        Ok(value) => Some(value),
        Err(diag) => {
            diagnostics.push(diag);
            None
        }
    };
    if let Some(config) = config.as_ref() {
        diagnostics.extend(validate_build_config(config, &opts.config_path));
    }
    let (mut bindgen_diagnostics, audit_entries) = match config.as_ref() {
        Some(config) => run_bindgen_if_enabled(config, &opts),
        None => (Vec::new(), Vec::new()),
    };
    diagnostics.append(&mut bindgen_diagnostics);
    let report = BuildLintReport::new(
        &opts,
        diagnostics.into_iter().map(guard_diag_to_report).collect(),
        config.is_some(),
        config
            .as_ref()
            .and_then(|value| value.ffi.as_ref())
            .is_some(),
        audit_entries,
    );
    print_build_report(&report, opts.output_format)?;
    Ok(report.exit_code())
}

fn config_lint(args: Vec<String>) -> Result<i32, CliError> {
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_config_lint_help();
        return Ok(0);
    }
    let opts = ConfigLintOptions::parse(args)?;
    let mut diagnostics = Vec::new();
    let manifest = match load_manifest(&opts.manifest_path) {
        Ok(value) => {
            if let Err(diag) = validate_manifest(&value) {
                diagnostics.push(diag);
            }
            Some(value)
        }
        Err(diag) => {
            diagnostics.push(diag);
            None
        }
    };
    if let (Some(manifest), Some(schema_path)) = (manifest.as_ref(), opts.schema_path.as_ref()) {
        let schema = load_schema(schema_path)?;
        if let Err(diag) = ensure_schema_version_compatibility(manifest, &schema) {
            diagnostics.push(diag);
        }
    }
    let report = ConfigLintReport::new(
        &opts,
        diagnostics.into_iter().map(guard_diag_to_report).collect(),
        manifest.is_some(),
        opts.schema_path.is_some(),
    );
    print_lint_report(&report, opts.output_format)?;
    Ok(report.exit_code())
}

fn config_diff(args: Vec<String>) -> Result<i32, CliError> {
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_config_diff_help();
        return Ok(0);
    }
    let opts = ConfigDiffOptions::parse(args)?;
    let base = ConfigDocument::load(&opts.base_path)?;
    let target = ConfigDocument::load(&opts.target_path)?;
    let report = build_config_diff_report(&base, &target)?;
    print_diff_report(&report, opts.output_format)?;
    Ok(0)
}

fn read_manifest(path: &Path) -> Result<Manifest, CliError> {
    let manifest = load_manifest(path)?;
    validate_manifest(&manifest)?;
    Ok(manifest)
}

#[derive(Debug)]
struct NewOptions {
    output_path: PathBuf,
    template: String,
}

impl NewOptions {
    fn parse(args: Vec<String>) -> Result<Self, CliError> {
        let mut output_path: Option<PathBuf> = None;
        let mut template = "lite".to_string();
        let mut iter = args.into_iter();
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--template" => {
                    let value = iter.next().ok_or_else(|| {
                        CliError::Usage("--template にはテンプレート名が必要です".to_string())
                    })?;
                    template = value;
                }
                other if other.starts_with("--") => {
                    return Err(CliError::Usage(format!(
                        "new コマンドに未知のオプション `{other}` が指定されました"
                    )));
                }
                other => {
                    if output_path.is_some() {
                        return Err(CliError::Usage(format!(
                            "出力先は 1 つだけ指定してください（追加: `{other}`）"
                        )));
                    }
                    output_path = Some(PathBuf::from(other));
                }
            }
        }
        let output_path = output_path
            .ok_or_else(|| CliError::Usage("出力先のパスを指定してください".to_string()))?;
        if template != "lite" {
            return Err(CliError::Usage(format!(
                "テンプレート `{template}` は未対応です（利用可能: lite）"
            )));
        }
        Ok(Self {
            output_path,
            template,
        })
    }
}

#[derive(Debug)]
struct ManifestDumpOptions {
    manifest_path: PathBuf,
    format: OutputFormat,
    output: Option<PathBuf>,
}

#[derive(Debug)]
struct BuildLintOptions {
    config_path: PathBuf,
    output_format: ReportFormat,
    emit_bindgen: bool,
    cache_dir: Option<PathBuf>,
}

impl Default for BuildLintOptions {
    fn default() -> Self {
        Self {
            config_path: PathBuf::from("reml.json"),
            output_format: ReportFormat::Json,
            emit_bindgen: false,
            cache_dir: None,
        }
    }
}

impl BuildLintOptions {
    fn parse(args: Vec<String>) -> Result<Self, CliError> {
        let mut opts = BuildLintOptions::default();
        let mut iter = args.into_iter();
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--config" => {
                    let value = iter.next().ok_or_else(|| {
                        CliError::Usage("--config はパスを伴う必要があります".into())
                    })?;
                    opts.config_path = PathBuf::from(value);
                }
                "--format" => {
                    let value = iter.next().ok_or_else(|| {
                        CliError::Usage("--format は human|json の値を伴う必要があります".into())
                    })?;
                    opts.output_format = ReportFormat::parse(&value)?;
                }
                "--emit-bindgen" => opts.emit_bindgen = true,
                "--cache-dir" => {
                    let value = iter.next().ok_or_else(|| {
                        CliError::Usage("--cache-dir はパスを伴う必要があります".into())
                    })?;
                    opts.cache_dir = Some(PathBuf::from(value));
                }
                other => {
                    return Err(CliError::Usage(format!(
                        "build コマンドの未知のオプション `{other}` が指定されました"
                    )))
                }
            }
        }
        Ok(opts)
    }
}

#[derive(Debug, Clone, Serialize)]
struct BuildLintReport {
    command: &'static str,
    config: String,
    diagnostics: Vec<LintDiagnostic>,
    audit: Vec<Value>,
    stats: BuildLintStats,
}

#[derive(Debug, Clone, Serialize)]
struct BuildLintStats {
    validated: bool,
    config_loaded: bool,
    ffi_present: bool,
}

impl BuildLintReport {
    fn new(
        opts: &BuildLintOptions,
        diagnostics: Vec<LintDiagnostic>,
        config_loaded: bool,
        ffi_present: bool,
        audit: Vec<Value>,
    ) -> Self {
        let validated = diagnostics.is_empty() && config_loaded;
        BuildLintReport {
            command: "build.lint",
            config: opts.config_path.display().to_string(),
            diagnostics,
            audit,
            stats: BuildLintStats {
                validated,
                config_loaded,
                ffi_present,
            },
        }
    }

    fn exit_code(&self) -> i32 {
        if self.stats.validated {
            0
        } else {
            1
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct BuildConfig {
    #[serde(default)]
    ffi: Option<FfiSection>,
}

#[derive(Debug, Clone, Deserialize)]
struct FfiSection {
    #[serde(default)]
    libraries: Vec<String>,
    #[serde(default)]
    headers: Vec<String>,
    #[serde(default)]
    bindgen: Option<BindgenSection>,
    #[serde(default)]
    linker: Option<LinkerSection>,
}

#[derive(Debug, Clone, Deserialize)]
struct BindgenSection {
    #[serde(default)]
    enabled: bool,
    #[serde(default)]
    output: Option<String>,
    #[serde(default)]
    config: Option<String>,
    #[serde(default)]
    manifest: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct LinkerSection {
    #[serde(default)]
    search_paths: Vec<String>,
    #[serde(default)]
    frameworks: Vec<String>,
    #[serde(default)]
    extra_args: Vec<String>,
}

fn load_build_config(path: &Path) -> Result<BuildConfig, GuardDiagnostic> {
    let body = fs::read_to_string(path).map_err(|err| {
        ffi_build_config_invalid(
            path,
            None,
            format!("reml.json の読み込みに失敗しました: {err}"),
        )
    })?;
    serde_json::from_str(&body).map_err(|err| {
        ffi_build_config_invalid(path, None, format!("reml.json の解析に失敗しました: {err}"))
    })
}

fn validate_build_config(config: &BuildConfig, path: &Path) -> Vec<GuardDiagnostic> {
    let mut diagnostics = Vec::new();
    let Some(ffi) = config.ffi.as_ref() else {
        return diagnostics;
    };
    for (index, library) in ffi.libraries.iter().enumerate() {
        if library.trim().is_empty() {
            diagnostics.push(ffi_build_config_invalid(
                path,
                Some(format!("ffi.libraries[{index}]")),
                "ffi.libraries に空のエントリがあります",
            ));
        }
    }
    for (index, header) in ffi.headers.iter().enumerate() {
        if header.trim().is_empty() {
            diagnostics.push(ffi_build_config_invalid(
                path,
                Some(format!("ffi.headers[{index}]")),
                "ffi.headers に空のエントリがあります",
            ));
        }
    }
    if let Some(bindgen) = ffi.bindgen.as_ref() {
        if bindgen.enabled {
            if bindgen
                .output
                .as_ref()
                .map_or(true, |v| v.trim().is_empty())
            {
                diagnostics.push(ffi_build_config_invalid(
                    path,
                    Some("ffi.bindgen.output".to_string()),
                    "ffi.bindgen.enabled=true の場合は output が必須です",
                ));
            }
        }
        if let Some(config_path) = bindgen.config.as_ref() {
            if config_path.trim().is_empty() {
                diagnostics.push(ffi_build_config_invalid(
                    path,
                    Some("ffi.bindgen.config".to_string()),
                    "ffi.bindgen.config が空文字列です",
                ));
            }
        }
    }
    if let Some(linker) = ffi.linker.as_ref() {
        for (index, path_value) in linker.search_paths.iter().enumerate() {
            if path_value.trim().is_empty() {
                diagnostics.push(ffi_build_config_invalid(
                    path,
                    Some(format!("ffi.linker.search_paths[{index}]")),
                    "ffi.linker.search_paths に空のエントリがあります",
                ));
            }
        }
        for (index, framework) in linker.frameworks.iter().enumerate() {
            if framework.trim().is_empty() {
                diagnostics.push(ffi_build_config_invalid(
                    path,
                    Some(format!("ffi.linker.frameworks[{index}]")),
                    "ffi.linker.frameworks に空のエントリがあります",
                ));
            }
        }
        for (index, arg) in linker.extra_args.iter().enumerate() {
            if arg.trim().is_empty() {
                diagnostics.push(ffi_build_config_invalid(
                    path,
                    Some(format!("ffi.linker.extra_args[{index}]")),
                    "ffi.linker.extra_args に空のエントリがあります",
                ));
            }
        }
    }
    diagnostics
}

fn run_bindgen_if_enabled(
    config: &BuildConfig,
    opts: &BuildLintOptions,
) -> (Vec<GuardDiagnostic>, Vec<Value>) {
    let mut diagnostics = Vec::new();
    let mut audit_entries = Vec::new();
    if !opts.emit_bindgen {
        return (diagnostics, audit_entries);
    }
    let Some(ffi) = config.ffi.as_ref() else {
        return (diagnostics, audit_entries);
    };
    let Some(bindgen) = ffi.bindgen.as_ref() else {
        return (diagnostics, audit_entries);
    };
    if !bindgen.enabled {
        return (diagnostics, audit_entries);
    }
    let output = match bindgen
        .output
        .as_ref()
        .map(|value| value.trim())
        .filter(|v| !v.is_empty())
    {
        Some(value) => value.to_string(),
        None => {
            diagnostics.push(ffi_build_config_invalid(
                &opts.config_path,
                Some("ffi.bindgen.output".to_string()),
                "ffi.bindgen.enabled=true の場合は output が必須です",
            ));
            return (diagnostics, audit_entries);
        }
    };
    let manifest_path = bindgen
        .manifest
        .as_ref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
        .unwrap_or_else(|| {
            Path::new(&output)
                .with_file_name("bindings.manifest.json")
                .to_string_lossy()
                .to_string()
        });
    let input_hash = compute_bindgen_input_hash(ffi, bindgen);
    let cache_path = cache_path_for_input_hash(opts.cache_dir.as_ref(), &input_hash);
    let tool_version = reml_bindgen_version();
    let cache_status = handle_bindgen_cache(cache_path.as_ref());
    if cache_status == "cache_hit" {
        let mut status = cache_status;
        let mut error_code: Option<String> = None;
        let mut error_message: Option<String> = None;
        if let Some(cache_path) = cache_path.as_ref() {
            if let Err(err) =
                restore_bindgen_cache(cache_path, Path::new(&output), Path::new(&manifest_path))
            {
                let (diag, code, message) = match err {
                    BindgenCacheRestoreError::OutputOverwrite(path) => {
                        let diag = ffi_bindgen_output_overwrite(path);
                        let message = diag.message.clone();
                        (diag, "ffi.bindgen.output_overwrite".to_string(), message)
                    }
                    BindgenCacheRestoreError::ManifestOverwrite(path) => {
                        let diag = ffi_bindgen_output_overwrite(path);
                        let message = diag.message.clone();
                        (diag, "ffi.bindgen.output_overwrite".to_string(), message)
                    }
                    BindgenCacheRestoreError::CacheMissing(path) => {
                        let message =
                            format!("生成物キャッシュが見つかりません: {}", path.display());
                        (
                            ffi_build_bindgen_failed(message.clone()),
                            "ffi.bindgen.generate_failed".to_string(),
                            message,
                        )
                    }
                    BindgenCacheRestoreError::Io(path, err) => {
                        let message = format!(
                            "生成物キャッシュの復元に失敗しました: {} ({})",
                            path.display(),
                            err
                        );
                        (
                            ffi_build_bindgen_failed(message.clone()),
                            "ffi.bindgen.generate_failed".to_string(),
                            message,
                        )
                    }
                };
                diagnostics.push(diag);
                error_code = Some(code);
                error_message = Some(message);
                status = "failed";
            }
        }
        audit_entries.push(ffi_bindgen_audit_entry(
            &opts.config_path,
            bindgen,
            ffi,
            &input_hash,
            status,
            cache_path.as_ref(),
            Some(&output),
            &tool_version,
            error_code.as_deref(),
            error_message.as_deref(),
        ));
        return (diagnostics, audit_entries);
    }
    let mut error_code: Option<String> = None;
    let mut error_message: Option<String> = None;
    let status = match invoke_reml_bindgen(bindgen, ffi, &output, &manifest_path) {
        Ok(()) => "success",
        Err(err) => {
            error_code = Some("ffi.bindgen.generate_failed".to_string());
            error_message = Some(err.message.clone());
            diagnostics.push(err);
            "failed"
        }
    };
    if status == "success" {
        if let Some(cache_path) = cache_path.as_ref() {
            if let Err(err) =
                cache_bindgen_outputs(cache_path, Path::new(&output), Path::new(&manifest_path))
            {
                diagnostics.push(ffi_build_bindgen_failed(format!(
                    "生成物キャッシュの格納に失敗しました: {err}"
                )));
            }
        }
    }
    audit_entries.push(ffi_bindgen_audit_entry(
        &opts.config_path,
        bindgen,
        ffi,
        &input_hash,
        status,
        cache_path.as_ref(),
        Some(&output),
        &tool_version,
        error_code.as_deref(),
        error_message.as_deref(),
    ));
    (diagnostics, audit_entries)
}

fn invoke_reml_bindgen(
    bindgen: &BindgenSection,
    ffi: &FfiSection,
    output: &str,
    manifest: &str,
) -> Result<(), GuardDiagnostic> {
    let mut cmd = std::process::Command::new("reml-bindgen");
    if let Some(config) = bindgen
        .config
        .as_ref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        cmd.arg("--config").arg(config);
    }
    for header in &ffi.headers {
        if !header.trim().is_empty() {
            cmd.arg("--header").arg(header);
        }
    }
    cmd.arg("--output").arg(output);
    cmd.arg("--manifest").arg(manifest);
    let status = cmd.status().map_err(|err| {
        ffi_build_bindgen_failed(format!("reml-bindgen の起動に失敗しました: {err}"))
    })?;
    if status.success() {
        Ok(())
    } else {
        Err(ffi_build_bindgen_failed(format!(
            "reml-bindgen が失敗しました (status={status})"
        )))
    }
}

fn cache_path_for_input_hash(cache_dir: Option<&PathBuf>, input_hash: &str) -> Option<PathBuf> {
    cache_dir.map(|root| root.join("ffi").join(input_hash))
}

fn handle_bindgen_cache(cache_path: Option<&PathBuf>) -> &'static str {
    let Some(cache_path) = cache_path else {
        return "success";
    };
    if cache_path.exists() {
        return "cache_hit";
    }
    let _ = fs::create_dir_all(cache_path);
    "success"
}

fn compute_bindgen_input_hash(ffi: &FfiSection, bindgen: &BindgenSection) -> String {
    use std::hash::{Hash, Hasher};
    let mut state = std::collections::hash_map::DefaultHasher::new();
    "reml-bindgen".hash(&mut state);
    for header in &ffi.headers {
        header.hash(&mut state);
    }
    for library in &ffi.libraries {
        library.hash(&mut state);
    }
    if let Some(config) = bindgen.config.as_ref() {
        config.hash(&mut state);
    }
    if let Some(output) = bindgen.output.as_ref() {
        output.hash(&mut state);
    }
    format!("{:016x}", state.finish())
}

fn ffi_bindgen_audit_entry(
    config_path: &Path,
    bindgen: &BindgenSection,
    ffi: &FfiSection,
    input_hash: &str,
    status: &str,
    cache_path: Option<&PathBuf>,
    output: Option<&str>,
    tool_version: &str,
    error_code: Option<&str>,
    error_message: Option<&str>,
) -> Value {
    let mut meta = Map::new();
    let mut bindgen_meta = Map::new();
    bindgen_meta.insert("event".into(), Value::String("ffi.bindgen".into()));
    bindgen_meta.insert("status".into(), Value::String(status.to_string()));
    bindgen_meta.insert("input_hash".into(), Value::String(input_hash.to_string()));
    if status != "cache_hit" {
        bindgen_meta.insert(
            "headers".into(),
            Value::Array(
                ffi.headers
                    .iter()
                    .map(|header| Value::String(header.clone()))
                    .collect(),
            ),
        );
    }
    bindgen_meta.insert(
        "config_path".into(),
        Value::String(
            bindgen
                .config
                .clone()
                .unwrap_or_else(|| "reml-bindgen.toml".to_string()),
        ),
    );
    bindgen_meta.insert(
        "manifest_path".into(),
        Value::String(config_path.display().to_string()),
    );
    if let Some(cache_path) = cache_path {
        bindgen_meta.insert(
            "cache_path".into(),
            Value::String(cache_path.display().to_string()),
        );
    }
    if let Some(output) = output {
        bindgen_meta.insert("output_path".into(), Value::String(output.to_string()));
    }
    bindgen_meta.insert(
        "tool_version".into(),
        Value::String(tool_version.to_string()),
    );
    if let Some(code) = error_code {
        bindgen_meta.insert("error.code".into(), Value::String(code.to_string()));
    }
    if let Some(message) = error_message {
        bindgen_meta.insert("error.message".into(), Value::String(message.to_string()));
    }
    meta.insert("ffi.bindgen".into(), Value::Object(bindgen_meta));
    let mut entry = Map::new();
    entry.insert("metadata".into(), Value::Object(meta));
    Value::Object(entry)
}

#[derive(Debug)]
enum BindgenCacheRestoreError {
    OutputOverwrite(PathBuf),
    ManifestOverwrite(PathBuf),
    CacheMissing(PathBuf),
    Io(PathBuf, std::io::Error),
}

fn restore_bindgen_cache(
    cache_path: &Path,
    output: &Path,
    manifest: &Path,
) -> Result<(), BindgenCacheRestoreError> {
    if output.exists() {
        return Err(BindgenCacheRestoreError::OutputOverwrite(
            output.to_path_buf(),
        ));
    }
    if manifest.exists() {
        return Err(BindgenCacheRestoreError::ManifestOverwrite(
            manifest.to_path_buf(),
        ));
    }
    let output_name = output
        .file_name()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|| "bindings.reml".to_string());
    let manifest_name = manifest
        .file_name()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|| "bindings.manifest.json".to_string());
    let cached_output = cache_path.join(output_name);
    let cached_manifest = cache_path.join(manifest_name);
    if !cached_output.exists() {
        return Err(BindgenCacheRestoreError::CacheMissing(cached_output));
    }
    if !cached_manifest.exists() {
        return Err(BindgenCacheRestoreError::CacheMissing(cached_manifest));
    }
    if let Some(parent) = output.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .map_err(|err| BindgenCacheRestoreError::Io(parent.to_path_buf(), err))?;
        }
    }
    if let Some(parent) = manifest.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .map_err(|err| BindgenCacheRestoreError::Io(parent.to_path_buf(), err))?;
        }
    }
    let _ = fs::copy(&cached_output, output)
        .map_err(|err| BindgenCacheRestoreError::Io(cached_output.clone(), err))?;
    let _ = fs::copy(&cached_manifest, manifest)
        .map_err(|err| BindgenCacheRestoreError::Io(cached_manifest.clone(), err))?;
    Ok(())
}

fn cache_bindgen_outputs(
    cache_path: &Path,
    output: &Path,
    manifest: &Path,
) -> Result<(), std::io::Error> {
    fs::create_dir_all(cache_path)?;
    let output_name = output
        .file_name()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|| "bindings.reml".to_string());
    let manifest_name = manifest
        .file_name()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|| "bindings.manifest.json".to_string());
    let cached_output = cache_path.join(output_name);
    let cached_manifest = cache_path.join(manifest_name);
    let _ = fs::copy(output, cached_output)?;
    let _ = fs::copy(manifest, cached_manifest)?;
    Ok(())
}

fn reml_bindgen_version() -> String {
    if let Ok(value) = std::env::var("REML_BINDGEN_VERSION") {
        if !value.trim().is_empty() {
            return value;
        }
    }
    let output = match std::process::Command::new("reml-bindgen")
        .arg("--version")
        .output()
    {
        Ok(value) => value,
        Err(_) => return "unknown".to_string(),
    };
    if !output.status.success() {
        return "unknown".to_string();
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_bindgen_version(&stdout)
}

fn parse_bindgen_version(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return "unknown".to_string();
    }
    if let Some(candidate) = trimmed
        .split_whitespace()
        .find(|token| token.chars().any(|ch| ch.is_ascii_digit()))
    {
        return candidate
            .trim_matches(|ch: char| ch == 'v' || ch == ':')
            .to_string();
    }
    trimmed.to_string()
}

fn ffi_build_bindgen_failed(message: impl Into<String>) -> GuardDiagnostic {
    GuardDiagnostic {
        code: "ffi.bindgen.generate_failed",
        domain: "ffi",
        severity: DiagnosticSeverity::Error,
        message: message.into(),
        notes: Vec::new(),
        extensions: Map::new(),
        audit_metadata: Map::new(),
    }
}

fn ffi_bindgen_output_overwrite(path: PathBuf) -> GuardDiagnostic {
    let mut extensions = Map::new();
    let mut payload = Map::new();
    payload.insert("path".into(), Value::String(path.display().to_string()));
    extensions.insert("ffi.bindgen".into(), Value::Object(payload));
    GuardDiagnostic {
        code: "ffi.bindgen.output_overwrite",
        domain: "ffi",
        severity: DiagnosticSeverity::Error,
        message: "キャッシュ復元先が既に存在するため上書きを中止しました".to_string(),
        notes: Vec::new(),
        extensions,
        audit_metadata: Map::new(),
    }
}

fn ffi_build_config_invalid(
    path: &Path,
    field: Option<String>,
    message: impl Into<String>,
) -> GuardDiagnostic {
    let mut build_info = Map::new();
    build_info.insert("path".into(), Value::String(path.display().to_string()));
    if let Some(field) = field {
        build_info.insert("field".into(), Value::String(field));
    }
    let mut extensions = Map::new();
    extensions.insert("ffi.build".into(), Value::Object(build_info));
    GuardDiagnostic {
        code: "ffi.build.config_invalid",
        domain: "ffi",
        severity: DiagnosticSeverity::Error,
        message: message.into(),
        notes: Vec::new(),
        extensions,
        audit_metadata: Map::new(),
    }
}

impl Default for ManifestDumpOptions {
    fn default() -> Self {
        Self {
            manifest_path: PathBuf::from("reml.toml"),
            format: OutputFormat::Json,
            output: None,
        }
    }
}

impl ManifestDumpOptions {
    fn parse(args: Vec<String>) -> Result<Self, CliError> {
        let mut opts = ManifestDumpOptions::default();
        let mut iter = args.into_iter();
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--manifest" => {
                    let path = iter.next().ok_or_else(|| {
                        CliError::Usage("--manifest オプションにはパスが必要です".to_string())
                    })?;
                    opts.manifest_path = PathBuf::from(path);
                }
                "--format" => {
                    let value = iter.next().ok_or_else(|| {
                        CliError::Usage("--format オプションには値が必要です".to_string())
                    })?;
                    opts.format = OutputFormat::parse(&value)?;
                }
                "--output" | "-o" => {
                    let value = iter.next().ok_or_else(|| {
                        CliError::Usage("--output オプションにはパスが必要です".to_string())
                    })?;
                    opts.output = Some(PathBuf::from(value));
                }
                _ => {
                    return Err(CliError::Usage(format!(
                        "未対応の引数 `{arg}` が指定されました"
                    )))
                }
            }
        }
        Ok(opts)
    }
}

#[derive(Debug, Clone)]
struct ConfigLintOptions {
    manifest_path: PathBuf,
    schema_path: Option<PathBuf>,
    output_format: ReportFormat,
}

impl Default for ConfigLintOptions {
    fn default() -> Self {
        Self {
            manifest_path: PathBuf::from("reml.toml"),
            schema_path: None,
            output_format: ReportFormat::Json,
        }
    }
}

impl ConfigLintOptions {
    fn parse(args: Vec<String>) -> Result<Self, CliError> {
        let mut opts = ConfigLintOptions::default();
        let mut iter = args.into_iter();
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--manifest" => {
                    let path = iter.next().ok_or_else(|| {
                        CliError::Usage("--manifest にはパスを指定してください".to_string())
                    })?;
                    opts.manifest_path = PathBuf::from(path);
                }
                "--schema" => {
                    let path = iter.next().ok_or_else(|| {
                        CliError::Usage("--schema にはパスを指定してください".to_string())
                    })?;
                    opts.schema_path = Some(PathBuf::from(path));
                }
                "--format" => {
                    let value = iter.next().ok_or_else(|| {
                        CliError::Usage(
                            "--format には human もしくは json を指定してください".to_string(),
                        )
                    })?;
                    opts.output_format = ReportFormat::parse(&value)?;
                }
                other => {
                    return Err(CliError::Usage(format!(
                        "config lint で未対応の引数 `{other}` が指定されました"
                    )));
                }
            }
        }
        Ok(opts)
    }
}

#[derive(Debug, Clone)]
struct ConfigDiffOptions {
    base_path: PathBuf,
    target_path: PathBuf,
    output_format: ReportFormat,
}

impl ConfigDiffOptions {
    fn parse(args: Vec<String>) -> Result<Self, CliError> {
        let mut opts = ConfigDiffOptions {
            base_path: PathBuf::new(),
            target_path: PathBuf::new(),
            output_format: ReportFormat::Json,
        };
        let mut positional = Vec::new();
        let mut iter = args.into_iter();
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--format" => {
                    let value = iter.next().ok_or_else(|| {
                        CliError::Usage(
                            "--format には human もしくは json を指定してください".to_string(),
                        )
                    })?;
                    opts.output_format = ReportFormat::parse(&value)?;
                }
                other if other.starts_with('-') => {
                    return Err(CliError::Usage(format!(
                        "config diff で未対応のオプション `{other}` が指定されました"
                    )));
                }
                value => positional.push(value.to_string()),
            }
        }
        if positional.len() != 2 {
            return Err(CliError::Usage(
                "config diff には <base.json> <target.json> の 2 つの引数が必要です".to_string(),
            ));
        }
        opts.base_path = PathBuf::from(&positional[0]);
        opts.target_path = PathBuf::from(&positional[1]);
        Ok(opts)
    }
}

#[derive(Debug, Serialize)]
struct ConfigLintReport {
    command: &'static str,
    manifest: String,
    schema: Option<String>,
    diagnostics: Vec<LintDiagnostic>,
    stats: LintStats,
    exit_code: i32,
}

impl ConfigLintReport {
    fn new(
        opts: &ConfigLintOptions,
        diagnostics: Vec<LintDiagnostic>,
        manifest_loaded: bool,
        schema_checked: bool,
    ) -> Self {
        let validated = diagnostics.is_empty() && manifest_loaded;
        let exit_code = if validated { 0 } else { 2 };
        Self {
            command: "config.lint",
            manifest: opts.manifest_path.display().to_string(),
            schema: opts
                .schema_path
                .as_ref()
                .map(|path| path.display().to_string()),
            diagnostics,
            stats: LintStats {
                validated,
                manifest_loaded,
                schema_checked,
            },
            exit_code,
        }
    }

    fn exit_code(&self) -> i32 {
        self.exit_code
    }
}

#[derive(Debug, Serialize)]
struct LintStats {
    validated: bool,
    manifest_loaded: bool,
    schema_checked: bool,
}

#[derive(Debug, Clone, Serialize)]
struct LintDiagnostic {
    code: String,
    domain: String,
    severity: String,
    message: String,
    extensions: Value,
    audit: Value,
}

#[derive(Debug, Serialize)]
struct ConfigDiffReport {
    command: &'static str,
    base: ConfigDiffEndpoint,
    target: ConfigDiffEndpoint,
    summary: DiffSummary,
    change_set: Value,
    schema_diff: ConfigChangeSummary,
}

#[derive(Debug, Serialize)]
struct ConfigDiffEndpoint {
    path: String,
    format: &'static str,
    entries: usize,
}

impl ConfigDiffEndpoint {
    fn from_document(doc: &ConfigDocument) -> Self {
        Self {
            path: doc.path.display().to_string(),
            format: doc.format,
            entries: doc.entries(),
        }
    }
}

#[derive(Debug, Serialize)]
struct DiffSummary {
    added: usize,
    removed: usize,
    updated: usize,
    total: usize,
}

impl DiffSummary {
    fn from_change_set(change_set: &ChangeSet) -> Self {
        let summary = change_set.summary();
        Self {
            added: summary.added,
            removed: summary.removed,
            updated: summary.updated,
            total: summary.total(),
        }
    }
}

#[derive(Clone)]
struct ConfigDocument {
    path: PathBuf,
    format: &'static str,
    flattened: BTreeMap<String, Value>,
}

impl ConfigDocument {
    fn load(path: &Path) -> Result<Self, CliError> {
        let body = fs::read_to_string(path)?;
        let value: Value = serde_json::from_str(&body).map_err(|err| CliError::Json(err))?;
        Ok(Self::from_value(path.to_path_buf(), value))
    }

    fn from_value(path: PathBuf, value: Value) -> Self {
        let flattened = flatten_config_tree(&value);
        Self {
            path,
            format: "json",
            flattened,
        }
    }

    fn entries(&self) -> usize {
        self.flattened.len()
    }
}

#[cfg(test)]
impl ConfigDocument {
    fn for_test(label: &str, value: Value) -> Self {
        Self::from_value(PathBuf::from(label), value)
    }
}

fn build_config_diff_report(
    base: &ConfigDocument,
    target: &ConfigDocument,
) -> Result<ConfigDiffReport, CliError> {
    let base_map = PersistentMap::from_map(base.flattened.clone());
    let target_map = PersistentMap::from_map(target.flattened.clone());
    let change_set = base_map
        .diff_change_set(&target_map)
        .map_err(CliError::ChangeSet)?;
    let schema_diff = ConfigChangeSummary::from_change_set(&change_set, None);
    Ok(ConfigDiffReport {
        command: "config.diff",
        base: ConfigDiffEndpoint::from_document(base),
        target: ConfigDiffEndpoint::from_document(target),
        summary: DiffSummary::from_change_set(&change_set),
        change_set: change_set.to_value(),
        schema_diff,
    })
}

fn load_schema(path: &Path) -> Result<Schema, CliError> {
    let body = fs::read_to_string(path)?;
    let schema: Schema = serde_json::from_str(&body)?;
    Ok(schema)
}

fn guard_diag_to_report(diag: GuardDiagnostic) -> LintDiagnostic {
    LintDiagnostic {
        code: diag.code.to_string(),
        domain: diag.domain.to_string(),
        severity: severity_label(diag.severity).to_string(),
        message: diag.message,
        extensions: Value::Object(diag.extensions),
        audit: Value::Object(diag.audit_metadata),
    }
}

fn severity_label(severity: DiagnosticSeverity) -> &'static str {
    match severity {
        DiagnosticSeverity::Error => "error",
        DiagnosticSeverity::Warning => "warning",
        DiagnosticSeverity::Info => "info",
        DiagnosticSeverity::Hint => "hint",
    }
}

fn flatten_config_tree(value: &Value) -> BTreeMap<String, Value> {
    let mut map = BTreeMap::new();
    flatten_config_value(value, "", &mut map);
    map
}

fn flatten_config_value(value: &Value, path: &str, out: &mut BTreeMap<String, Value>) {
    match value {
        Value::Object(entries) => {
            if entries.is_empty() {
                let key = if path.is_empty() { "$" } else { path };
                out.insert(key.to_string(), Value::Object(Map::new()));
            }
            for (key, child) in entries {
                let next = if path.is_empty() {
                    key.clone()
                } else {
                    format!("{path}.{key}")
                };
                flatten_config_value(child, &next, out);
            }
        }
        Value::Array(items) => {
            if items.is_empty() {
                let key = if path.is_empty() { "$" } else { path };
                out.insert(key.to_string(), Value::Array(vec![]));
                return;
            }
            for (index, child) in items.iter().enumerate() {
                let next = if path.is_empty() {
                    format!("[{index}]")
                } else {
                    format!("{path}[{index}]")
                };
                flatten_config_value(child, &next, out);
            }
        }
        _ => {
            let key = if path.is_empty() {
                "$".to_string()
            } else {
                path.to_string()
            };
            out.insert(key, value.clone());
        }
    }
}

fn print_lint_report(report: &ConfigLintReport, format: ReportFormat) -> Result<(), CliError> {
    match format {
        ReportFormat::Json => {
            let body = serde_json::to_string_pretty(report)?;
            println!("{body}");
        }
        ReportFormat::Human => {
            if report.stats.validated {
                println!("[config.lint] {} OK", report.manifest);
            } else {
                println!(
                    "[config.lint] {} で {} 件の問題が見つかりました",
                    report.manifest,
                    report.diagnostics.len()
                );
                for diag in &report.diagnostics {
                    println!("  - [{}] {}: {}", diag.severity, diag.code, diag.message);
                }
            }
        }
    }
    Ok(())
}

fn print_build_report(report: &BuildLintReport, format: ReportFormat) -> Result<(), CliError> {
    match format {
        ReportFormat::Json => {
            let body = serde_json::to_string_pretty(report)?;
            println!("{body}");
        }
        ReportFormat::Human => {
            if report.stats.validated {
                println!("[build.lint] {} OK", report.config);
            } else {
                println!(
                    "[build.lint] {} で {} 件の問題が見つかりました",
                    report.config,
                    report.diagnostics.len()
                );
                for diag in &report.diagnostics {
                    println!("  - [{}] {}: {}", diag.severity, diag.code, diag.message);
                }
            }
            if !report.audit.is_empty() {
                println!(
                    "[build.lint] ffi.bindgen の監査ログを {} 件生成しました",
                    report.audit.len()
                );
            }
        }
    }
    Ok(())
}

fn print_diff_report(report: &ConfigDiffReport, format: ReportFormat) -> Result<(), CliError> {
    match format {
        ReportFormat::Json => {
            let body = serde_json::to_string_pretty(report)?;
            println!("{body}");
        }
        ReportFormat::Human => {
            println!(
                "[config.diff] {} -> {}",
                report.base.path, report.target.path
            );
            println!(
                "  added: {} removed: {} updated: {} (total={})",
                report.summary.added,
                report.summary.removed,
                report.summary.updated,
                report.summary.total
            );
            for change in &report.schema_diff.changes {
                print_diff_change(change);
            }
        }
    }
    Ok(())
}

fn print_diff_change(change: &ConfigChange) {
    let key = value_to_string(&change.key);
    match change.kind {
        ChangeKind::Added => {
            if let Some(current) = &change.current {
                println!("    + {key} = {}", value_to_string(current));
            } else {
                println!("    + {key}");
            }
        }
        ChangeKind::Removed => {
            if let Some(previous) = &change.previous {
                println!("    - {key} = {}", value_to_string(previous));
            } else {
                println!("    - {key}");
            }
        }
        ChangeKind::Updated => {
            let before = change
                .previous
                .as_ref()
                .map(value_to_string)
                .unwrap_or_else(|| "null".into());
            let after = change
                .current
                .as_ref()
                .map(value_to_string)
                .unwrap_or_else(|| "null".into());
            println!("    ~ {key}: {before} -> {after}");
        }
    }
}

fn value_to_string(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| value.to_string())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReportFormat {
    Json,
    Human,
}

impl ReportFormat {
    fn parse(raw: &str) -> Result<Self, CliError> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "json" => Ok(ReportFormat::Json),
            "human" | "tty" | "text" => Ok(ReportFormat::Human),
            other => Err(CliError::Usage(format!(
                "--format に指定した値 `{other}` は human / json のいずれかである必要があります"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn flatten_config_tree_creates_dotted_keys() {
        let source = json!({
            "service": {
                "name": "alpha",
                "replicas": 2,
                "features": ["audit", "telemetry"]
            },
            "limits": {
                "memory": "1Gi"
            },
            "empty": {},
            "list": []
        });
        let flattened = flatten_config_tree(&source);
        assert!(flattened.contains_key("service.name"));
        assert!(flattened.contains_key("service.features[0]"));
        assert!(flattened.contains_key("service.features[1]"));
        assert!(flattened.contains_key("limits.memory"));
        assert!(flattened.contains_key("empty"));
        assert!(flattened.contains_key("list"));
    }

    #[test]
    fn diff_report_contains_expected_change_set() {
        let base = ConfigDocument::for_test(
            "base",
            json!({
                "service": {
                    "name": "alpha",
                    "replicas": 2,
                    "features": ["audit", "telemetry"]
                },
                "limits": {
                    "memory": "1Gi",
                    "cpu": "500m"
                }
            }),
        );
        let target = ConfigDocument::for_test(
            "target",
            json!({
                "service": {
                    "name": "alpha",
                    "replicas": 3,
                    "features": ["audit"]
                },
                "limits": {
                    "memory": "2Gi",
                    "cpu": "750m"
                },
                "telemetry": {
                    "enabled": true
                }
            }),
        );
        let report = build_config_diff_report(&base, &target).expect("diff report should succeed");
        assert_eq!(report.summary.added, 1);
        assert_eq!(report.summary.removed, 1);
        assert_eq!(report.summary.updated, 3);
        assert_eq!(report.schema_diff.changes.len(), 5);
        let keys: Vec<String> = report
            .schema_diff
            .changes
            .iter()
            .map(|change| change.key.as_str().unwrap().to_string())
            .collect();
        assert_eq!(
            keys,
            vec![
                "limits.cpu",
                "limits.memory",
                "service.features[1]",
                "service.replicas",
                "telemetry.enabled"
            ]
        );
        if let Value::Array(items) = &report.change_set["items"] {
            assert_eq!(items.len(), 5);
        } else {
            panic!("change_set items should be an array");
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum OutputFormat {
    Json,
}

impl OutputFormat {
    fn parse(raw: &str) -> Result<Self, CliError> {
        match raw.to_ascii_lowercase().as_str() {
            "json" => Ok(OutputFormat::Json),
            other => Err(CliError::Usage(format!(
                "出力形式 `{other}` には対応していません（json のみサポート）"
            ))),
        }
    }
}

#[derive(Debug)]
enum CliError {
    Usage(String),
    Io(std::io::Error),
    ManifestDiagnostic(GuardDiagnostic),
    Json(serde_json::Error),
    ChangeSet(AuditBridgeError),
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CliError::Usage(msg) => write!(f, "{msg}"),
            CliError::Io(err) => write!(f, "ファイル操作に失敗しました: {err}"),
            CliError::ManifestDiagnostic(diag) => {
                if let Some(code) = Some(diag.code) {
                    write!(
                        f,
                        "マニフェストの検証に失敗しました ({code}): {}",
                        diag.message
                    )
                } else {
                    write!(f, "マニフェストの検証に失敗しました: {}", diag.message)
                }
            }
            CliError::Json(err) => write!(f, "JSON の処理に失敗しました: {err}"),
            CliError::ChangeSet(err) => {
                write!(f, "ChangeSet の生成に失敗しました: {err}")
            }
        }
    }
}

impl std::error::Error for CliError {}

impl From<std::io::Error> for CliError {
    fn from(value: std::io::Error) -> Self {
        CliError::Io(value)
    }
}

impl From<serde_json::Error> for CliError {
    fn from(value: serde_json::Error) -> Self {
        CliError::Json(value)
    }
}

impl From<GuardDiagnostic> for CliError {
    fn from(value: GuardDiagnostic) -> Self {
        CliError::ManifestDiagnostic(value)
    }
}

impl From<AuditBridgeError> for CliError {
    fn from(value: AuditBridgeError) -> Self {
        CliError::ChangeSet(value)
    }
}

fn template_root_path() -> Result<PathBuf, CliError> {
    if let Ok(value) = env::var("REML_TEMPLATE_ROOT") {
        return Ok(PathBuf::from(value));
    }
    let cwd = env::current_dir()?;
    Ok(cwd.join("tooling").join("templates"))
}

fn is_dir_empty(path: &Path) -> Result<bool, CliError> {
    if !path.exists() {
        return Ok(true);
    }
    let mut entries = fs::read_dir(path)?;
    Ok(entries.next().is_none())
}

fn copy_dir_all(src: &Path, dst: &Path) -> Result<(), CliError> {
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if file_type.is_dir() {
            fs::create_dir_all(&dst_path)?;
            copy_dir_all(&src_path, &dst_path)?;
        } else if file_type.is_file() {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

fn print_help() {
    eprintln!(
        "使い方: remlc <command> [options]\n\nサブコマンド:\n\
  new <path>           テンプレートから新規プロジェクトを生成\n\
  manifest dump         reml.toml を JSON へダンプ\n\
  build                reml.json の FFI セクションを検証\n\
  config lint           マニフェスト/スキーマを検証して JSON レポートを表示\n\
  config diff <old> <new>  JSON 設定ファイル同士の差分を ChangeSet 形式で出力"
    );
}

fn print_new_help() {
    eprintln!(
        "使い方: remlc new <path> [--template <name>]\n\n\
        --template <name>  既定: lite（学習/試作向けの最小構成テンプレート）。\n\
                         CLI ヘルプには用途と project.stage 昇格の導線を含める。\n\
        環境変数 REML_TEMPLATE_ROOT を指定するとテンプレート探索先を変更できます。"
    );
}

fn print_manifest_help() {
    eprintln!(
        "使い方: remlc manifest dump [--manifest <path>] [--format json] [--output <path>]\n\n\
        --manifest <path>  読み込む reml.toml（既定: ./reml.toml）\n\
        --format json      現時点で JSON のみサポート\n\
        --output <path>    指定するとファイルへ書き出し、未指定なら stdout へ出力"
    );
}

fn print_config_help() {
    eprintln!(
        "使い方: remlc config <subcommand>\n\nサブコマンド:\n\
  lint   --manifest <reml.toml> [--schema schema.json] [--format human|json]\n\
  diff   <base.json> <target.json> [--format human|json]"
    );
}

fn print_config_lint_help() {
    eprintln!(
        "使い方: remlc config lint [--manifest <path>] [--schema <schema.json>] [--format human|json]\n\n\
        --manifest <path>  検証対象の reml.toml（既定: ./reml.toml）\n\
        --schema <path>    Schema(JSON) との互換チェックを有効化\n\
        --format human|json  出力形式を切替（既定: json）"
    );
}

fn print_config_diff_help() {
    eprintln!(
        "使い方: remlc config diff <old.json> <new.json> [--format human|json]\n\n\
        --format human|json  ChangeSet 出力を JSON か TTY に切替（既定: json）"
    );
}

fn print_build_help() {
    eprintln!(
        "使い方: remlc build [--config <path>] [--emit-bindgen] [--cache-dir <path>] [--format human|json]\n\n\
        --config <path>  読み込む reml.json（既定: ./reml.json）\n\
        --emit-bindgen  reml-bindgen を起動して生成を行う\n\
        --cache-dir <path>  生成キャッシュを格納するルートディレクトリ\n\
        --format human|json  出力形式を切替（既定: json）"
    );
}
