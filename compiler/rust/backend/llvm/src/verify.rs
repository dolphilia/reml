use std::collections::HashMap;

use crate::codegen::ModuleIr;
use crate::intrinsics::IntrinsicStatus;
use crate::target_diagnostics::TargetDiagnosticEmitter;
use crate::unstable::UnstableStatus;
use serde_json::Value;

/// 単一診断レコード。
#[derive(Clone, Debug)]
pub struct Diagnostic {
    pub domain: String,
    pub code: String,
    pub message: String,
    pub extensions: HashMap<String, String>,
}

impl Diagnostic {
    pub fn new(
        domain: impl Into<String>,
        code: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
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
        Self {
            entries: Vec::new(),
        }
    }

    pub fn record(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.entries.push(AuditEntry::new(key, value));
    }

    pub fn record_value(&mut self, key: impl Into<String>, value: &Value) {
        let serialized = serde_json::to_string(value).unwrap_or_else(|_| format!("{:?}", value));
        self.record(key, serialized);
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
        let target_report = TargetDiagnosticEmitter::new(module.target_context.clone()).emit();
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
        for target_diag in target_report.diagnostics {
            diagnostics.push(
                Diagnostic::new("Target", target_diag.code, target_diag.message)
                    .with_extension(
                        "target",
                        serde_json::to_string(&target_diag.extension)
                            .unwrap_or_else(|_| format!("{:?}", target_diag.extension)),
                    )
                    .with_extension("backend", "rust"),
            );
        }
        let mut audit = AuditLog::new();
        audit.record("audit.source", format!("opt.verify {}", module.name));
        audit.record("backend.triple", module.target.triple.to_string());
        audit.record("backend.abi", module.target.backend_abi().to_string());
        audit.record(
            "backend.datalayout",
            module.target.data_layout.description.clone(),
        );
        audit.record(
            "backend.reloc_model",
            format!("{:?}", module.target.reloc_model),
        );
        audit.record("audit.target", module.target.describe());
        audit.record("audit.module", module.name.clone());
        for entry in target_report.audit {
            audit.record_value(entry.key, &entry.value);
        }
        for (key, value) in module.bridge_metadata.audit_entries() {
            audit.record(key, value);
        }
        for intrinsic in &module.intrinsic_uses {
            audit.record("native.intrinsic.used", intrinsic.name.clone());
            audit.record("intrinsic.name", intrinsic.name.clone());
            audit.record("intrinsic.signature", intrinsic.signature.render());
            if intrinsic.status == IntrinsicStatus::Polyfill {
                audit.record("native.intrinsic.polyfill", intrinsic.name.clone());
            }
            if intrinsic.status == IntrinsicStatus::SignatureMismatch {
                let mut diagnostic = Diagnostic::new(
                    "Native",
                    "native.intrinsic.signature_mismatch",
                    format!(
                        "intrinsic `{}` のシグネチャが一致しません。",
                        intrinsic.name
                    ),
                )
                .with_extension("intrinsic.name", intrinsic.name.clone())
                .with_extension("intrinsic.signature", intrinsic.signature.render());
                if let Some(expected) = intrinsic.expected.as_ref() {
                    diagnostic = diagnostic.with_extension("intrinsic.expected", expected.render());
                }
                diagnostics.push(diagnostic);
            }
        }
        for unstable in &module.unstable_uses {
            audit.record("native.intrinsic.unstable_used", unstable.describe());
            if unstable.status == UnstableStatus::Disabled {
                diagnostics.push(
                    Diagnostic::new(
                        "Native",
                        "native.unstable.disabled",
                        format!(
                            "unstable 機能 `{}` は feature \"native-unstable\" が必要です。",
                            unstable.describe()
                        ),
                    )
                    .with_extension("unstable.function", unstable.function.clone())
                    .with_extension("unstable.kind", unstable.kind.as_label()),
                );
            }
        }
        for inline_asm in &module.inline_asm_uses {
            audit.record_value("native.inline_asm.used", &Value::Bool(true));
            audit.record("asm.template_hash", inline_asm.template_hash.clone());
            let constraint_values = Value::Array(
                inline_asm
                    .constraints
                    .iter()
                    .cloned()
                    .map(Value::String)
                    .collect(),
            );
            audit.record_value("asm.constraints", &constraint_values);
            if inline_asm
                .constraints
                .iter()
                .any(|constraint| !is_inline_asm_constraint_valid(constraint))
            {
                diagnostics.push(
                    Diagnostic::new(
                        "Native",
                        "native.inline_asm.invalid_constraint",
                        "Inline ASM の制約文字列が無効です。",
                    )
                    .with_extension("asm.template_hash", inline_asm.template_hash.clone())
                    .with_extension(
                        "asm.constraints",
                        serde_json::to_string(&inline_asm.constraints)
                            .unwrap_or_else(|_| format!("{:?}", inline_asm.constraints)),
                    ),
                );
            }
        }
        for llvm_ir in &module.llvm_ir_uses {
            audit.record_value("native.llvm_ir.used", &Value::Bool(true));
            audit.record("llvm_ir.template_hash", llvm_ir.template_hash.clone());
            let input_values = Value::Array(
                llvm_ir
                    .inputs
                    .iter()
                    .cloned()
                    .map(Value::String)
                    .collect(),
            );
            audit.record_value("llvm_ir.inputs", &input_values);
            if !llvm_ir.invalid_placeholders.is_empty() {
                diagnostics.push(
                    Diagnostic::new(
                        "Native",
                        "native.llvm_ir.invalid_placeholder",
                        "LLVM IR のプレースホルダが inputs と一致しません。",
                    )
                    .with_extension("llvm_ir.template_hash", llvm_ir.template_hash.clone())
                    .with_extension(
                        "llvm_ir.inputs",
                        serde_json::to_string(&llvm_ir.inputs)
                            .unwrap_or_else(|_| format!("{:?}", llvm_ir.inputs)),
                    ),
                );
            }
            if !llvm_ir.has_result && !is_void_llvm_ir_result(&llvm_ir.result_type) {
                diagnostics.push(
                    Diagnostic::new(
                        "Native",
                        "native.llvm_ir.verify_failed",
                        "LLVM IR テンプレートから結果 SSA を取得できません。",
                    )
                    .with_extension("llvm_ir.template_hash", llvm_ir.template_hash.clone())
                    .with_extension(
                        "llvm_ir.inputs",
                        serde_json::to_string(&llvm_ir.inputs)
                            .unwrap_or_else(|_| format!("{:?}", llvm_ir.inputs)),
                    ),
                );
            }
        }
        audit.record(
            "audit.verdict",
            if diagnostics.is_empty() {
                "pass"
            } else {
                "fail"
            },
        );
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

fn is_inline_asm_constraint_valid(constraint: &str) -> bool {
    let trimmed = constraint.trim();
    if trimmed.is_empty() {
        return false;
    }
    if trimmed.chars().any(|ch| ch.is_whitespace() || ch == ',') {
        return false;
    }
    true
}

fn is_void_llvm_ir_result(result_type: &str) -> bool {
    matches!(
        result_type.trim().to_ascii_lowercase().as_str(),
        "void" | "unit"
    )
}
