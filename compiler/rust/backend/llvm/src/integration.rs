use crate::codegen::{CodegenContext, GeneratedFunction, MirFunction};
use crate::ffi_lowering::FfiCallSignature;
use crate::target_machine::{
    CodeModel, DataLayoutSpec, OptimizationLevel, RelocModel, TargetMachine, TargetMachineBuilder,
    Triple, WindowsToolchainConfig,
};
use crate::type_mapping::RemlType;
use crate::verify::Verifier;
use serde::Deserialize;
use std::error::Error;
use std::{fmt, fs::File, io, path::Path};

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

/// モジュール全体と MIR 関数の構造を JSON から読み込む。
#[derive(Debug, Deserialize)]
struct MirModuleSpec {
    module: Option<String>,
    #[serde(default)]
    metadata: Vec<String>,
    #[serde(default)]
    runtime_symbols: Vec<String>,
    functions: Vec<MirFunctionJson>,
}

impl MirModuleSpec {
    fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, MirSnapshotError> {
        let file = File::open(path)?;
        let spec = serde_json::from_reader(file)?;
        Ok(spec)
    }

    fn into_functions(self) -> Vec<MirFunction> {
        self.functions
            .into_iter()
            .map(MirFunctionJson::into_mir)
            .collect()
    }
}

/// 単体 MIR 関数の JSON 表現。
#[derive(Debug, Deserialize)]
struct MirFunctionJson {
    name: String,
    calling_conv: String,
    #[serde(default)]
    params: Vec<String>,
    #[serde(alias = "return")]
    return_type: Option<String>,
    #[serde(default)]
    attributes: Vec<String>,
    #[serde(default)]
    ffi_calls: Vec<FfiCallJson>,
}

impl MirFunctionJson {
    fn into_mir(self) -> MirFunction {
        let mut builder = MirFunction::new(self.name, self.calling_conv);
        for param in self.params {
            builder = builder.with_param(parse_reml_type(&param));
        }
        if let Some(ret) = self.return_type {
            builder = builder.with_return(parse_reml_type(&ret));
        }
        for attr in self.attributes {
            builder = builder.with_attribute(attr);
        }
        for ffi in self.ffi_calls {
            builder = builder.with_ffi_call(ffi.into_signature());
        }
        builder
    }
}

/// FFI 呼び出しの JSON 抽象。
#[derive(Debug, Deserialize)]
struct FfiCallJson {
    name: String,
    calling_conv: String,
    #[serde(default)]
    args: Vec<String>,
    #[serde(alias = "return")]
    ret: Option<String>,
}

impl FfiCallJson {
    fn into_signature(self) -> FfiCallSignature {
        FfiCallSignature {
            name: self.name,
            calling_conv: self.calling_conv,
            args: self
                .args
                .into_iter()
                .map(|arg| parse_reml_type(&arg))
                .collect(),
            ret: self.ret.map(|ret| parse_reml_type(&ret)),
        }
    }
}

/// MIR JSON ロード/差分生成で発生するエラー。
#[derive(Debug)]
pub enum MirSnapshotError {
    Io(io::Error),
    Json(serde_json::Error),
}

impl fmt::Display for MirSnapshotError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MirSnapshotError::Io(err) => write!(f, "I/O エラー: {}", err),
            MirSnapshotError::Json(err) => write!(f, "JSON パースエラー: {}", err),
        }
    }
}

impl Error for MirSnapshotError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            MirSnapshotError::Io(err) => Some(err),
            MirSnapshotError::Json(err) => Some(err),
        }
    }
}

impl From<io::Error> for MirSnapshotError {
    fn from(err: io::Error) -> Self {
        MirSnapshotError::Io(err)
    }
}

impl From<serde_json::Error> for MirSnapshotError {
    fn from(err: serde_json::Error) -> Self {
        MirSnapshotError::Json(err)
    }
}

/// 生成した MIR 関数リストから差分スナップショットを生成する。
pub fn generate_snapshot(
    module_name: impl Into<String>,
    target_machine: TargetMachine,
    runtime_symbols: Vec<String>,
    metadata: Vec<String>,
    functions: Vec<MirFunction>,
) -> BackendDiffSnapshot {
    let module_name = module_name.into();
    let mut codegen = CodegenContext::new(target_machine.clone(), runtime_symbols);
    metadata
        .into_iter()
        .for_each(|entry| codegen.with_metadata(entry));
    for function in &functions {
        codegen.emit_function(function);
    }
    let module = codegen.finish_module(module_name.clone());
    let verification = Verifier::new().verify_module(&module);
    BackendDiffSnapshot {
        module_name,
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

/// MIR JSON から差分スナップショットを生成する補助。
pub fn generate_snapshot_from_mir_json<P: AsRef<Path>>(
    path: P,
    target_machine: TargetMachine,
    runtime_symbols: Vec<String>,
    metadata: Vec<String>,
    default_module_name: impl Into<String>,
) -> Result<BackendDiffSnapshot, MirSnapshotError> {
    let module_default = default_module_name.into();
    let spec = MirModuleSpec::from_file(path)?;
    let module_name = spec.module.unwrap_or_else(|| module_default.clone());
    let mut runtime_symbols = runtime_symbols;
    runtime_symbols.extend(spec.runtime_symbols);
    let mut metadata = metadata;
    metadata.extend(spec.metadata);
    let functions = spec.into_functions();
    Ok(generate_snapshot(
        module_name,
        target_machine,
        runtime_symbols,
        metadata,
        functions,
    ))
}

/// JSON ファイルから MIR 関数リストをロードする。
pub fn load_mir_functions_from_json<P: AsRef<Path>>(
    path: P,
) -> Result<Vec<MirFunction>, MirSnapshotError> {
    let spec = MirModuleSpec::from_file(path)?;
    Ok(spec.into_functions())
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

fn parse_reml_type(token: &str) -> RemlType {
    let normalized = token.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "bool" => RemlType::Bool,
        "i32" | "int32" => RemlType::I32,
        "i64" | "int64" => RemlType::I64,
        "f64" | "double" => RemlType::F64,
        "pointer" | "ptr" | "i8*" => RemlType::Pointer,
        "string" | "str" | "&str" => RemlType::String,
        _ => RemlType::Pointer,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        generate_snapshot_from_mir_json, load_mir_functions_from_json, parse_reml_type,
        MirSnapshotError,
    };
    use crate::target_machine::{
        CodeModel, DataLayoutSpec, OptimizationLevel, RelocModel, TargetMachineBuilder, Triple,
        WindowsToolchainConfig,
    };
    use crate::type_mapping::RemlType;
    use std::{env, fs};

    #[test]
    fn parse_reml_type_synonyms() {
        assert_eq!(parse_reml_type("i32"), RemlType::I32);
        assert_eq!(parse_reml_type("Int64"), RemlType::I64);
        assert_eq!(parse_reml_type("ptr"), RemlType::Pointer);
        assert_eq!(parse_reml_type("unknown"), RemlType::Pointer);
    }

    #[test]
    fn snapshot_from_json_file() -> Result<(), MirSnapshotError> {
        let spec = r#"
    {
      "module": "json_module",
      "metadata": ["phase=json"],
      "functions": [
        {
          "name": "@json_main",
          "calling_conv": "ccc",
          "params": ["pointer", "i64"],
          "return": "i32",
          "attributes": ["nounwind"],
          "ffi_calls": [
            {"name": "panic", "calling_conv": "ccc", "args": ["string"], "return": null}
          ]
        }
      ]
    }
    "#;
        let tmp = env::temp_dir().join("reml_mir_test.json");
        fs::write(&tmp, spec)?;
        let windows_toolchain = WindowsToolchainConfig {
            toolchain_name: "test-llvm".into(),
            llc_path: "llc".into(),
            opt_path: "opt".into(),
        };
        let target_machine = TargetMachineBuilder::new()
            .with_triple(Triple::LinuxGNU)
            .with_relocation_model(RelocModel::Static)
            .with_code_model(CodeModel::Small)
            .with_optimization_level(OptimizationLevel::O1)
            .with_data_layout(DataLayoutSpec::new("e-m:e-p:64:64-f64:64:64-a:0:64"))
            .with_windows_toolchain(windows_toolchain.clone())
            .build();
        let snapshot = generate_snapshot_from_mir_json(
            &tmp,
            target_machine,
            vec!["mem_alloc".into()],
            vec!["runtime=json".into()],
            "json_module",
        )?;
        assert_eq!(snapshot.module_name, "json_module");
        assert!(snapshot.passed);
        fs::remove_file(tmp)?;
        Ok(())
    }

    #[test]
    fn load_functions_from_json_file() -> Result<(), MirSnapshotError> {
        let spec = r#"
    {
      "functions": [
        {"name": "@json_main", "calling_conv": "ccc"}
      ]
    }
    "#;
        let tmp = env::temp_dir().join("reml_mir_list.json");
        fs::write(&tmp, spec)?;
        let functions = load_mir_functions_from_json(&tmp)?;
        assert_eq!(functions.len(), 1);
        assert_eq!(functions[0].name, "@json_main");
        fs::remove_file(tmp)?;
        Ok(())
    }
}
