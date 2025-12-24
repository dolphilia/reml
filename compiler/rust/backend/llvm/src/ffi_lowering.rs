use crate::target_machine::Triple;
use crate::type_mapping::{RemlType, TypeLayout, TypeMappingContext};

/// FFI 呼び出しの署名を表す構造。
#[derive(Clone, Debug)]
pub struct FfiCallSignature {
    pub name: String,
    pub calling_conv: String,
    pub args: Vec<RemlType>,
    pub ret: Option<RemlType>,
    pub variadic: bool,
}

/// Register Save Area 情報。
#[derive(Clone, Debug)]
pub struct RegisterSaveArea {
    pub gpr_count: u32,
    pub gpr_slot_size: u32,
    pub gpr_total_size: u32,
    pub vector_count: u32,
    pub vector_slot_size: u32,
    pub vector_total_size: u32,
    pub stack_alignment: u32,
}

/// Stub plan から生成される監査タグのベース情報。
#[derive(Clone, Debug)]
pub struct FfiStubPlan {
    pub extern_name: String,
    pub target_triple: String,
    pub platform: String,
    pub arch: String,
    pub callconv: String,
    pub abi: String,
    pub ownership: String,
    pub register_save_area: Option<RegisterSaveArea>,
}

impl FfiStubPlan {
    pub(crate) fn register_save_area_tags(&self) -> Vec<(String, String)> {
        let mut tags = Vec::new();
        if let Some(area) = &self.register_save_area {
            tags.push((
                "bridge.darwin.register_save_area.general.count".into(),
                area.gpr_count.to_string(),
            ));
            tags.push((
                "bridge.darwin.register_save_area.general.slot_size".into(),
                area.gpr_slot_size.to_string(),
            ));
            tags.push((
                "bridge.darwin.register_save_area.general.total_size".into(),
                area.gpr_total_size.to_string(),
            ));
            tags.push((
                "bridge.darwin.register_save_area.vector.count".into(),
                area.vector_count.to_string(),
            ));
            tags.push((
                "bridge.darwin.register_save_area.vector.slot_size".into(),
                area.vector_slot_size.to_string(),
            ));
            tags.push((
                "bridge.darwin.register_save_area.vector.total_size".into(),
                area.vector_total_size.to_string(),
            ));
            tags.push((
                "bridge.darwin.register_save_area.alignment".into(),
                area.stack_alignment.to_string(),
            ));
        }
        tags
    }

    pub fn audit_tags(&self) -> Vec<(String, String)> {
        let mut tags = vec![
            ("bridge.platform".into(), self.platform.clone()),
            ("bridge.target".into(), self.target_triple.clone()),
            ("bridge.arch".into(), self.arch.clone()),
            ("bridge.callconv".into(), self.callconv.clone()),
            ("bridge.abi".into(), self.abi.clone()),
            ("bridge.ownership".into(), self.ownership.clone()),
        ];
        tags.extend(self.register_save_area_tags());
        tags
    }
}

/// Lowered FFI 呼び出しの簡易表現。
#[derive(Clone, Debug)]
pub struct LoweredFfiCall {
    pub signature: String,
    pub lowered_type: TypeLayout,
    pub stub_plan: FfiStubPlan,
    pub audit_tags: Vec<(String, String)>,
}

impl LoweredFfiCall {
    pub fn describe(&self) -> String {
        format!(
            "{} -> {} via {} [{}/{}]",
            self.signature,
            self.lowered_type.description,
            self.stub_plan.callconv,
            self.stub_plan.platform,
            self.stub_plan.arch,
        )
    }
}

/// RC / panic などを含む FFI 境界のロワリング。
#[derive(Clone, Debug)]
pub struct FfiLowering {
    type_mapping: TypeMappingContext,
    runtime_symbols: Vec<String>,
    target_triple: Triple,
    platform_label: String,
    arch_label: String,
    backend_abi: String,
    ownership: String,
}

impl FfiLowering {
    pub fn new(
        type_mapping: TypeMappingContext,
        runtime_symbols: Vec<String>,
        target_triple: Triple,
        backend_abi: impl Into<String>,
    ) -> Self {
        let platform_label = target_triple.platform_label().into();
        let arch_label = target_triple.canonical_arch().into();
        Self {
            type_mapping,
            runtime_symbols,
            target_triple,
            platform_label,
            arch_label,
            backend_abi: backend_abi.into(),
            ownership: "borrowed".into(),
        }
    }

    pub fn lower_call(&self, sig: &FfiCallSignature) -> LoweredFfiCall {
        let layout = sig
            .ret
            .as_ref()
            .map(|ty| self.type_mapping.layout_of(ty))
            .unwrap_or_else(|| TypeLayout {
                size: 0,
                align: 1,
                description: "void".into(),
            });
        let stub_plan = self.build_stub_plan(sig);
        let audit_tags = stub_plan.audit_tags();
        LoweredFfiCall {
            signature: format!("{}::{}", sig.calling_conv, sig.name),
            lowered_type: layout,
            stub_plan,
            audit_tags,
        }
    }

    pub fn runtime_symbol_list(&self) -> &[String] {
        &self.runtime_symbols
    }

    fn build_stub_plan(&self, sig: &FfiCallSignature) -> FfiStubPlan {
        FfiStubPlan {
            extern_name: sig.name.clone(),
            target_triple: self.target_triple.as_str().into(),
            platform: self.platform_label.clone(),
            arch: self.arch_label.clone(),
            callconv: sig.calling_conv.clone(),
            abi: self.backend_abi.clone(),
            ownership: self.ownership.clone(),
            register_save_area: self.register_save_area(),
        }
    }

    fn register_save_area(&self) -> Option<RegisterSaveArea> {
        match self.target_triple {
            Triple::AppleDarwin => Some(RegisterSaveArea {
                gpr_count: 8,
                gpr_slot_size: 8,
                gpr_total_size: 64,
                vector_count: 8,
                vector_slot_size: 16,
                vector_total_size: 128,
                stack_alignment: 16,
            }),
            _ => None,
        }
    }
}
