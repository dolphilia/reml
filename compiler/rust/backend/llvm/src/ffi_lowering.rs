use crate::type_mapping::{RemlType, TypeLayout, TypeMappingContext};

/// FFI 呼び出しの署名を表す構造。
#[derive(Clone, Debug)]
pub struct FfiCallSignature {
  pub name: String,
  pub calling_conv: String,
  pub args: Vec<RemlType>,
  pub ret: Option<RemlType>,
}

/// Lowered FFI 呼び出しの簡易表現。
#[derive(Clone, Debug)]
pub struct LoweredFfiCall {
  pub signature: String,
  pub lowered_type: TypeLayout,
}

/// RC / panic などを含む FFI 境界のロワリング。
#[derive(Clone, Debug)]
pub struct FfiLowering {
  type_mapping: TypeMappingContext,
  runtime_symbols: Vec<String>,
}

impl FfiLowering {
  pub fn new(type_mapping: TypeMappingContext, runtime_symbols: Vec<String>) -> Self {
    Self { type_mapping, runtime_symbols }
  }

  pub fn lower_call(&self, sig: &FfiCallSignature) -> LoweredFfiCall {
    let layout = sig
      .ret
      .as_ref()
      .map(|ty| self.type_mapping.layout_of(ty))
      .unwrap_or_else(|| TypeLayout { size: 0, align: 1, description: "void".into() });
    LoweredFfiCall {
      signature: format!("{}::{}", sig.calling_conv, sig.name),
      lowered_type: layout,
    }
  }

  pub fn runtime_symbol_list(&self) -> &[String] {
    &self.runtime_symbols
  }
}
