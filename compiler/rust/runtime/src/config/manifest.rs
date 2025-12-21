use super::compat::{
    compatibility_profile_for_stage, CompatibilityLayer, ConfigCompatibility, ConfigFormat,
    ConfigTriviaProfile, DuplicateKeyPolicy, KeyPolicy, NumberCompatibility, TrailingCommaMode,
};
use crate::{
    capability::contract::{
        CapabilityContractSpan, ConductorCapabilityContract, ConductorCapabilityRequirement,
    },
    data::schema::{Schema, SchemaVersion},
    prelude::ensure::{DiagnosticSeverity, GuardDiagnostic},
    stage::{StageId, StageParseError, StageRequirement},
};
use serde::{Deserialize, Serialize};
use serde_json::{self, Map, Value};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    fmt, fs, io,
    path::{Path, PathBuf},
    str::FromStr,
};
use thiserror::Error;
use toml::de;

const CONFIG_DOMAIN: &str = "config";
const CONFIG_SOURCE_MANIFEST: &str = "manifest";
const CONFIG_MISSING_FIELD_CODE: &str = "config.missing_field";
const CONFIG_INVALID_STAGE_CODE: &str = "config.invalid_stage";
const CONFIG_PROJECT_KIND_UNKNOWN_CODE: &str = "config.project.kind_unknown";
const CONFIG_BUILD_OPTIMIZE_UNKNOWN_CODE: &str = "config.build.optimize_unknown";
const CONFIG_MANIFEST_IO_ERROR_CODE: &str = "config.manifest.io_error";
const CONFIG_MANIFEST_PARSE_ERROR_CODE: &str = "config.manifest.parse_error";
const CONFIG_MANIFEST_ENTRY_MISSING_CODE: &str = "manifest.entry.missing";
const CONFIG_DSL_NOT_FOUND_CODE: &str = "config.dsl.not_found";
const CONFIG_DSL_EXPORT_NOT_FOUND_CODE: &str = "config.dsl.export_not_found";
const CONFIG_DSL_UNKNOWN_KIND_CODE: &str = "config.dsl.unknown_kind";
const CONFIG_SIGNATURE_SERIALIZATION_CODE: &str = "config.manifest.signature_serialization";
const CONFIG_PROJECT_VERSION_PARSE_CODE: &str = "config.project.version_invalid";
const CONFIG_SCHEMA_VERSION_INCOMPATIBLE_CODE: &str = "config.schema.version_incompatible";

/// `reml.toml` のトップレベル構造。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Manifest {
    #[serde(default)]
    pub project: ProjectSection,
    #[serde(default)]
    pub dependencies: BTreeMap<String, DependencySpec>,
    #[serde(default)]
    pub dsl: BTreeMap<String, DslEntry>,
    #[serde(default)]
    pub build: BuildSection,
    #[serde(default)]
    pub registry: RegistrySection,
    #[serde(default)]
    pub config: ConfigRoot,
    #[serde(default)]
    pub run: RunSection,
    #[serde(skip)]
    manifest_path: Option<PathBuf>,
}

impl Manifest {
    pub fn builder() -> ManifestBuilder {
        ManifestBuilder::default()
    }

    pub fn manifest_path(&self) -> Option<&PathBuf> {
        self.manifest_path.as_ref()
    }

    pub fn with_manifest_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.manifest_path = Some(path.into());
        self
    }

    pub fn config_section(mut self, config: ConfigRoot) -> Self {
        self.config = config;
        self
    }

    pub fn parse_toml(input: &str) -> Result<Self, ManifestParseError> {
        let mut manifest: Manifest = de::from_str(input).map_err(ManifestParseError::from)?;
        for entry in manifest.dsl.values_mut() {
            entry.ensure_sane_defaults();
        }
        Ok(manifest)
    }

    /// マニフェストからフォーマット別互換レイヤーを取得する。
    pub fn compatibility_layer(
        &self,
        format: ConfigFormat,
        stage: StageId,
    ) -> Option<CompatibilityLayer> {
        let entry = self
            .config
            .compatibility
            .get(format.as_str())
            .or_else(|| self.config.compatibility.get(&format.as_str().to_string()))?;
        let base = compatibility_profile_for_stage(format, stage);
        Some(entry.to_layer(base))
    }

    /// `run.target.capabilities` から契約を生成する。
    pub fn conductor_capability_contract(
        &self,
    ) -> Result<ConductorCapabilityContract, ManifestCapabilityError> {
        let mut requirements = Vec::new();
        for entry in &self.run.target.capabilities {
            requirements.push(entry.to_requirement()?);
        }
        let mut contract = ConductorCapabilityContract::new(requirements);
        if let Some(path) = self.manifest_path() {
            contract.manifest_path = Some(path.clone());
        }
        Ok(contract)
    }
}

/// `config` ルートセクション。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConfigRoot {
    #[serde(default)]
    pub compatibility: BTreeMap<String, ConfigCompatibilityEntry>,
}

/// `config.compatibility.<format>` の項目。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConfigCompatibilityEntry {
    #[serde(default)]
    pub profile: Option<String>,
    #[serde(default)]
    pub trailing_comma: Option<TrailingCommaMode>,
    #[serde(default)]
    pub unquoted_key: Option<KeyPolicy>,
    #[serde(default)]
    pub duplicate_key: Option<DuplicateKeyPolicy>,
    #[serde(default)]
    pub number: Option<NumberCompatibility>,
    #[serde(default)]
    pub trivia: Option<ConfigTriviaProfile>,
    #[serde(default)]
    pub feature_guard: Option<BTreeSet<String>>,
}

impl ConfigCompatibilityEntry {
    fn to_layer(&self, base: ConfigCompatibility) -> CompatibilityLayer {
        let mut compatibility = base;
        if let Some(value) = self.trailing_comma {
            compatibility.trailing_comma = value;
        }
        if let Some(value) = self.unquoted_key {
            compatibility.unquoted_key = value;
        }
        if let Some(value) = self.duplicate_key {
            compatibility.duplicate_key = value;
        }
        if let Some(value) = self.number {
            compatibility.number = value;
        }
        if let Some(trivia) = &self.trivia {
            compatibility.trivia = trivia.clone();
        }
        if let Some(feature_guard) = &self.feature_guard {
            compatibility.feature_guard = feature_guard.clone();
        }
        let profile_label = self.profile.as_ref().map(|value| value.to_string());
        CompatibilityLayer::new(compatibility, profile_label)
    }
}

/// `run` ルートセクション。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RunSection {
    #[serde(default)]
    pub target: RunTargetSection,
}

/// `run.target` サブセクション。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RunTargetSection {
    #[serde(default)]
    pub capabilities: Vec<RunCapabilityEntry>,
}

/// `run.target.capabilities[]` の 1 エントリ。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RunCapabilityEntry {
    pub id: CapabilityId,
    #[serde(default)]
    pub stage: Option<String>,
    #[serde(default)]
    pub declared_effects: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_span: Option<CapabilityContractSpan>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
}

impl RunCapabilityEntry {
    fn to_requirement(&self) -> Result<ConductorCapabilityRequirement, ManifestCapabilityError> {
        let stage_label =
            self.stage
                .as_deref()
                .ok_or_else(|| ManifestCapabilityError::MissingStage {
                    capability: self.id.0.clone(),
                })?;
        let stage = StageRequirement::from_str(stage_label).map_err(|source| {
            ManifestCapabilityError::InvalidStage {
                capability: self.id.0.clone(),
                value: stage_label.to_string(),
                source,
            }
        })?;
        let mut declared_effects = self.declared_effects.clone();
        declared_effects.sort();
        declared_effects.dedup();
        Ok(ConductorCapabilityRequirement {
            id: self.id.0.clone(),
            stage,
            declared_effects,
            source_span: self.source_span.clone(),
        })
    }

    fn manifest_record(&self) -> Result<ManifestCapabilityRecord, ManifestCapabilityError> {
        let requirement = self.to_requirement()?;
        Ok(ManifestCapabilityRecord {
            stage: requirement.stage,
            declared_effects: requirement.declared_effects,
            source_span: requirement.source_span,
            provider: self.provider.clone(),
        })
    }
}

/// `run.target.capabilities` 読み込み時のエラー。
#[derive(Debug, Error)]
pub enum ManifestCapabilityError {
    #[error("Capability `{capability}` の stage が指定されていません")]
    MissingStage { capability: String },
    #[error("Capability `{capability}` の stage `{value}` を解析できません: {source}")]
    InvalidStage {
        capability: String,
        value: String,
        #[source]
        source: StageParseError,
    },
    #[error("Capability `{capability}` が重複しています")]
    DuplicateCapability { capability: String },
    #[error("`run.target.capabilities` の読み込みに失敗しました: {0}")]
    Io(#[from] std::io::Error),
    #[error("`run.target.capabilities` の解析に失敗しました: {0}")]
    Parse(String),
}

impl From<ManifestParseError> for ManifestCapabilityError {
    fn from(value: ManifestParseError) -> Self {
        ManifestCapabilityError::Parse(value.to_string())
    }
}

impl Clone for ManifestCapabilityError {
    fn clone(&self) -> Self {
        match self {
            ManifestCapabilityError::MissingStage { capability } => {
                ManifestCapabilityError::MissingStage {
                    capability: capability.clone(),
                }
            }
            ManifestCapabilityError::InvalidStage {
                capability,
                value,
                source,
            } => ManifestCapabilityError::InvalidStage {
                capability: capability.clone(),
                value: value.clone(),
                source: StageParseError::new(source.to_string()),
            },
            ManifestCapabilityError::DuplicateCapability { capability } => {
                ManifestCapabilityError::DuplicateCapability {
                    capability: capability.clone(),
                }
            }
            ManifestCapabilityError::Io(err) => {
                ManifestCapabilityError::Io(io::Error::new(err.kind(), err.to_string()))
            }
            ManifestCapabilityError::Parse(message) => {
                ManifestCapabilityError::Parse(message.clone())
            }
        }
    }
}

/// Manifest に書かれた Capability 1 件ぶんの情報。
#[derive(Debug, Clone)]
pub struct ManifestCapabilityRecord {
    pub stage: StageRequirement,
    pub declared_effects: Vec<String>,
    pub source_span: Option<CapabilityContractSpan>,
    pub provider: Option<String>,
}

/// `run.target.capabilities` から生成したマップ。
#[derive(Debug, Clone)]
pub struct ManifestCapabilities {
    entries: HashMap<String, ManifestCapabilityRecord>,
}

impl ManifestCapabilities {
    pub fn from_manifest(manifest: &Manifest) -> Result<Self, ManifestCapabilityError> {
        let mut entries = HashMap::new();
        for entry in &manifest.run.target.capabilities {
            if entries.contains_key(&entry.id.0) {
                return Err(ManifestCapabilityError::DuplicateCapability {
                    capability: entry.id.0.clone(),
                });
            }
            entries.insert(entry.id.0.clone(), entry.manifest_record()?);
        }
        Ok(Self { entries })
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self, ManifestCapabilityError> {
        let manifest_path = path.as_ref();
        let body = fs::read_to_string(manifest_path)?;
        let manifest = Manifest::parse_toml(&body)?.with_manifest_path(manifest_path.to_path_buf());
        Self::from_manifest(&manifest)
    }

    pub fn get(&self, capability: &str) -> Option<&ManifestCapabilityRecord> {
        self.entries.get(capability)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &ManifestCapabilityRecord)> {
        self.entries.iter()
    }

    pub fn ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.entries.keys().cloned().collect();
        ids.sort();
        ids
    }
}

/// `Manifest` を構築するための簡易ビルダー。
#[derive(Debug, Default)]
pub struct ManifestBuilder {
    manifest: Manifest,
}

impl ManifestBuilder {
    pub fn project(mut self, project: ProjectSection) -> Self {
        self.manifest.project = project;
        self
    }

    pub fn dependency(mut self, name: impl Into<String>, spec: DependencySpec) -> Self {
        self.manifest.dependencies.insert(name.into(), spec);
        self
    }

    pub fn dsl_entry(mut self, name: impl Into<String>, entry: DslEntry) -> Self {
        self.manifest.dsl.insert(name.into(), entry);
        self
    }

    pub fn build(mut self, build: BuildSection) -> Self {
        self.manifest.build = build;
        self
    }

    pub fn registry(mut self, registry: RegistrySection) -> Self {
        self.manifest.registry = registry;
        self
    }

    pub fn run(mut self, run: RunSection) -> Self {
        self.manifest.run = run;
        self
    }

    pub fn manifest_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.manifest.manifest_path = Some(path.into());
        self
    }

    pub fn finish(self) -> Manifest {
        self.manifest
    }
}

/// プロジェクトに関する基本情報。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSection {
    #[serde(default)]
    pub name: PackageName,
    #[serde(default)]
    pub version: SemanticVersion,
    #[serde(default)]
    pub authors: Vec<Contact>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub kind: ProjectKind,
    #[serde(default)]
    pub stage: ProjectStage,
    #[serde(default)]
    pub capabilities: Vec<CapabilityId>,
}

impl Default for ProjectSection {
    fn default() -> Self {
        Self {
            name: PackageName::default(),
            version: SemanticVersion::default(),
            authors: Vec::new(),
            license: None,
            description: None,
            kind: ProjectKind::Application,
            stage: ProjectStage::Stable,
            capabilities: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct DependencySpec {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rev: Option<String>,
    #[serde(default = "DependencySpec::default_true")]
    pub default_features: bool,
    #[serde(default)]
    pub optional: bool,
    #[serde(default)]
    pub features: BTreeSet<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registry: Option<String>,
}

impl Default for DependencySpec {
    fn default() -> Self {
        Self {
            package: None,
            version: None,
            path: None,
            git: None,
            branch: None,
            tag: None,
            rev: None,
            default_features: true,
            optional: false,
            features: BTreeSet::new(),
            registry: None,
        }
    }
}

impl<'de> Deserialize<'de> for DependencySpec {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Debug, Deserialize)]
        struct DetailedSpec {
            #[serde(default)]
            package: Option<String>,
            #[serde(default)]
            version: Option<String>,
            #[serde(default)]
            path: Option<PathBuf>,
            #[serde(default)]
            git: Option<String>,
            #[serde(default)]
            branch: Option<String>,
            #[serde(default)]
            tag: Option<String>,
            #[serde(default)]
            rev: Option<String>,
            #[serde(default = "DependencySpec::default_true")]
            default_features: bool,
            #[serde(default)]
            optional: bool,
            #[serde(default)]
            features: BTreeSet<String>,
            #[serde(default)]
            registry: Option<String>,
        }

        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Helper {
            Version(String),
            Detailed(DetailedSpec),
        }
        match Helper::deserialize(deserializer)? {
            Helper::Version(version) => Ok(DependencySpec {
                version: Some(version),
                ..DependencySpec::default()
            }),
            Helper::Detailed(spec) => Ok(DependencySpec {
                package: spec.package,
                version: spec.version,
                path: spec.path,
                git: spec.git,
                branch: spec.branch,
                tag: spec.tag,
                rev: spec.rev,
                default_features: spec.default_features,
                optional: spec.optional,
                features: spec.features,
                registry: spec.registry,
            }),
        }
    }
}

impl DependencySpec {
    fn default_true() -> bool {
        true
    }
}

/// DSL エントリの 1 件。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DslEntry {
    #[serde(default)]
    pub entry: PathBuf,
    #[serde(
        default,
        deserialize_with = "DslEntry::deserialize_exports",
        serialize_with = "DslEntry::serialize_exports"
    )]
    pub exports: Vec<DslExportRef>,
    #[serde(default)]
    pub kind: DslCategory,
    #[serde(default)]
    pub expect_effects: BTreeSet<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expect_effects_stage: Option<ProjectStage>,
    #[serde(default)]
    pub allow_prerelease: bool,
    #[serde(default)]
    pub capabilities: Vec<CapabilityId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}

impl DslEntry {
    fn deserialize_exports<'de, D>(deserializer: D) -> Result<Vec<DslExportRef>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Helper {
            Simple(String),
            Detailed(DslExportRef),
        }
        let raw: Vec<Helper> = Vec::deserialize(deserializer)?;
        Ok(raw
            .into_iter()
            .map(|item| match item {
                Helper::Simple(name) => DslExportRef {
                    name,
                    signature: None,
                },
                Helper::Detailed(entry) => entry,
            })
            .collect())
    }

    fn serialize_exports<S>(exports: &[DslExportRef], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let out: Vec<Value> = exports
            .iter()
            .map(|entry| {
                if entry.signature.is_none() {
                    Value::String(entry.name.clone())
                } else {
                    serde_json::to_value(entry).unwrap_or(Value::String(entry.name.clone()))
                }
            })
            .collect();
        out.serialize(serializer)
    }

    fn ensure_sane_defaults(&mut self) {
        if self.expect_effects_stage.is_none() && !self.expect_effects.is_empty() {
            self.expect_effects_stage = Some(ProjectStage::Stable);
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DslExportRef {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signature: Option<Value>,
}

/// DSL エクスポート署名のステージ境界。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DslSignatureStageBounds {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minimum: Option<ProjectStage>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maximum: Option<ProjectStage>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current: Option<ProjectStage>,
}

/// `@dsl_export` から得られた署名情報。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DslExportSignature {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(default)]
    pub allows_effects: Vec<String>,
    #[serde(default)]
    pub requires_capabilities: Vec<CapabilityId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stage_bounds: Option<DslSignatureStageBounds>,
    #[serde(default, flatten)]
    pub extra: Map<String, Value>,
}

impl DslExportSignature {
    fn to_value(&self) -> Result<Value, serde_json::Error> {
        serde_json::to_value(self)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildSection {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<TargetTriple>,
    #[serde(default)]
    pub targets: Vec<TargetTriple>,
    #[serde(default)]
    pub features: BTreeSet<String>,
    #[serde(default)]
    pub optimize: OptimizeLevel,
    #[serde(default)]
    pub warnings_as_errors: bool,
    #[serde(default)]
    pub profiles: BTreeMap<String, BuildProfile>,
}

impl Default for BuildSection {
    fn default() -> Self {
        Self {
            target: None,
            targets: Vec::new(),
            features: BTreeSet::new(),
            optimize: OptimizeLevel::Debug,
            warnings_as_errors: false,
            profiles: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BuildProfile {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inherits: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub optimize: Option<OptimizeLevel>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub warnings_as_errors: Option<bool>,
    #[serde(default)]
    pub features: BTreeSet<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RegistrySection {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upstream: Option<String>,
    #[serde(default)]
    pub mirrors: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth: Option<AuthConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AuthConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
}

/// `Manifest::parse_toml` から返す汎用エラー。
#[derive(Debug)]
pub struct ManifestParseError {
    message: String,
}

impl fmt::Display for ManifestParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ManifestParseError {}

impl From<de::Error> for ManifestParseError {
    fn from(value: de::Error) -> Self {
        Self {
            message: value.to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct PackageName(pub String);

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct SemanticVersion(pub String);

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct TargetTriple(pub String);

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
pub struct CapabilityId(pub String);

/// DSL カテゴリ。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DslCategory {
    Language,
    Analyzer,
    RuntimeBridge,
    Plugin,
    Unknown(String),
}

impl Default for DslCategory {
    fn default() -> Self {
        Self::Language
    }
}

impl fmt::Display for DslCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl DslCategory {
    fn as_str(&self) -> &str {
        match self {
            DslCategory::Language => "language",
            DslCategory::Analyzer => "analyzer",
            DslCategory::RuntimeBridge => "runtime_bridge",
            DslCategory::Plugin => "plugin",
            DslCategory::Unknown(value) => value.as_str(),
        }
    }
}

impl Serialize for DslCategory {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for DslCategory {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Ok(match value.as_str() {
            "language" => DslCategory::Language,
            "analyzer" => DslCategory::Analyzer,
            "runtime_bridge" => DslCategory::RuntimeBridge,
            "plugin" => DslCategory::Plugin,
            other => DslCategory::Unknown(other.to_string()),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectKind {
    Application,
    Library,
    Plugin,
    Tooling,
    Unknown(String),
}

impl Default for ProjectKind {
    fn default() -> Self {
        Self::Application
    }
}

impl ProjectKind {
    pub fn as_str(&self) -> &str {
        match self {
            ProjectKind::Application => "application",
            ProjectKind::Library => "library",
            ProjectKind::Plugin => "plugin",
            ProjectKind::Tooling => "tooling",
            ProjectKind::Unknown(value) => value.as_str(),
        }
    }
}

impl fmt::Display for ProjectKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl Serialize for ProjectKind {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for ProjectKind {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        Ok(match raw.as_str() {
            "application" => ProjectKind::Application,
            "library" => ProjectKind::Library,
            "plugin" => ProjectKind::Plugin,
            "tooling" => ProjectKind::Tooling,
            other => ProjectKind::Unknown(other.to_string()),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectStage {
    Experimental,
    Beta,
    Stable,
    Unknown(String),
}

impl ProjectStage {
    pub fn as_str(&self) -> &str {
        match self {
            ProjectStage::Experimental => "experimental",
            ProjectStage::Beta => "beta",
            ProjectStage::Stable => "stable",
            ProjectStage::Unknown(value) => value.as_str(),
        }
    }
}

impl Default for ProjectStage {
    fn default() -> Self {
        Self::Stable
    }
}

impl fmt::Display for ProjectStage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl Serialize for ProjectStage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for ProjectStage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        Ok(match raw.as_str() {
            "experimental" => ProjectStage::Experimental,
            "beta" => ProjectStage::Beta,
            "stable" => ProjectStage::Stable,
            other => ProjectStage::Unknown(other.to_string()),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OptimizeLevel {
    Debug,
    Release,
    Size,
    Speed,
    Unknown(String),
}

impl OptimizeLevel {
    pub fn as_str(&self) -> &str {
        match self {
            OptimizeLevel::Debug => "debug",
            OptimizeLevel::Release => "release",
            OptimizeLevel::Size => "size",
            OptimizeLevel::Speed => "speed",
            OptimizeLevel::Unknown(value) => value.as_str(),
        }
    }
}

impl Default for OptimizeLevel {
    fn default() -> Self {
        Self::Debug
    }
}

impl fmt::Display for OptimizeLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl Serialize for OptimizeLevel {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for OptimizeLevel {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        Ok(match raw.as_str() {
            "debug" => OptimizeLevel::Debug,
            "release" => OptimizeLevel::Release,
            "size" => OptimizeLevel::Size,
            "speed" => OptimizeLevel::Speed,
            other => OptimizeLevel::Unknown(other.to_string()),
        })
    }
}

/// マニフェストの `authors` フィールド。文字列とテーブル両方に対応する。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Contact {
    Simple(String),
    Detailed {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        email: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        handle: Option<String>,
    },
}

impl Default for Contact {
    fn default() -> Self {
        Contact::Simple(String::new())
    }
}

/// `reml.toml` を読み込むローダ。
#[derive(Debug, Default)]
pub struct ManifestLoader;

impl ManifestLoader {
    /// 新しいローダを生成する。
    pub fn new() -> Self {
        Self
    }

    /// 指定したパスからマニフェストを読み込む。
    pub fn load(&self, path: impl AsRef<Path>) -> Result<Manifest, GuardDiagnostic> {
        load_manifest(path)
    }
}

/// `reml.toml` を読み込み、DSL エントリの存在を確認する。
pub fn load_manifest(path: impl AsRef<Path>) -> Result<Manifest, GuardDiagnostic> {
    let manifest_path = path.as_ref().to_path_buf();
    let body = fs::read_to_string(&manifest_path)
        .map_err(|err| manifest_io_error(manifest_path.as_path(), err))?;
    let mut manifest = Manifest::parse_toml(&body)
        .map_err(|err| manifest_parse_error(manifest_path.as_path(), err))?;
    manifest = manifest.with_manifest_path(&manifest_path);
    ensure_dsl_entries_have_paths(&manifest)?;
    Ok(manifest)
}

/// マニフェストの単純な妥当性検証を行う。
pub fn validate_manifest(manifest: &Manifest) -> Result<(), GuardDiagnostic> {
    let manifest_path = manifest.manifest_path().map(|path| path.as_path());
    validate_project_section(&manifest.project, manifest_path)?;
    validate_build_section(&manifest.build, manifest_path)?;
    validate_build_profiles(&manifest.build.profiles, manifest_path)?;
    validate_dsl_sections(manifest, manifest_path)?;
    Ok(())
}

/// DSL 名に対応する効果集合を返す。
pub fn declared_effects(
    manifest: &Manifest,
    dsl: impl AsRef<str>,
) -> Result<BTreeSet<String>, GuardDiagnostic> {
    let key = dsl.as_ref();
    let manifest_path = manifest.manifest_path().map(|path| path.as_path());
    let entry = manifest
        .dsl
        .get(key)
        .ok_or_else(|| dsl_not_found_diagnostic(manifest_path, key))?;
    Ok(entry.expect_effects.clone())
}

/// DSL 署名をマニフェストへ書き戻す。
pub fn update_dsl_signature(
    mut manifest: Manifest,
    dsl: impl AsRef<str>,
    signature: DslExportSignature,
) -> Result<Manifest, GuardDiagnostic> {
    let manifest_path_buf = manifest.manifest_path().cloned();
    let manifest_path = manifest_path_buf.as_deref();
    let dsl_key = dsl.as_ref();
    let entry = manifest
        .dsl
        .get_mut(dsl_key)
        .ok_or_else(|| dsl_not_found_diagnostic(manifest_path, dsl_key))?;

    let export_name = signature.name.trim();
    if export_name.is_empty() {
        return Err(dsl_export_not_found_diagnostic(
            manifest_path,
            dsl_key,
            "<empty>",
        ));
    }

    if let Some(bounds) = signature.stage_bounds.as_ref() {
        validate_stage_bounds(bounds, manifest_path, dsl_key)?;
    }

    let requires_capabilities = signature.requires_capabilities.clone();
    let signature_value = signature
        .to_value()
        .map_err(|err| manifest_signature_error(manifest_path, err))?;

    if entry.expect_effects_stage.is_none() {
        if let Some(stage) = signature_stage(&signature) {
            entry.expect_effects_stage = Some(stage);
        }
    }

    let export = entry
        .exports
        .iter_mut()
        .find(|export| export.name == export_name)
        .ok_or_else(|| dsl_export_not_found_diagnostic(manifest_path, dsl_key, export_name))?;
    export.signature = Some(signature_value);

    if !requires_capabilities.is_empty() {
        entry.capabilities = dedup_capabilities(&requires_capabilities);
    }

    Ok(manifest)
}

fn validate_project_section(
    project: &ProjectSection,
    manifest_path: Option<&Path>,
) -> Result<(), GuardDiagnostic> {
    if project.name.0.trim().is_empty() {
        return Err(missing_field_diagnostic(
            manifest_path,
            &["project", "name"],
        ));
    }
    if project.version.0.trim().is_empty() {
        return Err(missing_field_diagnostic(
            manifest_path,
            &["project", "version"],
        ));
    }
    if let ProjectKind::Unknown(value) = &project.kind {
        return Err(project_kind_diagnostic(manifest_path, value));
    }
    ensure_stage_known(&project.stage, manifest_path, &["project", "stage"])
}

/// マニフェストの `project.version` とスキーマ `Schema.version` を比較し、
/// 互換条件（major が一致し、マニフェスト側のバージョンがスキーマ以上）を満たすか検証する。
pub fn ensure_schema_version_compatibility(
    manifest: &Manifest,
    schema: &Schema,
) -> Result<(), GuardDiagnostic> {
    let schema_version = match schema.version.as_ref() {
        Some(value) => value,
        None => return Ok(()),
    };
    let manifest_path = manifest.manifest_path().map(|path| path.as_path());
    let manifest_version = parse_manifest_semver(
        &manifest.project.version,
        manifest_path,
        schema.name.as_str(),
    )?;
    let schema_parts = SemanticVersionParts::from_schema(schema_version);
    if manifest_version.major != schema_parts.major {
        return Err(schema_version_incompatible(
            manifest_path,
            &manifest.project.version.0,
            schema.name.as_str(),
            schema_parts,
            SemanticVersionMismatch::Major,
        ));
    }
    if manifest_version < schema_parts {
        return Err(schema_version_incompatible(
            manifest_path,
            &manifest.project.version.0,
            schema.name.as_str(),
            schema_parts,
            SemanticVersionMismatch::SchemaAhead,
        ));
    }
    Ok(())
}

fn validate_build_section(
    build: &BuildSection,
    manifest_path: Option<&Path>,
) -> Result<(), GuardDiagnostic> {
    ensure_optimize_known(&build.optimize, manifest_path, &["build", "optimize"])
}

fn validate_build_profiles(
    profiles: &BTreeMap<String, BuildProfile>,
    manifest_path: Option<&Path>,
) -> Result<(), GuardDiagnostic> {
    for (name, profile) in profiles {
        if let Some(optimize) = profile.optimize.as_ref() {
            ensure_optimize_known(
                optimize,
                manifest_path,
                &["build", "profiles", name, "optimize"],
            )?;
        }
    }
    Ok(())
}

fn validate_dsl_sections(
    manifest: &Manifest,
    manifest_path: Option<&Path>,
) -> Result<(), GuardDiagnostic> {
    for (name, entry) in &manifest.dsl {
        if entry.entry.as_os_str().is_empty() {
            return Err(missing_field_diagnostic(
                manifest_path,
                &["dsl", name, "entry"],
            ));
        }
        if let DslCategory::Unknown(value) = &entry.kind {
            return Err(dsl_kind_diagnostic(manifest_path, name, value));
        }
        if let Some(stage) = entry.expect_effects_stage.as_ref() {
            ensure_stage_known(stage, manifest_path, &["dsl", name, "expect_effects_stage"])?;
        }
    }
    Ok(())
}

fn validate_stage_bounds(
    bounds: &DslSignatureStageBounds,
    manifest_path: Option<&Path>,
    dsl_key: &str,
) -> Result<(), GuardDiagnostic> {
    if let Some(stage) = bounds.minimum.as_ref() {
        ensure_stage_known(
            stage,
            manifest_path,
            &["dsl", dsl_key, "stage_bounds", "minimum"],
        )?;
    }
    if let Some(stage) = bounds.maximum.as_ref() {
        ensure_stage_known(
            stage,
            manifest_path,
            &["dsl", dsl_key, "stage_bounds", "maximum"],
        )?;
    }
    if let Some(stage) = bounds.current.as_ref() {
        ensure_stage_known(
            stage,
            manifest_path,
            &["dsl", dsl_key, "stage_bounds", "current"],
        )?;
    }
    Ok(())
}

fn ensure_stage_known(
    stage: &ProjectStage,
    manifest_path: Option<&Path>,
    key_path: &[&str],
) -> Result<(), GuardDiagnostic> {
    if let ProjectStage::Unknown(value) = stage {
        Err(invalid_stage_diagnostic(manifest_path, key_path, value))
    } else {
        Ok(())
    }
}

fn ensure_optimize_known(
    optimize: &OptimizeLevel,
    manifest_path: Option<&Path>,
    key_path: &[&str],
) -> Result<(), GuardDiagnostic> {
    if let OptimizeLevel::Unknown(value) = optimize {
        Err(build_optimize_diagnostic(manifest_path, key_path, value))
    } else {
        Ok(())
    }
}

fn ensure_dsl_entries_have_paths(manifest: &Manifest) -> Result<(), GuardDiagnostic> {
    let manifest_path = manifest.manifest_path().map(|path| path.as_path());
    for (dsl_name, entry) in &manifest.dsl {
        if entry.entry.as_os_str().is_empty() {
            return Err(missing_field_diagnostic(
                manifest_path,
                &["dsl", dsl_name, "entry"],
            ));
        }
        if let Some(resolved) = resolve_entry_path(&entry.entry, manifest_path) {
            if !resolved.exists() {
                return Err(manifest_entry_missing(
                    manifest_path,
                    dsl_name,
                    resolved.as_path(),
                ));
            }
        }
    }
    Ok(())
}

fn resolve_entry_path(entry: &PathBuf, manifest_path: Option<&Path>) -> Option<PathBuf> {
    if entry.as_os_str().is_empty() {
        return None;
    }
    if entry.is_absolute() {
        return Some(entry.clone());
    }
    manifest_path
        .and_then(|path| path.parent())
        .map(|parent| parent.join(entry))
}

fn dedup_capabilities(values: &[CapabilityId]) -> Vec<CapabilityId> {
    let mut caps = values.to_vec();
    caps.sort();
    caps.dedup();
    caps
}

fn signature_stage(signature: &DslExportSignature) -> Option<ProjectStage> {
    signature.stage_bounds.as_ref().and_then(|bounds| {
        bounds
            .current
            .clone()
            .or_else(|| bounds.minimum.clone())
            .or_else(|| bounds.maximum.clone())
    })
}

fn manifest_io_error(path: &Path, err: std::io::Error) -> GuardDiagnostic {
    manifest_diagnostic(
        CONFIG_MANIFEST_IO_ERROR_CODE,
        format!(
            "マニフェスト `{}` の読み込みに失敗しました: {err}",
            path.display()
        ),
        Some(path),
        &["manifest"],
    )
}

fn manifest_parse_error(path: &Path, err: ManifestParseError) -> GuardDiagnostic {
    manifest_diagnostic(
        CONFIG_MANIFEST_PARSE_ERROR_CODE,
        format!(
            "マニフェスト `{}` の解析に失敗しました: {err}",
            path.display()
        ),
        Some(path),
        &["manifest"],
    )
}

fn manifest_entry_missing(
    manifest_path: Option<&Path>,
    dsl_name: &str,
    resolved: &Path,
) -> GuardDiagnostic {
    manifest_diagnostic(
        CONFIG_MANIFEST_ENTRY_MISSING_CODE,
        format!(
            "DSL `{dsl_name}` のエントリ `{}` が存在しません",
            resolved.display()
        ),
        manifest_path,
        &["dsl", dsl_name, "entry"],
    )
}

fn missing_field_diagnostic(manifest_path: Option<&Path>, key_path: &[&str]) -> GuardDiagnostic {
    let label = join_key_path(key_path);
    manifest_diagnostic(
        CONFIG_MISSING_FIELD_CODE,
        format!("必須フィールド `{label}` が未設定です"),
        manifest_path,
        key_path,
    )
}

fn invalid_stage_diagnostic(
    manifest_path: Option<&Path>,
    key_path: &[&str],
    value: &str,
) -> GuardDiagnostic {
    let label = join_key_path(key_path);
    manifest_diagnostic(
        CONFIG_INVALID_STAGE_CODE,
        format!("`{label}` に未対応の Stage `{value}` が指定されました"),
        manifest_path,
        key_path,
    )
}

fn project_kind_diagnostic(manifest_path: Option<&Path>, value: &str) -> GuardDiagnostic {
    manifest_diagnostic(
        CONFIG_PROJECT_KIND_UNKNOWN_CODE,
        format!("`project.kind` に未対応の値 `{value}` が指定されました"),
        manifest_path,
        &["project", "kind"],
    )
}

fn build_optimize_diagnostic(
    manifest_path: Option<&Path>,
    key_path: &[&str],
    value: &str,
) -> GuardDiagnostic {
    let label = join_key_path(key_path);
    manifest_diagnostic(
        CONFIG_BUILD_OPTIMIZE_UNKNOWN_CODE,
        format!("`{label}` に未対応の optimize 値 `{value}` が指定されました"),
        manifest_path,
        key_path,
    )
}

fn dsl_kind_diagnostic(manifest_path: Option<&Path>, dsl: &str, value: &str) -> GuardDiagnostic {
    manifest_diagnostic(
        CONFIG_DSL_UNKNOWN_KIND_CODE,
        format!("DSL `{dsl}` の kind `{value}` は未対応です"),
        manifest_path,
        &["dsl", dsl, "kind"],
    )
}

fn dsl_not_found_diagnostic(manifest_path: Option<&Path>, key: &str) -> GuardDiagnostic {
    manifest_diagnostic(
        CONFIG_DSL_NOT_FOUND_CODE,
        format!("DSL `{key}` がマニフェストに存在しません"),
        manifest_path,
        &["dsl", key],
    )
}

fn dsl_export_not_found_diagnostic(
    manifest_path: Option<&Path>,
    dsl: &str,
    export: &str,
) -> GuardDiagnostic {
    manifest_diagnostic(
        CONFIG_DSL_EXPORT_NOT_FOUND_CODE,
        format!("DSL `{dsl}` にエクスポート `{export}` が存在しません"),
        manifest_path,
        &["dsl", dsl, "exports"],
    )
}

fn manifest_signature_error(
    manifest_path: Option<&Path>,
    err: serde_json::Error,
) -> GuardDiagnostic {
    manifest_diagnostic(
        CONFIG_SIGNATURE_SERIALIZATION_CODE,
        format!("DSL 署名のシリアライズに失敗しました: {err}"),
        manifest_path,
        &["dsl"],
    )
}

fn manifest_diagnostic(
    code: &'static str,
    message: String,
    manifest_path: Option<&Path>,
    key_path: &[&str],
) -> GuardDiagnostic {
    let mut config_info = Map::new();
    config_info.insert(
        "source".into(),
        Value::String(CONFIG_SOURCE_MANIFEST.into()),
    );
    if let Some(path) = manifest_path {
        config_info.insert("path".into(), Value::String(path.display().to_string()));
    }
    if !key_path.is_empty() {
        let segments: Vec<Value> = key_path
            .iter()
            .map(|segment| Value::String(segment.to_string()))
            .collect();
        config_info.insert("key_path".into(), Value::Array(segments));
    }

    let mut extensions = Map::new();
    extensions.insert("config".into(), Value::Object(config_info.clone()));

    let mut audit = Map::new();
    audit.insert(
        "config.source".into(),
        Value::String(CONFIG_SOURCE_MANIFEST.into()),
    );
    if let Some(path) = manifest_path {
        audit.insert(
            "config.path".into(),
            Value::String(path.display().to_string()),
        );
    }
    if let Some(key_path_value) = config_info.get("key_path") {
        audit.insert("config.key_path".into(), key_path_value.clone());
    }

    GuardDiagnostic {
        code,
        domain: CONFIG_DOMAIN,
        severity: DiagnosticSeverity::Error,
        message,
        notes: Vec::new(),
        extensions,
        audit_metadata: audit,
    }
}

fn join_key_path(parts: &[&str]) -> String {
    parts.join(".")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct SemanticVersionParts {
    major: u32,
    minor: u32,
    patch: u32,
}

impl SemanticVersionParts {
    fn from_schema(schema_version: &SchemaVersion) -> Self {
        Self {
            major: schema_version.major,
            minor: schema_version.minor,
            patch: schema_version.patch,
        }
    }
}

fn parse_manifest_semver(
    version: &SemanticVersion,
    manifest_path: Option<&Path>,
    schema_name: &str,
) -> Result<SemanticVersionParts, GuardDiagnostic> {
    let raw = version.0.trim();
    if raw.is_empty() {
        return Err(manifest_version_parse_error(
            manifest_path,
            raw,
            schema_name,
        ));
    }
    let core = raw.split(|ch| ch == '-' || ch == '+').next().unwrap_or(raw);
    let mut segments = core.split('.');
    let major = segments.next();
    let minor = segments.next();
    let patch = segments.next();
    let (major, minor, patch) = match (major, minor, patch) {
        (Some(major), Some(minor), Some(patch)) => (major, minor, patch),
        _ => {
            return Err(manifest_version_parse_error(
                manifest_path,
                raw,
                schema_name,
            ))
        }
    };
    let parse_component = |value: &str| -> Option<u32> {
        if value.trim().is_empty() {
            None
        } else {
            value.parse::<u32>().ok()
        }
    };
    let major = parse_component(major)
        .ok_or_else(|| manifest_version_parse_error(manifest_path, raw, schema_name))?;
    let minor = parse_component(minor)
        .ok_or_else(|| manifest_version_parse_error(manifest_path, raw, schema_name))?;
    let patch = parse_component(patch)
        .ok_or_else(|| manifest_version_parse_error(manifest_path, raw, schema_name))?;
    Ok(SemanticVersionParts {
        major,
        minor,
        patch,
    })
}

fn manifest_version_parse_error(
    manifest_path: Option<&Path>,
    raw: &str,
    schema_name: &str,
) -> GuardDiagnostic {
    let mut diagnostic = manifest_diagnostic(
        CONFIG_PROJECT_VERSION_PARSE_CODE,
        format!(
            "マニフェスト `project.version` の値 `{raw}` を SemVer として解析できません（schema: `{schema_name}`）。"
        ),
        manifest_path,
        &["project", "version"],
    );
    if let Some(Value::Object(config)) = diagnostic.extensions.get_mut("config") {
        config.insert(
            "version_mismatch".into(),
            Value::String("parse_error".into()),
        );
        config.insert("manifest_version".into(), Value::String(raw.to_string()));
        config.insert("schema_name".into(), Value::String(schema_name.to_string()));
    }
    diagnostic.audit_metadata.insert(
        "config.version_reason".into(),
        Value::String("parse_error".into()),
    );
    diagnostic.audit_metadata.insert(
        "config.schema_name".into(),
        Value::String(schema_name.to_string()),
    );
    diagnostic
}

#[derive(Debug, Clone, Copy)]
enum SemanticVersionMismatch {
    Major,
    SchemaAhead,
}

fn schema_version_incompatible(
    manifest_path: Option<&Path>,
    manifest_version: &str,
    schema_name: &str,
    schema_version: SemanticVersionParts,
    reason: SemanticVersionMismatch,
) -> GuardDiagnostic {
    let schema_version_str = format!(
        "{}.{}.{}",
        schema_version.major, schema_version.minor, schema_version.patch
    );
    let message = match reason {
        SemanticVersionMismatch::Major => format!(
            "Schema `{schema_name}` のバージョン {schema_version_str} は `project.version` ({manifest_version}) と major が一致しません。"
        ),
        SemanticVersionMismatch::SchemaAhead => format!(
            "Schema `{schema_name}` のバージョン {schema_version_str} は `project.version` ({manifest_version}) より新しく、互換条件を満たしません。"
        ),
    };
    let mut diagnostic = manifest_diagnostic(
        CONFIG_SCHEMA_VERSION_INCOMPATIBLE_CODE,
        message,
        manifest_path,
        &["project", "version"],
    );
    if let Some(Value::Object(config)) = diagnostic.extensions.get_mut("config") {
        config.insert(
            "manifest_version".into(),
            Value::String(manifest_version.to_string()),
        );
        config.insert(
            "schema_version".into(),
            Value::String(schema_version_str.clone()),
        );
        config.insert("schema_name".into(), Value::String(schema_name.to_string()));
        config.insert(
            "version_mismatch".into(),
            Value::String(
                match reason {
                    SemanticVersionMismatch::Major => "major",
                    SemanticVersionMismatch::SchemaAhead => "schema_ahead",
                }
                .into(),
            ),
        );
    }
    diagnostic.audit_metadata.insert(
        "config.schema_version".into(),
        Value::String(schema_version_str),
    );
    diagnostic.audit_metadata.insert(
        "config.schema_name".into(),
        Value::String(schema_name.to_string()),
    );
    diagnostic.audit_metadata.insert(
        "config.version_reason".into(),
        Value::String(
            match reason {
                SemanticVersionMismatch::Major => "major",
                SemanticVersionMismatch::SchemaAhead => "schema_ahead",
            }
            .into(),
        ),
    );
    diagnostic
}
