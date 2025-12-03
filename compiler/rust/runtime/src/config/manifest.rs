use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::PathBuf;
use toml::de;

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

    pub fn parse_toml(input: &str) -> Result<Self, ManifestParseError> {
        let mut manifest: Manifest = de::from_str(input).map_err(ManifestParseError::from)?;
        for entry in manifest.dsl.values_mut() {
            entry.ensure_sane_defaults();
        }
        Ok(manifest)
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

    pub fn dependency(
        mut self,
        name: impl Into<String>,
        spec: DependencySpec,
    ) -> Self {
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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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
    fn as_str(&self) -> &str {
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
    fn as_str(&self) -> &str {
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
    fn as_str(&self) -> &str {
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
