use serde_json::{json, Map, Value};
use std::collections::HashMap;

use crate::target_machine::{TargetMachine, Triple};

/// Rust LLVM バックエンドが `RunConfigTarget`/`PlatformInfo` を保持するためのコンテキスト。
#[derive(Clone, Debug)]
pub struct TargetDiagnosticContext {
    pub run_config: RunConfigTarget,
    pub platform_info: PlatformInfo,
}

impl TargetDiagnosticContext {
    pub fn from_target_machine(target_machine: &TargetMachine) -> Self {
        Self {
            run_config: RunConfigTarget::from_target_machine(target_machine),
            platform_info: PlatformInfo::detect(),
        }
    }

    pub fn with_run_config(mut self, run_config: RunConfigTarget) -> Self {
        self.run_config = run_config;
        self
    }

    pub fn with_platform_info(mut self, platform_info: PlatformInfo) -> Self {
        self.platform_info = platform_info;
        self
    }
}

/// CLI/環境から指定されたターゲット設定。
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

impl Default for RunConfigTarget {
    fn default() -> Self {
        Self {
            os: "unknown".to_string(),
            family: "other".to_string(),
            arch: "unknown".to_string(),
            abi: None,
            vendor: None,
            env: None,
            profile_id: None,
            triple: None,
            features: Vec::new(),
            feature_requirements: Vec::new(),
            capabilities: Vec::new(),
            stdlib_version: None,
            runtime_revision: None,
            diagnostics: true,
            extra: HashMap::new(),
        }
    }
}

impl RunConfigTarget {
    pub fn requested_payload(&self) -> Value {
        json!({
            "os": self.os,
            "family": self.family,
            "arch": self.arch,
            "abi": self.abi,
            "vendor": self.vendor,
            "env": self.env,
            "profile_id": self.profile_id,
            "triple": self.triple,
            "features": self.features,
            "feature_requirements": self.feature_requirements,
            "capabilities": self.capabilities,
            "stdlib_version": self.stdlib_version,
            "runtime_revision": self.runtime_revision,
            "diagnostics": self.diagnostics,
            "extra": self.extra,
        })
    }

    pub fn from_target_machine(machine: &TargetMachine) -> Self {
        let (os, family, arch) = match machine.triple {
            Triple::LinuxGNU => ("linux", "unix", "x86_64"),
            Triple::AppleDarwin => ("macos", "unix", "x86_64"),
            Triple::WindowsGNU | Triple::WindowsMSVC => ("windows", "windows", "x86_64"),
        };
        Self {
            os: os.to_string(),
            family: family.to_string(),
            arch: arch.to_string(),
            vendor: None,
            env: None,
            profile_id: None,
            triple: Some(machine.triple.to_string()),
            features: machine
                .features
                .split(',')
                .filter(|segment| !segment.trim().is_empty())
                .map(|feature| feature.trim().to_string())
                .collect(),
            feature_requirements: Vec::new(),
            capabilities: Vec::new(),
            stdlib_version: None,
            runtime_revision: None,
            diagnostics: true,
            extra: HashMap::new(),
            abi: Some(machine.backend_abi().to_string()),
        }
    }
}

/// 実行環境から取得したプラットフォーム情報。
#[derive(Clone, Debug)]
pub struct PlatformInfo {
    pub os: String,
    pub arch: String,
    pub family: String,
    pub variant: Option<String>,
    pub features: Vec<String>,
    pub runtime_capabilities: Vec<String>,
    pub target_capabilities: Vec<String>,
    pub profile_id: Option<String>,
    pub triple: Option<String>,
    pub stdlib_version: Option<String>,
    pub runtime_revision: Option<String>,
}

impl PlatformInfo {
    pub fn detect() -> Self {
        let os = std::env::consts::OS.to_string();
        let family = match os.as_str() {
            "linux" | "macos" | "freebsd" | "openbsd" | "android" | "ios" => "unix",
            "windows" => "windows",
            "wasm" => "wasm",
            _ => "other",
        }
        .to_string();
        Self {
            os,
            arch: std::env::consts::ARCH.to_string(),
            family,
            variant: None,
            features: Vec::new(),
            runtime_capabilities: Vec::new(),
            target_capabilities: Vec::new(),
            profile_id: None,
            triple: None,
            stdlib_version: None,
            runtime_revision: None,
        }
    }

    pub fn detected_payload(&self) -> Value {
        json!({
            "os": self.os,
            "arch": self.arch,
            "family": self.family,
            "variant": self.variant,
            "features": self.features,
            "runtime_capabilities": self.runtime_capabilities,
            "target_capabilities": self.target_capabilities,
            "profile_id": self.profile_id,
            "triple": self.triple,
            "stdlib_version": self.stdlib_version,
            "runtime_revision": self.runtime_revision,
        })
    }
}

#[derive(Clone, Debug)]
pub struct TargetDiagnostic {
    pub code: &'static str,
    pub message: String,
    pub extension: Value,
}

#[derive(Clone, Debug)]
pub struct TargetAudit {
    pub key: String,
    pub value: Value,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TargetVerdict {
    Pass,
    Fail,
}

impl TargetVerdict {
    fn as_str(&self) -> &'static str {
        match self {
            TargetVerdict::Pass => "pass",
            TargetVerdict::Fail => "fail",
        }
    }
}

#[derive(Clone, Debug)]
pub struct TargetDiagnosticReport {
    pub diagnostics: Vec<TargetDiagnostic>,
    pub audit: Vec<TargetAudit>,
    pub verdict: TargetVerdict,
}

#[derive(Clone, Debug)]
pub struct TargetDiagnosticEmitter {
    context: TargetDiagnosticContext,
}

impl TargetDiagnosticEmitter {
    pub fn new(context: TargetDiagnosticContext) -> Self {
        Self { context }
    }

    pub fn emit(&self) -> TargetDiagnosticReport {
        let requested = self.context.run_config.requested_payload();
        let detected = self.context.platform_info.detected_payload();
        let mismatches = self.collect_mismatches();
        let mut diagnostics = Vec::new();

        if self.context.run_config.profile_id.is_none() {
            diagnostics.push(self.build_profile_missing(&requested, &detected, &mismatches));
        }

        if !mismatches.is_empty() {
            diagnostics.push(self.build_config_mismatch(&requested, &detected, &mismatches));
        }

        let verdict = if diagnostics.is_empty() {
            TargetVerdict::Pass
        } else {
            TargetVerdict::Fail
        };

        let mut audit = Vec::new();
        audit.push(TargetAudit::new("target.requested", requested.clone()));
        audit.push(TargetAudit::new("target.detected", detected.clone()));
        audit.push(TargetAudit::new("target.verdict", json!(verdict.as_str())));

        TargetDiagnosticReport {
            diagnostics,
            audit,
            verdict,
        }
    }

    fn build_profile_missing(
        &self,
        requested: &Value,
        detected: &Value,
        mismatches: &[TargetMismatch],
    ) -> TargetDiagnostic {
        let message = "ターゲットプロファイルが指定されていません";
        TargetDiagnostic {
            code: "target.profile.missing",
            message: message.to_string(),
            extension: self.build_extension(requested, detected, mismatches),
        }
    }

    fn build_config_mismatch(
        &self,
        requested: &Value,
        detected: &Value,
        mismatches: &[TargetMismatch],
    ) -> TargetDiagnostic {
        let fields = mismatches
            .iter()
            .map(|entry| entry.field)
            .collect::<Vec<_>>()
            .join(", ");
        let message = format!(
            "要求されたターゲットフィールド ({}) が実行環境と一致しません",
            fields
        );
        TargetDiagnostic {
            code: "target.config.mismatch",
            message,
            extension: self.build_extension(requested, detected, mismatches),
        }
    }

    fn build_extension(
        &self,
        requested: &Value,
        detected: &Value,
        mismatches: &[TargetMismatch],
    ) -> Value {
        let mut extension = Map::new();
        if let Some(profile_id) = &self.context.run_config.profile_id {
            extension.insert("profile_id".to_string(), json!(profile_id));
        }
        if let Some(triple) = &self.context.run_config.triple {
            extension.insert("triple".to_string(), json!(triple));
        }
        extension.insert("requested".to_string(), requested.clone());
        extension.insert("detected".to_string(), detected.clone());
        if !mismatches.is_empty() {
            let compared = mismatches
                .iter()
                .map(|entry| {
                    json!({
                        "field": entry.field,
                        "requested": entry.requested,
                        "detected": entry.detected,
                    })
                })
                .collect::<Vec<_>>();
            extension.insert("compared_with".to_string(), Value::Array(compared));
        }
        Value::Object(extension)
    }

    fn collect_mismatches(&self) -> Vec<TargetMismatch> {
        let mut mismatches = Vec::new();
        mismatches.extend(self.compare_field(
            "os",
            &self.context.run_config.os,
            &self.context.platform_info.os,
        ));
        mismatches.extend(self.compare_field(
            "family",
            &self.context.run_config.family,
            &self.context.platform_info.family,
        ));
        mismatches.extend(self.compare_field(
            "arch",
            &self.context.run_config.arch,
            &self.context.platform_info.arch,
        ));
        if let Some(requested_triple) = &self.context.run_config.triple {
            if let Some(detected_triple) = &self.context.platform_info.triple {
                if requested_triple != detected_triple {
                    mismatches.push(TargetMismatch::new(
                        "triple",
                        Value::String(requested_triple.clone()),
                        Value::String(detected_triple.clone()),
                    ));
                }
            }
        }
        mismatches
    }

    fn compare_field(
        &self,
        name: &'static str,
        requested: &str,
        detected: &str,
    ) -> Vec<TargetMismatch> {
        if !requested.is_empty() && !detected.is_empty() && requested != detected {
            vec![TargetMismatch::new(
                name,
                Value::String(requested.to_string()),
                Value::String(detected.to_string()),
            )]
        } else {
            Vec::new()
        }
    }
}

impl TargetAudit {
    fn new(key: &'static str, value: Value) -> Self {
        Self {
            key: key.to_string(),
            value,
        }
    }
}

#[derive(Clone, Debug)]
struct TargetMismatch {
    field: &'static str,
    requested: Value,
    detected: Value,
}

impl TargetMismatch {
    fn new(field: &'static str, requested: Value, detected: Value) -> Self {
        Self {
            field,
            requested,
            detected,
        }
    }
}
