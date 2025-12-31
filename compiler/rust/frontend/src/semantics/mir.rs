use serde::Serialize;
use std::collections::BTreeMap;

use crate::parser::ast::{Ident, Literal};
use crate::semantics::typed;
use crate::span::Span;

pub const MIR_SCHEMA_VERSION: &str = "frontend-mir/0.2";

pub type MirExprId = usize;

fn is_false(value: &bool) -> bool {
    !*value
}

#[derive(Debug, Clone, Serialize)]
pub struct MirModule {
    pub schema_version: &'static str,
    pub functions: Vec<MirFunction>,
    pub active_patterns: Vec<MirActivePattern>,
    pub conductors: Vec<MirConductor>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub externs: Vec<MirExtern>,
    pub dict_refs: Vec<typed::DictRef>,
    pub impls: BTreeMap<String, MirImplSpec>,
    pub qualified_calls: BTreeMap<String, MirQualifiedCall>,
    pub impl_registry_duplicates: Vec<String>,
    pub impl_registry_unresolved: Vec<String>,
}

impl MirModule {
    pub fn from_typed_module(module: &typed::TypedModule) -> Self {
        let mut qualified_calls = BTreeMap::new();
        let functions = module
            .functions
            .iter()
            .map(|function| {
                let (mir_function, calls) = lower_function(function);
                qualified_calls.extend(calls);
                mir_function
            })
            .collect();
        let active_patterns = module
            .active_patterns
            .iter()
            .map(|pattern| {
                let (mir_pattern, calls) = lower_active_pattern(pattern);
                qualified_calls.extend(calls);
                mir_pattern
            })
            .collect();
        let conductors = module.conductors.iter().map(lower_conductor).collect();
        Self {
            schema_version: MIR_SCHEMA_VERSION,
            functions,
            active_patterns,
            conductors,
            externs: module
                .externs
                .iter()
                .map(|extern_item| MirExtern {
                    name: extern_item.name.clone(),
                    span: extern_item.span,
                    abi: extern_item.abi.clone(),
                    symbol: extern_item.symbol.clone(),
                })
                .collect(),
            dict_refs: module.dict_refs.clone(),
            impls: BTreeMap::new(),
            qualified_calls,
            impl_registry_duplicates: Vec::new(),
            impl_registry_unresolved: Vec::new(),
        }
    }
}

impl Default for MirModule {
    fn default() -> Self {
        Self {
            schema_version: MIR_SCHEMA_VERSION,
            functions: Vec::new(),
            active_patterns: Vec::new(),
            conductors: Vec::new(),
            externs: Vec::new(),
            dict_refs: Vec::new(),
            impls: BTreeMap::new(),
            qualified_calls: BTreeMap::new(),
            impl_registry_duplicates: Vec::new(),
            impl_registry_unresolved: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct MirImplSpec {
    #[serde(rename = "trait", skip_serializing_if = "Option::is_none")]
    pub trait_name: Option<String>,
    pub target: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub associated_types: Vec<MirAssociatedType>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub methods: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span: Option<Span>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MirAssociatedType {
    pub name: String,
    pub ty: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct MirQualifiedCall {
    pub kind: MirQualifiedCallKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub impl_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receiver_ty: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub impl_candidates: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span: Option<Span>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MirQualifiedCallKind {
    TypeMethod,
    TypeAssoc,
    TraitMethod,
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
pub struct MirFunction {
    pub name: String,
    pub span: Span,
    pub attributes: Vec<String>,
    pub params: Vec<MirParam>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub varargs: bool,
    pub return_type: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub is_async: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub is_unsafe: bool,
    pub body: MirExprId,
    pub exprs: Vec<MirExpr>,
    pub dict_ref_ids: Vec<typed::DictRefId>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MirExtern {
    pub name: String,
    pub span: Span,
    pub abi: String,
    pub symbol: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct MirActivePattern {
    pub name: String,
    pub span: Span,
    pub kind: typed::ActivePatternKind,
    pub return_carrier: typed::ActiveReturnCarrier,
    pub has_miss_path: bool,
    pub params: Vec<MirParam>,
    pub body: MirExprId,
    pub exprs: Vec<MirExpr>,
    pub dict_ref_ids: Vec<typed::DictRefId>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MirConductor {
    pub name: String,
    pub span: Span,
    pub dsl_defs: Vec<MirConductorDslDef>,
    pub channels: Vec<MirConductorChannel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution: Option<MirConductorBlock>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub monitoring: Option<MirConductorMonitoringBlock>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MirConductorDslDef {
    pub alias: String,
    pub target: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pipeline_type: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tails: Vec<MirConductorDslTail>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
pub struct MirConductorDslTail {
    pub stage: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub arg_types: Vec<String>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
pub struct MirConductorChannel {
    pub source: String,
    pub target: String,
    pub payload: String,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
pub struct MirConductorBlock {
    pub ty: String,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
pub struct MirConductorMonitoringBlock {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    pub ty: String,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
pub struct MirParam {
    pub name: String,
    pub span: Span,
    pub ty: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct MirLambdaCapture {
    pub name: String,
    pub span: Span,
    #[serde(default, skip_serializing_if = "is_false")]
    pub mutable: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct MirExpr {
    pub id: MirExprId,
    pub span: Span,
    pub ty: String,
    pub dict_ref_ids: Vec<typed::DictRefId>,
    pub kind: MirExprKind,
}

#[derive(Debug, Clone, Serialize)]
pub struct MirStmt {
    pub span: Span,
    pub kind: MirStmtKind,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MirStmtKind {
    Let {
        pattern: MirPattern,
        value: MirExprId,
        mutable: bool,
    },
    Expr {
        expr: MirExprId,
    },
    Assign {
        target: MirExprId,
        value: MirExprId,
    },
    Defer {
        expr: MirExprId,
    },
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MirExprKind {
    Literal(Literal),
    Identifier {
        ident: Ident,
    },
    Call {
        callee: MirExprId,
        args: Vec<MirExprId>,
    },
    Lambda {
        params: Vec<MirParam>,
        body: MirExprId,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        captures: Vec<MirLambdaCapture>,
    },
    Rec {
        target: MirExprId,
        #[serde(skip_serializing_if = "Option::is_none")]
        ident: Option<Ident>,
    },
    Block {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        statements: Vec<MirStmt>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tail: Option<MirExprId>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        defers: Vec<MirExprId>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        defer_lifo: Vec<MirExprId>,
    },
    Return {
        #[serde(skip_serializing_if = "Option::is_none")]
        value: Option<MirExprId>,
    },
    Propagate {
        expr: MirExprId,
    },
    Panic {
        #[serde(skip_serializing_if = "Option::is_none")]
        argument: Option<MirExprId>,
    },
    Binary {
        operator: String,
        left: MirExprId,
        right: MirExprId,
    },
    FieldAccess {
        target: MirExprId,
        field: String,
    },
    TupleAccess {
        target: MirExprId,
        index: u32,
    },
    Index {
        target: MirExprId,
        index: MirExprId,
    },
    Match {
        target: MirExprId,
        arms: Vec<MirMatchArm>,
        lowering: MatchLoweringPlan,
    },
    IfElse {
        condition: MirExprId,
        then_branch: MirExprId,
        else_branch: MirExprId,
    },
    PerformCall {
        call: MirEffectCall,
    },
    EffectBlock {
        body: MirExprId,
    },
    Async {
        body: MirExprId,
        #[serde(default, skip_serializing_if = "is_false")]
        is_move: bool,
    },
    Await {
        expr: MirExprId,
    },
    Unsafe {
        body: MirExprId,
    },
    InlineAsm {
        template: String,
        outputs: Vec<MirInlineAsmOutput>,
        inputs: Vec<MirInlineAsmInput>,
        clobbers: Vec<String>,
        options: Vec<String>,
    },
    LlvmIr {
        result_type: String,
        template: String,
        inputs: Vec<MirExprId>,
    },
    #[serde(rename = "unknown")]
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
pub struct MirEffectCall {
    pub effect: Ident,
    pub argument: MirExprId,
}

#[derive(Debug, Clone, Serialize)]
pub struct MirInlineAsmOutput {
    pub constraint: String,
    pub target: MirExprId,
}

#[derive(Debug, Clone, Serialize)]
pub struct MirInlineAsmInput {
    pub constraint: String,
    pub expr: MirExprId,
}

#[derive(Debug, Clone, Serialize)]
pub struct MirMatchArm {
    pub pattern: MirPattern,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guard: Option<MirExprId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
    pub body: MirExprId,
}

#[derive(Debug, Clone, Serialize)]
pub struct MirPattern {
    pub span: Span,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ty: Option<String>,
    pub kind: MirPatternKind,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MirPatternKind {
    Wildcard,
    Var {
        name: String,
    },
    Literal(Literal),
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

#[derive(Debug, Clone, Serialize)]
pub struct MirSlicePattern {
    pub head: Vec<MirPattern>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rest: Option<MirSliceRest>,
    pub tail: Vec<MirPattern>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MirSliceRest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binding: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MirPatternRecordField {
    pub key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<Box<MirPattern>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MirActivePatternCall {
    pub name: String,
    pub kind: typed::ActivePatternKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub argument: Option<Box<MirPattern>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_binding: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub miss_target: Option<MirJumpTarget>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MirJumpTarget {
    NextArm,
}

#[derive(Debug, Clone, Serialize)]
pub struct MatchLoweringPlan {
    pub owner: String,
    pub span: Span,
    pub target_type: String,
    pub arm_count: usize,
    pub arms: Vec<MatchArmLowering>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MatchArmLowering {
    pub pattern: PatternLowering,
    pub has_guard: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PatternLowering {
    pub label: String,
    pub miss_on_none: bool,
    pub always_matches: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<PatternLowering>,
}

fn normalize_mir_type_label(label: &str) -> String {
    let mut out = String::with_capacity(label.len());
    let mut token = String::new();
    for ch in label.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            token.push(ch);
        } else {
            if !token.is_empty() {
                out.push_str(normalize_mir_type_ident(&token));
                token.clear();
            }
            out.push(ch);
        }
    }
    if !token.is_empty() {
        out.push_str(normalize_mir_type_ident(&token));
    }
    out
}

fn normalize_mir_type_ident<'a>(token: &'a str) -> &'a str {
    match token {
        "Int" => "i64",
        "Unit" => "()",
        _ => token,
    }
}

fn lower_function(
    function: &typed::TypedFunction,
) -> (MirFunction, BTreeMap<String, MirQualifiedCall>) {
    let mut builder = MirExprBuilder::new(function.name.clone());
    let body = builder.lower_expr(&function.body);
    let (exprs, qualified_calls) = builder.finish();
    let mir_function = MirFunction {
        name: function.name.clone(),
        span: function.span,
        attributes: function.attributes.clone(),
        params: function
            .params
            .iter()
            .map(|param| MirParam {
                name: param.name.clone(),
                span: param.span,
                ty: normalize_mir_type_label(&param.ty),
            })
            .collect(),
        varargs: function.varargs,
        return_type: normalize_mir_type_label(&function.return_type),
        is_async: function.is_async,
        is_unsafe: function.is_unsafe,
        body,
        exprs,
        dict_ref_ids: function.dict_ref_ids.clone(),
    };
    (mir_function, qualified_calls)
}

fn lower_active_pattern(
    pattern: &typed::TypedActivePattern,
) -> (MirActivePattern, BTreeMap<String, MirQualifiedCall>) {
    let mut builder = MirExprBuilder::new(pattern.name.clone());
    let body = builder.lower_expr(&pattern.body);
    let (exprs, qualified_calls) = builder.finish();
    let mir_pattern = MirActivePattern {
        name: pattern.name.clone(),
        span: pattern.span,
        kind: pattern.kind.clone(),
        return_carrier: pattern.return_carrier.clone(),
        has_miss_path: pattern.has_miss_path,
        params: pattern
            .params
            .iter()
            .map(|param| MirParam {
                name: param.name.clone(),
                span: param.span,
                ty: normalize_mir_type_label(&param.ty),
            })
            .collect(),
        body,
        exprs,
        dict_ref_ids: pattern.dict_ref_ids.clone(),
    };
    (mir_pattern, qualified_calls)
}

fn lower_conductor(conductor: &typed::TypedConductor) -> MirConductor {
    MirConductor {
        name: conductor.name.clone(),
        span: conductor.span,
        dsl_defs: conductor
            .dsl_defs
            .iter()
            .map(|dsl_def| MirConductorDslDef {
                alias: dsl_def.alias.clone(),
                target: dsl_def.target.clone(),
                target_type: dsl_def
                    .target_type
                    .as_ref()
                    .map(|ty| normalize_mir_type_label(ty)),
                pipeline_type: dsl_def
                    .pipeline_type
                    .as_ref()
                    .map(|ty| normalize_mir_type_label(ty)),
                tails: dsl_def
                    .tails
                    .iter()
                    .map(|tail| MirConductorDslTail {
                        stage: tail.stage.clone(),
                        arg_types: tail
                            .arg_types
                            .iter()
                            .map(|ty| normalize_mir_type_label(ty))
                            .collect(),
                        span: tail.span,
                    })
                    .collect(),
                span: dsl_def.span,
            })
            .collect(),
        channels: conductor
            .channels
            .iter()
            .map(|channel| MirConductorChannel {
                source: channel.source.clone(),
                target: channel.target.clone(),
                payload: channel.payload.clone(),
                span: channel.span,
            })
            .collect(),
        execution: conductor.execution.as_ref().map(|block| MirConductorBlock {
            ty: normalize_mir_type_label(&block.ty),
            span: block.span,
        }),
        monitoring: conductor
            .monitoring
            .as_ref()
            .map(|block| MirConductorMonitoringBlock {
                target: block.target.clone(),
                ty: normalize_mir_type_label(&block.ty),
                span: block.span,
            }),
    }
}

struct MirExprBuilder {
    owner: String,
    exprs: Vec<MirExpr>,
    qualified_calls: BTreeMap<String, MirQualifiedCall>,
}

impl MirExprBuilder {
    fn new(owner: String) -> Self {
        Self {
            owner,
            exprs: Vec::new(),
            qualified_calls: BTreeMap::new(),
        }
    }

    fn lower_expr(&mut self, expr: &typed::TypedExpr) -> MirExprId {
        let kind = match &expr.kind {
            typed::TypedExprKind::Literal(literal) => MirExprKind::Literal(literal.clone()),
            typed::TypedExprKind::Identifier { ident } => MirExprKind::Identifier {
                ident: ident.clone(),
            },
            typed::TypedExprKind::Call {
                callee,
                args,
                qualified: _,
            } => {
                if let typed::TypedExprKind::Identifier { ident } = &callee.kind {
                    if ident.name == "panic" {
                        let argument = args.get(0).map(|arg| self.lower_expr(arg));
                        MirExprKind::Panic { argument }
                    } else {
                        MirExprKind::Call {
                            callee: self.lower_expr(callee),
                            args: args.iter().map(|arg| self.lower_expr(arg)).collect(),
                        }
                    }
                } else {
                    MirExprKind::Call {
                        callee: self.lower_expr(callee),
                        args: args.iter().map(|arg| self.lower_expr(arg)).collect(),
                    }
                }
            }
            typed::TypedExprKind::Lambda {
                params,
                body,
                captures,
                ..
            } => MirExprKind::Lambda {
                params: params
                    .iter()
                    .map(|param| MirParam {
                        name: param.name.clone(),
                        span: param.span,
                        ty: normalize_mir_type_label(&param.ty),
                    })
                    .collect(),
                body: self.lower_expr(body),
                captures: captures
                    .iter()
                    .map(|capture| MirLambdaCapture {
                        name: capture.name.clone(),
                        span: capture.span,
                        mutable: capture.mutable,
                    })
                    .collect(),
            },
            typed::TypedExprKind::Rec { target, ident } => MirExprKind::Rec {
                target: self.lower_expr(target),
                ident: ident.clone(),
            },
            typed::TypedExprKind::Block {
                statements,
                tail,
                defers,
            } => {
                let statements = statements
                    .iter()
                    .map(|stmt| self.lower_stmt(stmt))
                    .collect::<Vec<_>>();
                let defers = defers
                    .iter()
                    .map(|defer| self.lower_expr(defer))
                    .collect::<Vec<_>>();
                let defer_lifo = defers.iter().copied().rev().collect::<Vec<_>>();
                MirExprKind::Block {
                    statements,
                    tail: tail.as_ref().map(|tail| self.lower_expr(tail)),
                    defers,
                    defer_lifo,
                }
            }
            typed::TypedExprKind::Return { value } => MirExprKind::Return {
                value: value.as_ref().map(|value| self.lower_expr(value)),
            },
            typed::TypedExprKind::Propagate { expr } => MirExprKind::Propagate {
                expr: self.lower_expr(expr),
            },
            typed::TypedExprKind::Binary {
                operator,
                left,
                right,
            } => MirExprKind::Binary {
                operator: operator.clone(),
                left: self.lower_expr(left),
                right: self.lower_expr(right),
            },
            typed::TypedExprKind::FieldAccess { target, field } => MirExprKind::FieldAccess {
                target: self.lower_expr(target),
                field: field.name.clone(),
            },
            typed::TypedExprKind::TupleAccess { target, index } => MirExprKind::TupleAccess {
                target: self.lower_expr(target),
                index: *index,
            },
            typed::TypedExprKind::Index { target, index } => MirExprKind::Index {
                target: self.lower_expr(target),
                index: self.lower_expr(index),
            },
            typed::TypedExprKind::Match { target, arms } => MirExprKind::Match {
                target: self.lower_expr(target),
                arms: arms
                    .iter()
                    .map(|arm| MirMatchArm {
                        pattern: lower_pattern(&arm.pattern),
                        guard: arm.guard.as_ref().map(|guard| self.lower_expr(guard)),
                        alias: arm.alias.clone(),
                        body: self.lower_expr(&arm.body),
                    })
                    .collect(),
                lowering: build_match_lowering(expr.span, target, arms),
            },
            typed::TypedExprKind::IfElse {
                condition,
                then_branch,
                else_branch,
            } => MirExprKind::IfElse {
                condition: self.lower_expr(condition),
                then_branch: self.lower_expr(then_branch),
                else_branch: self.lower_expr(else_branch),
            },
            typed::TypedExprKind::PerformCall { call } => MirExprKind::PerformCall {
                call: MirEffectCall {
                    effect: call.effect.clone(),
                    argument: self.lower_expr(&call.argument),
                },
            },
            typed::TypedExprKind::EffectBlock { body } => MirExprKind::EffectBlock {
                body: self.lower_expr(body),
            },
            typed::TypedExprKind::Async { body, is_move } => MirExprKind::Async {
                body: self.lower_expr(body),
                is_move: *is_move,
            },
            typed::TypedExprKind::Await { expr } => MirExprKind::Await {
                expr: self.lower_expr(expr),
            },
            typed::TypedExprKind::Unsafe { body } => MirExprKind::Unsafe {
                body: self.lower_expr(body),
            },
            typed::TypedExprKind::InlineAsm {
                template,
                outputs,
                inputs,
                clobbers,
                options,
            } => MirExprKind::InlineAsm {
                template: template.clone(),
                outputs: outputs
                    .iter()
                    .map(|output| MirInlineAsmOutput {
                        constraint: output.constraint.clone(),
                        target: self.lower_expr(&output.target),
                    })
                    .collect(),
                inputs: inputs
                    .iter()
                    .map(|input| MirInlineAsmInput {
                        constraint: input.constraint.clone(),
                        expr: self.lower_expr(&input.expr),
                    })
                    .collect(),
                clobbers: clobbers.clone(),
                options: options.clone(),
            },
            typed::TypedExprKind::LlvmIr {
                result_type,
                template,
                inputs,
            } => MirExprKind::LlvmIr {
                result_type: result_type.clone(),
                template: template.clone(),
                inputs: inputs.iter().map(|input| self.lower_expr(input)).collect(),
            },
            typed::TypedExprKind::Unknown => MirExprKind::Unknown,
        };
        let qualified_call = match &expr.kind {
            typed::TypedExprKind::Call {
                qualified, args, ..
            } => qualified.as_ref().map(|call| {
                let receiver_ty = match call.kind {
                    typed::QualifiedCallKind::TypeMethod
                    | typed::QualifiedCallKind::TraitMethod => {
                        args.first().map(|arg| normalize_mir_type_label(&arg.ty))
                    }
                    _ => None,
                };
                MirQualifiedCall {
                    kind: map_qualified_call_kind(&call.kind),
                    owner: call.owner.clone(),
                    name: call.name.clone(),
                    impl_id: call.impl_id.clone(),
                    receiver_ty,
                    impl_candidates: Vec::new(),
                    span: Some(expr.span),
                }
            }),
            _ => None,
        };
        self.push_expr(
            expr.span,
            expr.ty.clone(),
            expr.dict_ref_ids.clone(),
            kind,
            qualified_call,
        )
    }

    fn lower_stmt(&mut self, stmt: &typed::TypedStmt) -> MirStmt {
        let kind = match &stmt.kind {
            typed::TypedStmtKind::Let { pattern, value } => MirStmtKind::Let {
                pattern: lower_pattern(pattern),
                value: self.lower_expr(value),
                mutable: false,
            },
            typed::TypedStmtKind::Var { pattern, value } => MirStmtKind::Let {
                pattern: lower_pattern(pattern),
                value: self.lower_expr(value),
                mutable: true,
            },
            typed::TypedStmtKind::Expr { expr } => MirStmtKind::Expr {
                expr: self.lower_expr(expr),
            },
            typed::TypedStmtKind::Assign { target, value } => MirStmtKind::Assign {
                target: self.lower_expr(target),
                value: self.lower_expr(value),
            },
            typed::TypedStmtKind::Defer { expr } => MirStmtKind::Defer {
                expr: self.lower_expr(expr),
            },
        };
        MirStmt {
            span: stmt.span,
            kind,
        }
    }

    fn push_expr(
        &mut self,
        span: Span,
        ty: String,
        dict_ref_ids: Vec<typed::DictRefId>,
        kind: MirExprKind,
        qualified_call: Option<MirQualifiedCall>,
    ) -> MirExprId {
        let id = self.exprs.len();
        let ty = normalize_mir_type_label(&ty);
        if let Some(call) = qualified_call {
            let key = format!("{}#{}", self.owner, id);
            self.qualified_calls.insert(key, call);
        }
        self.exprs.push(MirExpr {
            id,
            span,
            ty,
            dict_ref_ids,
            kind,
        });
        id
    }

    fn finish(self) -> (Vec<MirExpr>, BTreeMap<String, MirQualifiedCall>) {
        (self.exprs, self.qualified_calls)
    }
}

fn map_qualified_call_kind(kind: &typed::QualifiedCallKind) -> MirQualifiedCallKind {
    match kind {
        typed::QualifiedCallKind::TypeMethod => MirQualifiedCallKind::TypeMethod,
        typed::QualifiedCallKind::TypeAssoc => MirQualifiedCallKind::TypeAssoc,
        typed::QualifiedCallKind::TraitMethod => MirQualifiedCallKind::TraitMethod,
        typed::QualifiedCallKind::Unknown => MirQualifiedCallKind::Unknown,
    }
}

fn lower_pattern(pattern: &typed::TypedPattern) -> MirPattern {
    let kind = match &pattern.kind {
        typed::TypedPatternKind::Wildcard => MirPatternKind::Wildcard,
        typed::TypedPatternKind::Var { name } => MirPatternKind::Var { name: name.clone() },
        typed::TypedPatternKind::Literal(literal) => MirPatternKind::Literal(literal.clone()),
        typed::TypedPatternKind::Tuple { elements } => MirPatternKind::Tuple {
            elements: elements.iter().map(lower_pattern).collect(),
        },
        typed::TypedPatternKind::Record { fields, has_rest } => MirPatternKind::Record {
            fields: fields
                .iter()
                .map(|field| MirPatternRecordField {
                    key: field.key.clone(),
                    value: field
                        .value
                        .as_ref()
                        .map(|value| Box::new(lower_pattern(value))),
                })
                .collect(),
            has_rest: *has_rest,
        },
        typed::TypedPatternKind::Constructor { name, args } => MirPatternKind::Constructor {
            name: name.clone(),
            args: args.iter().map(lower_pattern).collect(),
        },
        typed::TypedPatternKind::Binding {
            name,
            pattern,
            via_at,
        } => MirPatternKind::Binding {
            name: name.clone(),
            pattern: Box::new(lower_pattern(pattern)),
            via_at: *via_at,
        },
        typed::TypedPatternKind::Or { variants } => MirPatternKind::Or {
            variants: variants.iter().map(lower_pattern).collect(),
        },
        typed::TypedPatternKind::Slice { elements } => {
            let mut head = Vec::new();
            let mut tail = Vec::new();
            let mut rest = None;
            let mut seen_rest = false;
            for item in elements {
                match item {
                    typed::TypedSlicePatternItem::Element(pat) => {
                        if seen_rest {
                            tail.push(lower_pattern(pat));
                        } else {
                            head.push(lower_pattern(pat));
                        }
                    }
                    typed::TypedSlicePatternItem::Rest { ident } => {
                        seen_rest = true;
                        rest = Some(MirSliceRest {
                            binding: ident.clone(),
                        });
                    }
                }
            }
            MirPatternKind::Slice(MirSlicePattern { head, rest, tail })
        }
        typed::TypedPatternKind::Range {
            start,
            end,
            inclusive,
        } => MirPatternKind::Range {
            start: start.as_ref().map(|value| Box::new(lower_pattern(value))),
            end: end.as_ref().map(|value| Box::new(lower_pattern(value))),
            inclusive: *inclusive,
        },
        typed::TypedPatternKind::Regex { pattern } => MirPatternKind::Regex {
            pattern: pattern.clone(),
        },
        typed::TypedPatternKind::ActivePattern {
            name,
            is_partial,
            argument,
        } => MirPatternKind::Active(MirActivePatternCall {
            name: name.clone(),
            kind: if *is_partial {
                typed::ActivePatternKind::Partial
            } else {
                typed::ActivePatternKind::Total
            },
            argument: argument
                .as_ref()
                .map(|value| Box::new(lower_pattern(value))),
            input_binding: None,
            miss_target: if *is_partial {
                Some(MirJumpTarget::NextArm)
            } else {
                None
            },
        }),
    };
    MirPattern {
        span: pattern.span,
        ty: None,
        kind,
    }
}

fn build_match_lowering(
    match_span: Span,
    target: &typed::TypedExpr,
    arms: &[typed::TypedMatchArm],
) -> MatchLoweringPlan {
    let arms_lowered = arms
        .iter()
        .map(|arm| MatchArmLowering {
            pattern: lower_pattern_for_lowering(&arm.pattern),
            has_guard: arm.guard.is_some(),
            alias: arm.alias.clone(),
        })
        .collect::<Vec<_>>();
    MatchLoweringPlan {
        owner: "<inline match>".to_string(),
        span: match_span,
        target_type: target.ty.clone(),
        arm_count: arms.len(),
        arms: arms_lowered,
    }
}

fn lower_pattern_for_lowering(pattern: &typed::TypedPattern) -> PatternLowering {
    match &pattern.kind {
        typed::TypedPatternKind::Wildcard => PatternLowering {
            label: "_".to_string(),
            miss_on_none: false,
            always_matches: true,
            children: Vec::new(),
        },
        typed::TypedPatternKind::Var { name } => PatternLowering {
            label: format!("var({})", name),
            miss_on_none: false,
            always_matches: true,
            children: Vec::new(),
        },
        typed::TypedPatternKind::Literal(literal) => PatternLowering {
            label: format!("literal({:?})", literal),
            miss_on_none: false,
            always_matches: false,
            children: Vec::new(),
        },
        typed::TypedPatternKind::Tuple { elements } => PatternLowering {
            label: "tuple".to_string(),
            miss_on_none: false,
            always_matches: false,
            children: elements.iter().map(lower_pattern_for_lowering).collect(),
        },
        typed::TypedPatternKind::Record { fields, has_rest } => {
            let mut children = fields
                .iter()
                .filter_map(|field| {
                    field
                        .value
                        .as_ref()
                        .map(|value| lower_pattern_for_lowering(value))
                })
                .collect::<Vec<_>>();
            if *has_rest {
                children.push(PatternLowering {
                    label: "rest".to_string(),
                    miss_on_none: false,
                    always_matches: true,
                    children: Vec::new(),
                });
            }
            PatternLowering {
                label: "record".to_string(),
                miss_on_none: false,
                always_matches: false,
                children,
            }
        }
        typed::TypedPatternKind::Constructor { name, args } => PatternLowering {
            label: format!("ctor({})", name),
            miss_on_none: false,
            always_matches: false,
            children: args.iter().map(lower_pattern_for_lowering).collect(),
        },
        typed::TypedPatternKind::Binding {
            name,
            pattern,
            via_at,
        } => {
            let child = lower_pattern_for_lowering(pattern);
            PatternLowering {
                label: if *via_at {
                    format!("binding(@ {})", name)
                } else {
                    format!("binding(as {})", name)
                },
                miss_on_none: child.miss_on_none,
                always_matches: child.always_matches,
                children: vec![child],
            }
        }
        typed::TypedPatternKind::Or { variants } => {
            let lowered_variants: Vec<_> =
                variants.iter().map(lower_pattern_for_lowering).collect();
            let miss_on_none = lowered_variants.iter().any(|variant| variant.miss_on_none);
            PatternLowering {
                label: "or".to_string(),
                miss_on_none,
                always_matches: false,
                children: lowered_variants,
            }
        }
        typed::TypedPatternKind::Slice { elements } => PatternLowering {
            label: "slice".to_string(),
            miss_on_none: false,
            always_matches: false,
            children: elements
                .iter()
                .map(|item| match item {
                    typed::TypedSlicePatternItem::Element(pat) => lower_pattern_for_lowering(pat),
                    typed::TypedSlicePatternItem::Rest { ident } => PatternLowering {
                        label: ident
                            .as_ref()
                            .map(|name| format!("rest({})", name))
                            .unwrap_or_else(|| "rest".to_string()),
                        miss_on_none: false,
                        always_matches: true,
                        children: Vec::new(),
                    },
                })
                .collect(),
        },
        typed::TypedPatternKind::Range {
            start,
            end,
            inclusive,
        } => {
            let mut children = Vec::new();
            if let Some(start_pat) = start {
                children.push(lower_pattern_for_lowering(start_pat));
            }
            if let Some(end_pat) = end {
                children.push(lower_pattern_for_lowering(end_pat));
            }
            PatternLowering {
                label: if *inclusive {
                    "range(..=)".to_string()
                } else {
                    "range(..)".to_string()
                },
                miss_on_none: false,
                always_matches: false,
                children,
            }
        }
        typed::TypedPatternKind::Regex { pattern } => PatternLowering {
            label: format!("regex({})", pattern),
            miss_on_none: false,
            always_matches: false,
            children: Vec::new(),
        },
        typed::TypedPatternKind::ActivePattern {
            name,
            is_partial,
            argument,
        } => {
            let child = argument.as_ref().map(|arg| lower_pattern_for_lowering(arg));
            PatternLowering {
                label: if *is_partial {
                    format!("active(|{}|_|)", name)
                } else {
                    format!("active(|{}|)", name)
                },
                miss_on_none: *is_partial,
                always_matches: !*is_partial,
                children: child.into_iter().collect(),
            }
        }
    }
}

pub fn build_match_lowerings(module: &typed::TypedModule) -> Vec<MatchLoweringPlan> {
    let mut plans = Vec::new();
    for function in &module.functions {
        collect_match_lowerings_from_expr(
            &function.body,
            &format!("fn {}", function.name),
            &mut plans,
        );
    }
    for active in &module.active_patterns {
        let owner = format!("active {}", active.name);
        collect_match_lowerings_from_expr(&active.body, &owner, &mut plans);
    }
    plans
}

fn collect_match_lowerings_from_expr(
    expr: &typed::TypedExpr,
    owner: &str,
    plans: &mut Vec<MatchLoweringPlan>,
) {
    match &expr.kind {
        typed::TypedExprKind::Match { target, arms } => {
            let lowering = build_match_lowering(expr.span, target, arms);
            plans.push(MatchLoweringPlan {
                owner: owner.to_string(),
                ..lowering
            });
            collect_match_lowerings_from_expr(target, owner, plans);
            for arm in arms {
                if let Some(guard) = &arm.guard {
                    collect_match_lowerings_from_expr(guard, owner, plans);
                }
                collect_match_lowerings_from_expr(&arm.body, owner, plans);
            }
        }
        typed::TypedExprKind::Call {
            callee,
            args,
            qualified: _,
        } => {
            collect_match_lowerings_from_expr(callee, owner, plans);
            for arg in args {
                collect_match_lowerings_from_expr(arg, owner, plans);
            }
        }
        typed::TypedExprKind::FieldAccess { target, .. }
        | typed::TypedExprKind::TupleAccess { target, .. } => {
            collect_match_lowerings_from_expr(target, owner, plans);
        }
        typed::TypedExprKind::Index { target, index } => {
            collect_match_lowerings_from_expr(target, owner, plans);
            collect_match_lowerings_from_expr(index, owner, plans);
        }
        typed::TypedExprKind::Binary { left, right, .. } => {
            collect_match_lowerings_from_expr(left, owner, plans);
            collect_match_lowerings_from_expr(right, owner, plans);
        }
        typed::TypedExprKind::IfElse {
            condition,
            then_branch,
            else_branch,
        } => {
            collect_match_lowerings_from_expr(condition, owner, plans);
            collect_match_lowerings_from_expr(then_branch, owner, plans);
            collect_match_lowerings_from_expr(else_branch, owner, plans);
        }
        typed::TypedExprKind::Lambda { body, .. } => {
            collect_match_lowerings_from_expr(body, owner, plans);
        }
        typed::TypedExprKind::Rec { target, .. } => {
            collect_match_lowerings_from_expr(target, owner, plans);
        }
        typed::TypedExprKind::Block {
            statements,
            tail,
            defers,
        } => {
            for stmt in statements {
                collect_match_lowerings_from_stmt(stmt, owner, plans);
            }
            if let Some(tail) = tail {
                collect_match_lowerings_from_expr(tail, owner, plans);
            }
            for defer in defers {
                collect_match_lowerings_from_expr(defer, owner, plans);
            }
        }
        typed::TypedExprKind::Return { value } => {
            if let Some(value) = value {
                collect_match_lowerings_from_expr(value, owner, plans);
            }
        }
        typed::TypedExprKind::Propagate { expr } => {
            collect_match_lowerings_from_expr(expr, owner, plans);
        }
        typed::TypedExprKind::PerformCall { call } => {
            collect_match_lowerings_from_expr(&call.argument, owner, plans);
        }
        typed::TypedExprKind::InlineAsm {
            outputs, inputs, ..
        } => {
            for output in outputs {
                collect_match_lowerings_from_expr(&output.target, owner, plans);
            }
            for input in inputs {
                collect_match_lowerings_from_expr(&input.expr, owner, plans);
            }
        }
        typed::TypedExprKind::LlvmIr { inputs, .. } => {
            for input in inputs {
                collect_match_lowerings_from_expr(input, owner, plans);
            }
        }
        typed::TypedExprKind::EffectBlock { body }
        | typed::TypedExprKind::Async { body, .. }
        | typed::TypedExprKind::Unsafe { body } => {
            collect_match_lowerings_from_expr(body, owner, plans);
        }
        typed::TypedExprKind::Await { expr } => {
            collect_match_lowerings_from_expr(expr, owner, plans);
        }
        typed::TypedExprKind::Literal(_)
        | typed::TypedExprKind::Identifier { .. }
        | typed::TypedExprKind::Unknown => {}
    }
}

fn collect_match_lowerings_from_stmt(
    stmt: &typed::TypedStmt,
    owner: &str,
    plans: &mut Vec<MatchLoweringPlan>,
) {
    match &stmt.kind {
        typed::TypedStmtKind::Let { value, .. } | typed::TypedStmtKind::Var { value, .. } => {
            collect_match_lowerings_from_expr(value, owner, plans);
        }
        typed::TypedStmtKind::Expr { expr } => {
            collect_match_lowerings_from_expr(expr, owner, plans);
        }
        typed::TypedStmtKind::Assign { target, value } => {
            collect_match_lowerings_from_expr(target, owner, plans);
            collect_match_lowerings_from_expr(value, owner, plans);
        }
        typed::TypedStmtKind::Defer { expr } => {
            collect_match_lowerings_from_expr(expr, owner, plans);
        }
    }
}
