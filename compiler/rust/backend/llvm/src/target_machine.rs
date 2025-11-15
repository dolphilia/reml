use std::fmt;

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
    Self { description: layout.into() }
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
}

impl Default for TargetMachineBuilder {
  fn default() -> Self {
    Self {
      triple: Triple::LinuxGNU,
      cpu: "generic".into(),
      features: "".into(),
      reloc_model: RelocModel::Default,
      code_model: CodeModel::Default,
      opt_level: OptimizationLevel::O2,
      data_layout: DataLayoutSpec::system_v(),
      windows_toolchain: None,
    }
  }
}

impl TargetMachineBuilder {
  pub fn new() -> Self {
    Self::default()
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
}

impl TargetMachine {
  pub fn describe(&self) -> String {
    format!(
      "Triple={} CPU={} features={} layout={}",
      self.triple,
      self.cpu,
      self.features,
      self.data_layout.description
    )
  }
}
