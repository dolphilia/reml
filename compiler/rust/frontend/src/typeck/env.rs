//! 型推論モジュール全体で共有する設定やデュアルライト補助ツール。

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use once_cell::sync::OnceCell;
use serde::Serialize;
use smol_str::SmolStr;
use thiserror::Error;

const DEFAULT_DUALWRITE_ROOT: &str = "reports/dual-write/front-end";

static GLOBAL_TYPECHECK_CONFIG: OnceCell<TypecheckConfig> = OnceCell::new();

/// 型推論フェーズで利用する設定値。
///
/// OCaml 版 `Type_inference.make_config` のパラメータを Rust でも
/// 再現する目的で導入している。今後 W3/W4 の実装に合わせて
/// 項目を拡張する前提のスケルトン。
#[derive(Debug, Clone, Serialize)]
pub struct TypecheckConfig {
    /// 効果プロファイルや Capability Stage を判定するための文脈。
    pub effect_context: StageContext,
    /// 型行（effect row）をどのように処理するかのモード。
    pub type_row_mode: TypeRowMode,
    /// Recover 拡張の挙動を制御する設定。
    pub recover: RecoverConfig,
}

impl TypecheckConfig {
    /// 既定値をベースにしたビルダーを返す。
    pub fn builder() -> TypecheckConfigBuilder {
        TypecheckConfigBuilder::default()
    }
}

impl Default for TypecheckConfig {
    fn default() -> Self {
        Self {
            effect_context: StageContext::default(),
            type_row_mode: TypeRowMode::Integrated,
            recover: RecoverConfig::default(),
        }
    }
}

/// TypecheckConfig を生成するためのビルダー。
#[derive(Debug, Default)]
pub struct TypecheckConfigBuilder {
    effect_context: Option<StageContext>,
    type_row_mode: Option<TypeRowMode>,
    recover: Option<RecoverConfig>,
}

impl TypecheckConfigBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn effect_context(mut self, ctx: StageContext) -> Self {
        self.effect_context = Some(ctx);
        self
    }

    pub fn type_row_mode(mut self, mode: TypeRowMode) -> Self {
        self.type_row_mode = Some(mode);
        self
    }

    pub fn recover(mut self, recover: RecoverConfig) -> Self {
        self.recover = Some(recover);
        self
    }

    pub fn build(self) -> TypecheckConfig {
        TypecheckConfig {
            effect_context: self.effect_context.unwrap_or_default(),
            type_row_mode: self.type_row_mode.unwrap_or(TypeRowMode::Integrated),
            recover: self.recover.unwrap_or_default(),
        }
    }
}

/// グローバル設定をインストールする。
pub fn install_config(config: TypecheckConfig) -> Result<(), InstallConfigError> {
    GLOBAL_TYPECHECK_CONFIG
        .set(config)
        .map_err(|_| InstallConfigError::AlreadyInstalled)
}

/// グローバル設定を取得する。
pub fn config() -> &'static TypecheckConfig {
    GLOBAL_TYPECHECK_CONFIG.get_or_init(TypecheckConfig::default)
}

/// install_config でエラーが発生した場合の種別。
#[derive(Debug, Error)]
pub enum InstallConfigError {
    #[error("TypecheckConfig はすでに設定済みです")]
    AlreadyInstalled,
}

/// 効果ステージに関する最小限の文脈情報。
#[derive(Debug, Clone, Serialize)]
pub struct StageContext {
    pub runtime: StageRequirement,
    pub capability: StageRequirement,
}

impl Default for StageContext {
    fn default() -> Self {
        Self {
            runtime: StageRequirement::AtLeast(StageId::stable()),
            capability: StageRequirement::AtLeast(StageId::beta()),
        }
    }
}

/// StageRequirement で使用する ID 種別。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct StageId(SmolStr);

impl StageId {
    pub fn new(value: impl Into<SmolStr>) -> Self {
        Self(value.into())
    }

    pub fn stable() -> Self {
        Self::new("stable")
    }

    pub fn beta() -> Self {
        Self::new("beta")
    }

    pub fn experimental() -> Self {
        Self::new("experimental")
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for StageId {
    fn default() -> Self {
        Self::stable()
    }
}

/// Capability Stage の要求を表す。
impl FromStr for StageId {
    type Err = StageIdParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "stable" => Ok(StageId::stable()),
            "beta" => Ok(StageId::beta()),
            "experimental" | "exper" | "exp" => Ok(StageId::experimental()),
            other if !other.is_empty() => Ok(StageId::new(other)),
            _ => Err(StageIdParseError::Empty),
        }
    }
}

#[derive(Debug, Error)]
pub enum StageIdParseError {
    #[error("stage_id が空です")]
    Empty,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum StageRequirement {
    Exact(StageId),
    AtLeast(StageId),
}

/// 型行の処理モードを表す列挙。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum TypeRowMode {
    MetadataOnly,
    DualWrite,
    Integrated,
}

impl Default for TypeRowMode {
    fn default() -> Self {
        TypeRowMode::Integrated
    }
}

impl FromStr for TypeRowMode {
    type Err = TypeRowModeParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "metadata-only" | "metadata_only" => Ok(TypeRowMode::MetadataOnly),
            "dual-write" | "dual_write" | "dual" => Ok(TypeRowMode::DualWrite),
            "integrated" | "full" | "default" => Ok(TypeRowMode::Integrated),
            other => Err(TypeRowModeParseError(other.to_string())),
        }
    }
}

#[derive(Debug, Error)]
#[error("未知の type_row_mode: {0}")]
pub struct TypeRowModeParseError(String);

/// Recover 拡張の挙動を制御する設定。
#[derive(Debug, Clone, Serialize)]
pub struct RecoverConfig {
    pub emit_expected_tokens: bool,
    pub emit_context: bool,
    pub max_suggestions: usize,
}

impl Default for RecoverConfig {
    fn default() -> Self {
        Self {
            emit_expected_tokens: true,
            emit_context: true,
            max_suggestions: 3,
        }
    }
}

/// Dual-write 結果を管理するヘルパ。
#[derive(Debug, Clone)]
pub struct DualWriteGuards {
    root: PathBuf,
    run_label: SmolStr,
    case_label: SmolStr,
}

impl DualWriteGuards {
    /// 既定ルートまたは `REML_FRONTEND_DUALWRITE_ROOT` を基に初期化する。
    pub fn new(run_label: impl Into<SmolStr>, case_label: impl Into<SmolStr>) -> io::Result<Self> {
        let base = std::env::var("REML_FRONTEND_DUALWRITE_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(DEFAULT_DUALWRITE_ROOT));
        Self::with_root(base, run_label, case_label)
    }

    /// ルートディレクトリを明示指定して初期化する。
    pub fn with_root(
        base: impl Into<PathBuf>,
        run_label: impl Into<SmolStr>,
        case_label: impl Into<SmolStr>,
    ) -> io::Result<Self> {
        let run_label = sanitize_label(run_label.into());
        let case_label = sanitize_label(case_label.into());
        let base = base.into();
        let root = base.join(run_label.as_str()).join(case_label.as_str());
        fs::create_dir_all(&root)?;
        Ok(Self {
            root,
            run_label,
            case_label,
        })
    }

    /// ルートディレクトリを返す。
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// ラベル情報を取得する。
    pub fn labels(&self) -> (&SmolStr, &SmolStr) {
        (&self.run_label, &self.case_label)
    }

    /// 任意の相対パスをルート以下へ連結する。
    pub fn path(&self, relative: impl AsRef<Path>) -> PathBuf {
        let path = self.root.join(relative);
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        path
    }

    /// JSON を pretty 形式で保存する。
    pub fn write_json<T: Serialize>(
        &self,
        relative: impl AsRef<Path>,
        value: &T,
    ) -> io::Result<PathBuf> {
        let path = self.path(relative);
        let json = serde_json::to_vec_pretty(value)?;
        fs::write(&path, json)?;
        Ok(path)
    }

    /// バイト列を保存する。
    pub fn write_bytes(
        &self,
        relative: impl AsRef<Path>,
        bytes: impl AsRef<[u8]>,
    ) -> io::Result<PathBuf> {
        let path = self.path(relative);
        fs::write(&path, bytes)?;
        Ok(path)
    }
}

fn sanitize_label(label: SmolStr) -> SmolStr {
    let mut sanitized = String::with_capacity(label.len());
    for ch in label.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
            sanitized.push(ch.to_ascii_lowercase());
        } else if ch.is_whitespace() || ch == '.' || ch == '/' {
            sanitized.push('_');
        }
    }
    if sanitized.is_empty() {
        sanitized.push_str("case");
    }
    SmolStr::new(sanitized)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_defaults_match_struct_default() {
        let built = TypecheckConfig::builder().build();
        assert_eq!(built.type_row_mode, TypeRowMode::Integrated);
        assert_eq!(built.recover.max_suggestions, 3);
    }

    #[test]
    fn dualwrite_creates_directory() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let guards =
            DualWriteGuards::with_root(tempdir.path(), "Run-01", "Case A").expect("guards");
        assert!(guards.root().exists());
        guards
            .write_bytes("foo/bar.txt", "hello")
            .expect("write bytes");
        assert!(guards.root().join("foo/bar.txt").exists());
    }
}
