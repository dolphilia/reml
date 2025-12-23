use serde::Serialize;

use crate::parser::ast::{Ident, Literal};
use crate::semantics::typed;
use crate::span::Span;

pub const MIR_SCHEMA_VERSION: &str = "frontend-mir/0.1";

pub type MirExprId = usize;

#[derive(Debug, Clone, Serialize)]
pub struct MirModule {
    pub schema_version: &'static str,
    pub functions: Vec<MirFunction>,
    pub active_patterns: Vec<MirActivePattern>,
    pub conductors: Vec<MirConductor>,
}

impl MirModule {
    pub fn from_typed_module(module: &typed::TypedModule) -> Self {
        let functions = module.functions.iter().map(lower_function).collect();
        let active_patterns = module
            .active_patterns
            .iter()
            .map(lower_active_pattern)
            .collect();
        let conductors = module.conductors.iter().map(lower_conductor).collect();
        Self {
            schema_version: MIR_SCHEMA_VERSION,
            functions,
            active_patterns,
            conductors,
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
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct MirFunction {
    pub name: String,
    pub span: Span,
    pub attributes: Vec<String>,
    pub params: Vec<MirParam>,
    pub return_type: String,
    pub body: MirExprId,
    pub exprs: Vec<MirExpr>,
    pub dict_ref_ids: Vec<typed::DictRefId>,
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
pub struct MirExpr {
    pub id: MirExprId,
    pub span: Span,
    pub ty: String,
    pub dict_ref_ids: Vec<typed::DictRefId>,
    pub kind: MirExprKind,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MirExprKind {
    Literal(Literal),
    Identifier { ident: Ident },
    Call {
        callee: MirExprId,
        args: Vec<MirExprId>,
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
    #[serde(rename = "unknown")]
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
pub struct MirEffectCall {
    pub effect: Ident,
    pub argument: MirExprId,
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
    Var { name: String },
    Literal(Literal),
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

fn lower_function(function: &typed::TypedFunction) -> MirFunction {
    let mut builder = MirExprBuilder::default();
    let body = builder.lower_expr(&function.body);
    MirFunction {
        name: function.name.clone(),
        span: function.span,
        attributes: function.attributes.clone(),
        params: function
            .params
            .iter()
            .map(|param| MirParam {
                name: param.name.clone(),
                span: param.span,
                ty: param.ty.clone(),
            })
            .collect(),
        return_type: function.return_type.clone(),
        body,
        exprs: builder.finish(),
        dict_ref_ids: function.dict_ref_ids.clone(),
    }
}

fn lower_active_pattern(pattern: &typed::TypedActivePattern) -> MirActivePattern {
    let mut builder = MirExprBuilder::default();
    let body = builder.lower_expr(&pattern.body);
    MirActivePattern {
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
                ty: param.ty.clone(),
            })
            .collect(),
        body,
        exprs: builder.finish(),
        dict_ref_ids: pattern.dict_ref_ids.clone(),
    }
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
                target_type: dsl_def.target_type.clone(),
                pipeline_type: dsl_def.pipeline_type.clone(),
                tails: dsl_def
                    .tails
                    .iter()
                    .map(|tail| MirConductorDslTail {
                        stage: tail.stage.clone(),
                        arg_types: tail.arg_types.clone(),
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
            ty: block.ty.clone(),
            span: block.span,
        }),
        monitoring: conductor
            .monitoring
            .as_ref()
            .map(|block| MirConductorMonitoringBlock {
                target: block.target.clone(),
                ty: block.ty.clone(),
                span: block.span,
            }),
    }
}

#[derive(Default)]
struct MirExprBuilder {
    exprs: Vec<MirExpr>,
}

impl MirExprBuilder {
    fn lower_expr(&mut self, expr: &typed::TypedExpr) -> MirExprId {
        let kind = match &expr.kind {
            typed::TypedExprKind::Literal(literal) => MirExprKind::Literal(literal.clone()),
            typed::TypedExprKind::Identifier { ident } => MirExprKind::Identifier {
                ident: ident.clone(),
            },
            typed::TypedExprKind::Call { callee, args } => MirExprKind::Call {
                callee: self.lower_expr(callee),
                args: args.iter().map(|arg| self.lower_expr(arg)).collect(),
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
        typed::TypedExprKind::Unknown => MirExprKind::Unknown,
    };
        self.push_expr(expr.span, expr.ty.clone(), expr.dict_ref_ids.clone(), kind)
    }

    fn push_expr(
        &mut self,
        span: Span,
        ty: String,
        dict_ref_ids: Vec<typed::DictRefId>,
        kind: MirExprKind,
    ) -> MirExprId {
        let id = self.exprs.len();
        self.exprs.push(MirExpr {
            id,
            span,
            ty,
            dict_ref_ids,
            kind,
        });
        id
    }

    fn finish(self) -> Vec<MirExpr> {
        self.exprs
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
                    value: field.value.as_ref().map(|value| Box::new(lower_pattern(value))),
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
            argument: argument.as_ref().map(|value| Box::new(lower_pattern(value))),
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
                .filter_map(|field| field.value.as_ref().map(|value| lower_pattern_for_lowering(value)))
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
            let lowered_variants: Vec<_> = variants.iter().map(lower_pattern_for_lowering).collect();
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
                    typed::TypedSlicePatternItem::Element(pat) => {
                        lower_pattern_for_lowering(pat)
                    }
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
        typed::TypedExprKind::Call { callee, args } => {
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
        typed::TypedExprKind::PerformCall { call } => {
            collect_match_lowerings_from_expr(&call.argument, owner, plans);
        }
        typed::TypedExprKind::Literal(_)
        | typed::TypedExprKind::Identifier { .. }
        | typed::TypedExprKind::Unknown => {}
    }
}
