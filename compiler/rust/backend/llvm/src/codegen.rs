use std::collections::HashMap;
use std::fmt::Write;

use crate::bridge_metadata::BridgeMetadataContext;
use crate::ffi_lowering::{FfiCallSignature, FfiLowering, LoweredFfiCall};
use crate::target_diagnostics::TargetDiagnosticContext;
use crate::target_machine::{TargetMachine, WindowsToolchainConfig};
use crate::type_mapping::{RemlType, TypeLayout, TypeMappingContext};

pub type MirExprId = usize;
pub type MirBlockLabel = String;

#[derive(Clone, Debug)]
pub struct MirExpr {
    pub id: MirExprId,
    pub kind: MirExprKind,
}

#[derive(Clone, Debug)]
pub enum MirExprKind {
    Literal { summary: String },
    Identifier { summary: String },
    Call { callee: MirExprId, args: Vec<MirExprId> },
    Binary {
        operator: String,
        left: MirExprId,
        right: MirExprId,
    },
    Match {
        target: MirExprId,
        arms: Vec<MirMatchArm>,
        lowering: Option<MatchLoweringPlan>,
    },
    IfElse {
        condition: MirExprId,
        then_branch: MirExprId,
        else_branch: MirExprId,
    },
    PerformCall { effect: String, argument: MirExprId },
    Unknown,
}

#[derive(Clone, Debug)]
pub struct MirMatchArm {
    pub pattern: MirPattern,
    pub guard: Option<MirExprId>,
    pub alias: Option<String>,
    pub body: MirExprId,
}

#[derive(Clone, Debug)]
pub struct MirPattern {
    pub kind: MirPatternKind,
}

#[derive(Clone, Debug)]
pub enum MirPatternKind {
    Wildcard,
    Var { name: String },
    Literal { summary: String },
    Tuple { elements: Vec<MirPattern> },
    Record {
        fields: Vec<MirPatternRecordField>,
        has_rest: bool,
    },
    Constructor { name: String, args: Vec<MirPattern> },
    Binding {
        name: String,
        pattern: Box<MirPattern>,
        via_at: bool,
    },
    Or { variants: Vec<MirPattern> },
    Slice(MirSlicePattern),
    Range {
        start: Option<Box<MirPattern>>,
        end: Option<Box<MirPattern>>,
        inclusive: bool,
    },
    Regex { pattern: String },
    Active(MirActivePatternCall),
}

#[derive(Clone, Debug)]
pub struct MirSlicePattern {
    pub head: Vec<MirPattern>,
    pub rest: Option<MirSliceRest>,
    pub tail: Vec<MirPattern>,
}

#[derive(Clone, Debug)]
pub struct MirSliceRest {
    pub binding: Option<String>,
}

#[derive(Clone, Debug)]
pub struct MirPatternRecordField {
    pub key: String,
    pub value: Option<Box<MirPattern>>,
}

#[derive(Clone, Debug)]
pub struct MirActivePatternCall {
    pub name: String,
    pub kind: ActivePatternKind,
    pub argument: Option<Box<MirPattern>>,
    pub input_binding: Option<String>,
    pub miss_target: Option<MirJumpTarget>,
}

#[derive(Clone, Debug)]
pub enum MirJumpTarget {
    NextArm,
}

#[derive(Clone, Debug)]
pub enum ActivePatternKind {
    Partial,
    Total,
    Unknown,
}

#[derive(Clone, Debug)]
pub struct MatchLoweringPlan {
    pub owner: Option<String>,
    pub target_type: Option<String>,
    pub arm_count: Option<usize>,
    pub arms: Vec<MatchArmLowering>,
}

#[derive(Clone, Debug)]
pub struct MatchArmLowering {
    pub pattern: PatternLowering,
    pub has_guard: bool,
    pub alias: Option<String>,
}

#[derive(Clone, Debug)]
pub struct PatternLowering {
    pub label: String,
    pub miss_on_none: bool,
    pub always_matches: bool,
    pub children: Vec<PatternLowering>,
}

/// ミニマルな MIR 関数表現。
#[derive(Clone, Debug)]
pub struct MirFunction {
    pub name: String,
    pub params: Vec<RemlType>,
    pub ret: Option<RemlType>,
    pub calling_conv: String,
    pub attributes: Vec<String>,
    pub ffi_calls: Vec<FfiCallSignature>,
    /// Match/Pattern ローアリングから得られた分岐計画のサマリ。
    pub match_plans: Vec<String>,
    /// フロントエンド MIR 式木。
    pub exprs: Vec<MirExpr>,
    /// エントリ式 ID。
    pub body: Option<MirExprId>,
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
            match_plans: Vec::new(),
            exprs: Vec::new(),
            body: None,
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

    pub fn with_match_plan(mut self, plan: impl Into<String>) -> Self {
        self.match_plans.push(plan.into());
        self
    }

    pub fn with_exprs(mut self, body: Option<MirExprId>, exprs: Vec<MirExpr>) -> Self {
        self.body = body;
        self.exprs = exprs;
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
    pub branch_plans: Vec<String>,
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
    pub target_context: TargetDiagnosticContext,
    pub bridge_metadata: BridgeMetadataContext,
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
        if self.bridge_metadata.has_stubs() {
            summary.push(format!(
                "bridge stubs: {}",
                self.bridge_metadata.stub_count()
            ));
        }
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
    target_context: TargetDiagnosticContext,
    bridge_metadata: BridgeMetadataContext,
}

impl CodegenContext {
  pub fn new(target_machine: TargetMachine, runtime_symbols: Vec<String>) -> Self {
    let layout = target_machine.data_layout.clone();
    let target_context = TargetDiagnosticContext::from_target_machine(&target_machine);
    let bridge_metadata = BridgeMetadataContext::new(&target_machine);
    let ffi_lowering = FfiLowering::new(
      TypeMappingContext::new(target_machine.data_layout.clone()),
      runtime_symbols,
      target_machine.triple,
      target_machine.backend_abi().to_string(),
    );
    Self {
      type_mapping: TypeMappingContext::new(layout),
      ffi_lowering,
      target_machine,
      functions: Vec::new(),
      module_metadata: Vec::new(),
      target_context,
      bridge_metadata,
    }
  }

    pub fn describe(&self) -> String {
        format!(
            "codegen(target={}, functions={})",
            self.target_machine.describe(),
            self.functions.len()
        )
    }

    pub fn target_context(&self) -> &TargetDiagnosticContext {
        &self.target_context
    }

    pub fn set_target_context(&mut self, context: TargetDiagnosticContext) {
        self.target_context = context;
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
        let mut lowered_calls = Vec::new();
        for sig in &mir.ffi_calls {
            let lowered = self.ffi_lowering.lower_call(sig);
            self.bridge_metadata.record_stub(&lowered.stub_plan);
            lowered_calls.push(lowered);
        }
        let branch_plans = if mir.exprs.is_empty() {
            mir.match_plans.clone()
        } else {
            render_branch_plans(&mir.exprs)
        };
        let generated = GeneratedFunction {
            name: mir.name.clone(),
            layout: ret_layout,
            calling_conv: mir.calling_conv.clone(),
            attributes: mir.attributes.clone(),
            lowered_calls,
            branch_plans,
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
            target_context: self.target_context.clone(),
            bridge_metadata: self.bridge_metadata.clone(),
        }
    }
}

fn render_branch_plans(exprs: &[MirExpr]) -> Vec<String> {
    let mut expr_map = HashMap::new();
    for expr in exprs {
        expr_map.insert(expr.id, expr);
    }
    let mut plans = Vec::new();
    for expr in exprs {
        if let MirExprKind::Match {
            target,
            arms,
            lowering,
        } = &expr.kind
        {
            let target_label = expr_map
                .get(target)
                .map(|node| match &node.kind {
                    MirExprKind::Identifier { summary } => summary.clone(),
                    _ => format!("#{}", target),
                })
                .unwrap_or_else(|| format!("#{}", target));
            let target_type = lowering
                .as_ref()
                .and_then(|plan| plan.target_type.clone())
                .unwrap_or_else(|| "unknown".to_string());
            let mut arm_descriptions = Vec::new();
            for (index, arm) in arms.iter().enumerate() {
                let mut steps = Vec::new();
                steps.push(format!("pat({})", summarize_pattern(&arm.pattern)));
                if let Some(guard_id) = arm.guard {
                    steps.push(format!("guard#{guard_id}"));
                }
                if let Some(alias) = &arm.alias {
                    steps.push(format!("alias:{alias}"));
                }
                steps.push(format!("body#{}", arm.body));
                let miss = if index + 1 == arms.len() {
                    "end".to_string()
                } else {
                    format!("arm{}", index + 1)
                };
                steps.push(format!("miss->{miss}"));
                arm_descriptions.push(format!("arm{index}: {}", steps.join(" -> ")));
            }
            plans.push(format!(
                "match#{id} target={target_label} ty={target_type} arms={count} {{ {} }}",
                arm_descriptions.join(" | "),
                id = expr.id,
                count = arms.len()
            ));
        }
    }
    plans
}

pub(crate) fn summarize_pattern(pattern: &MirPattern) -> String {
    match &pattern.kind {
        MirPatternKind::Wildcard => "_".into(),
        MirPatternKind::Var { name } => format!("var({name})"),
        MirPatternKind::Literal { summary } => format!("lit({summary})"),
        MirPatternKind::Tuple { elements } => format!("tuple({})", elements.len()),
        MirPatternKind::Record { fields, has_rest } => {
            let mut labels = Vec::new();
            for field in fields {
                if let Some(value) = &field.value {
                    labels.push(format!("{}:{}", field.key, summarize_pattern(value)));
                } else {
                    labels.push(field.key.clone());
                }
            }
            if *has_rest {
                labels.push("..".into());
            }
            format!("record({})", labels.join(","))
        }
        MirPatternKind::Constructor { name, args } => {
            if args.is_empty() {
                format!("ctor({name})")
            } else {
                let args_label = args
                    .iter()
                    .map(summarize_pattern)
                    .collect::<Vec<_>>()
                    .join("|");
                format!("ctor({name};{args_label})")
            }
        }
        MirPatternKind::Binding {
            name,
            pattern: inner,
            via_at,
        } => {
            let prefix = if *via_at { "@ " } else { "as " };
            format!("binding({prefix}{name}:{})", summarize_pattern(inner))
        }
        MirPatternKind::Or { variants } => variants
            .iter()
            .map(summarize_pattern)
            .collect::<Vec<_>>()
            .join("||"),
        MirPatternKind::Slice(MirSlicePattern { head, rest, tail }) => {
            let mut parts = Vec::new();
            if !head.is_empty() {
                parts.push(format!("head{}", head.len()));
            }
            if rest.is_some() {
                parts.push("rest".into());
            }
            if !tail.is_empty() {
                parts.push(format!("tail{}", tail.len()));
            }
            format!("slice({})", parts.join(","))
        }
        MirPatternKind::Range {
            start,
            end,
            inclusive,
        } => {
            let mut bounds = Vec::new();
            if let Some(lo) = start {
                bounds.push(format!("start={}", summarize_pattern(lo)));
            }
            if let Some(hi) = end {
                bounds.push(format!("end={}", summarize_pattern(hi)));
            }
            let base = if *inclusive { "range(..=)" } else { "range(..)" };
            if bounds.is_empty() {
                base.into()
            } else {
                format!("{base}[{}]", bounds.join(","))
            }
        }
        MirPatternKind::Regex { pattern } => format!("regex({pattern})"),
        MirPatternKind::Active(MirActivePatternCall {
            name,
            kind,
            argument,
            miss_target,
            ..
        }) => {
            let mut flags = Vec::new();
            match kind {
                ActivePatternKind::Partial => flags.push("partial"),
                ActivePatternKind::Total => flags.push("total"),
                ActivePatternKind::Unknown => {}
            }
            if miss_target.is_some() {
                flags.push("miss");
            }
            let mut label = if flags.is_empty() {
                format!("active({name})")
            } else {
                format!("active({name};{})", flags.join(","))
            };
            if let Some(arg) = argument {
                label = format!("{label}[{}]", summarize_pattern(arg));
            }
            label
        }
    }
}
