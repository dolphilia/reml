use crate::codegen::{CodegenContext, GeneratedFunction, MirFunction};
use crate::ffi_lowering::FfiCallSignature;
use crate::target_machine::{
    CodeModel, DataLayoutSpec, OptimizationLevel, RelocModel, TargetMachineBuilder, Triple,
    WindowsToolchainConfig,
};
use crate::type_mapping::RemlType;
use crate::verify::Verifier;

/// 生成関数の差分ログ用レコード。
#[derive(Clone, Debug)]
pub struct BackendFunctionRecord {
    pub name: String,
    pub return_layout: String,
    pub calling_conv: String,
    pub attributes: Vec<String>,
    pub lowered_calls: Vec<String>,
}

impl BackendFunctionRecord {
    fn from_generated(func: &GeneratedFunction) -> Self {
        Self {
            name: func.name.clone(),
            return_layout: func.layout.description.clone(),
            calling_conv: func.calling_conv.clone(),
            attributes: func.attributes.clone(),
            lowered_calls: func
                .lowered_calls
                .iter()
                .map(|call| call.describe())
                .collect(),
        }
    }
}

/// W3 デモ用の差分スナップショット。
#[derive(Clone, Debug)]
pub struct BackendDiffSnapshot {
    pub module_name: String,
    pub target_triple: String,
    pub data_layout: String,
    pub windows_toolchain: Option<String>,
    pub functions: Vec<BackendFunctionRecord>,
    pub diagnostics: Vec<String>,
    pub audit_entries: Vec<String>,
    pub passed: bool,
}

impl BackendDiffSnapshot {
    fn quote(value: &str) -> String {
        value.replace('\\', "\\\\").replace('"', "\\\"")
    }

    fn array_of_strings(values: &[String], indent: &str) -> String {
        let mut buf = String::new();
        buf.push('[');
        if !values.is_empty() {
            buf.push('\n');
            for (idx, value) in values.iter().enumerate() {
                buf.push_str(indent);
                buf.push_str("  \"");
                buf.push_str(&Self::quote(value));
                buf.push('"');
                if idx + 1 != values.len() {
                    buf.push(',');
                }
                buf.push('\n');
            }
            buf.push_str(indent);
        }
        buf.push(']');
        buf
    }

    fn function_record_json(&self, record: &BackendFunctionRecord, indent: &str) -> String {
        let mut buf = String::new();
        buf.push_str("{\n");
        buf.push_str(indent);
        buf.push_str("  \"name\": \"");
        buf.push_str(&Self::quote(&record.name));
        buf.push_str("\",\n");
        buf.push_str(indent);
        buf.push_str("  \"return_layout\": \"");
        buf.push_str(&Self::quote(&record.return_layout));
        buf.push_str("\",\n");
        buf.push_str(indent);
        buf.push_str("  \"calling_conv\": \"");
        buf.push_str(&Self::quote(&record.calling_conv));
        buf.push_str("\",\n");
        buf.push_str(indent);
        buf.push_str("  \"attributes\": ");
        buf.push_str(&Self::array_of_strings(
            &record.attributes,
            &(indent.to_string() + "  "),
        ));
        buf.push_str(",\n");
        buf.push_str(indent);
        buf.push_str("  \"ffi_calls\": ");
        buf.push_str(&Self::array_of_strings(
            &record.lowered_calls,
            &(indent.to_string() + "  "),
        ));
        buf.push('\n');
        buf.push_str(indent);
        buf.push('}');
        buf
    }

    /// JSON 形式のログを返す。
    pub fn to_pretty_json(&self) -> String {
        let mut buf = String::new();
        buf.push_str("{\n");
        buf.push_str("  \"module\": \"");
        buf.push_str(&Self::quote(&self.module_name));
        buf.push_str("\",\n");
        buf.push_str("  \"target_triple\": \"");
        buf.push_str(&Self::quote(&self.target_triple));
        buf.push_str("\",\n");
        buf.push_str("  \"data_layout\": \"");
        buf.push_str(&Self::quote(&self.data_layout));
        buf.push_str("\",\n");
        if let Some(toolchain) = &self.windows_toolchain {
            buf.push_str("  \"windows_toolchain\": \"");
            buf.push_str(&Self::quote(toolchain));
            buf.push_str("\",\n");
        }
        buf.push_str("  \"functions\": [\n");
        for (index, function) in self.functions.iter().enumerate() {
            buf.push_str("    ");
            buf.push_str(&self.function_record_json(function, "    "));
            if index + 1 != self.functions.len() {
                buf.push(',');
            }
            buf.push('\n');
        }
        buf.push_str("  ],\n");
        buf.push_str("  \"diagnostics\": ");
        buf.push_str(&Self::array_of_strings(&self.diagnostics, "  "));
        buf.push_str(",\n");
        buf.push_str("  \"audit_entries\": ");
        buf.push_str(&Self::array_of_strings(&self.audit_entries, "  "));
        buf.push_str(",\n");
        buf.push_str("  \"passed\": ");
        buf.push_str(if self.passed { "true" } else { "false" });
        buf.push('\n');
        buf.push('}');
        buf
    }
}

/// W3 相当の差分スナップショットを生成する。
pub fn generate_w3_snapshot() -> BackendDiffSnapshot {
    let windows_toolchain = WindowsToolchainConfig {
        toolchain_name: "msvc-llvm-19.1.1".into(),
        llc_path: "C:\\llvm-19.1.1\\bin\\llc.exe".into(),
        opt_path: "C:\\llvm-19.1.1\\bin\\opt.exe".into(),
    };
    let target_machine = TargetMachineBuilder::new()
        .with_triple(Triple::WindowsMSVC)
        .with_cpu("x86-64")
        .with_features("+sse4.2,+popcnt")
        .with_relocation_model(RelocModel::Static)
        .with_code_model(CodeModel::Large)
        .with_optimization_level(OptimizationLevel::O2)
        .with_data_layout(DataLayoutSpec::new(
            "e-m:w-p:64:64-f64:64:64-v128:128:128-a:0:64",
        ))
        .with_windows_toolchain(windows_toolchain.clone())
        .build();

    let mut codegen = CodegenContext::new(
        target_machine.clone(),
        vec![
            "mem_alloc".into(),
            "inc_ref".into(),
            "dec_ref".into(),
            "panic".into(),
        ],
    );
    codegen.with_metadata("phase=W3");
    codegen.with_metadata("runtime=llvm");

    let entry = MirFunction::new("@k__main", "ccc")
        .with_param(RemlType::Pointer)
        .with_param(RemlType::I64)
        .with_return(RemlType::I32)
        .with_attribute("nounwind")
        .with_attribute("uwtable")
        .with_ffi_call(FfiCallSignature {
            name: "mem_alloc".into(),
            calling_conv: "ccc".into(),
            args: vec![RemlType::I64],
            ret: Some(RemlType::Pointer),
        })
        .with_ffi_call(FfiCallSignature {
            name: "panic".into(),
            calling_conv: "ccc".into(),
            args: vec![RemlType::String],
            ret: None,
        });

    let _ = codegen.emit_function(&entry);
    let module = codegen.finish_module("reml_backend_module");
    let verification = Verifier::new().verify_module(&module);

    BackendDiffSnapshot {
        module_name: module.name.clone(),
        target_triple: module.target.triple.to_string(),
        data_layout: module.target.data_layout.description.clone(),
        windows_toolchain: module
            .windows_toolchain
            .as_ref()
            .map(|cfg| cfg.toolchain_name.clone()),
        functions: module
            .functions
            .iter()
            .map(BackendFunctionRecord::from_generated)
            .collect(),
        diagnostics: verification
            .diagnostics
            .into_iter()
            .map(|diag| format!("{}.{}: {}", diag.domain, diag.code, diag.message))
            .collect(),
        audit_entries: verification
            .audit_log
            .entries
            .into_iter()
            .map(|entry| format!("{}={}", entry.key, entry.value))
            .collect(),
        passed: verification.passed,
    }
}

#[cfg(test)]
mod tests {
    use super::generate_w3_snapshot;

    #[test]
    fn snapshot_should_pass_verification() {
        let snapshot = generate_w3_snapshot();
        assert_eq!(snapshot.module_name, "reml_backend_module");
        assert!(snapshot.passed);
        assert_eq!(snapshot.functions.len(), 1);
        assert!(snapshot.diagnostics.is_empty());
        assert!(snapshot
            .audit_entries
            .iter()
            .any(|entry| entry.starts_with("audit.verdict")));
    }

    #[test]
    #[ignore = "ログ生成時のみ使用"]
    fn dump_snapshot_json() {
        let snapshot = generate_w3_snapshot();
        println!("{}", snapshot.to_pretty_json());
        assert!(snapshot.passed);
    }
}
