use std::collections::HashMap;

use crate::codegen::ModuleIr;

/// 単一診断レコード。
#[derive(Clone, Debug)]
pub struct Diagnostic {
  pub domain: String,
  pub code: String,
  pub message: String,
  pub extensions: HashMap<String, String>,
}

impl Diagnostic {
  pub fn new(domain: impl Into<String>, code: impl Into<String>, message: impl Into<String>) -> Self {
    Self {
      domain: domain.into(),
      code: code.into(),
      message: message.into(),
      extensions: HashMap::new(),
    }
  }

  pub fn with_extension(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
    self.extensions.insert(key.into(), value.into());
    self
  }
}

/// 監査ログの一要素。
#[derive(Clone, Debug)]
pub struct AuditEntry {
  pub key: String,
  pub value: String,
}

impl AuditEntry {
  pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
    Self {
      key: key.into(),
      value: value.into(),
    }
  }
}

/// 監査ログ。
#[derive(Clone, Debug)]
pub struct AuditLog {
  pub entries: Vec<AuditEntry>,
}

impl AuditLog {
  pub fn new() -> Self {
    Self { entries: Vec::new() }
  }

  pub fn record(&mut self, key: impl Into<String>, value: impl Into<String>) {
    self.entries.push(AuditEntry::new(key, value));
  }
}

/// 検証結果。
#[derive(Clone, Debug)]
pub struct VerificationResult {
  pub passed: bool,
  pub diagnostics: Vec<Diagnostic>,
  pub audit_log: AuditLog,
}

/// LLVM バックエンドの検証を担当する構造。
#[derive(Clone, Debug, Default)]
pub struct Verifier;

impl Verifier {
  pub fn new() -> Self {
    Self
  }

  pub fn verify_module(&self, module: &ModuleIr) -> VerificationResult {
    let mut diagnostics = Vec::new();
    if module.functions.is_empty() {
      diagnostics.push(
        Diagnostic::new(
          "Backend",
          "llvm.module.empty",
          "LLVM モジュールに関数が含まれていません。",
        )
        .with_extension("backend", "rust"),
      );
    }
    if module.target.data_layout.description.is_empty() {
      diagnostics.push(
        Diagnostic::new(
          "Backend",
          "target.datalayout.missing",
          "TargetMachine の DataLayout が不正です。",
        )
        .with_extension("backend", "rust"),
      );
    }
    for func in &module.functions {
      if func.layout.size == 0 && func.layout.description != "void" {
        diagnostics.push(
          Diagnostic::new(
            "Backend",
            "type.layout.invalid",
            format!("関数 {} のレイアウトが不正です。", func.name),
          )
          .with_extension("backend", "rust"),
        );
      }
    }
    let mut audit = AuditLog::new();
    audit.record(
      "audit.source",
      format!("opt.verify {}", module.name),
    );
    audit.record("audit.target", module.target.describe());
    audit.record("audit.module", module.name.clone());
    audit.record("audit.verdict", if diagnostics.is_empty() { "pass" } else { "fail" });
    if let Some(toolchain) = &module.windows_toolchain {
      audit.record("audit.toolchain", &toolchain.toolchain_name);
      audit.record("audit.llc_path", &toolchain.llc_path);
      audit.record("audit.opt_path", &toolchain.opt_path);
    }
    VerificationResult {
      passed: diagnostics.is_empty(),
      diagnostics,
      audit_log: audit,
    }
  }
}
