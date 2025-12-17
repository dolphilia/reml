use std::fmt;

use crate::target_diagnostics::RunConfigTarget;

/// Reml LLVM バックエンドで想定するターゲット Triple。
#[derive(Clone, Copy, Debug)]
pub enum Triple {
    LinuxGNU,
    AppleDarwin,
    WindowsGNU,
    WindowsMSVC,
}

impl Triple {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Triple::LinuxGNU => "x86_64-unknown-linux-gnu",
            Triple::AppleDarwin => "x86_64-apple-darwin",
            Triple::WindowsGNU => "x86_64-pc-windows-gnu",
            Triple::WindowsMSVC => "x86_64-pc-windows-msvc",
        }
    }
}

impl Triple {
    pub fn platform_label(&self) -> &'static str {
        match self {
            Triple::LinuxGNU => "linux-x86_64",
            Triple::AppleDarwin => "macos-arm64",
            Triple::WindowsGNU | Triple::WindowsMSVC => "windows-msvc-x64",
        }
    }

    pub fn canonical_arch(&self) -> &'static str {
        match self {
            Triple::AppleDarwin => "arm64",
            _ => "x86_64",
        }
    }
}

impl Triple {
    fn from_str(triple: &str) -> Option<Self> {
        match triple.to_ascii_lowercase().as_str() {
            "x86_64-unknown-linux-gnu" | "x86_64-linux-gnu" | "x86_64-linux" => {
                Some(Triple::LinuxGNU)
            }
            "x86_64-apple-darwin" => Some(Triple::AppleDarwin),
            "x86_64-pc-windows-gnu" | "x86_64-windows-gnu" | "x86_64-pc-windows-gcc" => {
                Some(Triple::WindowsGNU)
            }
            "x86_64-pc-windows-msvc" | "x86_64-windows-msvc" => Some(Triple::WindowsMSVC),
            _ => None,
        }
    }
}

impl fmt::Display for Triple {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// 再配置モデル。
#[derive(Clone, Copy, Debug)]
pub enum RelocModel {
    Default,
    Static,
    PIC,
    DynamicNoPic,
}

/// コードモデル。
#[derive(Clone, Copy, Debug)]
pub enum CodeModel {
    Default,
    Small,
    Kernel,
    Medium,
    Large,
}

/// 最適化レベル。
#[derive(Clone, Copy, Debug)]
pub enum OptimizationLevel {
    O0,
    O1,
    O2,
    O3,
    Os,
}

/// DataLayout 文字列と関連情報。
#[derive(Clone, Debug)]
pub struct DataLayoutSpec {
    pub description: String,
}

impl DataLayoutSpec {
    pub fn new(layout: impl Into<String>) -> Self {
        Self {
            description: layout.into(),
        }
    }

    pub fn system_v() -> Self {
        Self::new("e-m:e-p:64:64-f64:64:64-v128:128:128-a:0:64")
    }
}

/// Windows 専用のツールチェーン設定。
#[derive(Clone, Debug)]
pub struct WindowsToolchainConfig {
    pub toolchain_name: String,
    pub llc_path: String,
    pub opt_path: String,
}

#[derive(Clone, Copy, Debug)]
struct TargetSpec {
    triple: Triple,
    cpu: &'static str,
    default_features: &'static str,
    data_layout: &'static str,
    abi: &'static str,
}

impl TargetSpec {
    fn for_triple(triple: Triple) -> Self {
        match triple {
            Triple::LinuxGNU => TargetSpec {
                triple,
                cpu: "x86-64",
                default_features: "+sse4.2,+popcnt",
                data_layout: "e-m:e-p:64:64-f64:64:64-v128:128:128-a:0:64",
                abi: "system_v",
            },
            Triple::AppleDarwin => TargetSpec {
                triple,
                cpu: "x86-64",
                default_features: "",
                data_layout: "e-m:o-i64:64-f80:128-n8:16:32:64-S128",
                abi: "darwin_aapcs64",
            },
            Triple::WindowsGNU => TargetSpec {
                triple,
                cpu: "x86-64",
                default_features: "+sse4.2,+popcnt",
                data_layout: "e-m:w-p:64:64-f64:64:64-v128:128:128-a:0:64",
                abi: "gnu",
            },
            Triple::WindowsMSVC => TargetSpec {
                triple,
                cpu: "x86-64",
                default_features: "+sse4.2,+popcnt",
                data_layout: "e-m:w-p:64:64-f64:64:64-v128:128:128-a:0:64",
                abi: "msvc",
            },
        }
    }

    fn from_run_config(run_config: &RunConfigTarget) -> Self {
        let triple = Self::resolve_triple(run_config);
        Self::for_triple(triple)
    }

    fn resolve_triple(run_config: &RunConfigTarget) -> Triple {
        if let Some(triple_name) = &run_config.triple {
            if let Some(triple) = Triple::from_str(triple_name) {
                return triple;
            }
        }
        let os = run_config.os.to_ascii_lowercase();
        if os.starts_with("mac") || os.starts_with("darwin") {
            return Triple::AppleDarwin;
        }
        if os.starts_with("windows") || os.starts_with("win") {
            if let Some(abi) = &run_config.abi {
                if abi.to_ascii_lowercase().contains("msvc") {
                    return Triple::WindowsMSVC;
                }
            }
            return Triple::WindowsGNU;
        }
        Triple::LinuxGNU
    }

    fn merge_features(&self, extras: &[String]) -> String {
        let mut parts = Vec::new();
        if !self.default_features.trim().is_empty() {
            parts.push(self.default_features.trim().to_string());
        }
        for feature in extras {
            let trimmed = feature.trim();
            if !trimmed.is_empty() {
                parts.push(trimmed.to_string());
            }
        }
        parts.join(",")
    }
}

/// TargetMachine を構成するためのビルダー。
#[derive(Clone, Debug)]
pub struct TargetMachineBuilder {
    triple: Triple,
    cpu: String,
    features: String,
    reloc_model: RelocModel,
    code_model: CodeModel,
    opt_level: OptimizationLevel,
    data_layout: DataLayoutSpec,
    windows_toolchain: Option<WindowsToolchainConfig>,
    backend_abi: String,
}

impl Default for TargetMachineBuilder {
    fn default() -> Self {
        let spec = TargetSpec::for_triple(Triple::LinuxGNU);
        Self {
            triple: spec.triple,
            cpu: spec.cpu.into(),
            features: spec.default_features.into(),
            reloc_model: RelocModel::Default,
            code_model: CodeModel::Default,
            opt_level: OptimizationLevel::O2,
            data_layout: DataLayoutSpec::new(spec.data_layout),
            windows_toolchain: None,
            backend_abi: spec.abi.into(),
        }
    }
}

impl TargetMachineBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_run_config(mut self, run_config: &RunConfigTarget) -> Self {
        let spec = TargetSpec::from_run_config(run_config);
        let merged_features = spec.merge_features(&run_config.features);
        self.triple = spec.triple;
        self.cpu = spec.cpu.into();
        self.features = spec.default_features.into();
        self.data_layout = DataLayoutSpec::new(spec.data_layout);
        self.backend_abi = spec.abi.into();
        if !merged_features.is_empty() && merged_features != spec.default_features {
            self.features = merged_features;
        }
        self
    }

    pub fn with_triple(mut self, triple: Triple) -> Self {
        self.triple = triple;
        self
    }

    pub fn with_cpu(mut self, cpu: impl Into<String>) -> Self {
        self.cpu = cpu.into();
        self
    }

    pub fn with_features(mut self, features: impl Into<String>) -> Self {
        self.features = features.into();
        self
    }

    pub fn with_relocation_model(mut self, model: RelocModel) -> Self {
        self.reloc_model = model;
        self
    }

    pub fn with_code_model(mut self, model: CodeModel) -> Self {
        self.code_model = model;
        self
    }

    pub fn with_optimization_level(mut self, level: OptimizationLevel) -> Self {
        self.opt_level = level;
        self
    }

    pub fn with_data_layout(mut self, layout: DataLayoutSpec) -> Self {
        self.data_layout = layout;
        self
    }

    pub fn with_windows_toolchain(mut self, config: WindowsToolchainConfig) -> Self {
        self.windows_toolchain = Some(config);
        self
    }

    pub fn with_backend_abi(mut self, abi: impl Into<String>) -> Self {
        self.backend_abi = abi.into();
        self
    }

    pub fn build(self) -> TargetMachine {
        TargetMachine {
            triple: self.triple,
            cpu: self.cpu,
            features: self.features,
            reloc_model: self.reloc_model,
            code_model: self.code_model,
            opt_level: self.opt_level,
            data_layout: self.data_layout,
            windows_toolchain: self.windows_toolchain,
            backend_abi: self.backend_abi,
        }
    }
}

/// 実際の TargetMachine 設定を保持する構造。
#[derive(Clone, Debug)]
pub struct TargetMachine {
    pub triple: Triple,
    pub cpu: String,
    pub features: String,
    pub reloc_model: RelocModel,
    pub code_model: CodeModel,
    pub opt_level: OptimizationLevel,
    pub data_layout: DataLayoutSpec,
    pub windows_toolchain: Option<WindowsToolchainConfig>,
    backend_abi: String,
}

impl TargetMachine {
    pub fn backend_abi(&self) -> &str {
        &self.backend_abi
    }

    pub fn describe(&self) -> String {
        format!(
            "Triple={} ABI={} CPU={} features={} layout={}",
            self.triple, self.backend_abi, self.cpu, self.features, self.data_layout.description
        )
    }
}
