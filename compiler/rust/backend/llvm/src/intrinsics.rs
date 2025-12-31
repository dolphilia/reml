use crate::target_machine::TargetMachine;
use crate::type_mapping::RemlType;

#[derive(Clone, Debug)]
pub struct IntrinsicSignature {
    pub args: Vec<RemlType>,
    pub ret: Option<RemlType>,
}

impl IntrinsicSignature {
    pub fn new(args: Vec<RemlType>, ret: Option<RemlType>) -> Self {
        Self { args, ret }
    }

    pub fn matches(&self, expected: &IntrinsicSignature) -> bool {
        self.args == expected.args && self.ret == expected.ret
    }

    pub fn render(&self) -> String {
        let args = self
            .args
            .iter()
            .map(format_reml_type)
            .collect::<Vec<_>>()
            .join(", ");
        let ret = self
            .ret
            .as_ref()
            .map(format_reml_type)
            .unwrap_or_else(|| "void".to_string());
        format!("({}) -> {}", args, ret)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IntrinsicStatus {
    Supported,
    Polyfill,
    SignatureMismatch,
}

#[derive(Clone, Debug)]
pub struct IntrinsicUse {
    pub function: String,
    pub name: String,
    pub signature: IntrinsicSignature,
    pub status: IntrinsicStatus,
    pub expected: Option<IntrinsicSignature>,
}

struct IntrinsicSpec {
    name: &'static str,
    signature: IntrinsicSignature,
    required_feature: Option<&'static str>,
}

pub fn parse_intrinsic_attribute(attr: &str) -> Option<String> {
    let trimmed = attr.trim();
    if let Some(value) = trimmed.strip_prefix("intrinsic:") {
        let name = value.trim();
        return if name.is_empty() {
            None
        } else {
            Some(name.to_string())
        };
    }
    if let Some(value) = trimmed.strip_prefix("intrinsic=") {
        let name = value.trim();
        return if name.is_empty() {
            None
        } else {
            Some(name.to_string())
        };
    }
    None
}

pub fn resolve_intrinsic_use(
    function: &str,
    name: &str,
    signature: IntrinsicSignature,
    target: &TargetMachine,
) -> IntrinsicUse {
    let spec = lookup_spec(name);
    let mut expected = None;
    let status = if let Some(spec) = spec {
        if signature.matches(&spec.signature) {
            if target_supports(target, spec.required_feature) {
                IntrinsicStatus::Supported
            } else {
                IntrinsicStatus::Polyfill
            }
        } else {
            expected = Some(spec.signature);
            IntrinsicStatus::SignatureMismatch
        }
    } else {
        IntrinsicStatus::SignatureMismatch
    };
    IntrinsicUse {
        function: function.to_string(),
        name: name.to_string(),
        signature,
        status,
        expected,
    }
}

fn lookup_spec(name: &str) -> Option<IntrinsicSpec> {
    match name {
        "llvm.sqrt.f64" => Some(IntrinsicSpec {
            name: "llvm.sqrt.f64",
            signature: IntrinsicSignature::new(vec![RemlType::F64], Some(RemlType::F64)),
            required_feature: None,
        }),
        "llvm.ctpop.i64" => Some(IntrinsicSpec {
            name: "llvm.ctpop.i64",
            signature: IntrinsicSignature::new(vec![RemlType::I64], Some(RemlType::I64)),
            required_feature: Some("popcnt"),
        }),
        "llvm.ctpop.i32" => Some(IntrinsicSpec {
            name: "llvm.ctpop.i32",
            signature: IntrinsicSignature::new(vec![RemlType::I32], Some(RemlType::I32)),
            required_feature: Some("popcnt"),
        }),
        "llvm.memcpy.p0.p0.i64" => Some(IntrinsicSpec {
            name: "llvm.memcpy.p0.p0.i64",
            signature: IntrinsicSignature::new(
                vec![RemlType::Pointer, RemlType::Pointer, RemlType::I64],
                None,
            ),
            required_feature: None,
        }),
        _ => None,
    }
}

fn target_supports(target: &TargetMachine, required_feature: Option<&str>) -> bool {
    match required_feature {
        Some(feature) => target
            .features
            .split(',')
            .any(|flag| flag.contains(feature)),
        None => true,
    }
}

fn format_reml_type(ty: &RemlType) -> String {
    match ty {
        RemlType::Bool => "bool".to_string(),
        RemlType::I32 => "i32".to_string(),
        RemlType::I64 => "i64".to_string(),
        RemlType::F64 => "f64".to_string(),
        RemlType::Pointer => "ptr".to_string(),
        RemlType::String => "string".to_string(),
        RemlType::Array { element, length } => {
            format!("[{}; {}]", format_reml_type(element), length)
        }
        RemlType::Slice(inner) => format!("[{}]", format_reml_type(inner)),
        RemlType::Set(inner) => format!("Set<{}>", format_reml_type(inner)),
        RemlType::Ref { mutable, to } => {
            if *mutable {
                format!("&mut {}", format_reml_type(to))
            } else {
                format!("&{}", format_reml_type(to))
            }
        }
        RemlType::Unit => "unit".to_string(),
        RemlType::RowTuple(items) => {
            let inner = items
                .iter()
                .map(format_reml_type)
                .collect::<Vec<_>>()
                .join(", ");
            format!("tuple({})", inner)
        }
        RemlType::Adt { .. } => "adt".to_string(),
    }
}
