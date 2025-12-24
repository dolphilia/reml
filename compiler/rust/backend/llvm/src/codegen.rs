use std::collections::HashMap;
use std::fmt::Write;

use crate::bridge_metadata::BridgeMetadataContext;
use crate::ffi_lowering::{FfiCallSignature, FfiLowering, LoweredFfiCall};
use crate::intrinsics::{
    parse_intrinsic_attribute, resolve_intrinsic_use, IntrinsicSignature, IntrinsicUse,
};
use crate::target_diagnostics::TargetDiagnosticContext;
use crate::target_machine::{TargetMachine, WindowsToolchainConfig};
use crate::type_mapping::{RemlType, TypeLayout, TypeMappingContext};
use crate::unstable::{
    native_unstable_enabled, parse_unstable_attribute, UnstableStatus, UnstableUse,
};

pub type MirExprId = usize;
pub type MirBlockLabel = String;

// LLVM 風 IR で使用する暫定 intrinsic（将来の実 LLVM IR/Runtime Bridge へ移行するための境界）。
const INTRINSIC_VALUE: &str = "@reml_value";
const INTRINSIC_MATCH_CHECK: &str = "@reml_match_check";
const INTRINSIC_REGEX_MATCH: &str = "@reml_regex_match";
const INTRINSIC_FIELD_ACCESS: &str = "@reml_field_access";
const INTRINSIC_CALL: &str = "@reml_call";
const INTRINSIC_STR_CONCAT: &str = "@reml_str_concat";
const INTRINSIC_IF_ELSE: &str = "@reml_if_else";
const INTRINSIC_PERFORM: &str = "@reml_perform";
const INTRINSIC_PANIC: &str = "@panic";

fn intrinsic_is_ctor(name: &str) -> String {
    format!("@reml_is_ctor_{name}")
}

fn intrinsic_ctor_payload(name: &str) -> String {
    format!("@reml_ctor_payload_{name}")
}

#[derive(Clone, Debug)]
pub struct MirExpr {
    pub id: MirExprId,
    pub kind: MirExprKind,
}

#[derive(Clone, Debug)]
pub enum MirExprKind {
    Literal {
        summary: String,
    },
    Identifier {
        summary: String,
    },
    FieldAccess {
        target: MirExprId,
        field: String,
    },
    Call {
        callee: MirExprId,
        args: Vec<MirExprId>,
    },
    Block {
        tail: Option<MirExprId>,
        defers: Vec<MirExprId>,
        defer_lifo: Vec<MirExprId>,
    },
    Return {
        value: Option<MirExprId>,
    },
    Propagate {
        expr: MirExprId,
    },
    Panic {
        argument: Option<MirExprId>,
    },
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
    PerformCall {
        effect: String,
        argument: MirExprId,
    },
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
    Var {
        name: String,
    },
    Literal {
        summary: String,
    },
    Tuple {
        elements: Vec<MirPattern>,
    },
    Record {
        fields: Vec<MirPatternRecordField>,
        has_rest: bool,
    },
    Constructor {
        name: String,
        args: Vec<MirPattern>,
    },
    Binding {
        name: String,
        pattern: Box<MirPattern>,
        via_at: bool,
    },
    Or {
        variants: Vec<MirPattern>,
    },
    Slice(MirSlicePattern),
    Range {
        start: Option<Box<MirPattern>>,
        end: Option<Box<MirPattern>>,
        inclusive: bool,
    },
    Regex {
        pattern: String,
    },
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
    pub llvm_ir: String,
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
    type_mapping: TypeMappingContext,
    counter: usize,
}

impl LlvmBuilder {
    fn new(type_mapping: TypeMappingContext) -> Self {
        Self {
            type_mapping,
            counter: 0,
        }
    }

    fn new_tmp(&mut self, hint: &str) -> String {
        self.counter += 1;
        format!("%{hint}{}", self.counter)
    }

    fn bool_type(&self) -> String {
        self.type_mapping.layout_of(&RemlType::Bool).description
    }

    fn pointer_type(&self) -> String {
        self.type_mapping.layout_of(&RemlType::Pointer).description
    }
}

#[derive(Clone, Debug)]
pub enum LlvmInstr {
    Comment(String),
    BinOp {
        result: String,
        op: String,
        ty: String,
        lhs: String,
        rhs: String,
    },
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
    Or {
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
            LlvmInstr::BinOp {
                result,
                op,
                ty,
                lhs,
                rhs,
            } => format!("{result} = {op} {ty} {lhs}, {rhs}"),
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
            LlvmInstr::Or { result, lhs, rhs } => format!("{result} = or i1 {lhs}, {rhs}"),
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
    Br {
        target: String,
    },
    BrCond {
        cond: String,
        then_bb: String,
        else_bb: String,
    },
    Ret(Option<String>),
    Unreachable,
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
            LlvmTerminator::Unreachable => "unreachable".into(),
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
    pub intrinsic_uses: Vec<IntrinsicUse>,
    pub unstable_uses: Vec<UnstableUse>,
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
        if !self.intrinsic_uses.is_empty() {
            summary.push(format!("intrinsics: {}", self.intrinsic_uses.len()));
        }
        if !self.unstable_uses.is_empty() {
            summary.push(format!("unstable: {}", self.unstable_uses.len()));
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
    intrinsic_uses: Vec<IntrinsicUse>,
    unstable_uses: Vec<UnstableUse>,
    target_context: TargetDiagnosticContext,
    bridge_metadata: BridgeMetadataContext,
    llvm_ir_builder: LlvmIrBuilder,
}

impl CodegenContext {
    pub fn new(target_machine: TargetMachine, runtime_symbols: Vec<String>) -> Self {
        let layout = target_machine.data_layout.clone();
        let target_context = TargetDiagnosticContext::from_target_machine(&target_machine);
        let bridge_metadata = BridgeMetadataContext::new(&target_machine);
        let type_mapping = TypeMappingContext::new(layout);
        let ffi_lowering = FfiLowering::new(
            type_mapping.clone(),
            runtime_symbols,
            target_machine.triple,
            target_machine.backend_abi().to_string(),
        );
        Self {
            llvm_ir_builder: LlvmIrBuilder::new(type_mapping.clone()),
            type_mapping,
            ffi_lowering,
            target_machine,
            functions: Vec::new(),
            llvm_functions: Vec::new(),
            module_metadata: Vec::new(),
            intrinsic_uses: Vec::new(),
            unstable_uses: Vec::new(),
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
        for attr in &mir.attributes {
            if let Some(name) = parse_intrinsic_attribute(attr) {
                let signature = IntrinsicSignature::new(mir.params.clone(), mir.ret.clone());
                let usage =
                    resolve_intrinsic_use(&mir.name, &name, signature, &self.target_machine);
                self.intrinsic_uses.push(usage);
            }
            if let Some(request) = parse_unstable_attribute(attr) {
                let status = if native_unstable_enabled() {
                    UnstableStatus::Enabled
                } else {
                    UnstableStatus::Disabled
                };
                self.unstable_uses.push(UnstableUse {
                    function: mir.name.clone(),
                    kind: request.kind,
                    payload: request.payload,
                    status,
                });
            }
        }
        let branch_plans = if mir.exprs.is_empty() {
            mir.match_plans.clone()
        } else {
            render_branch_plans(&mir.exprs)
        };
        let (basic_blocks, llvm_blocks) = if mir.exprs.is_empty() {
            (Vec::new(), Vec::new())
        } else {
            let (basic_blocks, llvm_blocks) = lower_match_to_blocks(&mir.exprs, &self.type_mapping);
            if llvm_blocks.is_empty() {
                if let Some(body) = mir.body {
                    lower_entry_expr_to_blocks(&mir.exprs, body, &self.type_mapping)
                } else {
                    (basic_blocks, llvm_blocks)
                }
            } else {
                (basic_blocks, llvm_blocks)
            }
        };
        let llvm_fn = self.llvm_ir_builder.build_function(
            &mir.name,
            &mir.params,
            mir.ret.as_ref(),
            llvm_blocks.clone(),
        );
        let llvm_ir = self.llvm_ir_builder.render_ir(&llvm_fn);
        let generated = GeneratedFunction {
            name: mir.name.clone(),
            layout: ret_layout,
            calling_conv: mir.calling_conv.clone(),
            attributes: mir.attributes.clone(),
            lowered_calls,
            branch_plans,
            basic_blocks,
            llvm_blocks,
            llvm_ir,
        };
        self.functions.push(generated.clone());
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
            intrinsic_uses: self.intrinsic_uses,
            unstable_uses: self.unstable_uses,
            windows_toolchain: self.target_machine.windows_toolchain.clone(),
            target_context: self.target_context.clone(),
            bridge_metadata: self.bridge_metadata.clone(),
        }
    }
}

#[derive(Clone, Debug)]
struct LlvmIrBuilder {
    type_mapping: TypeMappingContext,
}

impl LlvmIrBuilder {
    fn new(type_mapping: TypeMappingContext) -> Self {
        Self { type_mapping }
    }

    fn build_function(
        &self,
        name: &str,
        params: &[RemlType],
        ret: Option<&RemlType>,
        blocks: Vec<LlvmBlock>,
    ) -> LlvmFunction {
        let params = params
            .iter()
            .map(|ty| self.type_mapping.layout_of(ty).description)
            .collect();
        let ret = ret
            .map(|ty| self.type_mapping.layout_of(ty).description)
            .unwrap_or_else(|| "void".into());
        LlvmFunction {
            name: name.into(),
            params,
            ret,
            blocks,
        }
    }

    fn render_ir(&self, func: &LlvmFunction) -> String {
        func.describe()
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
                    arm_blocks.push(format!("arm{index}.alias:{alias} -> body#{}", arm.body));
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
            let target_desc = expr_map
                .get(target)
                .map(|node| match &node.kind {
                    MirExprKind::Identifier { summary } => summary.clone(),
                    _ => format!("#{}", target),
                })
                .unwrap_or_else(|| format!("#{}", target));
            let target_operand = format_operand_from_summary(&target_desc);
            let mut ssa = LlvmBuilder::new(type_mapping.clone());
            let mut phi_sources: Vec<(String, String)> = Vec::new();
            for (index, arm) in arms.iter().enumerate() {
                let next_arm = if index + 1 == arms.len() {
                    end_label.clone()
                } else {
                    format!("arm{}", index + 1)
                };
                let guard_label = arm.guard.map(|gid| format!("arm{index}.guard#{gid}"));
                let alias_label = arm.alias.as_ref().map(|_| format!("arm{index}.alias"));
                let body_label = format!("arm{index}.body#{}", arm.body);
                let post_guard_label = alias_label.clone().unwrap_or_else(|| body_label.clone());
                let success_label = guard_label
                    .clone()
                    .or(alias_label.clone())
                    .unwrap_or_else(|| body_label.clone());

                let (mut arm_blocks, mut arm_llvm_blocks) = emit_pattern_blocks(
                    index,
                    &arm.pattern,
                    &success_label,
                    &next_arm,
                    &target_operand,
                    &target_desc,
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
                            success = post_guard_label,
                            next = next_arm
                        ),
                    });
                    let (cond, mut guard_instrs) =
                        emit_guard_cond(arm.guard.unwrap_or(0), &expr_map, &mut ssa);
                    guard_instrs.insert(
                        0,
                        LlvmInstr::Comment(format!(
                            "guard {label} -> {success}/{next}",
                            success = post_guard_label,
                            next = next_arm
                        )),
                    );
                    llvm_blocks.push(LlvmBlock {
                        label: label.clone(),
                        instrs: guard_instrs,
                        terminator: LlvmTerminator::BrCond {
                            cond,
                            then_bb: post_guard_label.clone(),
                            else_bb: next_arm.clone(),
                        },
                    });
                }

                if let Some(alias) = &arm.alias {
                    let alias_block = alias_label
                        .clone()
                        .unwrap_or_else(|| format!("arm{index}.alias"));
                    blocks.push(BasicBlock {
                        label: alias_block.clone(),
                        instrs: vec![format!("alias {alias} = {target_desc}")],
                        terminator: format!("br {body}", body = body_label),
                    });
                    llvm_blocks.push(LlvmBlock {
                        label: alias_block.clone(),
                        instrs: vec![LlvmInstr::Comment(format!("alias {alias} = {target_desc}"))],
                        terminator: LlvmTerminator::Br {
                            target: body_label.clone(),
                        },
                    });
                }

                let result_type = lowering
                    .as_ref()
                    .and_then(|plan| plan.target_type.clone())
                    .unwrap_or_else(|| "unknown".into());
                let early_exit = detect_arm_early_exit(arm.body, &expr_map);
                match early_exit {
                    Some(ArmEarlyExit::Panic) => {
                        let value = emit_value_expr(arm.body, &expr_map, &mut ssa);
                        let (block, llvm_block) = lower_panic_value_to_named_block(
                            body_label.clone(),
                            arm.body,
                            value,
                        );
                        blocks.push(block);
                        llvm_blocks.push(llvm_block);
                    }
                    Some(ArmEarlyExit::Propagate) => {
                        let value = emit_value_expr(arm.body, &expr_map, &mut ssa);
                        let (prop_blocks, prop_llvm_blocks, phi_source) =
                            lower_propagate_value_to_match_blocks(
                                index,
                                arm.body,
                                value,
                                expr_map
                                    .get(&arm.body)
                                    .map(|expr| expr.ty.as_str())
                                    .unwrap_or("Result"),
                                &result_type,
                                &end_label,
                                &mut ssa,
                            );
                        blocks.extend(prop_blocks);
                        llvm_blocks.extend(prop_llvm_blocks);
                        phi_sources.push(phi_source);
                    }
                    None => {
                        blocks.push(BasicBlock {
                            label: body_label.clone(),
                            instrs: vec![format!("exec body#{}", arm.body)],
                            terminator: format!("br {}", end_label),
                        });
                        let (value, value_label, mut value_instrs) =
                            emit_body_value(index, arm.body, &expr_map, &result_type, &mut ssa);
                        phi_sources.push((value, value_label));
                        llvm_blocks.push(LlvmBlock {
                            label: body_label.clone(),
                            instrs: {
                                let mut instrs = Vec::new();
                                instrs.push(LlvmInstr::Comment(format!("exec body#{}", arm.body)));
                                instrs.append(&mut value_instrs);
                                instrs
                            },
                            terminator: LlvmTerminator::Br {
                                target: end_label.clone(),
                            },
                        });
                    }
                }
            }

            let result_type = lowering
                .as_ref()
                .and_then(|plan| plan.target_type.clone())
                .unwrap_or_else(|| "unknown".into());
            let phi_inputs = if phi_sources.is_empty() {
                "[]".into()
            } else {
                format!(
                    "[{}]",
                    phi_sources
                        .iter()
                        .map(|(_, lbl)| lbl.clone())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            };
            blocks.push(BasicBlock {
                label: end_label.clone(),
                instrs: vec![format!(
                    "phi match_result : {} <- {}",
                    result_type, phi_inputs
                )],
                terminator: "ret match_result".into(),
            });
            let phi_result = ssa.new_tmp("match");
            llvm_blocks.push(LlvmBlock {
                label: end_label.clone(),
                instrs: vec![LlvmInstr::Phi {
                    result: phi_result.clone(),
                    ty: result_type.clone(),
                    incomings: phi_sources,
                }],
                terminator: LlvmTerminator::Ret(Some(phi_result)),
            });
        }
    }
    (blocks, llvm_blocks)
}

#[derive(Clone, Copy, Debug)]
enum ArmEarlyExit {
    Panic,
    Propagate,
}

fn detect_arm_early_exit(
    expr_id: MirExprId,
    expr_map: &HashMap<MirExprId, &MirExpr>,
) -> Option<ArmEarlyExit> {
    let expr = expr_map.get(&expr_id)?;
    match &expr.kind {
        MirExprKind::Panic { .. } => Some(ArmEarlyExit::Panic),
        MirExprKind::Propagate { .. } => Some(ArmEarlyExit::Propagate),
        MirExprKind::Block { tail, .. } => {
            let tail_id = tail?;
            let tail_expr = expr_map.get(tail_id)?;
            match &tail_expr.kind {
                MirExprKind::Panic { .. } => Some(ArmEarlyExit::Panic),
                MirExprKind::Propagate { .. } => Some(ArmEarlyExit::Propagate),
                _ => None,
            }
        }
        _ => None,
    }
}

fn lower_entry_expr_to_blocks(
    exprs: &[MirExpr],
    body: MirExprId,
    type_mapping: &TypeMappingContext,
) -> (Vec<BasicBlock>, Vec<LlvmBlock>) {
    let mut expr_map = HashMap::new();
    for expr in exprs {
        expr_map.insert(expr.id, expr);
    }
    let mut ssa = LlvmBuilder::new(type_mapping.clone());
    if let Some(expr) = expr_map.get(&body) {
        match &expr.kind {
            MirExprKind::Panic { .. } => {
                let value = emit_value_expr(body, &expr_map, &mut ssa);
                return lower_panic_value_to_blocks(body, value, &mut ssa);
            }
            MirExprKind::Propagate { .. } => {
                let value = emit_value_expr(body, &expr_map, &mut ssa);
                return lower_propagate_value_to_blocks(body, value, &expr.ty, &mut ssa);
            }
            MirExprKind::Block { tail, defer_lifo, .. } => {
                if let Some(tail_id) = tail {
                    if let Some(tail_expr) = expr_map.get(tail_id) {
                        match &tail_expr.kind {
                            MirExprKind::Panic { .. } => {
                                let value = emit_value_expr(body, &expr_map, &mut ssa);
                                return lower_panic_value_to_blocks(body, value, &mut ssa);
                            }
                            MirExprKind::Propagate { .. } => {
                                let value = emit_value_expr(body, &expr_map, &mut ssa);
                                return lower_propagate_value_to_blocks(
                                    body,
                                    value,
                                    &expr.ty,
                                    &mut ssa,
                                );
                            }
                            MirExprKind::IfElse {
                                condition,
                                then_branch,
                                else_branch,
                            } if !defer_lifo.is_empty() => {
                                let then_kind = classify_branch_kind(*then_branch, &expr_map);
                                let else_kind = classify_branch_kind(*else_branch, &expr_map);
                                if then_kind.is_early_exit() || else_kind.is_early_exit() {
                                    let (blocks, llvm_blocks) =
                                        lower_block_tail_if_else_with_defer_to_blocks(
                                            body,
                                            *condition,
                                            *then_branch,
                                            *else_branch,
                                            defer_lifo,
                                            &expr_map,
                                            &mut ssa,
                                        );
                                    return (blocks, llvm_blocks);
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            MirExprKind::IfElse {
                condition,
                then_branch,
                else_branch,
            } => {
                let then_kind = classify_branch_kind(*then_branch, &expr_map);
                let else_kind = classify_branch_kind(*else_branch, &expr_map);
                if then_kind.is_early_exit() || else_kind.is_early_exit() {
                    let (blocks, llvm_blocks) = lower_if_else_with_propagate_to_blocks(
                        body,
                        *condition,
                        *then_branch,
                        *else_branch,
                        &expr_map,
                        &mut ssa,
                    );
                    return (blocks, llvm_blocks);
                }
            }
            MirExprKind::Call { callee, args } => {
                if expr_contains_early_exit(*callee, &expr_map)
                    || args
                        .iter()
                        .any(|arg| expr_contains_early_exit(*arg, &expr_map))
                {
                    let (blocks, llvm_blocks) = lower_call_with_propagate_to_blocks(
                        body,
                        *callee,
                        args,
                        &expr_map,
                        &mut ssa,
                    );
                    return (blocks, llvm_blocks);
                }
            }
            MirExprKind::Binary {
                operator,
                left,
                right,
            } => {
                if matches!(operator.as_str(), "+" | "-" | "*" | "/" | "%")
                    && (expr_contains_early_exit(*left, &expr_map)
                        || expr_contains_early_exit(*right, &expr_map))
                {
                    let (blocks, llvm_blocks) = lower_binary_with_propagate_to_blocks(
                        body,
                        operator,
                        *left,
                        *right,
                        &expr_map,
                        &mut ssa,
                    );
                    return (blocks, llvm_blocks);
                }
            }
            _ => {}
        }
    }
    let value = emit_value_expr(body, &expr_map, &mut ssa);
    let block = BasicBlock {
        label: "entry".into(),
        instrs: vec![format!("exec body#{body}")],
        terminator: format!("ret {}", value.operand),
    };
    let llvm_block = LlvmBlock {
        label: "entry".into(),
        instrs: {
            let mut instrs = vec![LlvmInstr::Comment(format!("exec body#{body}"))];
            instrs.extend(value.instrs);
            instrs
        },
        terminator: LlvmTerminator::Ret(Some(value.operand)),
    };
    (vec![block], vec![llvm_block])
}

#[derive(Clone, Copy, Debug)]
enum BranchKind {
    Normal,
    Propagate,
    Panic,
}

impl BranchKind {
    fn is_early_exit(self) -> bool {
        matches!(self, BranchKind::Propagate | BranchKind::Panic)
    }
}

fn classify_branch_kind(
    expr_id: MirExprId,
    expr_map: &HashMap<MirExprId, &MirExpr>,
) -> BranchKind {
    let Some(expr) = expr_map.get(&expr_id) else {
        return BranchKind::Normal;
    };
    match &expr.kind {
        MirExprKind::Propagate { .. } => BranchKind::Propagate,
        MirExprKind::Panic { .. } => BranchKind::Panic,
        MirExprKind::Block { tail, .. } => {
            let Some(tail_id) = tail else {
                return BranchKind::Normal;
            };
            let Some(tail_expr) = expr_map.get(tail_id) else {
                return BranchKind::Normal;
            };
            match &tail_expr.kind {
                MirExprKind::Propagate { .. } => BranchKind::Propagate,
                MirExprKind::Panic { .. } => BranchKind::Panic,
                _ => BranchKind::Normal,
            }
        }
        _ => BranchKind::Normal,
    }
}

fn expr_contains_early_exit(
    expr_id: MirExprId,
    expr_map: &HashMap<MirExprId, &MirExpr>,
) -> bool {
    let Some(expr) = expr_map.get(&expr_id) else {
        return false;
    };
    match &expr.kind {
        MirExprKind::Propagate { .. } | MirExprKind::Panic { .. } => true,
        MirExprKind::Block { tail, .. } => tail
            .and_then(|tail_id| expr_map.get(&tail_id))
            .map(|expr| expr_contains_early_exit(expr.id, expr_map))
            .unwrap_or(false),
        MirExprKind::IfElse {
            condition,
            then_branch,
            else_branch,
        } => {
            expr_contains_early_exit(*condition, expr_map)
                || expr_contains_early_exit(*then_branch, expr_map)
                || expr_contains_early_exit(*else_branch, expr_map)
        }
        MirExprKind::Call { callee, args } => {
            expr_contains_early_exit(*callee, expr_map)
                || args
                    .iter()
                    .any(|arg| expr_contains_early_exit(*arg, expr_map))
        }
        MirExprKind::Binary { left, right, .. } => {
            expr_contains_early_exit(*left, expr_map)
                || expr_contains_early_exit(*right, expr_map)
        }
        _ => false,
    }
}

fn lower_call_with_propagate_to_blocks(
    body: MirExprId,
    callee: MirExprId,
    args: &[MirExprId],
    expr_map: &HashMap<MirExprId, &MirExpr>,
    ssa: &mut LlvmBuilder,
) -> (Vec<BasicBlock>, Vec<LlvmBlock>) {
    let mut blocks = Vec::new();
    let mut llvm_blocks = Vec::new();
    let mut step_label = "entry".to_string();
    let mut next_index = 0usize;
    let mut operands: Vec<(String, String)> = Vec::new();
    let mut steps = Vec::new();
    steps.push(callee);
    steps.extend_from_slice(args);

    for expr_id in steps {
        let next_label = format!("call{}.step{}", body, next_index);
        next_index += 1;
        let (step_blocks, step_llvm_blocks, operand, terminated) =
            lower_expr_to_operand_blocks(
                step_label.clone(),
                expr_id,
                expr_map,
                ssa,
                &next_label,
            );
        blocks.extend(step_blocks);
        llvm_blocks.extend(step_llvm_blocks);
        if terminated {
            return (blocks, llvm_blocks);
        }
        if let Some((operand, ty)) = operand {
            operands.push((ty, operand));
        }
        step_label = next_label;
    }

    let callee_type = operands
        .first()
        .map(|(ty, _)| ty.clone())
        .unwrap_or_else(|| ssa.pointer_type());
    let callee_operand = operands
        .first()
        .map(|(_, op)| op.clone())
        .unwrap_or_else(|| "null".into());
    let mut call_args = Vec::new();
    call_args.push((callee_type, callee_operand));
    for (ty, op) in operands.into_iter().skip(1) {
        call_args.push((ty, op));
    }

    let ret_ty = infer_call_return_type(callee, expr_map, ssa);
    let result = ssa.new_tmp("call");
    let block = BasicBlock {
        label: step_label.clone(),
        instrs: vec![format!("exec call#{body}")],
        terminator: format!("ret {result}"),
    };
    let llvm_block = LlvmBlock {
        label: step_label,
        instrs: vec![
            LlvmInstr::Comment(format!("exec call#{body}")),
            LlvmInstr::Call {
                result: Some(result.clone()),
                ret_ty: ret_ty.clone(),
                callee: INTRINSIC_CALL.into(),
                args: call_args,
            },
        ],
        terminator: LlvmTerminator::Ret(Some(result)),
    };
    blocks.push(block);
    llvm_blocks.push(llvm_block);
    (blocks, llvm_blocks)
}

fn lower_call_with_propagate_to_operand_blocks(
    label: String,
    body: MirExprId,
    callee: MirExprId,
    args: &[MirExprId],
    expr_map: &HashMap<MirExprId, &MirExpr>,
    ssa: &mut LlvmBuilder,
    next_label: &str,
) -> (Vec<BasicBlock>, Vec<LlvmBlock>, Option<(String, String)>, bool) {
    let mut blocks = Vec::new();
    let mut llvm_blocks = Vec::new();
    let mut step_label = label;
    let mut next_index = 0usize;
    let mut operands: Vec<(String, String)> = Vec::new();
    let mut steps = Vec::new();
    steps.push(callee);
    steps.extend_from_slice(args);

    for expr_id in steps {
        let next_step_label = format!("call{}.step{}", body, next_index);
        next_index += 1;
        let (step_blocks, step_llvm_blocks, operand, terminated) =
            lower_expr_to_operand_blocks(
                step_label.clone(),
                expr_id,
                expr_map,
                ssa,
                &next_step_label,
            );
        blocks.extend(step_blocks);
        llvm_blocks.extend(step_llvm_blocks);
        if terminated {
            return (blocks, llvm_blocks, None, true);
        }
        if let Some((operand, ty)) = operand {
            operands.push((ty, operand));
        }
        step_label = next_step_label;
    }

    let callee_type = operands
        .first()
        .map(|(ty, _)| ty.clone())
        .unwrap_or_else(|| ssa.pointer_type());
    let callee_operand = operands
        .first()
        .map(|(_, op)| op.clone())
        .unwrap_or_else(|| "null".into());
    let mut call_args = Vec::new();
    call_args.push((callee_type, callee_operand));
    for (ty, op) in operands.into_iter().skip(1) {
        call_args.push((ty, op));
    }

    let ret_ty = infer_call_return_type(callee, expr_map, ssa);
    let result = ssa.new_tmp("call");
    let block = BasicBlock {
        label: step_label.clone(),
        instrs: vec![format!("exec call#{body}")],
        terminator: format!("br {next_label}"),
    };
    let llvm_block = LlvmBlock {
        label: step_label,
        instrs: vec![
            LlvmInstr::Comment(format!("exec call#{body}")),
            LlvmInstr::Call {
                result: Some(result.clone()),
                ret_ty: ret_ty.clone(),
                callee: INTRINSIC_CALL.into(),
                args: call_args,
            },
        ],
        terminator: LlvmTerminator::Br {
            target: next_label.to_string(),
        },
    };
    blocks.push(block);
    llvm_blocks.push(llvm_block);
    (blocks, llvm_blocks, Some((result, ret_ty)), false)
}

fn lower_binary_with_propagate_to_blocks(
    body: MirExprId,
    operator: &str,
    left: MirExprId,
    right: MirExprId,
    expr_map: &HashMap<MirExprId, &MirExpr>,
    ssa: &mut LlvmBuilder,
) -> (Vec<BasicBlock>, Vec<LlvmBlock>) {
    let mut blocks = Vec::new();
    let mut llvm_blocks = Vec::new();
    let mut step_label = "entry".to_string();
    let mut next_index = 0usize;
    let mut operands: Vec<String> = Vec::new();
    let mut operand_tys: Vec<String> = Vec::new();
    for expr_id in [left, right] {
        let next_label = format!("bin{}.step{}", body, next_index);
        next_index += 1;
        let (step_blocks, step_llvm_blocks, operand, terminated) =
            lower_expr_to_operand_blocks(
                step_label.clone(),
                expr_id,
                expr_map,
                ssa,
                &next_label,
            );
        blocks.extend(step_blocks);
        llvm_blocks.extend(step_llvm_blocks);
        if terminated {
            return (blocks, llvm_blocks);
        }
        if let Some((operand, ty)) = operand {
            operands.push(operand);
            operand_tys.push(ty);
        }
        step_label = next_label;
    }

    let lhs = operands.get(0).cloned().unwrap_or_else(|| "0".into());
    let rhs = operands.get(1).cloned().unwrap_or_else(|| "0".into());
    let result = ssa.new_tmp("bin");
    let op = match operator {
        "+" => "add",
        "-" => "sub",
        "*" => "mul",
        "/" => "sdiv",
        "%" => "srem",
        _ => "add",
    };
    let mut instrs = vec![LlvmInstr::Comment(format!("exec binary#{body}"))];
    let mut lhs_operand = lhs.clone();
    let mut rhs_operand = rhs.clone();
    if operand_tys.get(0).map(|ty| ty.as_str()) != Some("i64") {
        let cast = ssa.new_tmp("lhs_i64");
        instrs.push(LlvmInstr::Call {
            result: Some(cast.clone()),
            ret_ty: "i64".into(),
            callee: INTRINSIC_VALUE.into(),
            args: vec![("i64".into(), lhs_operand.clone())],
        });
        lhs_operand = cast;
    }
    if operand_tys.get(1).map(|ty| ty.as_str()) != Some("i64") {
        let cast = ssa.new_tmp("rhs_i64");
        instrs.push(LlvmInstr::Call {
            result: Some(cast.clone()),
            ret_ty: "i64".into(),
            callee: INTRINSIC_VALUE.into(),
            args: vec![("i64".into(), rhs_operand.clone())],
        });
        rhs_operand = cast;
    }
    instrs.push(LlvmInstr::BinOp {
        result: result.clone(),
        op: op.into(),
        ty: "i64".into(),
        lhs: lhs_operand,
        rhs: rhs_operand,
    });
    let block = BasicBlock {
        label: step_label.clone(),
        instrs: vec![format!("exec binary#{body}")],
        terminator: format!("ret {result}"),
    };
    let llvm_block = LlvmBlock {
        label: step_label,
        instrs,
        terminator: LlvmTerminator::Ret(Some(result)),
    };
    blocks.push(block);
    llvm_blocks.push(llvm_block);
    (blocks, llvm_blocks)
}

fn lower_binary_with_propagate_to_operand_blocks(
    label: String,
    body: MirExprId,
    operator: &str,
    left: MirExprId,
    right: MirExprId,
    expr_map: &HashMap<MirExprId, &MirExpr>,
    ssa: &mut LlvmBuilder,
    next_label: &str,
) -> (Vec<BasicBlock>, Vec<LlvmBlock>, Option<(String, String)>, bool) {
    let mut blocks = Vec::new();
    let mut llvm_blocks = Vec::new();
    let mut step_label = label;
    let mut next_index = 0usize;
    let mut operands: Vec<String> = Vec::new();
    let mut operand_tys: Vec<String> = Vec::new();
    for expr_id in [left, right] {
        let next_step_label = format!("bin{}.step{}", body, next_index);
        next_index += 1;
        let (step_blocks, step_llvm_blocks, operand, terminated) =
            lower_expr_to_operand_blocks(
                step_label.clone(),
                expr_id,
                expr_map,
                ssa,
                &next_step_label,
            );
        blocks.extend(step_blocks);
        llvm_blocks.extend(step_llvm_blocks);
        if terminated {
            return (blocks, llvm_blocks, None, true);
        }
        if let Some((operand, ty)) = operand {
            operands.push(operand);
            operand_tys.push(ty);
        }
        step_label = next_step_label;
    }

    let lhs = operands.get(0).cloned().unwrap_or_else(|| "0".into());
    let rhs = operands.get(1).cloned().unwrap_or_else(|| "0".into());
    let result = ssa.new_tmp("bin");
    let op = match operator {
        "+" => "add",
        "-" => "sub",
        "*" => "mul",
        "/" => "sdiv",
        "%" => "srem",
        _ => "add",
    };
    let mut instrs = vec![LlvmInstr::Comment(format!("exec binary#{body}"))];
    let mut lhs_operand = lhs.clone();
    let mut rhs_operand = rhs.clone();
    if operand_tys.get(0).map(|ty| ty.as_str()) != Some("i64") {
        let cast = ssa.new_tmp("lhs_i64");
        instrs.push(LlvmInstr::Call {
            result: Some(cast.clone()),
            ret_ty: "i64".into(),
            callee: INTRINSIC_VALUE.into(),
            args: vec![("i64".into(), lhs_operand.clone())],
        });
        lhs_operand = cast;
    }
    if operand_tys.get(1).map(|ty| ty.as_str()) != Some("i64") {
        let cast = ssa.new_tmp("rhs_i64");
        instrs.push(LlvmInstr::Call {
            result: Some(cast.clone()),
            ret_ty: "i64".into(),
            callee: INTRINSIC_VALUE.into(),
            args: vec![("i64".into(), rhs_operand.clone())],
        });
        rhs_operand = cast;
    }
    instrs.push(LlvmInstr::BinOp {
        result: result.clone(),
        op: op.into(),
        ty: "i64".into(),
        lhs: lhs_operand,
        rhs: rhs_operand,
    });
    let block = BasicBlock {
        label: step_label.clone(),
        instrs: vec![format!("exec binary#{body}")],
        terminator: format!("br {next_label}"),
    };
    let llvm_block = LlvmBlock {
        label: step_label,
        instrs,
        terminator: LlvmTerminator::Br {
            target: next_label.to_string(),
        },
    };
    blocks.push(block);
    llvm_blocks.push(llvm_block);
    (blocks, llvm_blocks, Some((result, "i64".into())), false)
}

fn lower_if_else_with_propagate_to_blocks(
    body: MirExprId,
    condition: MirExprId,
    then_branch: MirExprId,
    else_branch: MirExprId,
    expr_map: &HashMap<MirExprId, &MirExpr>,
    ssa: &mut LlvmBuilder,
) -> (Vec<BasicBlock>, Vec<LlvmBlock>) {
    let end_label = format!("ifelse{}.end", body);
    let then_label = format!("ifelse{}.then", body);
    let else_label = format!("ifelse{}.else", body);
    let then_kind = classify_branch_kind(then_branch, expr_map);
    let else_kind = classify_branch_kind(else_branch, expr_map);
    let then_ty = expr_map
        .get(&then_branch)
        .map(|expr| expr.ty.as_str())
        .unwrap_or("unknown");
    let else_ty = expr_map
        .get(&else_branch)
        .map(|expr| expr.ty.as_str())
        .unwrap_or("unknown");
    let result_type = if then_ty == else_ty {
        then_ty.to_string()
    } else {
        ssa.pointer_type()
    };

    let (cond, mut cond_instrs) = emit_bool_expr(condition, expr_map, ssa);
    cond_instrs.insert(
        0,
        LlvmInstr::Comment(format!("ifelse#{body} cond -> {then_label}/{else_label}")),
    );
    let entry_block = BasicBlock {
        label: "entry".into(),
        instrs: vec![format!("exec ifelse#{body} cond")],
        terminator: format!("br_if {cond} then {then_label} else {else_label}"),
    };
    let entry_llvm_block = LlvmBlock {
        label: "entry".into(),
        instrs: cond_instrs,
        terminator: LlvmTerminator::BrCond {
            cond: cond.clone(),
            then_bb: then_label.clone(),
            else_bb: else_label.clone(),
        },
    };

    let mut blocks = vec![entry_block];
    let mut llvm_blocks = vec![entry_llvm_block];
    let mut phi_sources: Vec<(String, String)> = Vec::new();

    match then_kind {
        BranchKind::Normal => {
            let (block, llvm_block, phi_source) = lower_if_else_branch_value(
                then_label.clone(),
                then_branch,
                expr_map,
                &result_type,
                &end_label,
                ssa,
            );
            blocks.push(block);
            llvm_blocks.push(llvm_block);
            phi_sources.push(phi_source);
        }
        BranchKind::Propagate => {
            let value = emit_value_expr(then_branch, expr_map, ssa);
            let (prop_blocks, prop_llvm_blocks, phi_source) =
                lower_propagate_value_to_if_blocks(
                    then_label.clone(),
                    then_branch,
                    value,
                    then_ty,
                    &result_type,
                    &end_label,
                    ssa,
                );
            blocks.extend(prop_blocks);
            llvm_blocks.extend(prop_llvm_blocks);
            phi_sources.push(phi_source);
        }
        BranchKind::Panic => {
            let value = emit_value_expr(then_branch, expr_map, ssa);
            let (block, llvm_block) =
                lower_panic_value_to_named_block(then_label.clone(), then_branch, value);
            blocks.push(block);
            llvm_blocks.push(llvm_block);
        }
    }

    match else_kind {
        BranchKind::Normal => {
            let (block, llvm_block, phi_source) = lower_if_else_branch_value(
                else_label.clone(),
                else_branch,
                expr_map,
                &result_type,
                &end_label,
                ssa,
            );
            blocks.push(block);
            llvm_blocks.push(llvm_block);
            phi_sources.push(phi_source);
        }
        BranchKind::Propagate => {
            let value = emit_value_expr(else_branch, expr_map, ssa);
            let (prop_blocks, prop_llvm_blocks, phi_source) =
                lower_propagate_value_to_if_blocks(
                    else_label.clone(),
                    else_branch,
                    value,
                    else_ty,
                    &result_type,
                    &end_label,
                    ssa,
                );
            blocks.extend(prop_blocks);
            llvm_blocks.extend(prop_llvm_blocks);
            phi_sources.push(phi_source);
        }
        BranchKind::Panic => {
            let value = emit_value_expr(else_branch, expr_map, ssa);
            let (block, llvm_block) =
                lower_panic_value_to_named_block(else_label.clone(), else_branch, value);
            blocks.push(block);
            llvm_blocks.push(llvm_block);
        }
    }

    if phi_sources.is_empty() {
        blocks.push(BasicBlock {
            label: end_label.clone(),
            instrs: vec![format!("ifelse#{body} end (unreachable)")],
            terminator: "unreachable".into(),
        });
        llvm_blocks.push(LlvmBlock {
            label: end_label,
            instrs: vec![LlvmInstr::Comment(format!(
                "ifelse#{body} end (unreachable)"
            ))],
            terminator: LlvmTerminator::Unreachable,
        });
        return (blocks, llvm_blocks);
    }

    let phi_inputs = format!(
        "[{}]",
        phi_sources
            .iter()
            .map(|(_, lbl)| lbl.clone())
            .collect::<Vec<_>>()
            .join(", ")
    );
    let phi_result = ssa.new_tmp("ifelse_result");
    blocks.push(BasicBlock {
        label: end_label.clone(),
        instrs: vec![format!(
            "phi ifelse_result : {} <- {}",
            result_type, phi_inputs
        )],
        terminator: "ret ifelse_result".into(),
    });
    llvm_blocks.push(LlvmBlock {
        label: end_label,
        instrs: vec![LlvmInstr::Phi {
            result: phi_result.clone(),
            ty: result_type,
            incomings: phi_sources,
        }],
        terminator: LlvmTerminator::Ret(Some(phi_result)),
    });
    (blocks, llvm_blocks)
}

fn lower_if_else_to_operand_blocks(
    label: String,
    body: MirExprId,
    condition: MirExprId,
    then_branch: MirExprId,
    else_branch: MirExprId,
    expr_map: &HashMap<MirExprId, &MirExpr>,
    ssa: &mut LlvmBuilder,
    next_label: &str,
) -> (Vec<BasicBlock>, Vec<LlvmBlock>, Option<(String, String)>, bool) {
    let end_label = format!("ifelse{}.end", body);
    let then_label = format!("ifelse{}.then", body);
    let else_label = format!("ifelse{}.else", body);
    let then_kind = classify_branch_kind(then_branch, expr_map);
    let else_kind = classify_branch_kind(else_branch, expr_map);
    let then_ty = expr_map
        .get(&then_branch)
        .map(|expr| expr.ty.as_str())
        .unwrap_or("unknown");
    let else_ty = expr_map
        .get(&else_branch)
        .map(|expr| expr.ty.as_str())
        .unwrap_or("unknown");
    let result_type = if then_ty == else_ty {
        then_ty.to_string()
    } else {
        ssa.pointer_type()
    };

    let (cond, mut cond_instrs) = emit_bool_expr(condition, expr_map, ssa);
    cond_instrs.insert(
        0,
        LlvmInstr::Comment(format!("ifelse#{body} cond -> {then_label}/{else_label}")),
    );
    let entry_block = BasicBlock {
        label: label.clone(),
        instrs: vec![format!("exec ifelse#{body} cond")],
        terminator: format!("br_if {cond} then {then_label} else {else_label}"),
    };
    let entry_llvm_block = LlvmBlock {
        label: label.clone(),
        instrs: cond_instrs,
        terminator: LlvmTerminator::BrCond {
            cond: cond.clone(),
            then_bb: then_label.clone(),
            else_bb: else_label.clone(),
        },
    };

    let mut blocks = vec![entry_block];
    let mut llvm_blocks = vec![entry_llvm_block];
    let mut phi_sources: Vec<(String, String)> = Vec::new();

    match then_kind {
        BranchKind::Normal => {
            let (block, llvm_block, phi_source) = lower_if_else_branch_value(
                then_label.clone(),
                then_branch,
                expr_map,
                &result_type,
                &end_label,
                ssa,
            );
            blocks.push(block);
            llvm_blocks.push(llvm_block);
            phi_sources.push(phi_source);
        }
        BranchKind::Propagate => {
            let value = emit_value_expr(then_branch, expr_map, ssa);
            let (prop_blocks, prop_llvm_blocks, phi_source) =
                lower_propagate_value_to_if_blocks(
                    then_label.clone(),
                    then_branch,
                    value,
                    then_ty,
                    &result_type,
                    &end_label,
                    ssa,
                );
            blocks.extend(prop_blocks);
            llvm_blocks.extend(prop_llvm_blocks);
            phi_sources.push(phi_source);
        }
        BranchKind::Panic => {
            let value = emit_value_expr(then_branch, expr_map, ssa);
            let (block, llvm_block) =
                lower_panic_value_to_named_block(then_label.clone(), then_branch, value);
            blocks.push(block);
            llvm_blocks.push(llvm_block);
        }
    }

    match else_kind {
        BranchKind::Normal => {
            let (block, llvm_block, phi_source) = lower_if_else_branch_value(
                else_label.clone(),
                else_branch,
                expr_map,
                &result_type,
                &end_label,
                ssa,
            );
            blocks.push(block);
            llvm_blocks.push(llvm_block);
            phi_sources.push(phi_source);
        }
        BranchKind::Propagate => {
            let value = emit_value_expr(else_branch, expr_map, ssa);
            let (prop_blocks, prop_llvm_blocks, phi_source) =
                lower_propagate_value_to_if_blocks(
                    else_label.clone(),
                    else_branch,
                    value,
                    else_ty,
                    &result_type,
                    &end_label,
                    ssa,
                );
            blocks.extend(prop_blocks);
            llvm_blocks.extend(prop_llvm_blocks);
            phi_sources.push(phi_source);
        }
        BranchKind::Panic => {
            let value = emit_value_expr(else_branch, expr_map, ssa);
            let (block, llvm_block) =
                lower_panic_value_to_named_block(else_label.clone(), else_branch, value);
            blocks.push(block);
            llvm_blocks.push(llvm_block);
        }
    }

    if phi_sources.is_empty() {
        return (blocks, llvm_blocks, None, true);
    }

    let phi_inputs = format!(
        "[{}]",
        phi_sources
            .iter()
            .map(|(_, lbl)| lbl.clone())
            .collect::<Vec<_>>()
            .join(", ")
    );
    let phi_result = ssa.new_tmp("ifelse_result");
    blocks.push(BasicBlock {
        label: end_label.clone(),
        instrs: vec![format!(
            "phi ifelse_result : {} <- {}",
            result_type, phi_inputs
        )],
        terminator: format!("br {next_label}"),
    });
    llvm_blocks.push(LlvmBlock {
        label: end_label,
        instrs: vec![LlvmInstr::Phi {
            result: phi_result.clone(),
            ty: result_type.clone(),
            incomings: phi_sources,
        }],
        terminator: LlvmTerminator::Br {
            target: next_label.to_string(),
        },
    });
    (
        blocks,
        llvm_blocks,
        Some((phi_result, result_type)),
        false,
    )
}

fn lower_block_tail_if_else_with_defer_to_blocks(
    body: MirExprId,
    condition: MirExprId,
    then_branch: MirExprId,
    else_branch: MirExprId,
    defer_lifo: &[MirExprId],
    expr_map: &HashMap<MirExprId, &MirExpr>,
    ssa: &mut LlvmBuilder,
) -> (Vec<BasicBlock>, Vec<LlvmBlock>) {
    let end_label = format!("block_ifelse{}.end", body);
    let then_label = format!("block_ifelse{}.then", body);
    let else_label = format!("block_ifelse{}.else", body);
    let then_kind = classify_branch_kind(then_branch, expr_map);
    let else_kind = classify_branch_kind(else_branch, expr_map);
    let then_ty = expr_map
        .get(&then_branch)
        .map(|expr| expr.ty.as_str())
        .unwrap_or("unknown");
    let else_ty = expr_map
        .get(&else_branch)
        .map(|expr| expr.ty.as_str())
        .unwrap_or("unknown");
    let result_type = if then_ty == else_ty {
        then_ty.to_string()
    } else {
        ssa.pointer_type()
    };

    let (cond, mut cond_instrs) = emit_bool_expr(condition, expr_map, ssa);
    cond_instrs.insert(
        0,
        LlvmInstr::Comment(format!(
            "block ifelse#{body} cond -> {then_label}/{else_label}"
        )),
    );
    let entry_block = BasicBlock {
        label: "entry".into(),
        instrs: vec![format!("exec block ifelse#{body} cond")],
        terminator: format!("br_if {cond} then {then_label} else {else_label}"),
    };
    let entry_llvm_block = LlvmBlock {
        label: "entry".into(),
        instrs: cond_instrs,
        terminator: LlvmTerminator::BrCond {
            cond: cond.clone(),
            then_bb: then_label.clone(),
            else_bb: else_label.clone(),
        },
    };

    let mut blocks = vec![entry_block];
    let mut llvm_blocks = vec![entry_llvm_block];
    let mut phi_sources: Vec<(String, String)> = Vec::new();

    match then_kind {
        BranchKind::Normal => {
            let (block, llvm_block, phi_source) = lower_if_else_branch_value_with_defers(
                then_label.clone(),
                then_branch,
                defer_lifo,
                expr_map,
                &result_type,
                &end_label,
                ssa,
            );
            blocks.push(block);
            llvm_blocks.push(llvm_block);
            phi_sources.push(phi_source);
        }
        BranchKind::Propagate => {
            let value = emit_value_expr(then_branch, expr_map, ssa);
            let (prop_blocks, prop_llvm_blocks, phi_source) =
                lower_block_propagate_with_defers_to_if_blocks(
                    then_label.clone(),
                    then_branch,
                    value,
                    then_ty,
                    &result_type,
                    &end_label,
                    defer_lifo,
                    expr_map,
                    ssa,
                );
            blocks.extend(prop_blocks);
            llvm_blocks.extend(prop_llvm_blocks);
            phi_sources.push(phi_source);
        }
        BranchKind::Panic => {
            let value = emit_value_expr(then_branch, expr_map, ssa);
            let (block, llvm_block) = lower_panic_value_to_named_block_with_defers(
                then_label.clone(),
                then_branch,
                value,
                defer_lifo,
                expr_map,
                ssa,
            );
            blocks.push(block);
            llvm_blocks.push(llvm_block);
        }
    }

    match else_kind {
        BranchKind::Normal => {
            let (block, llvm_block, phi_source) = lower_if_else_branch_value_with_defers(
                else_label.clone(),
                else_branch,
                defer_lifo,
                expr_map,
                &result_type,
                &end_label,
                ssa,
            );
            blocks.push(block);
            llvm_blocks.push(llvm_block);
            phi_sources.push(phi_source);
        }
        BranchKind::Propagate => {
            let value = emit_value_expr(else_branch, expr_map, ssa);
            let (prop_blocks, prop_llvm_blocks, phi_source) =
                lower_block_propagate_with_defers_to_if_blocks(
                    else_label.clone(),
                    else_branch,
                    value,
                    else_ty,
                    &result_type,
                    &end_label,
                    defer_lifo,
                    expr_map,
                    ssa,
                );
            blocks.extend(prop_blocks);
            llvm_blocks.extend(prop_llvm_blocks);
            phi_sources.push(phi_source);
        }
        BranchKind::Panic => {
            let value = emit_value_expr(else_branch, expr_map, ssa);
            let (block, llvm_block) = lower_panic_value_to_named_block_with_defers(
                else_label.clone(),
                else_branch,
                value,
                defer_lifo,
                expr_map,
                ssa,
            );
            blocks.push(block);
            llvm_blocks.push(llvm_block);
        }
    }

    if phi_sources.is_empty() {
        blocks.push(BasicBlock {
            label: end_label.clone(),
            instrs: vec![format!("block ifelse#{body} end (unreachable)")],
            terminator: "unreachable".into(),
        });
        llvm_blocks.push(LlvmBlock {
            label: end_label,
            instrs: vec![LlvmInstr::Comment(format!(
                "block ifelse#{body} end (unreachable)"
            ))],
            terminator: LlvmTerminator::Unreachable,
        });
        return (blocks, llvm_blocks);
    }

    let phi_inputs = format!(
        "[{}]",
        phi_sources
            .iter()
            .map(|(_, lbl)| lbl.clone())
            .collect::<Vec<_>>()
            .join(", ")
    );
    let phi_result = ssa.new_tmp("ifelse_result");
    blocks.push(BasicBlock {
        label: end_label.clone(),
        instrs: vec![format!(
            "phi ifelse_result : {} <- {}",
            result_type, phi_inputs
        )],
        terminator: "ret ifelse_result".into(),
    });
    llvm_blocks.push(LlvmBlock {
        label: end_label,
        instrs: vec![LlvmInstr::Phi {
            result: phi_result.clone(),
            ty: result_type,
            incomings: phi_sources,
        }],
        terminator: LlvmTerminator::Ret(Some(phi_result)),
    });
    (blocks, llvm_blocks)
}

fn lower_block_tail_if_else_with_defer_to_operand_blocks(
    label: String,
    body: MirExprId,
    condition: MirExprId,
    then_branch: MirExprId,
    else_branch: MirExprId,
    defer_lifo: &[MirExprId],
    expr_map: &HashMap<MirExprId, &MirExpr>,
    ssa: &mut LlvmBuilder,
    next_label: &str,
) -> (Vec<BasicBlock>, Vec<LlvmBlock>, Option<(String, String)>, bool) {
    let end_label = format!("block_ifelse{}.end", body);
    let then_label = format!("block_ifelse{}.then", body);
    let else_label = format!("block_ifelse{}.else", body);
    let then_kind = classify_branch_kind(then_branch, expr_map);
    let else_kind = classify_branch_kind(else_branch, expr_map);
    let then_ty = expr_map
        .get(&then_branch)
        .map(|expr| expr.ty.as_str())
        .unwrap_or("unknown");
    let else_ty = expr_map
        .get(&else_branch)
        .map(|expr| expr.ty.as_str())
        .unwrap_or("unknown");
    let result_type = if then_ty == else_ty {
        then_ty.to_string()
    } else {
        ssa.pointer_type()
    };

    let (cond, mut cond_instrs) = emit_bool_expr(condition, expr_map, ssa);
    cond_instrs.insert(
        0,
        LlvmInstr::Comment(format!(
            "block ifelse#{body} cond -> {then_label}/{else_label}"
        )),
    );
    let entry_block = BasicBlock {
        label: label.clone(),
        instrs: vec![format!("exec block ifelse#{body} cond")],
        terminator: format!("br_if {cond} then {then_label} else {else_label}"),
    };
    let entry_llvm_block = LlvmBlock {
        label: label.clone(),
        instrs: cond_instrs,
        terminator: LlvmTerminator::BrCond {
            cond: cond.clone(),
            then_bb: then_label.clone(),
            else_bb: else_label.clone(),
        },
    };

    let mut blocks = vec![entry_block];
    let mut llvm_blocks = vec![entry_llvm_block];
    let mut phi_sources: Vec<(String, String)> = Vec::new();

    match then_kind {
        BranchKind::Normal => {
            let (block, llvm_block, phi_source) = lower_if_else_branch_value_with_defers(
                then_label.clone(),
                then_branch,
                defer_lifo,
                expr_map,
                &result_type,
                &end_label,
                ssa,
            );
            blocks.push(block);
            llvm_blocks.push(llvm_block);
            phi_sources.push(phi_source);
        }
        BranchKind::Propagate => {
            let value = emit_value_expr(then_branch, expr_map, ssa);
            let (prop_blocks, prop_llvm_blocks, phi_source) =
                lower_block_propagate_with_defers_to_if_blocks(
                    then_label.clone(),
                    then_branch,
                    value,
                    then_ty,
                    &result_type,
                    &end_label,
                    defer_lifo,
                    expr_map,
                    ssa,
                );
            blocks.extend(prop_blocks);
            llvm_blocks.extend(prop_llvm_blocks);
            phi_sources.push(phi_source);
        }
        BranchKind::Panic => {
            let value = emit_value_expr(then_branch, expr_map, ssa);
            let (block, llvm_block) = lower_panic_value_to_named_block_with_defers(
                then_label.clone(),
                then_branch,
                value,
                defer_lifo,
                expr_map,
                ssa,
            );
            blocks.push(block);
            llvm_blocks.push(llvm_block);
        }
    }

    match else_kind {
        BranchKind::Normal => {
            let (block, llvm_block, phi_source) = lower_if_else_branch_value_with_defers(
                else_label.clone(),
                else_branch,
                defer_lifo,
                expr_map,
                &result_type,
                &end_label,
                ssa,
            );
            blocks.push(block);
            llvm_blocks.push(llvm_block);
            phi_sources.push(phi_source);
        }
        BranchKind::Propagate => {
            let value = emit_value_expr(else_branch, expr_map, ssa);
            let (prop_blocks, prop_llvm_blocks, phi_source) =
                lower_block_propagate_with_defers_to_if_blocks(
                    else_label.clone(),
                    else_branch,
                    value,
                    else_ty,
                    &result_type,
                    &end_label,
                    defer_lifo,
                    expr_map,
                    ssa,
                );
            blocks.extend(prop_blocks);
            llvm_blocks.extend(prop_llvm_blocks);
            phi_sources.push(phi_source);
        }
        BranchKind::Panic => {
            let value = emit_value_expr(else_branch, expr_map, ssa);
            let (block, llvm_block) = lower_panic_value_to_named_block_with_defers(
                else_label.clone(),
                else_branch,
                value,
                defer_lifo,
                expr_map,
                ssa,
            );
            blocks.push(block);
            llvm_blocks.push(llvm_block);
        }
    }

    if phi_sources.is_empty() {
        return (blocks, llvm_blocks, None, true);
    }

    let phi_inputs = format!(
        "[{}]",
        phi_sources
            .iter()
            .map(|(_, lbl)| lbl.clone())
            .collect::<Vec<_>>()
            .join(", ")
    );
    let phi_result = ssa.new_tmp("ifelse_result");
    blocks.push(BasicBlock {
        label: end_label.clone(),
        instrs: vec![format!(
            "phi ifelse_result : {} <- {}",
            result_type, phi_inputs
        )],
        terminator: format!("br {next_label}"),
    });
    llvm_blocks.push(LlvmBlock {
        label: end_label,
        instrs: vec![LlvmInstr::Phi {
            result: phi_result.clone(),
            ty: result_type.clone(),
            incomings: phi_sources,
        }],
        terminator: LlvmTerminator::Br {
            target: next_label.to_string(),
        },
    });
    (
        blocks,
        llvm_blocks,
        Some((phi_result, result_type)),
        false,
    )
}

fn lower_if_else_branch_value_with_defers(
    label: String,
    expr_id: MirExprId,
    defer_lifo: &[MirExprId],
    expr_map: &HashMap<MirExprId, &MirExpr>,
    result_type: &str,
    end_label: &str,
    ssa: &mut LlvmBuilder,
) -> (BasicBlock, LlvmBlock, (String, String)) {
    let value = emit_value_expr(expr_id, expr_map, ssa);
    let result = ssa.new_tmp("ifelse_result");
    let mut instrs = Vec::new();
    instrs.push(LlvmInstr::Comment(format!("exec expr#{expr_id}")));
    instrs.extend(value.instrs);
    emit_defer_lifo_instrs(defer_lifo, expr_map, ssa, &mut instrs);
    instrs.push(LlvmInstr::Call {
        result: Some(result.clone()),
        ret_ty: result_type.to_string(),
        callee: INTRINSIC_VALUE.into(),
        args: vec![(result_type.to_string(), value.operand)],
    });
    let block = BasicBlock {
        label: label.clone(),
        instrs: vec![format!("exec expr#{expr_id}")],
        terminator: format!("br {end_label}"),
    };
    let llvm_block = LlvmBlock {
        label: label.clone(),
        instrs,
        terminator: LlvmTerminator::Br {
            target: end_label.to_string(),
        },
    };
    (block, llvm_block, (result, label))
}

fn lower_block_propagate_with_defers_to_if_blocks(
    label: String,
    body: MirExprId,
    value: EmittedValue,
    ty_hint: &str,
    result_type: &str,
    end_label: &str,
    defer_lifo: &[MirExprId],
    expr_map: &HashMap<MirExprId, &MirExpr>,
    ssa: &mut LlvmBuilder,
) -> (Vec<BasicBlock>, Vec<LlvmBlock>, (String, String)) {
    let ok_label = format!("{label}.ok");
    let err_label = format!("{label}.err");
    let cond_label = format!("{label}.cond");
    let flavor = infer_propagate_flavor(ty_hint);
    let residual = value.operand.clone();
    let mut entry_instrs = vec![LlvmInstr::Comment(format!("exec propagate#{body}"))];
    entry_instrs.extend(value.instrs);
    let cond = ssa.new_tmp("propagate_ok");
    match flavor {
        PropagateFlavor::Option => {
            entry_instrs.push(LlvmInstr::Comment(format!(
                "{cond_label}: check Some/None"
            )));
            entry_instrs.push(LlvmInstr::Icmp {
                result: cond.clone(),
                pred: "ne".into(),
                ty: ssa.pointer_type(),
                lhs: residual.clone(),
                rhs: "null".into(),
            });
        }
        PropagateFlavor::Result => {
            entry_instrs.push(LlvmInstr::Comment(format!(
                "{cond_label}: check Ok/Err"
            )));
            entry_instrs.push(LlvmInstr::Call {
                result: Some(cond.clone()),
                ret_ty: ssa.bool_type(),
                callee: intrinsic_is_ctor("Ok"),
                args: vec![(ssa.pointer_type(), residual.clone())],
            });
        }
    }
    let entry_block = BasicBlock {
        label: label.clone(),
        instrs: vec![format!("exec propagate#{body}")],
        terminator: format!("br_if {cond} then {ok_label} else {err_label}"),
    };
    let entry_llvm_block = LlvmBlock {
        label: label.clone(),
        instrs: entry_instrs,
        terminator: LlvmTerminator::BrCond {
            cond: cond.clone(),
            then_bb: ok_label.clone(),
            else_bb: err_label.clone(),
        },
    };

    let payload = ssa.new_tmp("propagate_payload");
    let result = ssa.new_tmp("ifelse_result");
    let ctor_name = match flavor {
        PropagateFlavor::Option => "Some",
        PropagateFlavor::Result => "Ok",
    };
    let mut ok_instrs = vec![LlvmInstr::Comment(format!(
        "propagate ok#{body} -> payload"
    ))];
    ok_instrs.push(LlvmInstr::Call {
        result: Some(payload.clone()),
        ret_ty: ssa.pointer_type(),
        callee: intrinsic_ctor_payload(ctor_name),
        args: vec![(ssa.pointer_type(), residual.clone())],
    });
    emit_defer_lifo_instrs(defer_lifo, expr_map, ssa, &mut ok_instrs);
    ok_instrs.push(LlvmInstr::Call {
        result: Some(result.clone()),
        ret_ty: result_type.to_string(),
        callee: INTRINSIC_VALUE.into(),
        args: vec![(result_type.to_string(), payload)],
    });
    let ok_block = BasicBlock {
        label: ok_label.clone(),
        instrs: vec![format!("propagate ok#{body} -> {end_label}")],
        terminator: format!("br {end_label}"),
    };
    let ok_llvm_block = LlvmBlock {
        label: ok_label.clone(),
        instrs: ok_instrs,
        terminator: LlvmTerminator::Br {
            target: end_label.to_string(),
        },
    };

    let mut err_instrs = vec![LlvmInstr::Comment(format!(
        "propagate err#{body} -> return residual"
    ))];
    emit_defer_lifo_instrs(defer_lifo, expr_map, ssa, &mut err_instrs);
    let err_block = BasicBlock {
        label: err_label.clone(),
        instrs: vec![format!("propagate err#{body} -> return residual")],
        terminator: format!("ret {residual}"),
    };
    let err_llvm_block = LlvmBlock {
        label: err_label,
        instrs: err_instrs,
        terminator: LlvmTerminator::Ret(Some(residual)),
    };

    (
        vec![entry_block, ok_block, err_block],
        vec![entry_llvm_block, ok_llvm_block, err_llvm_block],
        (result, ok_label),
    )
}

fn lower_panic_value_to_named_block_with_defers(
    label: String,
    body: MirExprId,
    value: EmittedValue,
    defer_lifo: &[MirExprId],
    expr_map: &HashMap<MirExprId, &MirExpr>,
    ssa: &mut LlvmBuilder,
) -> (BasicBlock, LlvmBlock) {
    let mut instrs = value.instrs;
    emit_defer_lifo_instrs(defer_lifo, expr_map, ssa, &mut instrs);
    instrs.push(LlvmInstr::Comment(format!(
        "panic expr#{body} -> {INTRINSIC_PANIC}"
    )));
    instrs.push(LlvmInstr::Call {
        result: None,
        ret_ty: "void".into(),
        callee: INTRINSIC_PANIC.into(),
        args: vec![(value.ty, value.operand)],
    });
    let block = BasicBlock {
        label: label.clone(),
        instrs: vec![format!("panic expr#{body}")],
        terminator: "unreachable".into(),
    };
    let llvm_block = LlvmBlock {
        label,
        instrs: {
            let mut buf = vec![LlvmInstr::Comment(format!("panic expr#{body}"))];
            buf.extend(instrs);
            buf
        },
        terminator: LlvmTerminator::Unreachable,
    };
    (block, llvm_block)
}
fn lower_if_else_branch_value(
    label: String,
    expr_id: MirExprId,
    expr_map: &HashMap<MirExprId, &MirExpr>,
    result_type: &str,
    end_label: &str,
    ssa: &mut LlvmBuilder,
) -> (BasicBlock, LlvmBlock, (String, String)) {
    let value = emit_value_expr(expr_id, expr_map, ssa);
    let result = ssa.new_tmp("ifelse_result");
    let mut instrs = Vec::new();
    instrs.push(LlvmInstr::Comment(format!("exec expr#{expr_id}")));
    instrs.extend(value.instrs);
    instrs.push(LlvmInstr::Call {
        result: Some(result.clone()),
        ret_ty: result_type.to_string(),
        callee: INTRINSIC_VALUE.into(),
        args: vec![(result_type.to_string(), value.operand)],
    });
    let block = BasicBlock {
        label: label.clone(),
        instrs: vec![format!("exec expr#{expr_id}")],
        terminator: format!("br {end_label}"),
    };
    let llvm_block = LlvmBlock {
        label: label.clone(),
        instrs,
        terminator: LlvmTerminator::Br {
            target: end_label.to_string(),
        },
    };
    (block, llvm_block, (result, label))
}

#[derive(Clone, Copy, Debug)]
enum PropagateFlavor {
    Result,
    Option,
}

fn infer_propagate_flavor(ty_hint: &str) -> PropagateFlavor {
    let trimmed = ty_hint.trim();
    if trimmed.starts_with("Option") || trimmed.contains("Option<") {
        return PropagateFlavor::Option;
    }
    if trimmed.starts_with("Result") || trimmed.contains("Result<") {
        return PropagateFlavor::Result;
    }
    PropagateFlavor::Result
}

fn lower_panic_value_to_blocks(
    body: MirExprId,
    value: EmittedValue,
    ssa: &mut LlvmBuilder,
) -> (Vec<BasicBlock>, Vec<LlvmBlock>) {
    let mut instrs = value.instrs;
    instrs.push(LlvmInstr::Comment(format!(
        "panic expr#{body} -> {INTRINSIC_PANIC}"
    )));
    instrs.push(LlvmInstr::Call {
        result: None,
        ret_ty: "void".into(),
        callee: INTRINSIC_PANIC.into(),
        args: vec![(value.ty, value.operand)],
    });
    let block = BasicBlock {
        label: "entry".into(),
        instrs: vec![format!("panic expr#{body}")],
        terminator: "unreachable".into(),
    };
    let llvm_block = LlvmBlock {
        label: "entry".into(),
        instrs: {
            let mut buf = vec![LlvmInstr::Comment(format!("panic expr#{body}"))];
            buf.extend(instrs);
            buf
        },
        terminator: LlvmTerminator::Unreachable,
    };
    (vec![block], vec![llvm_block])
}

fn lower_panic_value_to_named_block(
    label: String,
    body: MirExprId,
    value: EmittedValue,
) -> (BasicBlock, LlvmBlock) {
    let mut instrs = value.instrs;
    instrs.push(LlvmInstr::Comment(format!(
        "panic expr#{body} -> {INTRINSIC_PANIC}"
    )));
    instrs.push(LlvmInstr::Call {
        result: None,
        ret_ty: "void".into(),
        callee: INTRINSIC_PANIC.into(),
        args: vec![(value.ty, value.operand)],
    });
    let block = BasicBlock {
        label: label.clone(),
        instrs: vec![format!("panic expr#{body}")],
        terminator: "unreachable".into(),
    };
    let llvm_block = LlvmBlock {
        label,
        instrs: {
            let mut buf = vec![LlvmInstr::Comment(format!("panic expr#{body}"))];
            buf.extend(instrs);
            buf
        },
        terminator: LlvmTerminator::Unreachable,
    };
    (block, llvm_block)
}

fn lower_propagate_value_to_blocks(
    body: MirExprId,
    value: EmittedValue,
    ty_hint: &str,
    ssa: &mut LlvmBuilder,
) -> (Vec<BasicBlock>, Vec<LlvmBlock>) {
    let ok_label = format!("propagate.ok#{body}");
    let err_label = format!("propagate.err#{body}");
    let cond_label = format!("propagate.cond#{body}");
    let flavor = infer_propagate_flavor(ty_hint);
    let residual = value.operand.clone();
    let mut entry_instrs = vec![LlvmInstr::Comment(format!("exec propagate#{body}"))];
    entry_instrs.extend(value.instrs);
    let cond = ssa.new_tmp("propagate_ok");
    match flavor {
        PropagateFlavor::Option => {
            entry_instrs.push(LlvmInstr::Comment(format!(
                "{cond_label}: check Some/None"
            )));
            entry_instrs.push(LlvmInstr::Icmp {
                result: cond.clone(),
                pred: "ne".into(),
                ty: ssa.pointer_type(),
                lhs: value.operand.clone(),
                rhs: "null".into(),
            });
        }
        PropagateFlavor::Result => {
            entry_instrs.push(LlvmInstr::Comment(format!(
                "{cond_label}: check Ok/Err"
            )));
            entry_instrs.push(LlvmInstr::Call {
                result: Some(cond.clone()),
                ret_ty: ssa.bool_type(),
                callee: intrinsic_is_ctor("Ok"),
                args: vec![(ssa.pointer_type(), value.operand.clone())],
            });
        }
    }

    let entry_block = BasicBlock {
        label: "entry".into(),
        instrs: vec![format!("exec propagate#{body}")],
        terminator: format!("br_if {cond} then {ok_label} else {err_label}"),
    };
    let entry_llvm_block = LlvmBlock {
        label: "entry".into(),
        instrs: entry_instrs,
        terminator: LlvmTerminator::BrCond {
            cond: cond.clone(),
            then_bb: ok_label.clone(),
            else_bb: err_label.clone(),
        },
    };

    let payload = ssa.new_tmp("propagate_payload");
    let mut ok_instrs = vec![LlvmInstr::Comment(format!("propagate ok#{body} -> payload"))];
    let ctor_name = match flavor {
        PropagateFlavor::Option => "Some",
        PropagateFlavor::Result => "Ok",
    };
    ok_instrs.push(LlvmInstr::Call {
        result: Some(payload.clone()),
        ret_ty: ssa.pointer_type(),
        callee: intrinsic_ctor_payload(ctor_name),
        args: vec![(ssa.pointer_type(), value.operand.clone())],
    });
    let ok_block = BasicBlock {
        label: ok_label.clone(),
        instrs: vec![format!("propagate ok#{body} -> payload")],
        terminator: format!("ret {payload}"),
    };
    let ok_llvm_block = LlvmBlock {
        label: ok_label,
        instrs: ok_instrs,
        terminator: LlvmTerminator::Ret(Some(payload)),
    };

    let err_block = BasicBlock {
        label: err_label.clone(),
        instrs: vec![format!("propagate err#{body} -> return residual")],
        terminator: format!("ret {}", residual),
    };
    let err_llvm_block = LlvmBlock {
        label: err_label,
        instrs: vec![LlvmInstr::Comment(format!(
            "propagate err#{body} -> return residual"
        ))],
        terminator: LlvmTerminator::Ret(Some(residual)),
    };

    (
        vec![entry_block, ok_block, err_block],
        vec![entry_llvm_block, ok_llvm_block, err_llvm_block],
    )
}

fn lower_propagate_value_to_match_blocks(
    arm_index: usize,
    body: MirExprId,
    value: EmittedValue,
    ty_hint: &str,
    result_type: &str,
    end_label: &str,
    ssa: &mut LlvmBuilder,
) -> (Vec<BasicBlock>, Vec<LlvmBlock>, (String, String)) {
    let body_label = format!("arm{arm_index}.body#{body}");
    let ok_label = format!("arm{arm_index}.propagate_ok#{body}");
    let err_label = format!("arm{arm_index}.propagate_err#{body}");
    let cond_label = format!("arm{arm_index}.propagate_cond#{body}");
    let flavor = infer_propagate_flavor(ty_hint);
    let residual = value.operand.clone();
    let mut entry_instrs = vec![LlvmInstr::Comment(format!("exec propagate#{body}"))];
    entry_instrs.extend(value.instrs);
    let cond = ssa.new_tmp("propagate_ok");
    match flavor {
        PropagateFlavor::Option => {
            entry_instrs.push(LlvmInstr::Comment(format!(
                "{cond_label}: check Some/None"
            )));
            entry_instrs.push(LlvmInstr::Icmp {
                result: cond.clone(),
                pred: "ne".into(),
                ty: ssa.pointer_type(),
                lhs: residual.clone(),
                rhs: "null".into(),
            });
        }
        PropagateFlavor::Result => {
            entry_instrs.push(LlvmInstr::Comment(format!(
                "{cond_label}: check Ok/Err"
            )));
            entry_instrs.push(LlvmInstr::Call {
                result: Some(cond.clone()),
                ret_ty: ssa.bool_type(),
                callee: intrinsic_is_ctor("Ok"),
                args: vec![(ssa.pointer_type(), residual.clone())],
            });
        }
    }
    let entry_block = BasicBlock {
        label: body_label.clone(),
        instrs: vec![format!("exec propagate#{body}")],
        terminator: format!("br_if {cond} then {ok_label} else {err_label}"),
    };
    let entry_llvm_block = LlvmBlock {
        label: body_label,
        instrs: entry_instrs,
        terminator: LlvmTerminator::BrCond {
            cond: cond.clone(),
            then_bb: ok_label.clone(),
            else_bb: err_label.clone(),
        },
    };

    let payload = ssa.new_tmp("propagate_payload");
    let result = ssa.new_tmp("match_result");
    let ctor_name = match flavor {
        PropagateFlavor::Option => "Some",
        PropagateFlavor::Result => "Ok",
    };
    let mut ok_instrs = vec![LlvmInstr::Comment(format!(
        "propagate ok#{body} -> payload"
    ))];
    ok_instrs.push(LlvmInstr::Call {
        result: Some(payload.clone()),
        ret_ty: ssa.pointer_type(),
        callee: intrinsic_ctor_payload(ctor_name),
        args: vec![(ssa.pointer_type(), residual.clone())],
    });
    ok_instrs.push(LlvmInstr::Call {
        result: Some(result.clone()),
        ret_ty: result_type.to_string(),
        callee: INTRINSIC_VALUE.into(),
        args: vec![(result_type.to_string(), payload)],
    });
    let ok_block = BasicBlock {
        label: ok_label.clone(),
        instrs: vec![format!("propagate ok#{body} -> end")],
        terminator: format!("br {end_label}"),
    };
    let ok_llvm_block = LlvmBlock {
        label: ok_label,
        instrs: ok_instrs,
        terminator: LlvmTerminator::Br {
            target: end_label.to_string(),
        },
    };

    let err_block = BasicBlock {
        label: err_label.clone(),
        instrs: vec![format!("propagate err#{body} -> return residual")],
        terminator: format!("ret {residual}"),
    };
    let err_llvm_block = LlvmBlock {
        label: err_label,
        instrs: vec![LlvmInstr::Comment(format!(
            "propagate err#{body} -> return residual"
        ))],
        terminator: LlvmTerminator::Ret(Some(residual)),
    };

    (
        vec![entry_block, ok_block, err_block],
        vec![entry_llvm_block, ok_llvm_block, err_llvm_block],
        (result, format!("arm{arm_index}.propagate_ok#{body}")),
    )
}

fn lower_propagate_value_to_if_blocks(
    label: String,
    body: MirExprId,
    value: EmittedValue,
    ty_hint: &str,
    result_type: &str,
    end_label: &str,
    ssa: &mut LlvmBuilder,
) -> (Vec<BasicBlock>, Vec<LlvmBlock>, (String, String)) {
    let ok_label = format!("{label}.ok");
    let err_label = format!("{label}.err");
    let cond_label = format!("{label}.cond");
    let flavor = infer_propagate_flavor(ty_hint);
    let residual = value.operand.clone();
    let mut entry_instrs = vec![LlvmInstr::Comment(format!("exec propagate#{body}"))];
    entry_instrs.extend(value.instrs);
    let cond = ssa.new_tmp("propagate_ok");
    match flavor {
        PropagateFlavor::Option => {
            entry_instrs.push(LlvmInstr::Comment(format!(
                "{cond_label}: check Some/None"
            )));
            entry_instrs.push(LlvmInstr::Icmp {
                result: cond.clone(),
                pred: "ne".into(),
                ty: ssa.pointer_type(),
                lhs: residual.clone(),
                rhs: "null".into(),
            });
        }
        PropagateFlavor::Result => {
            entry_instrs.push(LlvmInstr::Comment(format!(
                "{cond_label}: check Ok/Err"
            )));
            entry_instrs.push(LlvmInstr::Call {
                result: Some(cond.clone()),
                ret_ty: ssa.bool_type(),
                callee: intrinsic_is_ctor("Ok"),
                args: vec![(ssa.pointer_type(), residual.clone())],
            });
        }
    }
    let entry_block = BasicBlock {
        label: label.clone(),
        instrs: vec![format!("exec propagate#{body}")],
        terminator: format!("br_if {cond} then {ok_label} else {err_label}"),
    };
    let entry_llvm_block = LlvmBlock {
        label: label.clone(),
        instrs: entry_instrs,
        terminator: LlvmTerminator::BrCond {
            cond: cond.clone(),
            then_bb: ok_label.clone(),
            else_bb: err_label.clone(),
        },
    };

    let payload = ssa.new_tmp("propagate_payload");
    let result = ssa.new_tmp("ifelse_result");
    let ctor_name = match flavor {
        PropagateFlavor::Option => "Some",
        PropagateFlavor::Result => "Ok",
    };
    let mut ok_instrs = vec![LlvmInstr::Comment(format!(
        "propagate ok#{body} -> payload"
    ))];
    ok_instrs.push(LlvmInstr::Call {
        result: Some(payload.clone()),
        ret_ty: ssa.pointer_type(),
        callee: intrinsic_ctor_payload(ctor_name),
        args: vec![(ssa.pointer_type(), residual.clone())],
    });
    ok_instrs.push(LlvmInstr::Call {
        result: Some(result.clone()),
        ret_ty: result_type.to_string(),
        callee: INTRINSIC_VALUE.into(),
        args: vec![(result_type.to_string(), payload)],
    });
    let ok_block = BasicBlock {
        label: ok_label.clone(),
        instrs: vec![format!("propagate ok#{body} -> {end_label}")],
        terminator: format!("br {end_label}"),
    };
    let ok_llvm_block = LlvmBlock {
        label: ok_label.clone(),
        instrs: ok_instrs,
        terminator: LlvmTerminator::Br {
            target: end_label.to_string(),
        },
    };

    let err_block = BasicBlock {
        label: err_label.clone(),
        instrs: vec![format!("propagate err#{body} -> return residual")],
        terminator: format!("ret {residual}"),
    };
    let err_llvm_block = LlvmBlock {
        label: err_label,
        instrs: vec![LlvmInstr::Comment(format!(
            "propagate err#{body} -> return residual"
        ))],
        terminator: LlvmTerminator::Ret(Some(residual)),
    };

    (
        vec![entry_block, ok_block, err_block],
        vec![entry_llvm_block, ok_llvm_block, err_llvm_block],
        (result, ok_label),
    )
}

fn lower_propagate_operand_to_blocks(
    label: String,
    body: MirExprId,
    value: EmittedValue,
    ty_hint: &str,
    next_label: &str,
    ssa: &mut LlvmBuilder,
) -> (Vec<BasicBlock>, Vec<LlvmBlock>, String) {
    let ok_label = format!("{label}.ok");
    let err_label = format!("{label}.err");
    let cond_label = format!("{label}.cond");
    let flavor = infer_propagate_flavor(ty_hint);
    let residual = value.operand.clone();
    let mut entry_instrs = vec![LlvmInstr::Comment(format!("exec propagate#{body}"))];
    entry_instrs.extend(value.instrs);
    let cond = ssa.new_tmp("propagate_ok");
    match flavor {
        PropagateFlavor::Option => {
            entry_instrs.push(LlvmInstr::Comment(format!(
                "{cond_label}: check Some/None"
            )));
            entry_instrs.push(LlvmInstr::Icmp {
                result: cond.clone(),
                pred: "ne".into(),
                ty: ssa.pointer_type(),
                lhs: residual.clone(),
                rhs: "null".into(),
            });
        }
        PropagateFlavor::Result => {
            entry_instrs.push(LlvmInstr::Comment(format!(
                "{cond_label}: check Ok/Err"
            )));
            entry_instrs.push(LlvmInstr::Call {
                result: Some(cond.clone()),
                ret_ty: ssa.bool_type(),
                callee: intrinsic_is_ctor("Ok"),
                args: vec![(ssa.pointer_type(), residual.clone())],
            });
        }
    }
    let entry_block = BasicBlock {
        label: label.clone(),
        instrs: vec![format!("exec propagate#{body}")],
        terminator: format!("br_if {cond} then {ok_label} else {err_label}"),
    };
    let entry_llvm_block = LlvmBlock {
        label: label.clone(),
        instrs: entry_instrs,
        terminator: LlvmTerminator::BrCond {
            cond: cond.clone(),
            then_bb: ok_label.clone(),
            else_bb: err_label.clone(),
        },
    };

    let payload = ssa.new_tmp("propagate_payload");
    let ctor_name = match flavor {
        PropagateFlavor::Option => "Some",
        PropagateFlavor::Result => "Ok",
    };
    let mut ok_instrs = vec![LlvmInstr::Comment(format!(
        "propagate ok#{body} -> payload"
    ))];
    ok_instrs.push(LlvmInstr::Call {
        result: Some(payload.clone()),
        ret_ty: ssa.pointer_type(),
        callee: intrinsic_ctor_payload(ctor_name),
        args: vec![(ssa.pointer_type(), residual.clone())],
    });
    let ok_block = BasicBlock {
        label: ok_label.clone(),
        instrs: vec![format!("propagate ok#{body} -> {next_label}")],
        terminator: format!("br {next_label}"),
    };
    let ok_llvm_block = LlvmBlock {
        label: ok_label.clone(),
        instrs: ok_instrs,
        terminator: LlvmTerminator::Br {
            target: next_label.to_string(),
        },
    };

    let err_block = BasicBlock {
        label: err_label.clone(),
        instrs: vec![format!("propagate err#{body} -> return residual")],
        terminator: format!("ret {residual}"),
    };
    let err_llvm_block = LlvmBlock {
        label: err_label,
        instrs: vec![LlvmInstr::Comment(format!(
            "propagate err#{body} -> return residual"
        ))],
        terminator: LlvmTerminator::Ret(Some(residual)),
    };

    (
        vec![entry_block, ok_block, err_block],
        vec![entry_llvm_block, ok_llvm_block, err_llvm_block],
        payload,
    )
}

fn lower_expr_to_operand_blocks(
    label: String,
    expr_id: MirExprId,
    expr_map: &HashMap<MirExprId, &MirExpr>,
    ssa: &mut LlvmBuilder,
    next_label: &str,
) -> (Vec<BasicBlock>, Vec<LlvmBlock>, Option<(String, String)>, bool) {
    let Some(expr) = expr_map.get(&expr_id) else {
        let block = BasicBlock {
            label: label.clone(),
            instrs: vec![format!("exec expr#{expr_id} (missing)")],
            terminator: format!("br {next_label}"),
        };
        let llvm_block = LlvmBlock {
            label,
            instrs: vec![LlvmInstr::Comment(format!(
                "expr#{expr_id} missing -> fallback"
            ))],
            terminator: LlvmTerminator::Br {
                target: next_label.to_string(),
            },
        };
        return (
            vec![block],
            vec![llvm_block],
            Some((format!("#{}", expr_id), ssa.pointer_type())),
            false,
        );
    };

    match &expr.kind {
        MirExprKind::Propagate { .. } => {
            let value = emit_value_expr(expr_id, expr_map, ssa);
            let (blocks, llvm_blocks, payload) = lower_propagate_operand_to_blocks(
                label,
                expr_id,
                value,
                expr.ty.as_str(),
                next_label,
                ssa,
            );
            (
                blocks,
                llvm_blocks,
                Some((payload, ssa.pointer_type())),
                false,
            )
        }
        MirExprKind::Panic { .. } => {
            let value = emit_value_expr(expr_id, expr_map, ssa);
            let (block, llvm_block) =
                lower_panic_value_to_named_block(label, expr_id, value);
            (vec![block], vec![llvm_block], None, true)
        }
        MirExprKind::IfElse {
            condition,
            then_branch,
            else_branch,
        } if expr_contains_early_exit(expr_id, expr_map) => {
            lower_if_else_to_operand_blocks(
                label,
                expr_id,
                *condition,
                *then_branch,
                *else_branch,
                expr_map,
                ssa,
                next_label,
            )
        }
        MirExprKind::Call { callee, args }
            if expr_contains_early_exit(expr_id, expr_map) =>
        {
            lower_call_with_propagate_to_operand_blocks(
                label,
                expr_id,
                *callee,
                args,
                expr_map,
                ssa,
                next_label,
            )
        }
        MirExprKind::Binary {
            operator,
            left,
            right,
        } if matches!(operator.as_str(), "+" | "-" | "*" | "/" | "%")
            && expr_contains_early_exit(expr_id, expr_map) =>
        {
            lower_binary_with_propagate_to_operand_blocks(
                label,
                expr_id,
                operator,
                *left,
                *right,
                expr_map,
                ssa,
                next_label,
            )
        }
        MirExprKind::Block {
            tail: Some(tail_id),
            defer_lifo,
            ..
        } => {
            if let Some(tail_expr) = expr_map.get(tail_id) {
                if let MirExprKind::IfElse {
                    condition,
                    then_branch,
                    else_branch,
                } = &tail_expr.kind
                {
                    if expr_contains_early_exit(*tail_id, expr_map) {
                        if !defer_lifo.is_empty() {
                            return lower_block_tail_if_else_with_defer_to_operand_blocks(
                                label,
                                expr_id,
                                *condition,
                                *then_branch,
                                *else_branch,
                                defer_lifo,
                                expr_map,
                                ssa,
                                next_label,
                            );
                        }
                        return lower_if_else_to_operand_blocks(
                            label,
                            *tail_id,
                            *condition,
                            *then_branch,
                            *else_branch,
                            expr_map,
                            ssa,
                            next_label,
                        );
                    }
                }
            }
            match classify_branch_kind(expr_id, expr_map) {
                BranchKind::Propagate => {
                    let value = emit_value_expr(expr_id, expr_map, ssa);
                    let (blocks, llvm_blocks, payload) = lower_propagate_operand_to_blocks(
                        label,
                        expr_id,
                        value,
                        expr.ty.as_str(),
                        next_label,
                        ssa,
                    );
                    return (
                        blocks,
                        llvm_blocks,
                        Some((payload, ssa.pointer_type())),
                        false,
                    );
                }
                BranchKind::Panic => {
                    let value = emit_value_expr(expr_id, expr_map, ssa);
                    let (block, llvm_block) =
                        lower_panic_value_to_named_block(label, expr_id, value);
                    return (vec![block], vec![llvm_block], None, true);
                }
                BranchKind::Normal => {}
            }
            let value = emit_value_expr(expr_id, expr_map, ssa);
            let block = BasicBlock {
                label: label.clone(),
                instrs: vec![format!("exec expr#{expr_id}")],
                terminator: format!("br {next_label}"),
            };
            let llvm_block = LlvmBlock {
                label,
                instrs: {
                    let mut instrs = vec![LlvmInstr::Comment(format!(
                        "exec expr#{expr_id}"
                    ))];
                    instrs.extend(value.instrs);
                    instrs
                },
                terminator: LlvmTerminator::Br {
                    target: next_label.to_string(),
                },
            };
            (
                vec![block],
                vec![llvm_block],
                Some((value.operand, value.ty)),
                false,
            )
        }
        _ => {
            let value = emit_value_expr(expr_id, expr_map, ssa);
            let block = BasicBlock {
                label: label.clone(),
                instrs: vec![format!("exec expr#{expr_id}")],
                terminator: format!("br {next_label}"),
            };
            let llvm_block = LlvmBlock {
                label,
                instrs: {
                    let mut instrs = vec![LlvmInstr::Comment(format!(
                        "exec expr#{expr_id}"
                    ))];
                    instrs.extend(value.instrs);
                    instrs
                },
                terminator: LlvmTerminator::Br {
                    target: next_label.to_string(),
                },
            };
            (
                vec![block],
                vec![llvm_block],
                Some((value.operand, value.ty)),
                false,
            )
        }
    }
}

fn emit_guard_cond(
    expr_id: MirExprId,
    expr_map: &HashMap<MirExprId, &MirExpr>,
    ssa: &mut LlvmBuilder,
) -> (String, Vec<LlvmInstr>) {
    emit_bool_expr(expr_id, expr_map, ssa)
}

#[derive(Clone, Debug)]
struct EmittedValue {
    ty: String,
    operand: String,
    instrs: Vec<LlvmInstr>,
}

fn emit_unit_value(ssa: &LlvmBuilder) -> EmittedValue {
    EmittedValue {
        ty: ssa.pointer_type(),
        operand: "null".into(),
        instrs: vec![LlvmInstr::Comment("unit -> null pointer".into())],
    }
}

fn emit_defer_lifo_instrs(
    defer_lifo: &[MirExprId],
    expr_map: &HashMap<MirExprId, &MirExpr>,
    ssa: &mut LlvmBuilder,
    instrs: &mut Vec<LlvmInstr>,
) {
    // return / propagate / panic の終端挿入でも同じ順序で評価する。
    for defer_id in defer_lifo {
        let defer_value = emit_value_expr(*defer_id, expr_map, ssa);
        instrs.extend(defer_value.instrs);
        instrs.push(LlvmInstr::Comment(format!("defer_lifo expr#{defer_id}")));
    }
}

fn emit_bool_expr(
    expr_id: MirExprId,
    expr_map: &HashMap<MirExprId, &MirExpr>,
    ssa: &mut LlvmBuilder,
) -> (String, Vec<LlvmInstr>) {
    let expr = match expr_map.get(&expr_id) {
        Some(expr) => expr,
        None => {
            return (
                "true".into(),
                vec![LlvmInstr::Comment(format!(
                    "guard#{}: expr not found -> assume true",
                    expr_id
                ))],
            );
        }
    };

    match &expr.kind {
        MirExprKind::Literal { summary } => {
            if let Some(value) = extract_literal_operand(summary) {
                if value == "true" || value == "false" {
                    return (
                        value.clone(),
                        vec![LlvmInstr::Comment(format!(
                            "guard literal {summary} -> {value}"
                        ))],
                    );
                }
            }
            let value = emit_value_expr(expr_id, expr_map, ssa);
            let cond = ssa.new_tmp("guard");
            let mut instrs = value.instrs;
            let ty = value.ty.clone();
            let rhs = match ty.as_str() {
                "i64" => "0".into(),
                _ => "null".into(),
            };
            instrs.push(LlvmInstr::Comment(format!(
                "guard expr#{expr_id} -> truthy check"
            )));
            instrs.push(LlvmInstr::Icmp {
                result: cond.clone(),
                pred: "ne".into(),
                ty,
                lhs: value.operand,
                rhs,
            });
            (cond, instrs)
        }
        MirExprKind::Binary {
            operator,
            left,
            right,
        } => match operator.as_str() {
            "&&" | "and" => {
                let (lhs, mut instrs) = emit_bool_expr(*left, expr_map, ssa);
                let (rhs, mut rhs_instrs) = emit_bool_expr(*right, expr_map, ssa);
                instrs.append(&mut rhs_instrs);
                let cond = ssa.new_tmp("guard");
                instrs.push(LlvmInstr::And {
                    result: cond.clone(),
                    lhs,
                    rhs,
                });
                (cond, instrs)
            }
            "||" | "or" => {
                let (lhs, mut instrs) = emit_bool_expr(*left, expr_map, ssa);
                let (rhs, mut rhs_instrs) = emit_bool_expr(*right, expr_map, ssa);
                instrs.append(&mut rhs_instrs);
                let cond = ssa.new_tmp("guard");
                instrs.push(LlvmInstr::Or {
                    result: cond.clone(),
                    lhs,
                    rhs,
                });
                (cond, instrs)
            }
            "==" | "!=" | "<" | "<=" | ">" | ">=" => {
                let lhs = emit_value_expr(*left, expr_map, ssa);
                let rhs = emit_value_expr(*right, expr_map, ssa);
                let mut instrs = lhs.instrs;
                instrs.extend(rhs.instrs);
                let cond = ssa.new_tmp("guard");
                let pred = match operator.as_str() {
                    "==" => "eq",
                    "!=" => "ne",
                    "<" => "slt",
                    "<=" => "sle",
                    ">" => "sgt",
                    ">=" => "sge",
                    _ => "ne",
                };
                let ty = if (lhs.operand == "true" || lhs.operand == "false")
                    && (rhs.operand == "true" || rhs.operand == "false")
                {
                    ssa.bool_type()
                } else {
                    "i64".into()
                };
                instrs.push(LlvmInstr::Comment(format!(
                    "guard compare op={operator} lhs={} rhs={}",
                    lhs.operand, rhs.operand
                )));
                instrs.push(LlvmInstr::Icmp {
                    result: cond.clone(),
                    pred: pred.into(),
                    ty,
                    lhs: lhs.operand,
                    rhs: rhs.operand,
                });
                (cond, instrs)
            }
            _ => {
                let value = emit_value_expr(expr_id, expr_map, ssa);
                let cond = ssa.new_tmp("guard");
                let mut instrs = value.instrs;
                let ty = value.ty.clone();
                let rhs = match ty.as_str() {
                    "i64" => "0".into(),
                    _ => "null".into(),
                };
                instrs.push(LlvmInstr::Comment(format!(
                    "guard binary op={operator} unsupported -> truthy check"
                )));
                instrs.push(LlvmInstr::Icmp {
                    result: cond.clone(),
                    pred: "ne".into(),
                    ty,
                    lhs: value.operand,
                    rhs,
                });
                (cond, instrs)
            }
        },
        _ => {
            let value = emit_value_expr(expr_id, expr_map, ssa);
            let cond = match value.ty.as_str() {
                "i1" => value.operand.clone(),
                _ => {
                    let tmp = ssa.new_tmp("guard");
                    let mut instrs = value.instrs;
                    let ty = value.ty.clone();
                    let rhs = match ty.as_str() {
                        "i64" => "0".into(),
                        _ => "null".into(),
                    };
                    instrs.push(LlvmInstr::Comment(format!(
                        "guard expr#{expr_id} -> truthy check"
                    )));
                    instrs.push(LlvmInstr::Icmp {
                        result: tmp.clone(),
                        pred: "ne".into(),
                        ty,
                        lhs: value.operand,
                        rhs,
                    });
                    return (tmp, instrs);
                }
            };
            (cond, value.instrs)
        }
    }
}

fn emit_value_expr(
    expr_id: MirExprId,
    expr_map: &HashMap<MirExprId, &MirExpr>,
    ssa: &mut LlvmBuilder,
) -> EmittedValue {
    let expr = match expr_map.get(&expr_id) {
        Some(expr) => expr,
        None => {
            return EmittedValue {
                ty: ssa.pointer_type(),
                operand: format!("#{}", expr_id),
                instrs: vec![LlvmInstr::Comment(format!(
                    "expr#{expr_id} missing -> fallback operand"
                ))],
            };
        }
    };

    match &expr.kind {
        MirExprKind::Literal { summary } => {
            if summary.trim() == "unit" {
                let unit = emit_unit_value(ssa);
                return (unit.operand, unit.instrs);
            }
            if let Some(value) = extract_literal_operand(summary) {
                let ty = if value == "true" || value == "false" {
                    ssa.bool_type()
                } else if value.starts_with('"') {
                    "Str".into()
                } else {
                    "i64".into()
                };
                return EmittedValue {
                    ty,
                    operand: value,
                    instrs: vec![],
                };
            }
            EmittedValue {
                ty: ssa.pointer_type(),
                operand: format_operand_from_summary(summary),
                instrs: vec![LlvmInstr::Comment(format!(
                    "literal {summary} (unsupported) -> as operand"
                ))],
            }
        }
        MirExprKind::Identifier { summary } => EmittedValue {
            ty: ssa.pointer_type(),
            operand: format_operand_from_summary(summary),
            instrs: vec![],
        },
        MirExprKind::FieldAccess { target, field } => {
            let target_value = emit_value_expr(*target, expr_map, ssa);
            let result = ssa.new_tmp("field");
            let mut instrs = target_value.instrs;
            instrs.push(LlvmInstr::Comment(format!(
                "field_access {}.{}",
                target_value.operand, field
            )));
            instrs.push(LlvmInstr::Call {
                result: Some(result.clone()),
                ret_ty: ssa.pointer_type(),
                callee: INTRINSIC_FIELD_ACCESS.into(),
                args: vec![
                    (ssa.pointer_type(), target_value.operand),
                    (ssa.pointer_type(), format!("\"{}\"", field.replace('"', "\\\""))),
                ],
            });
            EmittedValue {
                ty: ssa.pointer_type(),
                operand: result,
                instrs,
            }
        }
        MirExprKind::Call { callee, args } => {
            let callee_value = emit_value_expr(*callee, expr_map, ssa);
            let mut instrs = callee_value.instrs;
            let mut lowered_args: Vec<(String, String)> = Vec::new();
            lowered_args.push((ssa.pointer_type(), callee_value.operand));
            for arg in args {
                let value = emit_value_expr(*arg, expr_map, ssa);
                instrs.extend(value.instrs);
                lowered_args.push((value.ty, value.operand));
            }
            let result = ssa.new_tmp("call");
            let ret_ty = infer_call_return_type(*callee, expr_map, ssa);
            instrs.push(LlvmInstr::Call {
                result: Some(result.clone()),
                ret_ty: ret_ty.clone(),
                callee: INTRINSIC_CALL.into(),
                args: lowered_args,
            });
            EmittedValue {
                ty: ret_ty,
                operand: result,
                instrs,
            }
        }
        MirExprKind::Block {
            tail,
            defer_lifo,
            ..
        } => {
            if let Some(tail_id) = tail {
                if let Some(tail_expr) = expr_map.get(tail_id) {
                    match &tail_expr.kind {
                        MirExprKind::Return { value } => {
                            let mut value = if let Some(value_id) = value {
                                emit_value_expr(*value_id, expr_map, ssa)
                            } else {
                                emit_unit_value(ssa)
                            };
                            emit_defer_lifo_instrs(
                                defer_lifo,
                                expr_map,
                                ssa,
                                &mut value.instrs,
                            );
                            value
                                .instrs
                                .push(LlvmInstr::Comment(format!("return expr#{tail_id}")));
                            return value;
                        }
                        MirExprKind::Propagate { expr } => {
                            let mut value = emit_value_expr(*expr, expr_map, ssa);
                            emit_defer_lifo_instrs(
                                defer_lifo,
                                expr_map,
                                ssa,
                                &mut value.instrs,
                            );
                            value.instrs.push(LlvmInstr::Comment(format!(
                                "propagate expr#{tail_id}"
                            )));
                            return value;
                        }
                        MirExprKind::Panic { argument } => {
                            let mut value = if let Some(arg_id) = argument {
                                emit_value_expr(*arg_id, expr_map, ssa)
                            } else {
                                emit_unit_value(ssa)
                            };
                            emit_defer_lifo_instrs(
                                defer_lifo,
                                expr_map,
                                ssa,
                                &mut value.instrs,
                            );
                            value
                                .instrs
                                .push(LlvmInstr::Comment(format!("panic expr#{tail_id}")));
                            return value;
                        }
                        _ => {}
                    }
                }
            }
            let mut tail_value = if let Some(tail_id) = tail {
                emit_value_expr(*tail_id, expr_map, ssa)
            } else {
                emit_unit_value(ssa)
            };
            emit_defer_lifo_instrs(defer_lifo, expr_map, ssa, &mut tail_value.instrs);
            tail_value
        }
        MirExprKind::Return { value } => {
            let mut inner = if let Some(value_id) = value {
                emit_value_expr(*value_id, expr_map, ssa)
            } else {
                emit_unit_value(ssa)
            };
            inner
                .instrs
                .push(LlvmInstr::Comment(format!("return expr#{expr_id}")));
            inner
        }
        MirExprKind::Propagate { expr } => {
            let mut inner = emit_value_expr(*expr, expr_map, ssa);
            inner
                .instrs
                .push(LlvmInstr::Comment(format!("propagate expr#{expr_id}")));
            inner
        }
        MirExprKind::Panic { argument } => {
            let mut inner = if let Some(arg_id) = argument {
                emit_value_expr(*arg_id, expr_map, ssa)
            } else {
                emit_unit_value(ssa)
            };
            inner
                .instrs
                .push(LlvmInstr::Comment(format!("panic expr#{expr_id}")));
            inner
        }
        MirExprKind::Binary {
            operator,
            left,
            right,
        } => {
            match operator.as_str() {
                "&&" | "and" | "||" | "or" | "==" | "!=" | "<" | "<=" | ">" | ">=" => {
                    let (cond, instrs) = emit_bool_expr(expr_id, expr_map, ssa);
                    return EmittedValue {
                        ty: ssa.bool_type(),
                        operand: cond,
                        instrs,
                    };
                }
                "+" => {
                    let lhs = emit_value_expr(*left, expr_map, ssa);
                    let rhs = emit_value_expr(*right, expr_map, ssa);
                    let mut instrs = lhs.instrs;
                    instrs.extend(rhs.instrs);
                    let is_stringish = lhs.ty == "Str"
                        || rhs.ty == "Str"
                        || lhs.operand.starts_with('"')
                        || rhs.operand.starts_with('"');
                    if is_stringish {
                        let result = ssa.new_tmp("concat");
                        instrs.push(LlvmInstr::Call {
                            result: Some(result.clone()),
                            ret_ty: "Str".into(),
                            callee: INTRINSIC_STR_CONCAT.into(),
                            args: vec![
                                ("Str".into(), lhs.operand),
                                ("Str".into(), rhs.operand),
                            ],
                        });
                        return EmittedValue {
                            ty: "Str".into(),
                            operand: result,
                            instrs,
                        };
                    }
                    let result = ssa.new_tmp("add");
                    instrs.push(LlvmInstr::BinOp {
                        result: result.clone(),
                        op: "add".into(),
                        ty: "i64".into(),
                        lhs: lhs.operand,
                        rhs: rhs.operand,
                    });
                    return EmittedValue {
                        ty: "i64".into(),
                        operand: result,
                        instrs,
                    };
                }
                "%" => {
                    let lhs = emit_value_expr(*left, expr_map, ssa);
                    let rhs = emit_value_expr(*right, expr_map, ssa);
                    let mut instrs = lhs.instrs;
                    instrs.extend(rhs.instrs);
                    let result = ssa.new_tmp("mod");
                    instrs.push(LlvmInstr::BinOp {
                        result: result.clone(),
                        op: "srem".into(),
                        ty: "i64".into(),
                        lhs: lhs.operand,
                        rhs: rhs.operand,
                    });
                    return EmittedValue {
                        ty: "i64".into(),
                        operand: result,
                        instrs,
                    };
                }
                "-" => {
                    let lhs = emit_value_expr(*left, expr_map, ssa);
                    let rhs = emit_value_expr(*right, expr_map, ssa);
                    let mut instrs = lhs.instrs;
                    instrs.extend(rhs.instrs);
                    let result = ssa.new_tmp("sub");
                    instrs.push(LlvmInstr::BinOp {
                        result: result.clone(),
                        op: "sub".into(),
                        ty: "i64".into(),
                        lhs: lhs.operand,
                        rhs: rhs.operand,
                    });
                    return EmittedValue {
                        ty: "i64".into(),
                        operand: result,
                        instrs,
                    };
                }
                "*" => {
                    let lhs = emit_value_expr(*left, expr_map, ssa);
                    let rhs = emit_value_expr(*right, expr_map, ssa);
                    let mut instrs = lhs.instrs;
                    instrs.extend(rhs.instrs);
                    let result = ssa.new_tmp("mul");
                    instrs.push(LlvmInstr::BinOp {
                        result: result.clone(),
                        op: "mul".into(),
                        ty: "i64".into(),
                        lhs: lhs.operand,
                        rhs: rhs.operand,
                    });
                    return EmittedValue {
                        ty: "i64".into(),
                        operand: result,
                        instrs,
                    };
                }
                "/" => {
                    let lhs = emit_value_expr(*left, expr_map, ssa);
                    let rhs = emit_value_expr(*right, expr_map, ssa);
                    let mut instrs = lhs.instrs;
                    instrs.extend(rhs.instrs);
                    let result = ssa.new_tmp("div");
                    instrs.push(LlvmInstr::BinOp {
                        result: result.clone(),
                        op: "sdiv".into(),
                        ty: "i64".into(),
                        lhs: lhs.operand,
                        rhs: rhs.operand,
                    });
                    return EmittedValue {
                        ty: "i64".into(),
                        operand: result,
                        instrs,
                    };
                }
                _ => EmittedValue {
                    ty: ssa.pointer_type(),
                    operand: format!("#{}", expr_id),
                    instrs: vec![LlvmInstr::Comment(format!(
                        "binary op {operator} unsupported -> fallback #{expr_id}"
                    ))],
                },
            }
        }
        MirExprKind::IfElse {
            condition,
            then_branch,
            else_branch,
        } => {
            let (cond, mut instrs) = emit_bool_expr(*condition, expr_map, ssa);
            let then_value = emit_value_expr(*then_branch, expr_map, ssa);
            let else_value = emit_value_expr(*else_branch, expr_map, ssa);
            instrs.extend(then_value.instrs);
            instrs.extend(else_value.instrs);
            let ret_ty = if then_value.ty == else_value.ty {
                then_value.ty
            } else {
                ssa.pointer_type()
            };
            let result = ssa.new_tmp("ifelse");
            instrs.push(LlvmInstr::Call {
                result: Some(result.clone()),
                ret_ty: ret_ty.clone(),
                callee: INTRINSIC_IF_ELSE.into(),
                args: vec![
                    (ssa.bool_type(), cond),
                    (ret_ty.clone(), then_value.operand),
                    (ret_ty.clone(), else_value.operand),
                ],
            });
            EmittedValue {
                ty: ret_ty,
                operand: result,
                instrs,
            }
        }
        MirExprKind::PerformCall { effect, argument } => {
            let value = emit_value_expr(*argument, expr_map, ssa);
            let mut instrs = value.instrs;
            let result = ssa.new_tmp("perform");
            instrs.push(LlvmInstr::Call {
                result: Some(result.clone()),
                ret_ty: ssa.pointer_type(),
                callee: INTRINSIC_PERFORM.into(),
                args: vec![
                    (ssa.pointer_type(), format!("\"{}\"", effect.replace('"', "\\\""))),
                    (value.ty, value.operand),
                ],
            });
            EmittedValue {
                ty: ssa.pointer_type(),
                operand: result,
                instrs,
            }
        }
        MirExprKind::Match { .. } | MirExprKind::Unknown => EmittedValue {
            ty: ssa.pointer_type(),
            operand: format!("#{}", expr_id),
            instrs: vec![LlvmInstr::Comment(format!(
                "expr#{expr_id} unsupported -> fallback operand"
            ))],
        },
    }
}

fn infer_call_return_type(
    callee_id: MirExprId,
    expr_map: &HashMap<MirExprId, &MirExpr>,
    ssa: &LlvmBuilder,
) -> String {
    let Some(expr) = expr_map.get(&callee_id) else {
        return ssa.pointer_type();
    };
    match &expr.kind {
        MirExprKind::FieldAccess { field, .. } => match field.as_str() {
            "is_empty" | "starts_with" => ssa.bool_type(),
            "len" => "i64".into(),
            "to_string" | "format" => "Str".into(),
            _ => ssa.pointer_type(),
        },
        MirExprKind::Identifier { summary } => {
            let name = summary.trim();
            match name {
                "len" => "i64".into(),
                _ => ssa.pointer_type(),
            }
        }
        _ => ssa.pointer_type(),
    }
}

fn emit_body_value(
    arm_index: usize,
    expr_id: MirExprId,
    expr_map: &HashMap<MirExprId, &MirExpr>,
    result_type: &str,
    ssa: &mut LlvmBuilder,
) -> (String, String, Vec<LlvmInstr>) {
    let body_label = format!("arm{arm_index}.body#{}", expr_id);
    let value = emit_value_expr(expr_id, expr_map, ssa);
    let result = ssa.new_tmp("match_result");
    let mut instrs = value.instrs;
    instrs.push(LlvmInstr::Comment(format!(
        "match_result <- expr#{expr_id} ({})",
        value.operand
    )));
    instrs.push(LlvmInstr::Call {
        result: Some(result.clone()),
        ret_ty: result_type.to_string(),
        callee: INTRINSIC_VALUE.into(),
        args: vec![(result_type.to_string(), value.operand)],
    });
    (result, body_label, instrs)
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
        MirPatternKind::Constructor { name, args } if !args.is_empty() => {
            let payload_label = format!("arm{arm_index}.ctor_payload");
            let outer = format!(
                "arm{arm_index}.pat: ctor_check({name}, args={} on {target_label}) -> match:{payload} / miss:{miss}",
                args.len(),
                payload = payload_label,
                miss = next_arm_label
            );
            let payload = format!(
                "{payload_label}: ctor_payload({name}) -> match:{success} / miss:{miss}",
                success = success_label,
                miss = next_arm_label
            );
            vec![outer, payload]
        }
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

fn synthesize_pattern_check_cond(
    ssa: &mut LlvmBuilder,
    check_label: String,
    target_label: &str,
    hint: &str,
) -> (String, Vec<LlvmInstr>) {
    let mut instrs = Vec::new();
    instrs.push(LlvmInstr::Comment(check_label));
    let call_result = ssa.new_tmp(hint);
    instrs.push(LlvmInstr::Call {
        result: Some(call_result.clone()),
        ret_ty: ssa.bool_type(),
        callee: INTRINSIC_MATCH_CHECK.into(),
        args: vec![("ptr".into(), target_label.to_string())],
    });
    let cond = ssa.new_tmp("cmp");
    instrs.push(LlvmInstr::Icmp {
        result: cond.clone(),
        pred: "ne".into(),
        ty: ssa.bool_type(),
        lhs: call_result,
        rhs: "false".into(),
    });
    (cond, instrs)
}

fn format_operand_from_summary(summary: &str) -> String {
    let trimmed = summary.trim();
    if let Some(rest) = trimmed.strip_prefix('#') {
        if let Ok(index) = rest.parse::<usize>() {
            return format!("%arg{index}");
        }
    }
    if trimmed.starts_with('{') && trimmed.ends_with('}') {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) {
            if let Some(name) = value.get("name").and_then(|v| v.as_str()) {
                return format!("%{name}");
            }
        }
    }
    if trimmed == "true" || trimmed == "false" {
        return trimmed.to_string();
    }
    if trimmed.parse::<i64>().is_ok() {
        return trimmed.to_string();
    }
    trimmed.to_string()
}

fn extract_literal_operand(summary: &str) -> Option<String> {
    let trimmed = summary.trim();
    if trimmed == "true" || trimmed == "false" {
        return Some(trimmed.to_string());
    }
    if let Ok(value) = trimmed.parse::<i64>() {
        return Some(value.to_string());
    }
    if !(trimmed.starts_with('{') && trimmed.ends_with('}')) {
        return None;
    }
    let value = serde_json::from_str::<serde_json::Value>(trimmed).ok()?;
    if let Some(number) = value.get("value").and_then(|v| v.as_i64()) {
        return Some(number.to_string());
    }
    if let Some(kind) = value.get("kind").and_then(|v| v.as_str()) {
        match kind {
            "int" => {
                if let Some(number) = value.get("value").and_then(|v| v.as_i64()) {
                    return Some(number.to_string());
                }
            }
            "string" => {
                if let Some(text) = value.get("value").and_then(|v| v.as_str()) {
                    return Some(format!("\"{}\"", text.replace('"', "\\\"")));
                }
            }
            _ => {}
        }
    }
    None
}

fn emit_pattern_cond(
    ssa: &mut LlvmBuilder,
    pattern: &MirPattern,
    target_operand: &str,
    target_desc: &str,
    miss_label: &str,
    hint: &str,
) -> (String, Vec<LlvmInstr>) {
    match &pattern.kind {
        MirPatternKind::Wildcard | MirPatternKind::Var { .. } => (
            "true".into(),
            vec![LlvmInstr::Comment(pattern_check_label(
                pattern,
                target_desc,
                miss_label,
            ))],
        ),
        MirPatternKind::Binding {
            pattern: inner,
            name,
            ..
        } => {
            let (cond, mut instrs) =
                emit_pattern_cond(ssa, inner, target_operand, target_desc, miss_label, hint);
            instrs.insert(
                0,
                LlvmInstr::Comment(format!("binding {name} <- {target_desc}")),
            );
            (cond, instrs)
        }
        MirPatternKind::Regex { pattern: regex } => {
            let mut instrs = Vec::new();
            instrs.push(LlvmInstr::Comment(pattern_check_label(
                pattern,
                target_desc,
                miss_label,
            )));
            let call_result = ssa.new_tmp("regex");
            instrs.push(LlvmInstr::Call {
                result: Some(call_result.clone()),
                ret_ty: ssa.bool_type(),
                callee: INTRINSIC_REGEX_MATCH.into(),
                args: vec![
                    (ssa.pointer_type(), target_operand.to_string()),
                    (
                        ssa.pointer_type(),
                        format!("\"{}\"", regex.replace('"', "\\\"")),
                    ),
                ],
            });
            let cond = ssa.new_tmp("cmp");
            instrs.push(LlvmInstr::Icmp {
                result: cond.clone(),
                pred: "ne".into(),
                ty: ssa.bool_type(),
                lhs: call_result,
                rhs: "false".into(),
            });
            (cond, instrs)
        }
        MirPatternKind::Constructor { name, args } => {
            if !args.is_empty() {
                return synthesize_pattern_check_cond(
                    ssa,
                    format!(
                        "ctor_check({name}, args={} on {target_desc}) (payload matching handled in emit_pattern_blocks)",
                        args.len()
                    ),
                    target_operand,
                    hint,
                );
            }
            let mut instrs = Vec::new();
            instrs.push(LlvmInstr::Comment(pattern_check_label(
                pattern,
                target_desc,
                miss_label,
            )));
            let cond = ssa.new_tmp("ctor");
            if name == "None" {
                instrs.push(LlvmInstr::Icmp {
                    result: cond.clone(),
                    pred: "eq".into(),
                    ty: ssa.pointer_type(),
                    lhs: target_operand.to_string(),
                    rhs: "null".into(),
                });
                return (cond, instrs);
            }
            if name == "Some" {
                instrs.push(LlvmInstr::Icmp {
                    result: cond.clone(),
                    pred: "ne".into(),
                    ty: ssa.pointer_type(),
                    lhs: target_operand.to_string(),
                    rhs: "null".into(),
                });
                return (cond, instrs);
            }
            instrs.push(LlvmInstr::Call {
                result: Some(cond.clone()),
                ret_ty: ssa.bool_type(),
                callee: intrinsic_is_ctor(name),
                args: vec![(ssa.pointer_type(), target_operand.to_string())],
            });
            (cond, instrs)
        }
        MirPatternKind::Literal { summary } => {
            if let Some(lit) = extract_literal_operand(summary) {
                let mut instrs = Vec::new();
                instrs.push(LlvmInstr::Comment(pattern_check_label(
                    pattern,
                    target_desc,
                    miss_label,
                )));
                let cond = ssa.new_tmp(hint);
                let ty = if lit == "true" || lit == "false" {
                    ssa.bool_type()
                } else {
                    "i64".into()
                };
                instrs.push(LlvmInstr::Icmp {
                    result: cond.clone(),
                    pred: "eq".into(),
                    ty,
                    lhs: target_operand.to_string(),
                    rhs: lit,
                });
                return (cond, instrs);
            }
            synthesize_pattern_check_cond(
                ssa,
                pattern_check_label(pattern, target_desc, miss_label),
                target_operand,
                hint,
            )
        }
        _ => synthesize_pattern_check_cond(
            ssa,
            pattern_check_label(pattern, target_desc, miss_label),
            target_operand,
            hint,
        ),
    }
}

fn emit_pattern_blocks(
    arm_index: usize,
    pattern: &MirPattern,
    success_label: &str,
    next_arm_label: &str,
    target_operand: &str,
    target_desc: &str,
    ssa: &mut LlvmBuilder,
) -> (Vec<BasicBlock>, Vec<LlvmBlock>) {
    match &pattern.kind {
        MirPatternKind::Constructor { name, args } if !args.is_empty() => {
            let outer_label = format!("arm{arm_index}.pat");
            let payload_label = format!("arm{arm_index}.ctor_payload");

            let (outer_cond, mut outer_instrs) = if name == "Some" {
                let cond = ssa.new_tmp("ctor");
                (
                    cond.clone(),
                    vec![
                        LlvmInstr::Comment(format!(
                            "ctor_check(Some) on {target_desc} -> non-null then {payload_label} else {next_arm_label}"
                        )),
                        LlvmInstr::Icmp {
                            result: cond,
                            pred: "ne".into(),
                            ty: ssa.pointer_type(),
                            lhs: target_operand.to_string(),
                            rhs: "null".into(),
                        },
                    ],
                )
            } else {
                let cond = ssa.new_tmp("ctor");
                (
                    cond.clone(),
                    vec![
                        LlvmInstr::Comment(format!(
                            "ctor_check({name}) on {target_desc} -> then {payload_label} else {next_arm_label}"
                        )),
                        LlvmInstr::Call {
                            result: Some(cond),
                            ret_ty: ssa.bool_type(),
                            callee: intrinsic_is_ctor(name),
                            args: vec![(ssa.pointer_type(), target_operand.to_string())],
                        },
                    ],
                )
            };

            let outer_bb = BasicBlock {
                label: outer_label.clone(),
                instrs: vec![format!(
                    "check ctor({name}, args={}) on {target_desc}",
                    args.len()
                )],
                terminator: format!(
                    "br_if {outer_cond} then {payload_label} else {next_arm_label}"
                ),
            };
            let outer_llvm_bb = LlvmBlock {
                label: outer_label.clone(),
                instrs: {
                    outer_instrs.insert(
                        0,
                        LlvmInstr::Comment(pattern_check_label(pattern, target_desc, next_arm_label)),
                    );
                    outer_instrs
                },
                terminator: LlvmTerminator::BrCond {
                    cond: outer_cond,
                    then_bb: payload_label.clone(),
                    else_bb: next_arm_label.to_string(),
                },
            };

            let payload_var = ssa.new_tmp("payload");
            let payload_desc = format!("payload({target_desc}.{name})");
            let mut payload_instrs = Vec::new();
            payload_instrs.push(LlvmInstr::Comment(format!(
                "{payload_desc} <- {target_desc}"
            )));
            payload_instrs.push(LlvmInstr::Call {
                result: Some(payload_var.clone()),
                ret_ty: ssa.pointer_type(),
                callee: intrinsic_ctor_payload(name),
                args: vec![(ssa.pointer_type(), target_operand.to_string())],
            });
            let (inner_cond, mut inner_instrs) = match args.len() {
                1 => emit_pattern_cond(
                    ssa,
                    &args[0],
                    &payload_var,
                    &payload_desc,
                    next_arm_label,
                    "ctor",
                ),
                _ => synthesize_pattern_check_cond(
                    ssa,
                    format!(
                        "ctor_check({name}, args={}) (multi-arg payload matching unsupported)",
                        args.len()
                    ),
                    &payload_var,
                    "ctor",
                ),
            };
            payload_instrs.append(&mut inner_instrs);
            let payload_bb = BasicBlock {
                label: payload_label.clone(),
                instrs: vec![format!(
                    "check ctor payload args={} on {payload_desc}",
                    args.len()
                )],
                terminator: format!(
                    "br_if {inner_cond} then {success_label} else {next_arm_label}"
                ),
            };
            let payload_llvm_bb = LlvmBlock {
                label: payload_label.clone(),
                instrs: payload_instrs,
                terminator: LlvmTerminator::BrCond {
                    cond: inner_cond,
                    then_bb: success_label.to_string(),
                    else_bb: next_arm_label.to_string(),
                },
            };

            (
                vec![outer_bb, payload_bb],
                vec![outer_llvm_bb, payload_llvm_bb],
            )
        }
        MirPatternKind::Or { variants } => {
            let mut blocks = Vec::new();
            let mut llvm_blocks = Vec::new();
            for (idx, variant) in variants.iter().enumerate() {
                let miss_target = if idx + 1 == variants.len() {
                    next_arm_label.to_string()
                } else {
                    format!("arm{arm_index}.or{}", idx + 1)
                };
                let check = pattern_check_label(variant, target_desc, &miss_target);
                let (cond, llvm_instrs) = emit_pattern_cond(
                    ssa,
                    variant,
                    target_operand,
                    target_desc,
                    &miss_target,
                    "or",
                );
                let label = format!("arm{arm_index}.or{idx}");
                blocks.push(BasicBlock {
                    label: label.clone(),
                    instrs: vec![format!("check {}", check)],
                    terminator: format!(
                        "br_if {cond} then {success} else {miss}",
                        cond = cond,
                        success = success_label,
                        miss = miss_target
                    ),
                });
                llvm_blocks.push(LlvmBlock {
                    label: label.clone(),
                    instrs: llvm_instrs,
                    terminator: LlvmTerminator::BrCond {
                        cond,
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
                instrs.push(format!("{var} = icmp_ge {target_desc}, {lhs}"));
                llvm_instrs.push(LlvmInstr::Icmp {
                    result: var.clone(),
                    pred: "sge".into(),
                    ty: "i64".into(),
                    lhs: target_operand.into(),
                    rhs: lhs,
                });
                cond = var.clone();
            }
            if let Some(hi) = end {
                let rhs = render_range_bound(hi);
                let op = if *inclusive { "icmp_le" } else { "icmp_lt" };
                let var = format!("tmp_arm{arm_index}_hi");
                instrs.push(format!("{var} = {op} {target_desc}, {rhs}"));
                llvm_instrs.push(LlvmInstr::Icmp {
                    result: var.clone(),
                    pred: if *inclusive {
                        "sle".into()
                    } else {
                        "slt".into()
                    },
                    ty: "i64".into(),
                    lhs: target_operand.into(),
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
            instrs.push(format!("{len_var} = len({target_desc})"));
            llvm_instrs.push(LlvmInstr::Call {
                result: Some(len_var.clone()),
                ret_ty: "i64".into(),
                callee: "@len".into(),
                args: vec![(ssa.pointer_type(), target_operand.into())],
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
            instrs.push(format!("{call_var} = call active {name}({target_desc})"));
            llvm_instrs.push(LlvmInstr::Call {
                result: Some(call_var.clone()),
                ret_ty: "ptr".into(),
                callee: format!("@{}", name),
                args: vec![(ssa.pointer_type(), target_operand.into())],
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
            let check = pattern_check_label(pattern, target_desc, next_arm_label);
            let (cond, llvm_instrs) = emit_pattern_cond(
                ssa,
                pattern,
                target_operand,
                target_desc,
                next_arm_label,
                "pat",
            );
            let bb = BasicBlock {
                label: format!("arm{arm_index}.pat"),
                instrs: vec![format!("check {}", check)],
                terminator: format!(
                    "br_if {cond} then {success} else {miss}",
                    cond = cond,
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
            format!("ctor_check({name}, args={} on {target_label})", args.len())
        }
        MirPatternKind::Binding { pattern, .. } => {
            pattern_check_label(pattern, target_label, miss_label)
        }
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
            let base = if *inclusive {
                "range(..=)"
            } else {
                "range(..)"
            };
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
