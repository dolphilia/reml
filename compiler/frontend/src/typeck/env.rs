//! 型推論モジュール全体で共有する設定やデュアルライト補助ツール。

use std::collections::HashSet;
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use serde_json::{Map, Value};

use super::capability::RuntimeCapability;
use super::scheme::Scheme;
use super::types::TypeVariable;
use crate::parser::ast::{TypeDeclBody, TypeDeclVariantPayload};
use crate::span::Span;
use indexmap::IndexMap;
use once_cell::sync::OnceCell;
use serde::Serialize;
use smol_str::SmolStr;
use thiserror::Error;

const DEFAULT_DUALWRITE_ROOT: &str = "reports/dual-write/front-end";

static GLOBAL_TYPECHECK_CONFIG: OnceCell<TypecheckConfig> = OnceCell::new();

/// 型推論フェーズで利用する設定値。
///
/// 型推論の構成要素をまとめるために導入しており、今後 W3/W4 の実装に合わせて
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
    pub runtime_capabilities: Vec<RuntimeCapability>,
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
    runtime_capabilities: Option<Vec<RuntimeCapability>>,
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

    pub fn runtime_capabilities(mut self, capabilities: Vec<RuntimeCapability>) -> Self {
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

/// Stage トレースの各ステップ。
#[derive(Debug, Clone, Serialize)]
pub struct StageTraceStep {
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capability: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
}

impl StageTraceStep {
    pub fn to_value(&self) -> Value {
        let mut metadata = Map::new();
        metadata.insert("source".to_string(), Value::String(self.source.clone()));
        if let Some(stage) = &self.stage {
            metadata.insert("stage".to_string(), Value::String(stage.clone()));
        }
        if let Some(capability) = &self.capability {
            metadata.insert("capability".to_string(), Value::String(capability.clone()));
        }
        if let Some(note) = &self.note {
            metadata.insert("note".to_string(), Value::String(note.clone()));
        }
        if let Some(file) = &self.file {
            metadata.insert("file".to_string(), Value::String(file.clone()));
        }
        if let Some(target) = &self.target {
            metadata.insert("target".to_string(), Value::String(target.clone()));
        }
        Value::Object(metadata)
    }
}

pub type StageTrace = Vec<StageTraceStep>;

/// 効果ステージに関する最小限の文脈情報。
#[derive(Debug, Clone, Serialize)]
pub struct StageContext {
    pub runtime: StageRequirement,
    pub capability: StageRequirement,
    pub stage_trace: StageTrace,
}

impl Default for StageContext {
    fn default() -> Self {
        Self {
            runtime: StageRequirement::AtLeast(StageId::stable()),
            capability: StageRequirement::AtLeast(StageId::beta()),
            stage_trace: Vec::new(),
        }
    }
}

impl StageContext {
    pub fn resolve(
        cli_stage_override: Option<StageId>,
        runtime_stage_override: Option<StageRequirement>,
        capability_stage_override: Option<StageRequirement>,
        runtime_capabilities: &[RuntimeCapability],
        target_triple: Option<&str>,
    ) -> Self {
        let (registry, registry_path) = load_runtime_registry();
        let env_stage = stage_from_env_var(ENV_STAGE_VAR);
        let legacy_stage = stage_from_env_var(LEGACY_STAGE_VAR);
        let default_stage = cli_stage_override
            .clone()
            .or_else(|| registry.stage.clone())
            .or_else(|| env_stage.clone())
            .or_else(|| legacy_stage.clone())
            .unwrap_or_else(StageId::stable);
        let capability_stages = build_capability_stage_map(
            &registry,
            target_triple,
            &default_stage,
            runtime_capabilities,
        );
        let required_caps = unique_required_capabilities(runtime_capabilities);
        let stage_trace = build_stage_trace(
            cli_stage_override.as_ref(),
            env_stage.as_ref(),
            registry_path.as_ref(),
            &registry,
            &capability_stages,
            &required_caps,
        );
        let runtime = runtime_stage_override
            .unwrap_or_else(|| StageRequirement::AtLeast(default_stage.clone()));
        let capability =
            capability_stage_override.unwrap_or_else(|| StageRequirement::AtLeast(StageId::beta()));
        Self {
            runtime,
            capability,
            stage_trace,
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

const ENV_STAGE_VAR: &str = "REMLC_EFFECT_STAGE";
const LEGACY_STAGE_VAR: &str = "REML_RUNTIME_STAGE";
const REGISTRY_ENV_VAR: &str = "REML_RUNTIME_CAPABILITIES";
const RUN_CONFIG_NOTE: &str = "run_config.effects.required_capabilities";

#[derive(Debug, Clone)]
struct CapabilityEntry {
    name: String,
    stage: Option<StageId>,
}

impl CapabilityEntry {
    fn new(name: &str, stage: Option<StageId>) -> Option<Self> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return None;
        }
        Some(Self {
            name: trimmed.to_ascii_lowercase(),
            stage,
        })
    }
}

#[derive(Debug, Clone)]
struct RuntimeCapabilityRegistry {
    stage: Option<StageId>,
    capabilities: Vec<CapabilityEntry>,
    overrides: Vec<(String, Vec<CapabilityEntry>)>,
}

impl RuntimeCapabilityRegistry {
    fn empty() -> Self {
        Self {
            stage: None,
            capabilities: Vec::new(),
            overrides: Vec::new(),
        }
    }
}

fn load_runtime_registry() -> (RuntimeCapabilityRegistry, Option<PathBuf>) {
    match env::var(REGISTRY_ENV_VAR) {
        Ok(raw) => {
            let path_string = raw.trim();
            if path_string.is_empty() {
                return (RuntimeCapabilityRegistry::empty(), None);
            }
            let path = PathBuf::from(path_string);
            let registry = load_registry_from_file(&path);
            (registry, Some(path))
        }
        Err(_) => (RuntimeCapabilityRegistry::empty(), None),
    }
}

fn load_registry_from_file(path: &Path) -> RuntimeCapabilityRegistry {
    let contents = match fs::read_to_string(path) {
        Ok(value) => value,
        Err(err) => {
            eprintln!(
                "[RUNTIME CAPABILITY] {} の読み込みに失敗しました: {err}",
                path.display()
            );
            return RuntimeCapabilityRegistry::empty();
        }
    };
    match serde_json::from_str::<Value>(&contents) {
        Ok(Value::Object(map)) => parse_registry_object(&map),
        Ok(_) => RuntimeCapabilityRegistry::empty(),
        Err(err) => {
            eprintln!(
                "[RUNTIME CAPABILITY] {} を JSON として解析できません: {err}",
                path.display()
            );
            RuntimeCapabilityRegistry::empty()
        }
    }
}

fn parse_registry_object(map: &Map<String, Value>) -> RuntimeCapabilityRegistry {
    let stage = map.get("stage").and_then(parse_stage_opt);
    let capabilities = map
        .get("capabilities")
        .map(|value| parse_capability_entries(value))
        .unwrap_or_default();
    let overrides = map
        .get("overrides")
        .and_then(|value| value.as_object())
        .map(|entries| {
            entries
                .iter()
                .filter_map(|(target, data)| {
                    let normalized_target = normalize_key(target);
                    let parsed = parse_override_section(data);
                    if parsed.is_empty() {
                        None
                    } else {
                        Some((normalized_target, parsed))
                    }
                })
                .collect()
        })
        .unwrap_or_default();
    RuntimeCapabilityRegistry {
        stage,
        capabilities,
        overrides,
    }
}

fn parse_capability_entries(value: &Value) -> Vec<CapabilityEntry> {
    match value {
        Value::String(name) => CapabilityEntry::new(name, None).into_iter().collect(),
        Value::Array(items) => items
            .iter()
            .flat_map(|entry| parse_capability_entries(entry))
            .collect(),
        Value::Object(map) => {
            if let Some(name) = map.get("name").and_then(|value| value.as_str()) {
                CapabilityEntry::new(name, map.get("stage").and_then(parse_stage_opt))
                    .into_iter()
                    .collect()
            } else {
                map.iter()
                    .filter_map(|(cap_name, data)| {
                        CapabilityEntry::new(cap_name, parse_stage_opt(data))
                    })
                    .collect()
            }
        }
        _ => Vec::new(),
    }
}

fn parse_override_section(value: &Value) -> Vec<CapabilityEntry> {
    match value {
        Value::Object(map) => {
            let stage_override = map.get("stage").and_then(parse_stage_opt);
            let capabilities_value = map.get("capabilities").unwrap_or(value);
            let mut entries = parse_capability_entries(capabilities_value);
            if let Some(stage) = stage_override {
                for entry in &mut entries {
                    if entry.stage.is_none() {
                        entry.stage = Some(stage.clone());
                    }
                }
            }
            entries
        }
        other => parse_capability_entries(other),
    }
}

fn parse_stage_opt(value: &Value) -> Option<StageId> {
    match value {
        Value::String(stage) => StageId::from_str(stage).ok(),
        Value::Object(map) => map.get("stage").and_then(|inner| parse_stage_opt(inner)),
        _ => None,
    }
}

fn stage_from_env_var(key: &str) -> Option<StageId> {
    env::var(key)
        .ok()
        .and_then(|value| StageId::from_str(&value).ok())
}

fn normalize_key(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn unique_required_capabilities(capabilities: &[RuntimeCapability]) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut result = Vec::new();
    for capability in capabilities {
        let normalized = normalize_key(capability.id());
        if normalized.is_empty() || !seen.insert(normalized.clone()) {
            continue;
        }
        result.push(normalized);
    }
    result
}

fn build_capability_stage_map(
    registry: &RuntimeCapabilityRegistry,
    target_triple: Option<&str>,
    default_stage: &StageId,
    runtime_capabilities: &[RuntimeCapability],
) -> Vec<(String, StageId)> {
    let mut entries = registry.capabilities.clone();
    if let Some(triple) = target_triple {
        let normalized_target = normalize_key(triple);
        for (target_key, overrides) in &registry.overrides {
            if target_key == &normalized_target {
                entries.extend(overrides.clone());
            }
        }
    }
    for capability in runtime_capabilities {
        if let Some(entry) = CapabilityEntry::new(capability.id().as_str(), None) {
            entries.push(entry);
        }
    }
    dedup_capability_entries(entries, default_stage)
}

fn dedup_capability_entries(
    entries: Vec<CapabilityEntry>,
    default_stage: &StageId,
) -> Vec<(String, StageId)> {
    let mut accumulator = Vec::new();
    for mut entry in entries {
        let normalized = normalize_key(&entry.name);
        if normalized.is_empty() {
            continue;
        }
        accumulator.retain(|(existing, _)| existing != &normalized);
        let stage = entry.stage.take().unwrap_or_else(|| default_stage.clone());
        accumulator.push((normalized, stage));
    }
    accumulator
}

fn build_stage_trace(
    cli_stage: Option<&StageId>,
    env_stage: Option<&StageId>,
    registry_path: Option<&PathBuf>,
    registry: &RuntimeCapabilityRegistry,
    capability_stages: &[(String, StageId)],
    required_caps: &[String],
) -> StageTrace {
    let mut trace = Vec::new();
    trace.push(stage_trace_cli_step(cli_stage));
    trace.push(stage_trace_env_step(env_stage, ENV_STAGE_VAR));
    let required_set: HashSet<String> = required_caps.iter().cloned().collect();
    for (capability, stage) in capability_stages {
        if required_set.contains(capability) {
            trace.push(stage_trace_run_config(capability, stage));
        }
    }
    if let Some(path) = registry_path {
        trace.push(stage_trace_registry(path, registry.stage.as_ref()));
    }
    trace.extend(stage_trace_override_steps(registry, registry_path));
    trace
}

fn stage_trace_cli_step(stage: Option<&StageId>) -> StageTraceStep {
    let note = if let Some(stage) = stage {
        Some(format!("--effect-stage {}", stage.as_str()))
    } else {
        Some("not provided".to_string())
    };
    StageTraceStep {
        source: "cli_option".to_string(),
        stage: stage.map(|value| value.as_str().to_string()),
        capability: None,
        note,
        file: None,
        target: None,
    }
}

fn stage_trace_env_step(stage: Option<&StageId>, var_name: &str) -> StageTraceStep {
    let note = if stage.is_some() {
        Some(var_name.to_string())
    } else {
        Some(format!("{var_name} not set"))
    };
    StageTraceStep {
        source: "env_var".to_string(),
        stage: stage.map(|value| value.as_str().to_string()),
        capability: None,
        note,
        file: None,
        target: None,
    }
}

fn stage_trace_run_config(capability: &str, stage: &StageId) -> StageTraceStep {
    StageTraceStep {
        source: "run_config".to_string(),
        stage: Some(stage.as_str().to_string()),
        capability: Some(capability.to_string()),
        note: Some(RUN_CONFIG_NOTE.to_string()),
        file: None,
        target: None,
    }
}

fn stage_trace_registry(path: &PathBuf, stage: Option<&StageId>) -> StageTraceStep {
    StageTraceStep {
        source: "capability_json".to_string(),
        stage: stage.map(|value| value.as_str().to_string()),
        capability: None,
        note: None,
        file: Some(path.display().to_string()),
        target: None,
    }
}

fn stage_trace_override_steps(
    registry: &RuntimeCapabilityRegistry,
    registry_path: Option<&PathBuf>,
) -> StageTrace {
    registry
        .overrides
        .iter()
        .map(|(target_key, entries)| {
            let stage_candidate = entries
                .first()
                .and_then(|entry| entry.stage.clone())
                .or_else(|| registry.stage.clone());
            StageTraceStep {
                source: "runtime_candidate".to_string(),
                stage: stage_candidate.map(|value| value.as_str().to_string()),
                capability: None,
                note: None,
                file: registry_path.map(|path| path.display().to_string()),
                target: Some(target_key.clone()),
            }
        })
        .collect()
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

/// 型宣言の種別。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeDeclKind {
    Alias,
    Newtype,
    Sum,
    Opaque,
}

/// 型環境で保持する型宣言情報。
#[derive(Debug, Clone)]
pub struct TypeDeclBinding {
    pub name: String,
    pub generics: Vec<String>,
    pub kind: TypeDeclKind,
    pub body: Option<TypeDeclBody>,
    pub span: Span,
    pub body_span: Option<Span>,
}

impl TypeDeclBinding {
    pub fn new(
        name: impl Into<String>,
        generics: Vec<String>,
        kind: TypeDeclKind,
        body: Option<TypeDeclBody>,
        span: Span,
        body_span: Option<Span>,
    ) -> Self {
        Self {
            name: name.into(),
            generics,
            kind,
            body,
            span,
            body_span,
        }
    }
}

/// 合成型のコンストラクタ情報。
#[derive(Debug, Clone)]
pub struct TypeConstructorBinding {
    pub name: String,
    pub parent: String,
    pub generics: Vec<String>,
    pub payload: Option<TypeDeclVariantPayload>,
    pub span: Span,
}

impl TypeConstructorBinding {
    pub fn new(
        name: impl Into<String>,
        parent: impl Into<String>,
        generics: Vec<String>,
        payload: Option<TypeDeclVariantPayload>,
        span: Span,
    ) -> Self {
        Self {
            name: name.into(),
            parent: parent.into(),
            generics,
            payload,
            span,
        }
    }
}

/// 型推論で利用する環境。新しいスコープは `enter_scope` で作られ、`exit_scope` で親に戻る。
#[derive(Debug, Clone)]
pub struct TypeEnv {
    bindings: IndexMap<String, Binding>,
    type_decls: IndexMap<String, TypeDeclBinding>,
    type_constructors: IndexMap<String, TypeConstructorBinding>,
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
            type_decls: IndexMap::new(),
            type_constructors: IndexMap::new(),
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

    pub fn insert_type_decl(&mut self, binding: TypeDeclBinding) {
        self.type_decls.insert(binding.name.clone(), binding);
    }

    pub fn lookup_type_decl(&self, name: &str) -> Option<&TypeDeclBinding> {
        if let Some(binding) = self.type_decls.get(name) {
            Some(binding)
        } else {
            self.parent
                .as_deref()
                .and_then(|parent| parent.lookup_type_decl(name))
        }
    }

    pub fn insert_type_constructor(&mut self, binding: TypeConstructorBinding) {
        self.type_constructors.insert(binding.name.clone(), binding);
    }

    pub fn lookup_type_constructor(&self, name: &str) -> Option<&TypeConstructorBinding> {
        if let Some(binding) = self.type_constructors.get(name) {
            Some(binding)
        } else {
            self.parent
                .as_deref()
                .and_then(|parent| parent.lookup_type_constructor(name))
        }
    }

    pub fn enter_scope(&self) -> TypeEnv {
        TypeEnv {
            bindings: IndexMap::new(),
            type_decls: IndexMap::new(),
            type_constructors: IndexMap::new(),
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
