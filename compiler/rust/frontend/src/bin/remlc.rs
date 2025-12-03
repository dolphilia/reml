use reml_runtime::config::{load_manifest, validate_manifest, Manifest};
use reml_runtime::prelude::ensure::GuardDiagnostic;
use std::env;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    if let Err(err) = try_main() {
        eprintln!("remlc: {err}");
        std::process::exit(1);
    }
}

fn try_main() -> Result<(), CliError> {
    let mut args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        print_help();
        return Ok(());
    }
    match args.remove(0).as_str() {
        "manifest" => handle_manifest(args),
        "--help" | "-h" => {
            print_help();
            Ok(())
        }
        other => Err(CliError::Usage(format!(
            "未知のサブコマンド `{other}` が指定されました"
        ))),
    }
}

fn handle_manifest(mut args: Vec<String>) -> Result<(), CliError> {
    if args.is_empty() {
        print_manifest_help();
        return Ok(());
    }
    match args.remove(0).as_str() {
        "dump" => manifest_dump(args),
        "--help" | "-h" => {
            print_manifest_help();
            Ok(())
        }
        other => Err(CliError::Usage(format!(
            "manifest コマンドに未知のサブコマンド `{other}` が指定されました"
        ))),
    }
}

fn manifest_dump(args: Vec<String>) -> Result<(), CliError> {
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
            Ok(())
        }
    }
}

fn read_manifest(path: &Path) -> Result<Manifest, CliError> {
    let manifest = load_manifest(path)?;
    validate_manifest(&manifest)?;
    Ok(manifest)
}

#[derive(Debug)]
struct ManifestDumpOptions {
    manifest_path: PathBuf,
    format: OutputFormat,
    output: Option<PathBuf>,
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
            CliError::Json(err) => write!(f, "JSON 生成に失敗しました: {err}"),
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

fn print_help() {
    eprintln!(
        "使い方: remlc <command> [options]\n\nサブコマンド:\n  manifest dump  reml.toml を JSON へダンプ"
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
