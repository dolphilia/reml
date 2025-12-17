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
    pub basic_blocks: Vec<BasicBlock>,
    pub llvm_blocks: Vec<LlvmBlock>,
}

impl GeneratedFunction {
    pub fn describe(&self) -> String {
        format!(
            "{} -> {} via {} {:?}",
            self.name, self.layout.description, self.calling_conv, self.attributes
        )
    }
}

/// LLVM IR への変換に使う簡易 BasicBlock モデル。
#[derive(Clone, Debug)]
pub struct BasicBlock {
    pub label: String,
    pub instrs: Vec<String>,
    pub terminator: String,
}

impl BasicBlock {
    pub fn describe(&self) -> String {
        self.describe_llvm()
    }

    pub fn describe_llvm(&self) -> String {
        if self.instrs.is_empty() {
            format!("{}: {}", self.label, render_terminator(&self.terminator))
        } else {
            format!(
                "{}:\n  {}",
                self.label,
                render_block_body(&self.instrs, &self.terminator).join("\n  ")
            )
        }
    }
}

fn render_block_body(instrs: &[String], term: &str) -> Vec<String> {
    let mut lines: Vec<String> = instrs.iter().map(|i| render_instr(i)).collect();
    lines.push(render_terminator(term));
    lines
}

fn render_instr(instr: &str) -> String {
    if let Some(rest) = instr.strip_prefix("len(") {
        return format!("%tmp_len = call i64 @len({rest}");
    }
    if instr.contains(" = icmp_") {
        // keep as-is but add type marker
        return format!("{instr} : i1");
    }
    if instr.contains(" = and ") {
        return format!("{instr} : i1");
    }
    if instr.contains("option_is_some") {
        return instr.replace("option_is_some", "icmp_ne ptr null");
    }
    if instr.contains("slice_bind") {
        return format!("; {}", instr);
    }
    if instr.contains("call active") {
        return instr.replace("call active", "call %active");
    }
    if instr.starts_with("check ") {
        return format!("; {}", instr);
    }
    format!("; {}", instr)
}

fn render_terminator(term: &str) -> String {
    if let Some(rest) = term.strip_prefix("br_if ") {
        let mut parts = rest.split_whitespace();
        let cond = parts.next().unwrap_or("cond");
        let then = parts.nth(1).unwrap_or("then");
        let else_lbl = parts.next().unwrap_or("else");
        return format!("br i1 {cond}, label %{then}, label %{else_lbl}");
    }
    if let Some(rest) = term.strip_prefix("br ") {
        return format!("br label %{rest}");
    }
    if term.starts_with("ret ") {
        return term.to_string();
    }
    term.to_string()
}

/// LLVM IR 風の構造化ブロック。
#[derive(Clone, Debug)]
pub struct LlvmBlock {
    pub label: String,
    pub instrs: Vec<LlvmInstr>,
    pub terminator: LlvmTerminator,
}

impl LlvmBlock {
    pub fn describe(&self) -> String {
        let mut buf = Vec::new();
        buf.push(format!("{}:", self.label));
        for instr in &self.instrs {
            buf.push(format!("  {}", instr.describe()));
        }
        buf.push(format!("  {}", self.terminator.describe()));
        buf.join("\n")
    }
}

/// LLVM 関数の簡易表現。
#[derive(Clone, Debug)]
pub struct LlvmFunction {
    pub name: String,
    pub params: Vec<String>,
    pub ret: String,
    pub blocks: Vec<LlvmBlock>,
}

impl LlvmFunction {
    pub fn describe(&self) -> String {
        let mut buf = Vec::new();
        buf.push(format!(
            "define {} {}({}) {{",
            self.ret,
            self.name,
            self.params.join(", ")
        ));
        for block in &self.blocks {
            buf.push(block.describe());
        }
        buf.push("}".into());
        buf.join("\n")
    }
}

#[derive(Clone, Debug)]
struct LlvmBuilder {
    _type_mapping: TypeMappingContext,
    counter: usize,
}

impl LlvmBuilder {
    fn new(type_mapping: TypeMappingContext) -> Self {
        Self {
            _type_mapping: type_mapping,
            counter: 0,
        }
    }

    fn new_tmp(&mut self, hint: &str) -> String {
        self.counter += 1;
        format!("%{hint}{}", self.counter)
    }
}

#[derive(Clone, Debug)]
pub enum LlvmInstr {
    Comment(String),
    Icmp {
        result: String,
        pred: String,
        ty: String,
        lhs: String,
        rhs: String,
    },
    And {
        result: String,
        lhs: String,
        rhs: String,
    },
    Call {
        result: Option<String>,
        ret_ty: String,
        callee: String,
        args: Vec<(String, String)>,
    },
    Phi {
        result: String,
        ty: String,
        incomings: Vec<(String, String)>,
    },
}

impl LlvmInstr {
    pub fn describe(&self) -> String {
        match self {
            LlvmInstr::Comment(text) => format!("; {text}"),
            LlvmInstr::Icmp {
                result,
                pred,
                ty,
                lhs,
                rhs,
            } => format!("{result} = icmp {pred} {ty} {lhs}, {rhs}"),
            LlvmInstr::And { result, lhs, rhs } => {
                format!("{result} = and i1 {lhs}, {rhs}")
            }
            LlvmInstr::Call {
                result,
                ret_ty,
                callee,
                args,
            } => {
                let args_rendered = args
                    .iter()
                    .map(|(ty, val)| format!("{ty} {val}"))
                    .collect::<Vec<_>>()
                    .join(", ");
                if let Some(var) = result {
                    format!("{var} = call {ret_ty} {callee}({args_rendered})")
                } else {
                    format!("call {ret_ty} {callee}({args_rendered})")
                }
            }
            LlvmInstr::Phi {
                result,
                ty,
                incomings,
            } => {
                let inputs = incomings
                    .iter()
                    .map(|(val, lbl)| format!("[ {val}, %{lbl} ]"))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{result} = phi {ty} {inputs}")
            }
        }
    }
}

#[derive(Clone, Debug)]
pub enum LlvmTerminator {
    Br { target: String },
    BrCond {
        cond: String,
        then_bb: String,
        else_bb: String,
    },
    Ret(Option<String>),
}

impl LlvmTerminator {
    pub fn describe(&self) -> String {
        match self {
            LlvmTerminator::Br { target } => format!("br label %{target}"),
            LlvmTerminator::BrCond {
                cond,
                then_bb,
                else_bb,
            } => format!("br i1 {cond}, label %{then_bb}, label %{else_bb}"),
            LlvmTerminator::Ret(Some(val)) => format!("ret {}", val),
            LlvmTerminator::Ret(None) => "ret void".into(),
        }
    }
}

/// LLVM 風モジュール IR。
#[derive(Clone, Debug)]
pub struct ModuleIr {
    pub name: String,
    pub target: TargetMachine,
    pub functions: Vec<GeneratedFunction>,
    pub llvm_functions: Vec<LlvmFunction>,
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
        summary.push(format!("llvm_functions: {}", self.llvm_functions.len()));
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
    llvm_functions: Vec<LlvmFunction>,
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
      llvm_functions: Vec::new(),
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
        let (basic_blocks, llvm_blocks) = if mir.exprs.is_empty() {
            (Vec::new(), Vec::new())
        } else {
            lower_match_to_blocks(&mir.exprs, &self.type_mapping)
        };
        let generated = GeneratedFunction {
            name: mir.name.clone(),
            layout: ret_layout,
            calling_conv: mir.calling_conv.clone(),
            attributes: mir.attributes.clone(),
            lowered_calls,
            branch_plans,
            basic_blocks,
            llvm_blocks,
        };
        self.functions.push(generated.clone());
        let llvm_fn = LlvmFunction {
            name: mir.name.clone(),
            params: mir
                .params
                .iter()
                .map(|ty| self.type_mapping.layout_of(ty).description)
                .collect(),
            ret: mir
                .ret
                .as_ref()
                .map(|ty| self.type_mapping.layout_of(ty).description.clone())
                .unwrap_or_else(|| "void".into()),
            blocks: generated.llvm_blocks.clone(),
        };
        self.llvm_functions.push(llvm_fn);
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
            llvm_functions: self.llvm_functions,
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
            let mut arm_blocks = Vec::new();
            for (index, arm) in arms.iter().enumerate() {
                let next_arm = if index + 1 == arms.len() {
                    "end".to_string()
                } else {
                    format!("arm{}", index + 1)
                };
                let success_label = arm_success_label(arm);
                arm_blocks.extend(render_pattern_blocks(
                    index,
                    &arm.pattern,
                    &success_label,
                    &next_arm,
                    &target_label,
                ));
                if let Some(guard_id) = arm.guard {
                    arm_blocks.push(format!(
                        "arm{index}.guard#{guard_id}: true->{success} / false->{next}",
                        success = success_label,
                        next = next_arm
                    ));
                }
                if let Some(alias) = &arm.alias {
                    arm_blocks.push(format!(
                        "arm{index}.alias:{alias} -> body#{}",
                        arm.body
                    ));
                }
                arm_blocks.push(format!("arm{index}.body#{} -> end", arm.body));
            }
            plans.push(format!(
                "match#{id} target={target_label} ty={target_type} blocks=[{}]",
                arm_blocks.join("; "),
                id = expr.id,
            ));
        }
    }
    plans
}

fn lower_match_to_blocks(
    exprs: &[MirExpr],
    type_mapping: &TypeMappingContext,
) -> (Vec<BasicBlock>, Vec<LlvmBlock>) {
    let mut expr_map = HashMap::new();
    for expr in exprs {
        expr_map.insert(expr.id, expr);
    }
    let mut blocks = Vec::new();
    let mut llvm_blocks = Vec::new();
    for expr in exprs {
        if let MirExprKind::Match {
            target,
            arms,
            lowering,
        } = &expr.kind
        {
            let end_label = format!("match{}.end", expr.id);
            let target_label = expr_map
                .get(target)
                .map(|node| match &node.kind {
                    MirExprKind::Identifier { summary } => summary.clone(),
                    _ => format!("#{}", target),
                })
                .unwrap_or_else(|| format!("#{}", target));
            let mut ssa = LlvmBuilder::new(type_mapping.clone());
            let mut phi_sources = Vec::new();
            for (index, arm) in arms.iter().enumerate() {
                let next_arm = if index + 1 == arms.len() {
                    end_label.clone()
                } else {
                    format!("arm{}", index + 1)
                };
                let guard_label = arm.guard.map(|gid| format!("arm{index}.guard#{gid}"));
                let alias_label = arm.alias.as_ref().map(|_| format!("arm{index}.alias"));
                let body_label = format!("arm{index}.body#{}", arm.body);
                phi_sources.push(body_label.clone());
                let success_label = guard_label
                    .clone()
                    .or(alias_label.clone())
                    .unwrap_or_else(|| body_label.clone());

                let (mut arm_blocks, mut arm_llvm_blocks) = emit_pattern_blocks(
                    index,
                    &arm.pattern,
                    &success_label,
                    &next_arm,
                    &target_label,
                    &mut ssa,
                );
                blocks.append(&mut arm_blocks);
                llvm_blocks.append(&mut arm_llvm_blocks);

                if let Some(label) = guard_label {
                    blocks.push(BasicBlock {
                        label: label.clone(),
                        instrs: vec![format!("guard check {}", label)],
                        terminator: format!(
                            "br_if {label} then {success} else {next}",
                            success = success_label,
                            next = next_arm
                        ),
                    });
                    let cond = ssa.new_tmp("guard");
                    llvm_blocks.push(LlvmBlock {
                        label: label.clone(),
                        instrs: vec![LlvmInstr::Comment(format!("guard {label} -> {success}/{next}", success = success_label, next = next_arm)), LlvmInstr::Icmp {
                            result: cond.clone(),
                            pred: "ne".into(),
                            ty: "i1".into(),
                            lhs: "true".into(),
                            rhs: "false".into(),
                        }],
                        terminator: LlvmTerminator::BrCond {
                            cond,
                            then_bb: success_label.clone(),
                            else_bb: next_arm.clone(),
                        },
                    });
                }

                if let Some(alias) = &arm.alias {
                    let alias_block = alias_label.clone().unwrap_or_else(|| format!("arm{index}.alias"));
                    blocks.push(BasicBlock {
                        label: alias_block.clone(),
                        instrs: vec![format!("alias {alias} = {target_label}")],
                        terminator: format!("br {body}", body = body_label),
                    });
                    llvm_blocks.push(LlvmBlock {
                        label: alias_block.clone(),
                        instrs: vec![LlvmInstr::Comment(format!(
                            "alias {alias} = {target_label}"
                        ))],
                        terminator: LlvmTerminator::Br {
                            target: body_label.clone(),
                        },
                    });
                }

                blocks.push(BasicBlock {
                    label: body_label.clone(),
                    instrs: vec![format!("exec body#{}", arm.body)],
                    terminator: format!("br {}", end_label),
                });
                llvm_blocks.push(LlvmBlock {
                    label: body_label.clone(),
                    instrs: vec![LlvmInstr::Comment(format!("exec body#{}", arm.body))],
                    terminator: LlvmTerminator::Br {
                        target: end_label.clone(),
                    },
                });
            }

            let result_type = lowering
                .as_ref()
                .and_then(|plan| plan.target_type.clone())
                .unwrap_or_else(|| "unknown".into());
            let phi_inputs = if phi_sources.is_empty() {
                "[]".into()
            } else {
                format!("[{}]", phi_sources.join(", "))
            };
            blocks.push(BasicBlock {
                label: end_label.clone(),
                instrs: vec![format!("phi match_result : {} <- {}", result_type, phi_inputs)],
                terminator: "ret match_result".into(),
            });
            let phi_result = ssa.new_tmp("match");
            llvm_blocks.push(LlvmBlock {
                label: end_label.clone(),
                instrs: vec![LlvmInstr::Phi {
                    result: phi_result.clone(),
                    ty: result_type.clone(),
                    incomings: phi_sources
                        .iter()
                        .map(|lbl| ("match_result".into(), lbl.clone()))
                        .collect(),
                }],
                terminator: LlvmTerminator::Ret(Some(phi_result)),
            });
        }
    }
    (blocks, llvm_blocks)
}

fn arm_success_label(arm: &MirMatchArm) -> String {
    if let Some(guard) = arm.guard {
        format!("guard#{guard}")
    } else if let Some(alias) = &arm.alias {
        format!("alias:{alias}")
    } else {
        format!("body#{}", arm.body)
    }
}

fn render_pattern_blocks(
    arm_index: usize,
    pattern: &MirPattern,
    success_label: &str,
    next_arm_label: &str,
    target_label: &str,
) -> Vec<String> {
    match &pattern.kind {
        MirPatternKind::Or { variants } => {
            let mut blocks = Vec::new();
            for (idx, variant) in variants.iter().enumerate() {
                let miss_target = if idx + 1 == variants.len() {
                    next_arm_label.to_string()
                } else {
                    format!("arm{arm_index}.or{}", idx + 1)
                };
                let label = pattern_check_label(variant, target_label, &miss_target);
                blocks.push(format!(
                    "arm{arm_index}.or{idx}: {label} -> match:{success} / miss:{miss}",
                    success = success_label,
                    miss = miss_target
                ));
            }
            blocks
        }
        _ => {
            let label = pattern_check_label(pattern, target_label, next_arm_label);
            vec![format!(
                "arm{arm_index}.pat: {label} -> match:{success} / miss:{miss}",
                success = success_label,
                miss = next_arm_label
            )]
        }
    }
}

fn emit_pattern_blocks(
    arm_index: usize,
    pattern: &MirPattern,
    success_label: &str,
    next_arm_label: &str,
    target_label: &str,
    ssa: &mut LlvmBuilder,
) -> (Vec<BasicBlock>, Vec<LlvmBlock>) {
    match &pattern.kind {
        MirPatternKind::Or { variants } => {
            let mut blocks = Vec::new();
            let mut llvm_blocks = Vec::new();
            for (idx, variant) in variants.iter().enumerate() {
                let miss_target = if idx + 1 == variants.len() {
                    next_arm_label.to_string()
                } else {
                    format!("arm{arm_index}.or{}", idx + 1)
                };
                let check = pattern_check_label(variant, target_label, &miss_target);
                let label = format!("arm{arm_index}.or{idx}");
                blocks.push(BasicBlock {
                    label: label.clone(),
                    instrs: vec![format!("check {}", check)],
                    terminator: format!(
                        "br_if {check} then {success} else {miss}",
                        success = success_label,
                        miss = miss_target
                    ),
                });
                llvm_blocks.push(LlvmBlock {
                    label: label.clone(),
                    instrs: vec![LlvmInstr::Comment(check.clone())],
                    terminator: LlvmTerminator::BrCond {
                        cond: ssa.new_tmp("or"),
                        then_bb: success_label.to_string(),
                        else_bb: miss_target.clone(),
                    },
                });
            }
            (blocks, llvm_blocks)
        }
        MirPatternKind::Range {
            start,
            end,
            inclusive,
        } => {
            let mut instrs = Vec::new();
            let mut llvm_instrs = Vec::new();
            let mut cond = String::from("true");
            if let Some(lo) = start {
                let lhs = render_range_bound(lo);
                let var = format!("tmp_arm{arm_index}_ge");
                instrs.push(format!("{var} = icmp_ge {target_label}, {lhs}"));
                llvm_instrs.push(LlvmInstr::Icmp {
                    result: var.clone(),
                    pred: "sge".into(),
                    ty: "i64".into(),
                    lhs: target_label.into(),
                    rhs: lhs,
                });
                cond = var.clone();
            }
            if let Some(hi) = end {
                let rhs = render_range_bound(hi);
                let op = if *inclusive { "icmp_le" } else { "icmp_lt" };
                let var = format!("tmp_arm{arm_index}_hi");
                instrs.push(format!("{var} = {op} {target_label}, {rhs}"));
                llvm_instrs.push(LlvmInstr::Icmp {
                    result: var.clone(),
                    pred: if *inclusive { "sle".into() } else { "slt".into() },
                    ty: "i64".into(),
                    lhs: target_label.into(),
                    rhs,
                });
                if cond != "true" {
                    let and_var = format!("tmp_arm{arm_index}_range");
                    instrs.push(format!("{and_var} = and {cond}, {var}"));
                    llvm_instrs.push(LlvmInstr::And {
                        result: and_var.clone(),
                        lhs: cond.clone(),
                        rhs: var.clone(),
                    });
                    cond = and_var;
                } else {
                    cond = var;
                }
            }
            let bb = BasicBlock {
                label: format!("arm{arm_index}.pat"),
                instrs,
                terminator: format!(
                    "br_if {cond} then {success} else {miss}",
                    success = success_label,
                    miss = next_arm_label
                ),
            };
            let llvm_bb = LlvmBlock {
                label: bb.label.clone(),
                instrs: llvm_instrs,
                terminator: LlvmTerminator::BrCond {
                    cond,
                    then_bb: success_label.to_string(),
                    else_bb: next_arm_label.to_string(),
                },
            };
            (vec![bb], vec![llvm_bb])
        }
        MirPatternKind::Slice(MirSlicePattern { head, rest, tail }) => {
            let mut instrs = Vec::new();
            let mut llvm_instrs = Vec::new();
            let len_var = format!("len_arm{arm_index}");
            let need = head.len() + tail.len();
            instrs.push(format!("{len_var} = len({target_label})"));
            llvm_instrs.push(LlvmInstr::Call {
                result: Some(len_var.clone()),
                ret_ty: "i64".into(),
                callee: "@len".into(),
                args: vec![("ptr".into(), target_label.into())],
            });
            let check_var = format!("tmp_arm{arm_index}_len");
            if rest.is_some() {
                instrs.push(format!("{check_var} = icmp_uge {len_var}, {need}"));
                llvm_instrs.push(LlvmInstr::Icmp {
                    result: check_var.clone(),
                    pred: "uge".into(),
                    ty: "i64".into(),
                    lhs: len_var.clone(),
                    rhs: need.to_string(),
                });
            } else {
                instrs.push(format!("{check_var} = icmp_eq {len_var}, {need}"));
                llvm_instrs.push(LlvmInstr::Icmp {
                    result: check_var.clone(),
                    pred: "eq".into(),
                    ty: "i64".into(),
                    lhs: len_var.clone(),
                    rhs: need.to_string(),
                });
            }
            instrs.push(format!(
                "slice_bind head[{}], tail[{}], rest={}",
                head.len(),
                tail.len(),
                rest.is_some()
            ));
            let bb = BasicBlock {
                label: format!("arm{arm_index}.pat"),
                instrs,
                terminator: format!(
                    "br_if {check} then {success} else {miss}",
                    check = check_var,
                    success = success_label,
                    miss = next_arm_label
                ),
            };
            let llvm_bb = LlvmBlock {
                label: bb.label.clone(),
                instrs: llvm_instrs,
                terminator: LlvmTerminator::BrCond {
                    cond: check_var,
                    then_bb: success_label.to_string(),
                    else_bb: next_arm_label.to_string(),
                },
            };
            (vec![bb], vec![llvm_bb])
        }
        MirPatternKind::Active(MirActivePatternCall {
            name,
            kind,
            miss_target,
            ..
        }) => {
            let mut instrs = Vec::new();
            let mut llvm_instrs = Vec::new();
            let call_var = format!("tmp_arm{arm_index}_active");
            instrs.push(format!("{call_var} = call active {name}({target_label})"));
            llvm_instrs.push(LlvmInstr::Call {
                result: Some(call_var.clone()),
                ret_ty: "ptr".into(),
                callee: format!("@{}", name),
                args: vec![("ptr".into(), target_label.into())],
            });
            let cond = match kind {
                ActivePatternKind::Partial => {
                    let check_var = format!("tmp_arm{arm_index}_is_some");
                    instrs.push(format!("{check_var} = option_is_some {call_var}"));
                    llvm_instrs.push(LlvmInstr::Icmp {
                        result: check_var.clone(),
                        pred: "ne".into(),
                        ty: "ptr".into(),
                        lhs: call_var.clone(),
                        rhs: "null".into(),
                    });
                    check_var
                }
                _ => "true".into(),
            };
            let miss = miss_target
                .as_ref()
                .map(|_| next_arm_label.to_string())
                .unwrap_or_else(|| next_arm_label.to_string());
            let bb = BasicBlock {
                label: format!("arm{arm_index}.pat"),
                instrs,
                terminator: format!(
                    "br_if {cond} then {success} else {miss}",
                    success = success_label,
                    miss = miss
                ),
            };
            let llvm_bb = LlvmBlock {
                label: bb.label.clone(),
                instrs: llvm_instrs,
                terminator: LlvmTerminator::BrCond {
                    cond,
                    then_bb: success_label.to_string(),
                    else_bb: miss,
                },
            };
            (vec![bb], vec![llvm_bb])
        }
        _ => {
            let check = pattern_check_label(pattern, target_label, next_arm_label);
            let bb = BasicBlock {
                label: format!("arm{arm_index}.pat"),
                instrs: vec![format!("check {}", check)],
                terminator: format!(
                    "br_if {check} then {success} else {miss}",
                    success = success_label,
                    miss = next_arm_label
                ),
            };
            let llvm_bb = LlvmBlock {
                label: bb.label.clone(),
                instrs: vec![LlvmInstr::Comment(check)],
                terminator: LlvmTerminator::BrCond {
                    cond: ssa.new_tmp("check"),
                    then_bb: success_label.to_string(),
                    else_bb: next_arm_label.to_string(),
                },
            };
            (vec![bb], vec![llvm_bb])
        }
    }
}

fn render_range_bound(pattern: &MirPattern) -> String {
    match &pattern.kind {
        MirPatternKind::Literal { summary } => summary.clone(),
        MirPatternKind::Var { name } => name.clone(),
        _ => summarize_pattern(pattern),
    }
}

fn pattern_check_label(pattern: &MirPattern, target_label: &str, miss_label: &str) -> String {
    match &pattern.kind {
        MirPatternKind::Wildcard => format!("match_any({target_label})"),
        MirPatternKind::Var { name } => format!("bind({name})"),
        MirPatternKind::Literal { summary } => format!("eq({target_label},{summary})"),
        MirPatternKind::Tuple { elements } => {
            format!("tuple_check(len={} on {target_label})", elements.len())
        }
        MirPatternKind::Record { fields, has_rest } => {
            let rest = if *has_rest { "with_rest" } else { "exact" };
            format!(
                "record_check({} fields,{rest} on {target_label})",
                fields.len()
            )
        }
        MirPatternKind::Constructor { name, args } => {
            format!(
                "ctor_check({name}, args={} on {target_label})",
                args.len()
            )
        }
        MirPatternKind::Binding { pattern, .. } => pattern_check_label(pattern, target_label, miss_label),
        MirPatternKind::Or { variants } => {
            format!("or({} variants)", variants.len())
        }
        MirPatternKind::Slice(MirSlicePattern { head, rest, tail }) => {
            let base_len = head.len() + tail.len();
            let len_rule = if rest.is_some() {
                format!("len>={}", base_len)
            } else {
                format!("len=={}", base_len)
            };
            format!(
                "slice_check({len_rule};head={};tail={};rest={} on {target_label})",
                head.len(),
                tail.len(),
                rest.is_some()
            )
        }
        MirPatternKind::Range {
            start,
            end,
            inclusive,
        } => {
            let bound = if *inclusive { "..=" } else { ".." };
            let mut parts = Vec::new();
            if start.is_some() {
                parts.push("start".to_string());
            }
            if end.is_some() {
                parts.push("end".to_string());
            }
            let bounds = if parts.is_empty() {
                "open".to_string()
            } else {
                parts.join("+")
            };
            format!("range_check({bound}{bounds} on {target_label})")
        }
        MirPatternKind::Regex { pattern } => format!("regex_match({pattern} on {target_label})"),
        MirPatternKind::Active(MirActivePatternCall {
            name,
            kind,
            miss_target,
            ..
        }) => match kind {
            ActivePatternKind::Partial => {
                let miss = miss_target
                    .as_ref()
                    .map(|_| miss_label.to_string())
                    .unwrap_or_else(|| miss_label.to_string());
                format!("active_partial({name} miss->{miss})")
            }
            ActivePatternKind::Total => format!("active_total({name})"),
            ActivePatternKind::Unknown => format!("active({name})"),
        },
    }
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
