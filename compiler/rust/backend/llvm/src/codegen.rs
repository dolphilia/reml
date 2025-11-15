use std::fmt::Write;

use crate::ffi_lowering::{FfiCallSignature, FfiLowering, LoweredFfiCall};
use crate::target_machine::{TargetMachine, WindowsToolchainConfig};
use crate::type_mapping::{RemlType, TypeLayout, TypeMappingContext};

/// ミニマルな MIR 関数表現。
#[derive(Clone, Debug)]
pub struct MirFunction {
    pub name: String,
    pub params: Vec<RemlType>,
    pub ret: Option<RemlType>,
    pub calling_conv: String,
    pub attributes: Vec<String>,
    pub ffi_calls: Vec<FfiCallSignature>,
}

impl MirFunction {
    pub fn new(name: impl Into<String>, calling_conv: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            params: Vec::new(),
            ret: None,
            calling_conv: calling_conv.into(),
            attributes: Vec::new(),
            ffi_calls: Vec::new(),
        }
    }

    pub fn describe(&self) -> String {
        let mut buf = String::new();
        writeln!(
            &mut buf,
            "fn {}({}) -> {:?} [{}]",
            self.name,
            self.params
                .iter()
                .enumerate()
                .map(|(i, ty)| format!("arg{}:{:?}", i, ty))
                .collect::<Vec<_>>()
                .join(", "),
            self.ret,
            self.calling_conv
        )
        .ok();
        buf
    }

    pub fn with_param(mut self, ty: RemlType) -> Self {
        self.params.push(ty);
        self
    }

    pub fn with_return(mut self, ret: RemlType) -> Self {
        self.ret = Some(ret);
        self
    }

    pub fn with_attribute(mut self, attr: impl Into<String>) -> Self {
        self.attributes.push(attr.into());
        self
    }

    pub fn with_ffi_call(mut self, call: FfiCallSignature) -> Self {
        self.ffi_calls.push(call);
        self
    }
}

/// 生成された関数の LLVM 風表現。
#[derive(Clone, Debug)]
pub struct GeneratedFunction {
    pub name: String,
    pub layout: TypeLayout,
    pub calling_conv: String,
    pub attributes: Vec<String>,
    pub lowered_calls: Vec<LoweredFfiCall>,
}

impl GeneratedFunction {
    pub fn describe(&self) -> String {
        format!(
            "{} -> {} via {} {:?}",
            self.name, self.layout.description, self.calling_conv, self.attributes
        )
    }
}

/// LLVM 風モジュール IR。
#[derive(Clone, Debug)]
pub struct ModuleIr {
    pub name: String,
    pub target: TargetMachine,
    pub functions: Vec<GeneratedFunction>,
    pub metadata: Vec<String>,
    pub windows_toolchain: Option<WindowsToolchainConfig>,
}

impl ModuleIr {
    pub fn describe(&self) -> String {
        let mut summary = Vec::new();
        summary.push(format!(
            "module {} (target: {})",
            self.name,
            self.target.describe()
        ));
        if let Some(toolchain) = &self.windows_toolchain {
            summary.push(format!("windows_toolchain({})", toolchain.toolchain_name));
        }
        summary.push(format!("functions: {}", self.functions.len()));
        self.metadata
            .iter()
            .cloned()
            .for_each(|item| summary.push(item));
        summary.join(" | ")
    }
}

/// CodegenContext は MIR → LLVM IR の変換責務を担う。
pub struct CodegenContext {
    target_machine: TargetMachine,
    type_mapping: TypeMappingContext,
    ffi_lowering: FfiLowering,
    functions: Vec<GeneratedFunction>,
    module_metadata: Vec<String>,
}

impl CodegenContext {
    pub fn new(target_machine: TargetMachine, runtime_symbols: Vec<String>) -> Self {
        let layout = target_machine.data_layout.clone();
        Self {
            type_mapping: TypeMappingContext::new(layout),
            ffi_lowering: FfiLowering::new(
                TypeMappingContext::new(target_machine.data_layout.clone()),
                runtime_symbols,
            ),
            target_machine,
            functions: Vec::new(),
            module_metadata: Vec::new(),
        }
    }

    pub fn describe(&self) -> String {
        format!(
            "codegen(target={}, functions={})",
            self.target_machine.describe(),
            self.functions.len()
        )
    }

    pub fn target_machine(&self) -> &TargetMachine {
        &self.target_machine
    }

    pub fn emit_function(&mut self, mir: &MirFunction) -> GeneratedFunction {
        let ret_layout = mir
            .ret
            .as_ref()
            .map(|ty| self.type_mapping.layout_of(ty))
            .unwrap_or_else(|| TypeLayout {
                size: 0,
                align: 1,
                description: "void".into(),
            });
        let lowered_calls = mir
            .ffi_calls
            .iter()
            .map(|sig| self.ffi_lowering.lower_call(sig))
            .collect();
        let generated = GeneratedFunction {
            name: mir.name.clone(),
            layout: ret_layout,
            calling_conv: mir.calling_conv.clone(),
            attributes: mir.attributes.clone(),
            lowered_calls,
        };
        self.functions.push(generated.clone());
        generated
    }

    pub fn with_metadata(&mut self, entry: impl Into<String>) {
        self.module_metadata.push(entry.into());
    }

    pub fn finish_module(self, name: impl Into<String>) -> ModuleIr {
        ModuleIr {
            name: name.into(),
            target: self.target_machine.clone(),
            functions: self.functions,
            metadata: self.module_metadata,
            windows_toolchain: self.target_machine.windows_toolchain.clone(),
        }
    }
}
