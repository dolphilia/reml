use crate::{
    prelude::ensure::{DiagnosticSeverity, GuardDiagnostic},
    stage::StageId,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::{
    collections::BTreeSet,
    fmt,
    path::{Path, PathBuf},
    str::FromStr,
};

const CONFIG_COMPAT_DOMAIN: &str = "config";

pub const CONFIG_COMPAT_TRAILING_COMMA_CODE: &str = "config.compat.trailing_comma";
pub const CONFIG_COMPAT_UNQUOTED_KEY_CODE: &str = "config.compat.unquoted_key";
pub const CONFIG_COMPAT_DUPLICATE_KEY_CODE: &str = "config.compat.duplicate_key";
pub const CONFIG_COMPAT_NUMBER_CODE: &str = "config.compat.number_format";

/// Config ファイル互換モードを表す。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConfigCompatibility {
    #[serde(default)]
    pub trivia: ConfigTriviaProfile,
    #[serde(default)]
    pub trailing_comma: TrailingCommaMode,
    #[serde(default)]
    pub unquoted_key: KeyPolicy,
    #[serde(default)]
    pub duplicate_key: DuplicateKeyPolicy,
    #[serde(default)]
    pub number: NumberCompatibility,
    #[serde(default)]
    pub feature_guard: BTreeSet<String>,
}

impl Default for ConfigCompatibility {
    fn default() -> Self {
        Self::strict_json()
    }
}

impl ConfigCompatibility {
    /// JSON Strict プロファイル（Stage::Stable 相当）。
    pub fn strict_json() -> Self {
        Self {
            trivia: ConfigTriviaProfile::strict_json(),
            trailing_comma: TrailingCommaMode::Forbid,
            unquoted_key: KeyPolicy::Forbid,
            duplicate_key: DuplicateKeyPolicy::Error,
            number: NumberCompatibility::Strict,
            feature_guard: BTreeSet::new(),
        }
    }

    /// JSON Relaxed プロファイル（Stage::Beta/Experimental で利用）。
    pub fn relaxed_json() -> Self {
        Self {
            trivia: ConfigTriviaProfile::json_relaxed(),
            trailing_comma: TrailingCommaMode::ArraysAndObjects,
            unquoted_key: KeyPolicy::AllowAlpha,
            duplicate_key: DuplicateKeyPolicy::LastWriteWins,
            number: NumberCompatibility::AllowLeadingPlus,
            feature_guard: BTreeSet::new(),
        }
    }

    /// TOML Strict プロファイル（互換機能を明示的に制限）。
    pub fn strict_toml() -> Self {
        Self {
            trivia: ConfigTriviaProfile::toml_relaxed(),
            trailing_comma: TrailingCommaMode::Forbid,
            unquoted_key: KeyPolicy::AllowAlphaNumeric,
            duplicate_key: DuplicateKeyPolicy::Error,
            number: NumberCompatibility::Strict,
            feature_guard: BTreeSet::new(),
        }
    }

    /// TOML Relaxed プロファイル（Stage::Stable の既定値）。
    pub fn relaxed_toml() -> Self {
        Self {
            trivia: ConfigTriviaProfile::toml_relaxed(),
            trailing_comma: TrailingCommaMode::ArraysAndObjects,
            unquoted_key: KeyPolicy::AllowAlphaNumeric,
            duplicate_key: DuplicateKeyPolicy::Error,
            number: NumberCompatibility::Strict,
            feature_guard: BTreeSet::new(),
        }
    }

    /// Stage::Stable のフォーマット別初期値。
    pub fn stable(format: ConfigFormat) -> Self {
        match format {
            ConfigFormat::Json => Self::strict_json(),
            ConfigFormat::Toml => Self::relaxed_toml(),
        }
    }
}

/// Stage / Format 組み合わせから推奨互換プロファイルを構築する。
pub fn compatibility_profile_for_stage(
    format: ConfigFormat,
    stage: StageId,
) -> ConfigCompatibility {
    match stage {
        StageId::Stable => ConfigCompatibility::stable(format),
        StageId::Beta => match format {
            ConfigFormat::Json => ConfigCompatibility::relaxed_json(),
            ConfigFormat::Toml => ConfigCompatibility::relaxed_toml(),
        },
        StageId::Alpha => match format {
            ConfigFormat::Json => ConfigCompatibility::relaxed_json(),
            ConfigFormat::Toml => ConfigCompatibility::strict_toml(),
        },
        StageId::Experimental => match format {
            ConfigFormat::Json => ConfigCompatibility::relaxed_json(),
            ConfigFormat::Toml => ConfigCompatibility::relaxed_toml(),
        },
    }
}

/// `config.compatibility` セクションで想定するフォーマット。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConfigFormat {
    Json,
    Toml,
}

impl ConfigFormat {
    pub fn as_str(&self) -> &'static str {
        match self {
            ConfigFormat::Json => "json",
            ConfigFormat::Toml => "toml",
        }
    }
}

/// 互換性プロファイルの識別子。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompatibilityProfile {
    StrictJson,
    JsonRelaxed,
    TomlStrict,
    TomlRelaxed,
}

impl CompatibilityProfile {
    pub fn format(&self) -> ConfigFormat {
        match self {
            CompatibilityProfile::StrictJson | CompatibilityProfile::JsonRelaxed => {
                ConfigFormat::Json
            }
            CompatibilityProfile::TomlStrict | CompatibilityProfile::TomlRelaxed => {
                ConfigFormat::Toml
            }
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            CompatibilityProfile::StrictJson => "strict-json",
            CompatibilityProfile::JsonRelaxed => "json-relaxed",
            CompatibilityProfile::TomlStrict => "toml-strict",
            CompatibilityProfile::TomlRelaxed => "toml-relaxed",
        }
    }

    pub fn into_compat(self) -> ConfigCompatibility {
        match self {
            CompatibilityProfile::StrictJson => ConfigCompatibility::strict_json(),
            CompatibilityProfile::JsonRelaxed => ConfigCompatibility::relaxed_json(),
            CompatibilityProfile::TomlStrict => ConfigCompatibility::strict_toml(),
            CompatibilityProfile::TomlRelaxed => ConfigCompatibility::relaxed_toml(),
        }
    }
}

impl FromStr for CompatibilityProfile {
    type Err = CompatibilityProfileError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let normalized = s.trim().to_ascii_lowercase();
        match normalized.as_str() {
            "strict" | "strict-json" | "json-strict" => Ok(CompatibilityProfile::StrictJson),
            "json-relaxed" | "relaxed-json" => Ok(CompatibilityProfile::JsonRelaxed),
            "toml" | "toml-strict" | "strict-toml" => Ok(CompatibilityProfile::TomlStrict),
            "toml-relaxed" | "relaxed-toml" => Ok(CompatibilityProfile::TomlRelaxed),
            _ => Err(CompatibilityProfileError::new(s)),
        }
    }
}

/// プロファイル名解析時のエラー。
#[derive(Debug, Clone)]
pub struct CompatibilityProfileError {
    requested: String,
}

impl CompatibilityProfileError {
    pub fn new(requested: impl Into<String>) -> Self {
        Self {
            requested: requested.into(),
        }
    }

    pub fn requested(&self) -> &str {
        &self.requested
    }
}

impl fmt::Display for CompatibilityProfileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "未知の互換プロファイル `{}`", self.requested)
    }
}

impl std::error::Error for CompatibilityProfileError {}

/// 代表プロファイル名を `ConfigCompatibility` へ変換する。
pub fn compatibility_profile(name: &str) -> Result<ConfigCompatibility, CompatibilityProfileError> {
    CompatibilityProfile::from_str(name).map(CompatibilityProfile::into_compat)
}

/// 互換レイヤーの表現。CLI/Env/Manifest などソース別に保持する。
#[derive(Debug, Clone)]
pub struct CompatibilityLayer {
    pub compatibility: ConfigCompatibility,
    pub profile_label: Option<String>,
}

impl CompatibilityLayer {
    pub fn new(compatibility: ConfigCompatibility, profile_label: Option<String>) -> Self {
        Self {
            compatibility,
            profile_label,
        }
    }
}

/// 互換性解決に使用するソース。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigCompatibilitySource {
    Cli,
    Env,
    Manifest,
    Default,
}

impl ConfigCompatibilitySource {
    pub fn as_str(&self) -> &'static str {
        match self {
            ConfigCompatibilitySource::Cli => "cli",
            ConfigCompatibilitySource::Env => "env",
            ConfigCompatibilitySource::Manifest => "manifest",
            ConfigCompatibilitySource::Default => "default",
        }
    }
}

/// `resolve_compat` の入力。
#[derive(Debug, Clone)]
pub struct ResolveCompatOptions {
    pub format: ConfigFormat,
    pub stage: StageId,
    pub cli: Option<CompatibilityLayer>,
    pub env: Option<CompatibilityLayer>,
    pub manifest: Option<CompatibilityLayer>,
}

impl Default for ResolveCompatOptions {
    fn default() -> Self {
        Self {
            format: ConfigFormat::Toml,
            stage: StageId::Stable,
            cli: None,
            env: None,
            manifest: None,
        }
    }
}

/// 解決後の互換設定。
#[derive(Debug, Clone)]
pub struct ResolvedConfigCompatibility {
    pub format: ConfigFormat,
    pub source: ConfigCompatibilitySource,
    pub profile_label: Option<String>,
    pub compatibility: ConfigCompatibility,
}

impl ResolvedConfigCompatibility {
    fn from_layer(
        format: ConfigFormat,
        source: ConfigCompatibilitySource,
        layer: CompatibilityLayer,
    ) -> Self {
        Self {
            format,
            source,
            profile_label: layer.profile_label,
            compatibility: layer.compatibility,
        }
    }
}

/// CLI > Env > Manifest > Default の優先順位で互換設定を決定する。
pub fn resolve_compat(options: ResolveCompatOptions) -> ResolvedConfigCompatibility {
    if let Some(layer) = options.cli {
        return ResolvedConfigCompatibility::from_layer(
            options.format,
            ConfigCompatibilitySource::Cli,
            layer,
        );
    }
    if let Some(layer) = options.env {
        return ResolvedConfigCompatibility::from_layer(
            options.format,
            ConfigCompatibilitySource::Env,
            layer,
        );
    }
    if let Some(layer) = options.manifest {
        return ResolvedConfigCompatibility::from_layer(
            options.format,
            ConfigCompatibilitySource::Manifest,
            layer,
        );
    }
    let compatibility = compatibility_profile_for_stage(options.format, options.stage);
    ResolvedConfigCompatibility {
        format: options.format,
        source: ConfigCompatibilitySource::Default,
        profile_label: Some(default_profile_label(options.format, options.stage)),
        compatibility,
    }
}

fn default_profile_label(format: ConfigFormat, stage: StageId) -> String {
    match (format, stage) {
        (ConfigFormat::Json, StageId::Stable) => "strict-json".to_string(),
        (ConfigFormat::Json, _) => "json-relaxed".to_string(),
        (ConfigFormat::Toml, StageId::Alpha) => "toml-strict".to_string(),
        (ConfigFormat::Toml, _) => "toml-relaxed".to_string(),
    }
}

/// コンフィグ設定の互換違反種別。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompatibilityViolationKind {
    TrailingComma,
    UnquotedKey,
    DuplicateKey,
    NumberFormat,
}

impl CompatibilityViolationKind {
    pub fn code(&self) -> &'static str {
        match self {
            CompatibilityViolationKind::TrailingComma => CONFIG_COMPAT_TRAILING_COMMA_CODE,
            CompatibilityViolationKind::UnquotedKey => CONFIG_COMPAT_UNQUOTED_KEY_CODE,
            CompatibilityViolationKind::DuplicateKey => CONFIG_COMPAT_DUPLICATE_KEY_CODE,
            CompatibilityViolationKind::NumberFormat => CONFIG_COMPAT_NUMBER_CODE,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            CompatibilityViolationKind::TrailingComma => "trailing_comma",
            CompatibilityViolationKind::UnquotedKey => "unquoted_key",
            CompatibilityViolationKind::DuplicateKey => "duplicate_key",
            CompatibilityViolationKind::NumberFormat => "number_format",
        }
    }

    fn default_message(&self) -> &'static str {
        match self {
            CompatibilityViolationKind::TrailingComma => {
                "許可されていないトレーリングカンマが検出されました"
            }
            CompatibilityViolationKind::UnquotedKey => {
                "互換ポリシーで禁止された bare key が検出されました"
            }
            CompatibilityViolationKind::DuplicateKey => "設定内で重複したキーが検出されました",
            CompatibilityViolationKind::NumberFormat => {
                "互換ポリシーに反する数値表現が検出されました"
            }
        }
    }
}

/// 互換違反診断のメタデータ。
pub fn compatibility_violation_diagnostic(
    kind: CompatibilityViolationKind,
    source: &str,
    path: Option<&Path>,
    key_path: &[&str],
    format: Option<ConfigFormat>,
    profile_label: Option<&str>,
    stage: Option<StageId>,
    detail: Option<String>,
) -> GuardDiagnostic {
    let message = detail.unwrap_or_else(|| kind.default_message().to_string());
    let mut config_payload = Map::new();
    config_payload.insert("source".into(), Value::String(source.to_string()));
    if let Some(path) = path {
        config_payload.insert("path".into(), Value::String(path.display().to_string()));
    }
    if !key_path.is_empty() {
        config_payload.insert(
            "key_path".into(),
            Value::Array(
                key_path
                    .iter()
                    .map(|segment| Value::String((*segment).to_string()))
                    .collect(),
            ),
        );
    }
    let mut audit = Map::new();
    audit.insert("config.source".into(), Value::String(source.to_string()));
    if let Some(path) = path {
        audit.insert(
            "config.path".into(),
            Value::String(path.display().to_string()),
        );
    }
    if let Some(key_path_value) = config_payload.get("key_path") {
        audit.insert("config.key_path".into(), key_path_value.clone());
    }
    let mut compat_object = Map::new();
    compat_object.insert("violation".into(), Value::String(kind.label().to_string()));
    if let Some(format) = format {
        compat_object.insert("format".into(), Value::String(format.as_str().to_string()));
    }
    if let Some(profile) = profile_label {
        compat_object.insert("profile".into(), Value::String(profile.to_string()));
    }
    if let Some(stage) = stage {
        compat_object.insert("stage".into(), Value::String(stage.as_str().to_string()));
    }
    config_payload.insert("compatibility".into(), Value::Object(compat_object.clone()));
    audit.insert("config.compatibility".into(), Value::Object(compat_object));

    let mut extensions = Map::new();
    extensions.insert("config".into(), Value::Object(config_payload));

    GuardDiagnostic {
        code: kind.code(),
        domain: CONFIG_COMPAT_DOMAIN,
        severity: DiagnosticSeverity::Error,
        message,
        notes: Vec::new(),
        extensions,
        audit_metadata: audit,
    }
}

/// 字句レベル互換設定。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConfigTriviaProfile {
    #[serde(default = "ConfigTriviaProfile::default_line")]
    pub line: Vec<String>,
    #[serde(default = "ConfigTriviaProfile::default_block")]
    pub block: Vec<CommentPair>,
    #[serde(default)]
    pub shebang: bool,
    #[serde(default)]
    pub hash_inline: bool,
    #[serde(default)]
    pub doc_comment: Option<String>,
}

impl Default for ConfigTriviaProfile {
    fn default() -> Self {
        Self::strict_json()
    }
}

impl ConfigTriviaProfile {
    pub fn strict_json() -> Self {
        Self {
            line: Vec::new(),
            block: Vec::new(),
            shebang: false,
            hash_inline: false,
            doc_comment: None,
        }
    }

    pub fn json_relaxed() -> Self {
        Self {
            line: vec!["//".into()],
            block: vec![CommentPair::non_nested("/*", "*/")],
            shebang: true,
            hash_inline: false,
            doc_comment: None,
        }
    }

    pub fn toml_relaxed() -> Self {
        Self {
            line: vec!["#".into(), "//".into()],
            block: Vec::new(),
            shebang: false,
            hash_inline: true,
            doc_comment: None,
        }
    }

    fn default_line() -> Vec<String> {
        vec!["//".into()]
    }

    fn default_block() -> Vec<CommentPair> {
        vec![CommentPair::default()]
    }
}

/// ブロックコメントペア。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommentPair {
    pub start: String,
    pub end: String,
    #[serde(default = "CommentPair::default_nested")]
    pub nested: bool,
}

impl CommentPair {
    pub fn new(start: impl Into<String>, end: impl Into<String>, nested: bool) -> Self {
        Self {
            start: start.into(),
            end: end.into(),
            nested,
        }
    }

    pub fn non_nested(start: impl Into<String>, end: impl Into<String>) -> Self {
        Self::new(start, end, false)
    }

    fn default() -> Self {
        Self::non_nested("/*", "*/")
    }

    fn default_nested() -> bool {
        true
    }
}

/// トレーリングカンマ許容設定。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TrailingCommaMode {
    Forbid,
    Arrays,
    Objects,
    ArraysAndObjects,
}

impl Default for TrailingCommaMode {
    fn default() -> Self {
        TrailingCommaMode::Forbid
    }
}

/// bare key 許容ポリシー。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum KeyPolicy {
    Forbid,
    AllowAlpha,
    AllowAlphaNumeric,
}

impl Default for KeyPolicy {
    fn default() -> Self {
        KeyPolicy::Forbid
    }
}

/// 重複キー検出ポリシー。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DuplicateKeyPolicy {
    Error,
    LastWriteWins,
    CollectAll,
}

impl Default for DuplicateKeyPolicy {
    fn default() -> Self {
        DuplicateKeyPolicy::Error
    }
}

/// 数値互換モード。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NumberCompatibility {
    Strict,
    AllowLeadingPlus,
    AllowHexFloat,
}

impl Default for NumberCompatibility {
    fn default() -> Self {
        NumberCompatibility::Strict
    }
}

/// 互換違反診断のユーティリティを簡便に生成するためのヘルパー。
pub struct CompatibilityDiagnosticBuilder {
    kind: CompatibilityViolationKind,
    source: String,
    path: Option<PathBuf>,
    key_path: Vec<String>,
    format: Option<ConfigFormat>,
    profile_label: Option<String>,
    stage: Option<StageId>,
    detail: Option<String>,
}

impl CompatibilityDiagnosticBuilder {
    pub fn new(kind: CompatibilityViolationKind, source: impl Into<String>) -> Self {
        Self {
            kind,
            source: source.into(),
            path: None,
            key_path: Vec::new(),
            format: None,
            profile_label: None,
            stage: None,
            detail: None,
        }
    }

    pub fn path(mut self, path: impl Into<PathBuf>) -> Self {
        self.path = Some(path.into());
        self
    }

    pub fn key_path(mut self, segments: &[&str]) -> Self {
        self.key_path = segments.iter().map(|s| s.to_string()).collect();
        self
    }

    pub fn format(mut self, format: ConfigFormat) -> Self {
        self.format = Some(format);
        self
    }

    pub fn profile_label(mut self, label: impl Into<String>) -> Self {
        self.profile_label = Some(label.into());
        self
    }

    pub fn stage(mut self, stage: StageId) -> Self {
        self.stage = Some(stage);
        self
    }

    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    pub fn build(self) -> GuardDiagnostic {
        let key_path = self.key_path;
        let key_path_refs: Vec<&str> = key_path.iter().map(|s| s.as_str()).collect();
        compatibility_violation_diagnostic(
            self.kind,
            &self.source,
            self.path.as_deref(),
            &key_path_refs,
            self.format,
            self.profile_label.as_deref(),
            self.stage,
            self.detail,
        )
    }
}
