use std::{
    collections::{BTreeSet, HashMap},
    env,
};

use serde_json::{Map, Number, Value};

use crate::env::{EnvContext, EnvError, EnvErrorKind, EnvOperation, PlatformSnapshot};

/// Phase2 で共有するターゲット情報。
#[derive(Clone, Debug)]
pub struct TargetProfile {
    pub id: String,
    pub triple: Option<String>,
    pub os: String,
    pub family: String,
    pub arch: String,
    pub abi: Option<String>,
    pub vendor: Option<String>,
    pub env: Option<String>,
    pub stdlib_version: Option<String>,
    pub runtime_revision: Option<String>,
    pub features: Vec<String>,
    pub capabilities: Vec<String>,
    pub diagnostics: bool,
    pub extra: HashMap<String, String>,
}

impl TargetProfile {
    fn merge_triple(&mut self, triple: &str) {
        let components: Vec<&str> = triple.split('-').collect();
        if let Some(arch) = components.get(0) {
            if !arch.is_empty() {
                self.arch = arch.to_string();
            }
        }
        if let Some(vendor) = components.get(1) {
            if !vendor.is_empty() {
                self.vendor = Some(vendor.to_string());
            }
        }
        if let Some(os) = components.get(2) {
            if !os.is_empty() {
                self.os = os.to_string();
                self.family = default_family(os);
            }
        }
        if let Some(abi) = components.get(3) {
            if !abi.is_empty() {
                self.abi = Some(abi.to_string());
            }
        }
    }

    fn trimmed_feature_set(value: &str) -> Vec<String> {
        let mut normalized = BTreeSet::new();
        for entry in value.split(',') {
            let cleaned = entry.trim().to_ascii_lowercase();
            if !cleaned.is_empty() {
                normalized.insert(cleaned);
            }
        }
        normalized.into_iter().collect()
    }
}

/// CLI/Runtime で共有する実行時ターゲット情報。
#[derive(Clone, Debug)]
pub struct RunConfigTarget {
    pub os: String,
    pub family: String,
    pub arch: String,
    pub abi: Option<String>,
    pub vendor: Option<String>,
    pub env: Option<String>,
    pub profile_id: Option<String>,
    pub triple: Option<String>,
    pub features: Vec<String>,
    pub feature_requirements: Vec<String>,
    pub capabilities: Vec<String>,
    pub stdlib_version: Option<String>,
    pub runtime_revision: Option<String>,
    pub diagnostics: bool,
    pub extra: HashMap<String, String>,
}

/// ターゲット推論の成果物。
#[derive(Clone, Debug)]
pub struct TargetInference {
    pub profile: TargetProfile,
    pub detected: PlatformSnapshot,
}

impl TargetInference {
    pub fn inferred_payload(&self) -> Value {
        let mut payload = Map::new();
        payload.insert(
            "profile_id".to_string(),
            Value::String(self.profile.id.clone()),
        );
        if let Some(triple) = self.profile.triple.as_ref() {
            payload.insert("triple".to_string(), Value::String(triple.clone()));
        } else {
            payload.insert("triple".to_string(), Value::Null);
        }
        payload.insert("requested".to_string(), self.requested_payload());
        payload.insert("detected".to_string(), self.detected_payload());
        payload.insert(
            "features".to_string(),
            Value::Array(
                self.profile
                    .features
                    .iter()
                    .map(|value| Value::String(value.clone()))
                    .collect(),
            ),
        );
        payload.insert(
            "capabilities".to_string(),
            Value::Array(
                self.profile
                    .capabilities
                    .iter()
                    .map(|value| Value::String(value.clone()))
                    .collect(),
            ),
        );
        payload.insert(
            "diagnostics".to_string(),
            Value::Bool(self.profile.diagnostics),
        );
        payload.insert(
            "stdlib_version".to_string(),
            match &self.profile.stdlib_version {
                Some(value) => Value::String(value.clone()),
                None => Value::Null,
            },
        );
        payload.insert(
            "runtime_revision".to_string(),
            match &self.profile.runtime_revision {
                Some(value) => Value::String(value.clone()),
                None => Value::Null,
            },
        );
        payload.insert(
            "extra".to_string(),
            Value::Object(extra_map(&self.profile.extra)),
        );
        Value::Object(payload)
    }

    pub fn cfg_extension(&self, target_config_errors: usize) -> Value {
        let mut payload = Map::new();
        payload.insert(
            "target_config_errors".to_string(),
            Value::Number(Number::from(target_config_errors as u64)),
        );
        let mut profile_payload = Map::new();
        profile_payload.insert(
            "profile_id".to_string(),
            Value::String(self.profile.id.clone()),
        );
        if let Some(triple) = self.profile.triple.as_ref() {
            profile_payload.insert("triple".to_string(), Value::String(triple.clone()));
        }
        profile_payload.insert("requested".to_string(), self.requested_payload());
        profile_payload.insert("detected".to_string(), self.detected_payload());
        payload.insert("target_profile".to_string(), Value::Object(profile_payload));
        Value::Object(payload)
    }

    pub fn host_default() -> Self {
        let detected = PlatformSnapshot::detect();
        let profile_id = format!("{}-{}", detected.os, detected.arch);
        let profile = TargetProfile {
            id: profile_id,
            triple: detected.triple.clone(),
            os: detected.os.clone(),
            family: detected.family.clone(),
            arch: detected.arch.clone(),
            abi: None,
            vendor: None,
            env: None,
            stdlib_version: detected.stdlib_version.clone(),
            runtime_revision: detected.runtime_revision.clone(),
            features: Vec::new(),
            capabilities: Vec::new(),
            diagnostics: true,
            extra: HashMap::new(),
        };
        Self { profile, detected }
    }

    fn requested_payload(&self) -> Value {
        let mut payload = Map::new();
        payload.insert("os".to_string(), Value::String(self.profile.os.clone()));
        payload.insert("arch".to_string(), Value::String(self.profile.arch.clone()));
        payload.insert(
            "family".to_string(),
            Value::String(self.profile.family.clone()),
        );
        if let Some(abi) = self.profile.abi.as_ref() {
            payload.insert("abi".to_string(), Value::String(abi.clone()));
        }
        if let Some(env) = self.profile.env.as_ref() {
            payload.insert("env".to_string(), Value::String(env.clone()));
        }
        if let Some(vendor) = self.profile.vendor.as_ref() {
            payload.insert("vendor".to_string(), Value::String(vendor.clone()));
        }
        payload.insert(
            "features".to_string(),
            Value::Array(
                self.profile
                    .features
                    .iter()
                    .map(|value| Value::String(value.clone()))
                    .collect(),
            ),
        );
        payload.insert(
            "capabilities".to_string(),
            Value::Array(
                self.profile
                    .capabilities
                    .iter()
                    .map(|value| Value::String(value.clone()))
                    .collect(),
            ),
        );
        payload.insert(
            "diagnostics".to_string(),
            Value::Bool(self.profile.diagnostics),
        );
        Value::Object(payload)
    }

    fn detected_payload(&self) -> Value {
        let mut payload = Map::new();
        payload.insert("os".to_string(), Value::String(self.detected.os.clone()));
        payload.insert(
            "arch".to_string(),
            Value::String(self.detected.arch.clone()),
        );
        payload.insert(
            "family".to_string(),
            Value::String(self.detected.family.clone()),
        );
        if let Some(triple) = self.detected.triple.as_ref() {
            payload.insert("triple".to_string(), Value::String(triple.clone()));
        }
        if let Some(profile_id) = self.detected.profile_id.as_ref() {
            payload.insert("profile_id".to_string(), Value::String(profile_id.clone()));
        }
        payload.insert(
            "stdlib_version".to_string(),
            match &self.detected.stdlib_version {
                Some(value) => Value::String(value.clone()),
                None => Value::Null,
            },
        );
        payload.insert(
            "runtime_revision".to_string(),
            match &self.detected.runtime_revision {
                Some(value) => Value::String(value.clone()),
                None => Value::Null,
            },
        );
        Value::Object(payload)
    }
}

pub fn infer_target_from_env() -> Result<TargetInference, EnvError> {
    let mut inference = TargetInference::host_default();
    let mut extra = inference.profile.extra.clone();
    for (key, value) in env::vars() {
        match key.as_str() {
            "REML_TARGET_PROFILE" => inference.profile.id = value.clone(),
            "REML_TARGET_TRIPLE" => {
                inference.profile.triple = Some(value.clone());
                inference.profile.merge_triple(&value);
            }
            "REML_TARGET_OS" => {
                inference.profile.os = value.clone();
                inference.profile.family = default_family(&value);
            }
            "REML_TARGET_FAMILY" => inference.profile.family = value.clone(),
            "REML_TARGET_ARCH" => inference.profile.arch = value.clone(),
            "REML_TARGET_ENV" => inference.profile.env = Some(value.clone()),
            "REML_TARGET_VENDOR" => inference.profile.vendor = Some(value.clone()),
            "REML_TARGET_ABI" => inference.profile.abi = Some(value.clone()),
            "REML_STD_VERSION" => inference.profile.stdlib_version = Some(value.clone()),
            "REML_RUNTIME_REVISION" => inference.profile.runtime_revision = Some(value.clone()),
            "REML_TARGET_FEATURES" => {
                inference.profile.features = TargetProfile::trimmed_feature_set(&value);
            }
            "REML_TARGET_CAPABILITIES" => {
                inference.profile.capabilities = TargetProfile::trimmed_feature_set(&value);
            }
            "REML_TARGET_DIAGNOSTICS" => {
                inference.profile.diagnostics = parse_bool(&value, "REML_TARGET_DIAGNOSTICS")?;
            }
            key if key.starts_with("REML_TARGET_EXTRA_") => {
                if let Some(extra_key) = key.strip_prefix("REML_TARGET_EXTRA_") {
                    extra.insert(extra_key.to_string(), value.clone());
                }
            }
            _ => {}
        }
    }
    inference.profile.extra = extra;
    Ok(inference)
}

pub fn resolve_run_config_target(
    profile: TargetProfile,
    feature_requirements: &[String],
) -> RunConfigTarget {
    RunConfigTarget {
        os: profile.os.clone(),
        family: profile.family.clone(),
        arch: profile.arch.clone(),
        abi: profile.abi.clone(),
        vendor: profile.vendor.clone(),
        env: profile.env.clone(),
        profile_id: Some(profile.id.clone()),
        triple: profile.triple.clone(),
        features: profile.features.clone(),
        feature_requirements: feature_requirements.to_vec(),
        capabilities: profile.capabilities.clone(),
        stdlib_version: profile.stdlib_version.clone(),
        runtime_revision: profile.runtime_revision.clone(),
        diagnostics: profile.diagnostics,
        extra: profile.extra.clone(),
    }
}

fn extra_map(source: &HashMap<String, String>) -> Map<String, Value> {
    let mut map = Map::new();
    let mut keys: Vec<String> = source.keys().cloned().collect();
    keys.sort();
    for key in keys {
        if let Some(value) = source.get(&key) {
            map.insert(key, Value::String(value.clone()));
        }
    }
    map
}

fn default_family(os: &str) -> String {
    match os {
        "linux" | "macos" | "freebsd" | "openbsd" | "android" | "ios" => "unix".to_string(),
        "windows" => "windows".to_string(),
        "wasm" => "wasm".to_string(),
        _ => "other".to_string(),
    }
}

fn parse_bool(value: &str, key: &str) -> Result<bool, EnvError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "on" | "yes" => Ok(true),
        "0" | "false" | "off" | "no" => Ok(false),
        other => Err(EnvError {
            kind: EnvErrorKind::InvalidEncoding,
            message: format!("{key} に `{other}` は不正な bool 値です"),
            key: Some(key.to_string()),
            context: Some(EnvContext::detect(EnvOperation::Get)),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn clear_target_env() {
        for key in [
            "REML_TARGET_PROFILE",
            "REML_TARGET_TRIPLE",
            "REML_TARGET_OS",
            "REML_TARGET_FAMILY",
            "REML_TARGET_ARCH",
            "REML_TARGET_ENV",
            "REML_TARGET_VENDOR",
            "REML_TARGET_ABI",
            "REML_STD_VERSION",
            "REML_RUNTIME_REVISION",
            "REML_TARGET_FEATURES",
            "REML_TARGET_CAPABILITIES",
            "REML_TARGET_DIAGNOSTICS",
            "REML_TARGET_EXTRA_io.blocking",
        ] {
            env::remove_var(key);
        }
    }

    #[test]
    fn infer_target_from_env_defaults_to_host() {
        clear_target_env();
        let inference = infer_target_from_env().expect("should not fail");
        assert_eq!(inference.profile.os, inference.detected.os);
        assert_eq!(
            canonical_arch(&inference.profile.arch),
            canonical_arch(&inference.detected.arch)
        );
    }

    #[test]
    fn infer_target_from_env_respects_overrides() {
        clear_target_env();
        env::set_var("REML_TARGET_PROFILE", "desktop-x86_64");
        env::set_var("REML_TARGET_ARCH", "arm64");
        env::set_var("REML_TARGET_FAMILY", "unix");
        env::set_var("REML_TARGET_FEATURES", "SIMD,packrat");
        env::set_var("REML_TARGET_CAPABILITIES", "unicode.nfc,fs.case_sensitive");
        env::set_var("REML_TARGET_DIAGNOSTICS", "0");
        env::set_var("REML_TARGET_EXTRA_io.blocking", "strict");
        let inference = infer_target_from_env().expect("should parse overrides");
        assert_eq!(inference.profile.id, "desktop-x86_64");
        assert_eq!(inference.profile.arch, "arm64");
        assert_eq!(inference.profile.family, "unix");
        assert!(!inference.profile.diagnostics);
        assert!(inference.profile.features.contains(&"packrat".into()));
        assert!(inference
            .profile
            .capabilities
            .contains(&"fs.case_sensitive".into()));
        assert_eq!(
            inference.profile.extra.get("io.blocking"),
            Some(&"strict".to_string())
        );
    }

    fn canonical_arch(arch: &str) -> &str {
        match arch {
            "arm64" => "aarch64",
            other => other,
        }
    }
}
