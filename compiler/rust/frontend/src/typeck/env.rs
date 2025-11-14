//! 型推論モジュール全体で共有する設定やデュアルライト補助ツール。

use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use super::scheme::Scheme;
use super::types::TypeVariable;
use indexmap::IndexMap;
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
    /// 実験的効果を許可するかどうか。
    pub experimental_effects: bool,
    /// CLI から指定された Runtime Capabilities。
    pub runtime_capabilities: Vec<String>,
    /// 型推論で詳細トレースを出力するかどうか。
    pub trace_enabled: bool,
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
            experimental_effects: false,
            runtime_capabilities: Vec::new(),
            trace_enabled: false,
        }
    }
}

/// TypecheckConfig を生成するためのビルダー。
#[derive(Debug, Default)]
pub struct TypecheckConfigBuilder {
    effect_context: Option<StageContext>,
    type_row_mode: Option<TypeRowMode>,
    recover: Option<RecoverConfig>,
    experimental_effects: Option<bool>,
    runtime_capabilities: Option<Vec<String>>,
    trace_enabled: Option<bool>,
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

    pub fn experimental_effects(mut self, enabled: bool) -> Self {
        self.experimental_effects = Some(enabled);
        self
    }

    pub fn runtime_capabilities(mut self, capabilities: Vec<String>) -> Self {
        self.runtime_capabilities = Some(capabilities);
        self
    }

    pub fn trace_enabled(mut self, enabled: bool) -> Self {
        self.trace_enabled = Some(enabled);
        self
    }

    pub fn build(self) -> TypecheckConfig {
        TypecheckConfig {
            effect_context: self.effect_context.unwrap_or_default(),
            type_row_mode: self.type_row_mode.unwrap_or(TypeRowMode::Integrated),
            recover: self.recover.unwrap_or_default(),
            experimental_effects: self.experimental_effects.unwrap_or(false),
            runtime_capabilities: self.runtime_capabilities.unwrap_or_else(Vec::new),
            trace_enabled: self.trace_enabled.unwrap_or(false),
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

    fn rank_value(&self) -> u8 {
        match self.as_str().to_ascii_lowercase().as_str() {
            "stable" => 0,
            "beta" => 1,
            "experimental" => 2,
            _ => 3,
        }
    }

    pub fn max(a: &StageId, b: &StageId) -> StageId {
        if a.rank_value() >= b.rank_value() {
            a.clone()
        } else {
            b.clone()
        }
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

impl StageRequirement {
    pub fn base_stage(&self) -> &StageId {
        match self {
            StageRequirement::Exact(stage) | StageRequirement::AtLeast(stage) => stage,
        }
    }

    pub fn is_exact(&self) -> bool {
        matches!(self, StageRequirement::Exact(_))
    }

    pub fn label(&self) -> String {
        match self {
            StageRequirement::Exact(stage) => stage.as_str().to_string(),
            StageRequirement::AtLeast(stage) => format!("at_least:{}", stage.as_str()),
        }
    }

    pub fn rank(&self) -> u8 {
        self.base_stage().rank_value()
    }

    pub fn satisfies(&self, required: &StageRequirement) -> bool {
        let actual_rank = self.rank();
        let required_rank = required.rank();
        if actual_rank < required_rank {
            return false;
        }
        if required.is_exact() {
            actual_rank == required_rank
        } else {
            true
        }
    }

    pub fn merged_with(lhs: &StageRequirement, rhs: &StageRequirement) -> StageRequirement {
        let max_stage = StageId::max(lhs.base_stage(), rhs.base_stage());
        StageRequirement::AtLeast(max_stage)
    }
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
            "metadata-only" | "metadata_only" | "metadata" => Ok(TypeRowMode::MetadataOnly),
            "dual-write" | "dual_write" | "dual" | "dualwrite" => Ok(TypeRowMode::DualWrite),
            "integrated" | "full" | "default" | "ty-integrated" => Ok(TypeRowMode::Integrated),
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

/// 型環境で保持する束縛。
#[derive(Debug, Clone)]
pub struct Binding {
    pub scheme: Scheme,
}

/// 型推論で利用する環境。新しいスコープは `enter_scope` で作られ、`exit_scope` で親に戻る。
#[derive(Debug, Clone)]
pub struct TypeEnv {
    bindings: IndexMap<String, Binding>,
    parent: Option<Box<TypeEnv>>,
}

impl Default for TypeEnv {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeEnv {
    pub fn new() -> Self {
        Self {
            bindings: IndexMap::new(),
            parent: None,
        }
    }

    pub fn insert(&mut self, name: impl Into<String>, scheme: Scheme) {
        self.bindings.insert(name.into(), Binding { scheme });
    }

    pub fn lookup(&self, name: &str) -> Option<&Binding> {
        if let Some(binding) = self.bindings.get(name) {
            Some(binding)
        } else {
            self.parent
                .as_deref()
                .and_then(|parent| parent.lookup(name))
        }
    }

    pub fn enter_scope(&self) -> TypeEnv {
        TypeEnv {
            bindings: IndexMap::new(),
            parent: Some(Box::new(self.clone())),
        }
    }

    pub fn exit_scope(self) -> Option<TypeEnv> {
        self.parent.map(|parent| *parent)
    }

    pub fn free_type_variables(&self) -> HashSet<TypeVariable> {
        let mut vars = HashSet::new();
        self.collect_free_type_variables(&mut vars);
        vars
    }

    fn collect_free_type_variables(&self, vars: &mut HashSet<TypeVariable>) {
        for binding in self.bindings.values() {
            vars.extend(binding.scheme.ty.free_type_variables());
            for constraint_ty in binding.scheme.constraints.values() {
                vars.extend(constraint_ty.free_type_variables());
            }
        }
        if let Some(parent) = &self.parent {
            parent.collect_free_type_variables(vars);
        }
    }
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
