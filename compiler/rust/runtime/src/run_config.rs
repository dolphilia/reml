use crate::{
    config::{compat::CompatibilityLayer, manifest::Manifest, ConfigFormat},
    stage::StageId,
    text::LocaleId,
};
use serde_json::{json, Map, Value};
use std::collections::HashMap;

/// 左再帰処理のモード。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LeftRecursionStrategy {
    Off,
    On,
    Auto,
}

impl Default for LeftRecursionStrategy {
    fn default() -> Self {
        Self::Auto
    }
}

/// RunConfig 拡張のネームスペースごとの値を保持する。
pub type RunConfigExtensionValue = Map<std::string::String, Value>;

/// `extensions` 全体を表すマップ。名前空間ごとに JSON 互換の値を保持する。
pub type RunConfigExtensions = HashMap<std::string::String, RunConfigExtensionValue>;

/// パーサー実行時に利用する設定。
#[derive(Clone, Debug, PartialEq)]
pub struct RunConfig {
    pub require_eof: bool,
    pub packrat: bool,
    pub profile: bool,
    pub left_recursion: LeftRecursionStrategy,
    pub trace: bool,
    pub merge_warnings: bool,
    pub legacy_result: bool,
    pub locale: Option<LocaleId>,
    pub extensions: RunConfigExtensions,
}

impl Default for RunConfig {
    fn default() -> Self {
        Self {
            require_eof: false,
            packrat: false,
            profile: false,
            left_recursion: LeftRecursionStrategy::Auto,
            trace: false,
            merge_warnings: true,
            legacy_result: false,
            locale: None,
            extensions: RunConfigExtensions::new(),
        }
    }
}

impl RunConfig {
    /// 指定した名前空間の拡張設定をイミュータブルに更新する。
    pub fn with_extension<F>(&self, key: &str, update: F) -> Self
    where
        F: FnOnce(RunConfigExtensionValue) -> RunConfigExtensionValue,
    {
        let mut extensions = self.extensions.clone();
        let current = extensions.remove(key).unwrap_or_default();
        extensions.insert(key.to_string(), update(current));
        Self {
            extensions,
            ..self.clone()
        }
    }
}

/// `apply_manifest_overrides` の入出力を表す。
#[derive(Debug, Clone, Default)]
pub struct RunConfigManifestOverrides {
    pub manifest_extension: Map<String, Value>,
    pub compatibility_layer: Option<CompatibilityLayer>,
}

/// マニフェスト由来の RunConfig 拡張を構築する際の入力。
pub struct ApplyManifestOverridesArgs<'a> {
    pub manifest: &'a Manifest,
    pub format: ConfigFormat,
    pub stage: StageId,
}

/// `reml.toml` から RunConfig に転写する情報を構築する。
pub fn apply_manifest_overrides(
    args: ApplyManifestOverridesArgs<'_>,
) -> RunConfigManifestOverrides {
    let mut manifest_payload = Map::new();
    manifest_payload.insert("source".into(), json!("manifest"));
    if let Some(path) = args.manifest.manifest_path() {
        manifest_payload.insert("path".into(), Value::String(path.display().to_string()));
    }
    manifest_payload.insert(
        "runtime_stage".into(),
        Value::String(args.stage.as_str().to_string()),
    );
    manifest_payload.insert("project".into(), project_payload(args.manifest));
    if let Some(build) = build_payload(args.manifest) {
        manifest_payload.insert("build".into(), build);
    }

    let compatibility_layer = args.manifest.compatibility_layer(args.format, args.stage);
    if let Some(layer) = &compatibility_layer {
        manifest_payload.insert("compatibility_source".into(), json!("manifest"));
        if let Some(label) = layer.profile_label.as_ref() {
            manifest_payload.insert("compatibility_profile".into(), json!(label));
        }
        if let Ok(value) = serde_json::to_value(&layer.compatibility) {
            manifest_payload.insert("compatibility".into(), value);
        }
        let feature_guard: Vec<Value> = layer
            .compatibility
            .feature_guard
            .iter()
            .cloned()
            .map(Value::String)
            .collect();
        if !feature_guard.is_empty() {
            manifest_payload.insert("feature_guard".into(), Value::Array(feature_guard));
        }
    }

    RunConfigManifestOverrides {
        manifest_extension: manifest_payload,
        compatibility_layer,
    }
}

fn project_payload(manifest: &Manifest) -> Value {
    let mut project = Map::new();
    project.insert("name".into(), json!(manifest.project.name.0));
    project.insert("version".into(), json!(manifest.project.version.0));
    project.insert("stage".into(), json!(manifest.project.stage.as_str()));
    if !manifest.project.capabilities.is_empty() {
        let caps = manifest
            .project
            .capabilities
            .iter()
            .map(|capability| Value::String(capability.0.clone()))
            .collect();
        project.insert("capabilities".into(), Value::Array(caps));
    }
    Value::Object(project)
}

fn build_payload(manifest: &Manifest) -> Option<Value> {
    let mut build = Map::new();
    if let Some(target) = manifest.build.target.as_ref() {
        if !target.0.is_empty() {
            build.insert("target".into(), json!(target.0));
        }
    }
    if !manifest.build.targets.is_empty() {
        let targets = manifest
            .build
            .targets
            .iter()
            .filter(|triple| !triple.0.is_empty())
            .map(|triple| Value::String(triple.0.clone()))
            .collect::<Vec<_>>();
        if !targets.is_empty() {
            build.insert("targets".into(), Value::Array(targets));
        }
    }
    if !manifest.build.features.is_empty() {
        let features = manifest
            .build
            .features
            .iter()
            .map(|feature| Value::String(feature.clone()))
            .collect::<Vec<_>>();
        build.insert("features".into(), Value::Array(features));
    }
    build.insert("optimize".into(), json!(manifest.build.optimize.as_str()));
    build.insert(
        "warnings_as_errors".into(),
        json!(manifest.build.warnings_as_errors),
    );
    if build.values().all(|value| *value == Value::Null) {
        None
    } else {
        Some(Value::Object(build))
    }
}
