use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{anyhow, bail, Context, Result};

#[derive(Debug)]
struct Args {
    scenario: String,
    root: Option<PathBuf>,
}

fn parse_args() -> Result<Args> {
    let mut scenario: Option<String> = None;
    let mut root: Option<PathBuf> = None;
    let mut iter = env::args().skip(1);
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--scenario" => {
                let value = iter
                    .next()
                    .ok_or_else(|| anyhow!("--scenario の値が指定されていません"))?;
                scenario = Some(value);
            }
            "--root" => {
                let value = iter
                    .next()
                    .ok_or_else(|| anyhow!("--root の値が指定されていません"))?;
                root = Some(PathBuf::from(value));
            }
            other if other.starts_with("--") => {
                bail!("未対応のオプションです: {other}");
            }
            other => {
                bail!("不明な引数です: {other}");
            }
        }
    }
    let scenario = scenario.ok_or_else(|| anyhow!("--scenario が指定されていません"))?;
    Ok(Args { scenario, root })
}

fn main() {
    if let Err(err) = run() {
        eprintln!("reml_e2e: {err:?}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let args = parse_args()?;
    let root = args
        .root
        .map(Ok)
        .unwrap_or_else(default_root)?
        .canonicalize()
        .context("リポジトリルートの解決に失敗しました")?;

    match args.scenario.as_str() {
        "spec-core" | "spec_core" => run_phase4_suite(&root, "spec_core")?,
        other => bail!("未対応のシナリオです: {other}"),
    }
    Ok(())
}

fn default_root() -> Result<PathBuf> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(|path| path.parent())
        .map(Path::to_path_buf)
        .ok_or_else(|| anyhow!("ルートディレクトリを特定できません"))
}

fn run_phase4_suite(root: &Path, suite: &str) -> Result<()> {
    let runner = root.join("tooling/examples/run_phase4_suite.py");
    if !runner.exists() {
        bail!("suite runner が見つかりません: {}", runner.display());
    }
    let python = std::env::var("PYTHON")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("python3"));

    let status = Command::new(&python)
        .arg(&runner)
        .arg("--suite")
        .arg(suite)
        .arg("--root")
        .arg(root)
        .arg("--allow-failures")
        .status()
        .with_context(|| format!("Phase4 suite runner の起動に失敗しました ({suite})"))?;

    if !status.success() {
        bail!("Phase4 suite runner が異常終了しました (status={status})");
    }
    Ok(())
}
