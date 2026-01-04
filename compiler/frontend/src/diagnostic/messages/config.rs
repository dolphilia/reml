//! Config/Data 系の診断テンプレートとメタデータ補助。

use crate::{
    diagnostic::{DiagnosticDomain, DiagnosticSeverity, FrontendDiagnostic},
    error::Recoverability,
};
use serde_json::{json, Map, Value};
use std::path::PathBuf;

/// Config 診断で共有するメタデータ。
#[derive(Debug, Clone, Default)]
pub struct ConfigDiagnosticMetadata {
    pub manifest_path: Option<PathBuf>,
    pub key_path: Option<String>,
    pub source: Option<String>,
    pub profile: Option<String>,
    pub format: Option<String>,
    pub compatibility: Option<Value>,
    pub diff: Option<Value>,
    pub feature_guard: Option<Value>,
}

impl ConfigDiagnosticMetadata {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_manifest_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.manifest_path = Some(path.into());
        self
    }

    pub fn with_key_path(mut self, key_path: impl Into<String>) -> Self {
        self.key_path = Some(key_path.into());
        self
    }

    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    pub fn with_profile(mut self, profile: impl Into<String>) -> Self {
        self.profile = Some(profile.into());
        self
    }

    pub fn with_format(mut self, format: impl Into<String>) -> Self {
        self.format = Some(format.into());
        self
    }

    pub fn with_compatibility(mut self, compatibility: Value) -> Self {
        self.compatibility = Some(compatibility);
        self
    }

    pub fn with_diff(mut self, diff: Value) -> Self {
        self.diff = Some(diff);
        self
    }

    pub fn with_feature_guard(mut self, guard: Value) -> Self {
        self.feature_guard = Some(guard);
        self
    }
}

/// `reml.toml` が見つからない場合の共通診断。
pub fn missing_manifest(metadata: ConfigDiagnosticMetadata) -> FrontendDiagnostic {
    let mut diagnostic = FrontendDiagnostic::new("`reml.toml` を検出できませんでした")
        .with_code("config.missing_manifest")
        .with_severity(DiagnosticSeverity::Error)
        .with_domain(DiagnosticDomain::Config)
        .with_recoverability(Recoverability::Fatal);
    apply_config_metadata(&mut diagnostic, &metadata);
    diagnostic
}

/// Manifest / Schema のバージョン不一致を報告する診断。
pub fn schema_mismatch(
    expected: impl Into<String>,
    actual: impl Into<String>,
    metadata: ConfigDiagnosticMetadata,
) -> FrontendDiagnostic {
    let expected_value = expected.into();
    let actual_value = actual.into();
    let message = format!(
        "Schema バージョンが一致しません（期待値: {expected}, 入力: {actual}）",
        expected = expected_value.as_str(),
        actual = actual_value.as_str()
    );
    let mut diagnostic = FrontendDiagnostic::new(message)
        .with_code("config.schema_mismatch")
        .with_severity(DiagnosticSeverity::Error)
        .with_domain(DiagnosticDomain::Config)
        .with_recoverability(Recoverability::Fatal);
    let mut config_extension = Map::new();
    config_extension.insert(
        "schema".to_string(),
        json!({ "expected": expected_value, "actual": actual_value }),
    );
    diagnostic
        .extensions
        .insert("config".to_string(), Value::Object(config_extension));
    apply_config_metadata(&mut diagnostic, &metadata);
    diagnostic
}

/// サポートしていない互換プロファイル／フォーマット組み合わせを報告する診断。
pub fn compatibility_unsupported(
    format_label: impl Into<String>,
    profile_label: impl Into<String>,
    reason: impl Into<String>,
    metadata: ConfigDiagnosticMetadata,
) -> FrontendDiagnostic {
    let format_value = format_label.into();
    let profile_value = profile_label.into();
    let reason_value = reason.into();
    let message = format!(
        "{format} 互換プロファイル `{profile}` は未サポートです: {reason}",
        format = format_value.as_str(),
        profile = profile_value.as_str(),
        reason = reason_value.as_str()
    );
    let mut diagnostic = FrontendDiagnostic::new(message)
        .with_code("config.compat.unsupported")
        .with_severity(DiagnosticSeverity::Error)
        .with_domain(DiagnosticDomain::Config)
        .with_recoverability(Recoverability::Fatal);
    let mut config_extension = Map::new();
    config_extension.insert(
        "compatibility_violation".to_string(),
        json!({
            "format": format_value,
            "profile": profile_value,
            "reason": reason_value
        }),
    );
    diagnostic
        .extensions
        .insert("config".to_string(), Value::Object(config_extension));
    apply_config_metadata(&mut diagnostic, &metadata);
    diagnostic
}

fn apply_config_metadata(diag: &mut FrontendDiagnostic, metadata: &ConfigDiagnosticMetadata) {
    let mut config_extension = take_config_extension(diag);
    if let Some(path) = metadata.manifest_path.as_ref() {
        let path_str = path.display().to_string();
        config_extension.insert("path".to_string(), json!(path_str));
        diag.audit_metadata
            .insert("config.path".to_string(), json!(path_str));
    }
    if let Some(key_path) = metadata.key_path.as_ref() {
        config_extension.insert("key_path".to_string(), json!(key_path));
        diag.audit_metadata
            .insert("config.key_path".to_string(), json!(key_path));
    }
    if let Some(source) = metadata.source.as_ref() {
        config_extension.insert("source".to_string(), json!(source));
        diag.audit_metadata
            .insert("config.source".to_string(), json!(source));
    }
    if let Some(profile) = metadata.profile.as_ref() {
        config_extension.insert("profile".to_string(), json!(profile));
        diag.audit_metadata
            .insert("config.profile".to_string(), json!(profile));
    }
    if let Some(format) = metadata.format.as_ref() {
        config_extension.insert("format".to_string(), json!(format));
        diag.audit_metadata
            .insert("config.format".to_string(), json!(format));
    }
    if let Some(value) = metadata.compatibility.as_ref() {
        config_extension.insert("compatibility".to_string(), value.clone());
        diag.audit_metadata
            .insert("config.compatibility".to_string(), value.clone());
    }
    if let Some(value) = metadata.diff.as_ref() {
        config_extension.insert("diff".to_string(), value.clone());
        diag.audit_metadata
            .insert("config.diff".to_string(), value.clone());
    }
    if let Some(value) = metadata.feature_guard.as_ref() {
        config_extension.insert("feature_guard".to_string(), value.clone());
        diag.audit_metadata
            .insert("config.feature_guard".to_string(), value.clone());
    }
    store_config_extension(diag, config_extension);
}

fn take_config_extension(diag: &mut FrontendDiagnostic) -> Map<String, Value> {
    match diag.extensions.remove("config") {
        Some(Value::Object(map)) => map,
        Some(other) => other.as_object().cloned().unwrap_or_else(Map::new),
        None => Map::new(),
    }
}

fn store_config_extension(diag: &mut FrontendDiagnostic, map: Map<String, Value>) {
    diag.extensions
        .insert("config".to_string(), Value::Object(map));
}

#[cfg(test)]
mod tests {
    use super::{missing_manifest, ConfigDiagnosticMetadata};
    use serde_json::json;
    use std::path::PathBuf;

    #[test]
    fn missing_manifest_applies_metadata() {
        let metadata = ConfigDiagnosticMetadata::new()
            .with_manifest_path(PathBuf::from("examples/reml.toml"))
            .with_source("cli")
            .with_profile("strict")
            .with_compatibility(json!({ "format": "toml", "stage": "beta" }));
        let diag = missing_manifest(metadata);
        assert_eq!(diag.code.as_deref(), Some("config.missing_manifest"));
        let config_extension = diag
            .extensions
            .get("config")
            .and_then(|value| value.as_object())
            .expect("config extension missing");
        assert_eq!(
            config_extension.get("path").and_then(|v| v.as_str()),
            Some("examples/reml.toml")
        );
        assert_eq!(
            diag.audit_metadata
                .get("config.source")
                .and_then(|v| v.as_str()),
            Some("cli")
        );
        assert!(diag
            .audit_metadata
            .get("config.compatibility")
            .and_then(|v| v.get("format"))
            .is_some());
    }
}
