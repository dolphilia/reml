use std::env;
use std::ffi::OsString;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Output};
use std::time::{SystemTime, UNIX_EPOCH};

/// 対象プラットフォーム。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Platform {
    Linux,
    MacOS,
    Windows,
    Unknown,
}

impl Platform {
    /// 環境依存の `os` 名からプラットフォームを判定する。
    pub fn from_os_name(os_name: &str) -> Self {
        match os_name.to_ascii_lowercase().as_str() {
            "linux" => Platform::Linux,
            "macos" | "darwin" => Platform::MacOS,
            "windows" => Platform::Windows,
            _ => Platform::Unknown,
        }
    }

    /// このバイナリを実行している環境のプラットフォーム。
    pub fn detect() -> Self {
        Platform::from_os_name(env::consts::OS)
    }

    /// 人間向けのラベル。
    pub fn label(&self) -> &'static str {
        match self {
            Platform::Linux => "linux",
            Platform::MacOS => "macos",
            Platform::Windows => "windows",
            Platform::Unknown => "unknown",
        }
    }
}

/// リンカー呼び出し情報。
#[derive(Clone, Debug)]
pub struct LinkCommand {
    program: String,
    args: Vec<OsString>,
}

impl LinkCommand {
    /// リンカーで使用する実行ファイル名。
    pub fn program(&self) -> &str {
        &self.program
    }

    /// コマンド引数。
    pub fn args(&self) -> &[OsString] {
        &self.args
    }

    /// `std::process::Command` に変換する。
    fn to_command(&self) -> Command {
        let mut cmd = Command::new(&self.program);
        cmd.args(&self.args);
        cmd
    }
}

/// ランタイムリンク処理で発生するエラー。
#[derive(Debug)]
pub enum RuntimeLinkError {
    /// I/O エラー。
    Io(io::Error),
    /// 指定したランタイムライブラリが見つからない。
    RuntimeLibraryMissing {
        env_value: Option<OsString>,
        candidates: Vec<PathBuf>,
    },
    /// 対応していないプラットフォーム。
    UnsupportedPlatform(String),
    /// 外部コマンドが失敗した。
    CommandFailed {
        program: String,
        exit_status: Option<i32>,
        stdout: String,
        stderr: String,
    },
}

impl fmt::Display for RuntimeLinkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RuntimeLinkError::Io(err) => write!(f, "I/O エラー: {}", err),
            RuntimeLinkError::RuntimeLibraryMissing {
                env_value,
                candidates,
            } => {
                write!(
                    f,
                    "ランタイムライブラリが見つかりません (REML_RUNTIME_PATH={:?})。\n\
                     検査した候補: {:?}",
                    env_value, candidates
                )
            }
            RuntimeLinkError::UnsupportedPlatform(platform) => {
                write!(
                    f,
                    "ランタイムリンクはサポートされていないプラットフォーム: {}",
                    platform
                )
            }
            RuntimeLinkError::CommandFailed {
                program,
                exit_status,
                stdout,
                stderr,
            } => write!(
                f,
                "`{}` が失敗 (code={:?}) stdout={} stderr={}",
                program, exit_status, stdout, stderr
            ),
        }
    }
}

impl std::error::Error for RuntimeLinkError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            RuntimeLinkError::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<io::Error> for RuntimeLinkError {
    fn from(err: io::Error) -> Self {
        RuntimeLinkError::Io(err)
    }
}

/// ランタイムライブラリの候補パス。
fn runtime_library_candidates() -> Vec<PathBuf> {
    vec![
        PathBuf::from("compiler/runtime/native/build/libreml_runtime.a"),
        PathBuf::from("/usr/local/lib/reml/libreml_runtime.a"),
    ]
}

/// `REML_RUNTIME_PATH` / デフォルト情報に基づいてランタイムライブラリを返す。
pub fn find_runtime_library() -> Result<PathBuf, RuntimeLinkError> {
    let env_value = env::var_os("REML_RUNTIME_PATH");
    if let Some(path) = env_value.as_ref() {
        let candidate = PathBuf::from(path);
        if candidate.exists() {
            return Ok(candidate);
        }
    }
    for candidate in runtime_library_candidates() {
        if candidate.exists() {
            return Ok(candidate);
        }
    }
    Err(RuntimeLinkError::RuntimeLibraryMissing {
        env_value,
        candidates: runtime_library_candidates(),
    })
}

fn describe_execution(output: &Output) -> (String, String) {
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    (stdout, stderr)
}

fn map_failure(
    program: &str,
    status: &ExitStatus,
    stdout: String,
    stderr: String,
) -> RuntimeLinkError {
    RuntimeLinkError::CommandFailed {
        program: program.to_string(),
        exit_status: status.code(),
        stdout,
        stderr,
    }
}

fn execute_command(cmd: LinkCommand) -> Result<(), RuntimeLinkError> {
    let output = cmd.to_command().output()?;
    if output.status.success() {
        Ok(())
    } else {
        let (stdout, stderr) = describe_execution(&output);
        Err(map_failure(&cmd.program, &output.status, stdout, stderr))
    }
}

/// LLVM IR ファイルをオブジェクトファイルへ変換する。
pub fn compile_ir_with_llc(ir_file: &Path, obj_file: &Path) -> Result<(), RuntimeLinkError> {
    let output = Command::new("llc")
        .arg("-filetype=obj")
        .arg(ir_file)
        .arg("-o")
        .arg(obj_file)
        .output()?;
    if output.status.success() {
        Ok(())
    } else {
        let (stdout, stderr) = describe_execution(&output);
        Err(map_failure("llc", &output.status, stdout, stderr))
    }
}

/// オブジェクトファイルとランタイムをリンクするコマンドを生成する。
pub fn generate_link_command(
    platform: Platform,
    obj_file: &Path,
    runtime_lib: &Path,
    output_file: &Path,
) -> Result<LinkCommand, RuntimeLinkError> {
    let mut args = Vec::new();
    match platform {
        Platform::MacOS => {
            args.push(obj_file.as_os_str().to_owned());
            args.push(runtime_lib.as_os_str().to_owned());
            args.push(OsString::from("-o"));
            args.push(output_file.as_os_str().to_owned());
            args.push(OsString::from("-lSystem"));
            Ok(LinkCommand {
                program: "clang".into(),
                args,
            })
        }
        Platform::Linux => {
            args.push(obj_file.as_os_str().to_owned());
            args.push(runtime_lib.as_os_str().to_owned());
            args.push(OsString::from("-o"));
            args.push(output_file.as_os_str().to_owned());
            args.push(OsString::from("-lc"));
            args.push(OsString::from("-lm"));
            Ok(LinkCommand {
                program: "clang".into(),
                args,
            })
        }
        Platform::Windows => Err(RuntimeLinkError::UnsupportedPlatform("windows".into())),
        Platform::Unknown => Err(RuntimeLinkError::UnsupportedPlatform("unknown".into())),
    }
}

/// オブジェクトファイルとランタイムライブラリをリンクする。
pub fn link_object_with_runtime(
    obj_file: &Path,
    output_file: &Path,
    platform: Platform,
    runtime_lib: &Path,
) -> Result<(), RuntimeLinkError> {
    let link_cmd = generate_link_command(platform, obj_file, runtime_lib, output_file)?;
    execute_command(link_cmd)
}

/// LLVM IR から実行ファイルを生成し、リンク後のオブジェクトファイルを削除する。
pub fn link_with_runtime(ir_file: &Path, output_file: &Path) -> Result<(), RuntimeLinkError> {
    let obj_file = env::temp_dir().join(link_object_name());
    let runtime_lib = find_runtime_library()?;
    let platform = Platform::detect();
    let result = compile_ir_with_llc(ir_file, &obj_file)
        .and_then(|_| link_object_with_runtime(&obj_file, output_file, platform, &runtime_lib));
    let _ = fs::remove_file(&obj_file);
    result
}

fn link_object_name() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_nanos())
        .unwrap_or_default();
    format!("reml_runtime_{}.o", timestamp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;

    #[test]
    fn platform_from_name() {
        assert_eq!(Platform::from_os_name("linux"), Platform::Linux);
        assert_eq!(Platform::from_os_name("macos"), Platform::MacOS);
        assert_eq!(Platform::from_os_name("darwin"), Platform::MacOS);
        assert_eq!(Platform::from_os_name("windows"), Platform::Windows);
        assert_eq!(Platform::from_os_name("unknown-os"), Platform::Unknown);
    }

    #[test]
    fn generate_link_command_linux() {
        let platform = Platform::Linux;
        let obj = PathBuf::from("/tmp/test.o");
        let runtime = PathBuf::from("/tmp/libreml_runtime.a");
        let output = PathBuf::from("/tmp/result");
        let cmd = generate_link_command(platform, &obj, &runtime, &output).unwrap();
        assert_eq!(cmd.program(), "clang");
        let args: Vec<String> = cmd
            .args()
            .iter()
            .map(|arg| arg.to_string_lossy().to_string())
            .collect();
        assert!(args.contains(&obj.to_string_lossy().to_string()));
        assert!(args.contains(&runtime.to_string_lossy().to_string()));
        assert!(args.contains(&output.to_string_lossy().to_string()));
        assert!(args.contains(&"-lc".to_string()));
        assert!(args.contains(&"-lm".to_string()));
    }

    #[test]
    fn generate_link_command_macos() {
        let platform = Platform::MacOS;
        let obj = PathBuf::from("/tmp/test.o");
        let runtime = PathBuf::from("/tmp/libreml_runtime.a");
        let output = PathBuf::from("/tmp/result");
        let cmd = generate_link_command(platform, &obj, &runtime, &output).unwrap();
        assert_eq!(cmd.program(), "clang");
        let args: Vec<String> = cmd
            .args()
            .iter()
            .map(|arg| arg.to_string_lossy().to_string())
            .collect();
        assert!(args.contains(&"-lSystem".to_string()));
    }

    #[test]
    fn find_runtime_library_prefers_env() {
        let temp_path = env::temp_dir().join("reml_runtime_test_dummy.a");
        fs::write(&temp_path, b"dummy").unwrap();
        env::set_var("REML_RUNTIME_PATH", &temp_path);
        let found = find_runtime_library().expect("Runtime path should be found");
        assert_eq!(found, temp_path);
        env::remove_var("REML_RUNTIME_PATH");
        let _ = fs::remove_file(&temp_path);
    }
}
