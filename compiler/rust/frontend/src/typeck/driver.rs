use std::collections::HashSet;

use once_cell::sync::Lazy;
use serde::Serialize;

use super::capability::{CapabilityDescriptor, EffectUsage};
use super::constraint::{iterator, Constraint, ConstraintSolver, Substitution};
use super::env::{StageRequirement, TypeEnv, TypecheckConfig};
use super::metrics::TypecheckMetrics;
use super::scheme::Scheme;
use super::types::{BuiltinType, Type, TypeVarGen};
use crate::diagnostic::{ExpectedToken, ExpectedTokenCollector, ExpectedTokensSummary};
use crate::parser::ast::{Expr, ExprKind, Function, Ident, Literal, LiteralKind, Module};
use crate::semantics::typed;
use crate::span::Span;

/// 型推論の簡易ドライバ。現時点では AST を走査して
/// メトリクスとサマリ情報のみを生成する。
pub struct TypecheckDriver;

impl TypecheckDriver {
    pub fn infer_module(module: Option<&Module>, config: &TypecheckConfig) -> TypecheckReport {
        match module {
            Some(module) => Self::infer_module_from_ast(module, config),
            None => {
                let mut report = TypecheckReport::default();
                report
                    .violations
                    .push(TypecheckViolation::ast_unavailable());
                report
            }
        }
    }

    fn infer_module_from_ast(module: &Module, config: &TypecheckConfig) -> TypecheckReport {
        let mut metrics = TypecheckMetrics::default();
        let mut functions = Vec::new();
        let mut violations = Vec::new();
        let mut typed_module = typed::TypedModule::default();
        let mut dict_ref_drafts = Vec::new();
        let mut all_constraints = Vec::new();

        if config.trace_enabled {
            eprintln!(
                "[TRACE] typecheck.start functions={}",
                module.functions.len()
            );
        }

        let mut solver = ConstraintSolver::new();
        let mut var_gen = TypeVarGen::default();
        let mut module_env = TypeEnv::new();

        for function in &module.functions {
            metrics.record_function();
            let mut stats = FunctionStats::default();
            let mut constraints = Vec::new();
            let mut env = module_env.clone();
            let mut param_bindings = Vec::new();

            for param in &function.params {
                let ty = var_gen.fresh_type();
                env.insert(param.name.name.clone(), Scheme::simple(ty.clone()));
                param_bindings.push(ParamBinding {
                    name: param.name.name.clone(),
                    span: param.name.span,
                    ty,
                });
            }

            let typed_body = infer_function(
                function,
                function.name.name.as_str(),
                &mut env,
                &mut var_gen,
                &mut solver,
                &mut constraints,
                &mut stats,
                &mut metrics,
                &mut violations,
                &mut dict_ref_drafts,
            );

            all_constraints.extend(constraints.drain(..));

            let substitution = solver.substitution().clone();
            let resolved_return = substitution.apply(&typed_body.ty);
            let param_types = param_bindings
                .iter()
                .map(|binding| substitution.apply(&binding.ty))
                .collect::<Vec<_>>();
            let function_type = Type::arrow(param_types.clone(), resolved_return.clone());
            let scheme = generalize_function_type(&module_env, function_type);
            let scheme_id = typed_module.schemes.len();
            typed_module
                .schemes
                .push(build_scheme_info(scheme_id, &scheme, &substitution));
            module_env.insert(function.name.name.clone(), scheme.clone());

            let typed_params = param_bindings
                .into_iter()
                .map(|binding| typed::TypedParam {
                    name: binding.name,
                    span: binding.span,
                    ty: substitution.apply(&binding.ty).label(),
                })
                .collect::<Vec<_>>();
            let param_type_labels = typed_params
                .iter()
                .map(|param| param.ty.clone())
                .collect::<Vec<_>>();

            let typed_body = finalize_typed_expr(typed_body, &substitution);
            let dict_ref_ids = typed_body.dict_ref_ids.clone();
            let return_label = resolved_return.label();

            functions.push(TypedFunctionSummary {
                name: function.name.name.clone(),
                param_types: param_type_labels,
                return_type: return_label.clone(),
                typed_exprs: stats.typed_exprs,
                constraints: stats.constraints,
                unresolved_identifiers: stats.unresolved_identifiers,
            });

            typed_module.functions.push(typed::TypedFunction {
                name: function.name.name.clone(),
                span: function.span,
                params: typed_params,
                return_type: return_label,
                body: typed_body,
                dict_ref_ids,
                scheme_id: Some(scheme_id),
            });
        }

        if config.trace_enabled {
            eprintln!("[TRACE] typecheck.finish");
        }

        let final_substitution = solver.substitution().clone();
        let iterator_stage_violations =
            detect_iterator_stage_mismatches(&dict_ref_drafts, &final_substitution, config);
        violations.extend(iterator_stage_violations);
        violations.extend(detect_capability_violations(module, config));
        let violations = compress_typecheck_violations(violations);

        let used_impls = all_constraints
            .iter()
            .filter_map(|constraint| match constraint {
                Constraint::ImplBound { implementation, .. } => Some(implementation.to_string()),
                _ => None,
            })
            .collect::<Vec<_>>();
        let dict_refs = dict_ref_drafts
            .into_iter()
            .enumerate()
            .map(|(id, draft)| typed::DictRef {
                id,
                impl_id: draft.impl_id,
                span: draft.span,
                requirements: draft.requirements,
                ty: final_substitution.apply(&draft.ty).label(),
            })
            .collect::<Vec<_>>();
        typed_module.dict_refs = dict_refs;

        TypecheckReport {
            metrics,
            functions,
            violations,
            typed_module,
            constraints: all_constraints,
            used_impls,
        }
    }
}

#[derive(Debug, Serialize, Default, Clone)]
pub struct TypecheckReport {
    pub metrics: TypecheckMetrics,
    pub functions: Vec<TypedFunctionSummary>,
    pub violations: Vec<TypecheckViolation>,
    pub typed_module: typed::TypedModule,
    pub constraints: Vec<Constraint>,
    pub used_impls: Vec<String>,
}

static TOP_LEVEL_DECLARATION_SUMMARY: Lazy<ExpectedTokensSummary> = Lazy::new(|| {
    let mut collector = ExpectedTokenCollector::new();
    collector.extend([
        ExpectedToken::keyword("effect"),
        ExpectedToken::keyword("extern"),
        ExpectedToken::keyword("fn"),
        ExpectedToken::keyword("handler"),
        ExpectedToken::keyword("impl"),
        ExpectedToken::keyword("let"),
        ExpectedToken::keyword("pub"),
        ExpectedToken::keyword("trait"),
        ExpectedToken::keyword("type"),
        ExpectedToken::keyword("var"),
    ]);
    collector.push_token("@");
    collector.push(ExpectedToken::eof());
    collector.summarize()
});

fn top_level_declaration_summary() -> ExpectedTokensSummary {
    TOP_LEVEL_DECLARATION_SUMMARY.clone()
}

#[derive(Debug, Serialize, Clone)]
pub struct TypedFunctionSummary {
    pub name: String,
    pub param_types: Vec<String>,
    pub return_type: String,
    pub typed_exprs: usize,
    pub constraints: usize,
    pub unresolved_identifiers: usize,
}

#[derive(Debug, Serialize, Clone)]
pub struct TypecheckViolation {
    pub kind: TypecheckViolationKind,
    pub code: &'static str,
    pub message: String,
    pub span: Option<Span>,
    pub notes: Vec<ViolationNote>,
    pub capability: Option<String>,
    pub function: Option<String>,
    #[serde(skip_serializing)]
    expected: Option<ExpectedTokensSummary>,
    #[serde(skip_serializing)]
    pub iterator_stage: Option<IteratorStageViolationInfo>,
}

#[derive(Debug, Serialize, Clone)]
pub enum TypecheckViolationKind {
    ConditionLiteralBool,
    AstUnavailable,
    ResidualLeak,
    StageMismatch,
    IteratorStageMismatch,
}

#[derive(Debug, Serialize, Clone)]
pub struct IteratorStageViolationInfo {
    pub required: StageRequirement,
    pub actual: StageRequirement,
    pub capability: Option<String>,
    pub kind: String,
    pub source: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct ViolationNote {
    pub label: Option<String>,
    pub message: String,
}

impl ViolationNote {
    fn plain(message: impl Into<String>) -> Self {
        Self {
            label: None,
            message: message.into(),
        }
    }

    #[allow(dead_code)]
    fn labeled(label: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            label: Some(label.into()),
            message: message.into(),
        }
    }
}

impl TypecheckViolation {
    fn condition_literal_bool(span: Span, actual: Type, function: Option<String>) -> Self {
        Self {
            kind: TypecheckViolationKind::ConditionLiteralBool,
            code: "E7006",
            message: "条件式は Bool 型である必要があります".to_string(),
            span: Some(span),
            notes: vec![ViolationNote::plain(format!(
                "この条件式の型は {} です",
                actual.label()
            ))],
            capability: None,
            function,
            expected: None,
            iterator_stage: None,
        }
    }

    fn residual_leak(span: Option<Span>, capability: Option<String>) -> Self {
        let note_message = capability
            .as_ref()
            .map(|cap| format!("`{cap}` のハンドラが宣言されていません"))
            .unwrap_or_else(|| "宣言された効果集合が残余集合を包含していません".to_string());
        Self {
            kind: TypecheckViolationKind::ResidualLeak,
            code: "effects.contract.residual_leak",
            message: "残余効果が閉じていません".to_string(),
            span,
            notes: vec![ViolationNote::plain(note_message)],
            capability,
            function: None,
            expected: None,
            iterator_stage: None,
        }
    }

    fn stage_mismatch(
        span: Option<Span>,
        capability: String,
        required: StageRequirement,
        actual: StageRequirement,
    ) -> Self {
        let message = format!(
            "`{capability}` を呼び出すにはステージ `{}` が必要ですが、実行時ステージ `{}` では許可されていません",
            required.label(),
            actual.label()
        );
        let note_message = format!(
            "要求: `{}` / 実行時: `{}`",
            required.label(),
            actual.label()
        );
        Self {
            kind: TypecheckViolationKind::StageMismatch,
            code: "effects.contract.stage_mismatch",
            message,
            span,
            notes: vec![ViolationNote::plain(note_message)],
            capability: Some(capability),
            function: None,
            expected: None,
            iterator_stage: None,
        }
        .with_expected_summary(top_level_declaration_summary())
    }

    fn iterator_stage_mismatch(
        span: Option<Span>,
        snapshot: iterator::IteratorStageSnapshot,
        actual: StageRequirement,
    ) -> Self {
        let capability = snapshot
            .capability
            .clone()
            .map(|cap| cap.to_string())
            .unwrap_or_else(|| "core.iter.custom".to_string());
        let required_label = snapshot.required.label();
        let actual_label = actual.label();
        let message = format!(
            "Iterator `{}` はステージ `{}` を要求しますが、実行時ステージ `{}` では利用できません",
            snapshot.source, required_label, actual_label
        );
        let kind_label = snapshot.kind.clone().to_string();
        let note_message = format!(
            "Iterator kind `{}` / capability `{}`",
            kind_label, capability
        );
        Self {
            kind: TypecheckViolationKind::IteratorStageMismatch,
            code: "typeclass.iterator.stage_mismatch",
            message,
            span,
            notes: vec![ViolationNote::plain(note_message)],
            capability: Some(capability.clone()),
            function: None,
            expected: None,
            iterator_stage: Some(IteratorStageViolationInfo {
                required: snapshot.required.clone(),
                actual,
                capability: Some(capability),
                kind: kind_label.clone(),
                source: snapshot.source,
            }),
        }
        .with_expected_summary(top_level_declaration_summary())
    }

    fn ast_unavailable() -> Self {
        Self {
            kind: TypecheckViolationKind::AstUnavailable,
            code: "typeck.aborted.ast_unavailable",
            message: "AST 生成に失敗したため型推論を実行できませんでした".to_string(),
            span: None,
            notes: vec![ViolationNote::plain(
                "パーサ診断を確認し、構文エラーを解消したうえで再実行してください",
            )],
            capability: None,
            function: None,
            expected: None,
            iterator_stage: None,
        }
    }

    pub fn domain(&self) -> &'static str {
        match self.kind {
            TypecheckViolationKind::ConditionLiteralBool
            | TypecheckViolationKind::AstUnavailable => "type",
            TypecheckViolationKind::ResidualLeak
            | TypecheckViolationKind::StageMismatch
            | TypecheckViolationKind::IteratorStageMismatch => "effects",
        }
    }

    fn with_expected_summary(mut self, summary: ExpectedTokensSummary) -> Self {
        self.expected = Some(summary);
        self
    }

    pub fn expected_summary(&self) -> Option<&ExpectedTokensSummary> {
        self.expected.as_ref()
    }
}

#[derive(Default)]
struct FunctionStats {
    typed_exprs: usize,
    constraints: usize,
    unresolved_identifiers: usize,
}

fn infer_function(
    function: &Function,
    function_name: &str,
    env: &mut TypeEnv,
    var_gen: &mut TypeVarGen,
    solver: &mut ConstraintSolver,
    constraints: &mut Vec<Constraint>,
    stats: &mut FunctionStats,
    metrics: &mut TypecheckMetrics,
    violations: &mut Vec<TypecheckViolation>,
    dict_refs: &mut Vec<DictRefDraft>,
) -> TypedExprDraft {
    infer_expr(
        &function.body,
        env,
        var_gen,
        solver,
        constraints,
        stats,
        metrics,
        violations,
        dict_refs,
        Some(function_name),
    )
}

fn infer_expr(
    expr: &Expr,
    env: &TypeEnv,
    var_gen: &mut TypeVarGen,
    solver: &mut ConstraintSolver,
    constraints: &mut Vec<Constraint>,
    stats: &mut FunctionStats,
    metrics: &mut TypecheckMetrics,
    violations: &mut Vec<TypecheckViolation>,
    dict_refs: &mut Vec<DictRefDraft>,
    function_name: Option<&str>,
) -> TypedExprDraft {
    stats.typed_exprs += 1;
    metrics.record_expr();
    metrics.record_ast_node();
    metrics.record_token_count(expr.span.len() as usize);
    match &expr.kind {
        ExprKind::Literal(literal) => {
            let ty = type_for_literal(literal);
            make_typed(
                expr,
                TypedExprKindDraft::Literal(literal.clone()),
                ty,
                Vec::new(),
            )
        }
        ExprKind::Identifier(ident) => {
            let mut ty = match env.lookup(ident.name.as_str()) {
                Some(binding) => binding.scheme.instantiate(var_gen),
                None => {
                    stats.unresolved_identifiers += 1;
                    metrics.record_unresolved_identifier();
                    Type::builtin(BuiltinType::Unknown)
                }
            };
            ty = solver.substitution().apply(&ty);
            make_typed(
                expr,
                TypedExprKindDraft::Identifier {
                    ident: ident.clone(),
                },
                ty,
                Vec::new(),
            )
        }
        ExprKind::Binary {
            operator,
            left,
            right,
        } => {
            metrics.record_binary_expr();
            let left_result = infer_expr(
                left,
                env,
                var_gen,
                solver,
                constraints,
                stats,
                metrics,
                violations,
                dict_refs,
                function_name,
            );
            let right_result = infer_expr(
                right,
                env,
                var_gen,
                solver,
                constraints,
                stats,
                metrics,
                violations,
                dict_refs,
                function_name,
            );
            stats.constraints += 1;
            metrics.record_constraint("binary.operands");
            constraints.push(Constraint::equal(
                left_result.ty.clone(),
                right_result.ty.clone(),
            ));
            metrics.record_unify_call();
            let _ = solver.unify(left_result.ty.clone(), right_result.ty.clone());
            let ty = combine_numeric_types(&left_result.ty, &right_result.ty);
            make_typed(
                expr,
                TypedExprKindDraft::Binary {
                    operator: operator.symbol().to_string(),
                    left: Box::new(left_result),
                    right: Box::new(right_result),
                },
                ty,
                Vec::new(),
            )
        }
        ExprKind::Call { callee, args } => {
            metrics.record_call_site();
            stats.constraints += 1;
            metrics.record_constraint("call.arity");
            let callee_result = infer_expr(
                callee,
                env,
                var_gen,
                solver,
                constraints,
                stats,
                metrics,
                violations,
                dict_refs,
                function_name,
            );
            let typed_args = args
                .iter()
                .map(|arg| {
                    infer_expr(
                        arg,
                        env,
                        var_gen,
                        solver,
                        constraints,
                        stats,
                        metrics,
                        violations,
                        dict_refs,
                        function_name,
                    )
                })
                .collect();
            make_typed(
                expr,
                TypedExprKindDraft::Call {
                    callee: Box::new(callee_result),
                    args: typed_args,
                },
                Type::builtin(BuiltinType::Unknown),
                Vec::new(),
            )
        }
        ExprKind::PerformCall { call } => {
            let argument_result = infer_expr(
                &call.argument,
                env,
                var_gen,
                solver,
                constraints,
                stats,
                metrics,
                violations,
                dict_refs,
                function_name,
            );
            constraints.push(Constraint::has_capability(
                Type::builtin(BuiltinType::Unknown),
                call.effect.name.clone(),
            ));
            let dict_ref_id = register_dict_ref(
                dict_refs,
                expr.span,
                call.effect.name.clone(),
                &argument_result.ty,
            );
            make_typed(
                expr,
                TypedExprKindDraft::PerformCall {
                    call: TypedEffectCallDraft {
                        effect: call.effect.clone(),
                        argument: Box::new(argument_result),
                    },
                },
                Type::builtin(BuiltinType::Unknown),
                vec![dict_ref_id],
            )
        }
        ExprKind::IfElse {
            condition,
            then_branch,
            else_branch,
        } => {
            let condition_result = infer_expr(
                condition,
                env,
                var_gen,
                solver,
                constraints,
                stats,
                metrics,
                violations,
                dict_refs,
                function_name,
            );
            check_bool_condition(
                condition.span(),
                &condition_result.ty,
                violations,
                function_name,
            );
            let then_result = infer_expr(
                then_branch,
                env,
                var_gen,
                solver,
                constraints,
                stats,
                metrics,
                violations,
                dict_refs,
                function_name,
            );
            let synthetic_else = Expr::literal(
                Literal {
                    value: LiteralKind::Unit,
                },
                expr.span(),
            );
            let else_expr = else_branch.as_deref().unwrap_or_else(|| &synthetic_else);
            let else_result = infer_expr(
                else_expr,
                env,
                var_gen,
                solver,
                constraints,
                stats,
                metrics,
                violations,
                dict_refs,
                function_name,
            );
            stats.constraints += 1;
            metrics.record_constraint("conditional");
            constraints.push(Constraint::equal(
                then_result.ty.clone(),
                else_result.ty.clone(),
            ));
            metrics.record_unify_call();
            let _ = solver.unify(then_result.ty.clone(), else_result.ty.clone());
            let ty = if then_result.ty == else_result.ty {
                then_result.ty.clone()
            } else {
                Type::builtin(BuiltinType::Unknown)
            };
            make_typed(
                expr,
                TypedExprKindDraft::IfElse {
                    condition: Box::new(condition_result),
                    then_branch: Box::new(then_result),
                    else_branch: Box::new(else_result),
                },
                ty,
                Vec::new(),
            )
        }
        _ => make_typed(
            expr,
            TypedExprKindDraft::Unknown,
            Type::builtin(BuiltinType::Unknown),
            Vec::new(),
        ),
    }
}

fn type_for_literal(literal: &Literal) -> Type {
    match literal {
        Literal {
            value: LiteralKind::Int { .. },
        } => Type::builtin(BuiltinType::Int),
        Literal {
            value: LiteralKind::Bool { .. },
        } => Type::builtin(BuiltinType::Bool),
        _ => Type::builtin(BuiltinType::Unknown),
    }
}

fn combine_numeric_types(left: &Type, right: &Type) -> Type {
    if matches!(left, Type::Builtin(BuiltinType::Int))
        && matches!(right, Type::Builtin(BuiltinType::Int))
    {
        Type::builtin(BuiltinType::Int)
    } else {
        Type::builtin(BuiltinType::Unknown)
    }
}

struct TypedExprDraft {
    span: Span,
    kind: TypedExprKindDraft,
    ty: Type,
    dict_ref_ids: Vec<typed::DictRefId>,
}

enum TypedExprKindDraft {
    Literal(Literal),
    Identifier {
        ident: Ident,
    },
    Call {
        callee: Box<TypedExprDraft>,
        args: Vec<TypedExprDraft>,
    },
    Binary {
        operator: String,
        left: Box<TypedExprDraft>,
        right: Box<TypedExprDraft>,
    },
    PerformCall {
        call: TypedEffectCallDraft,
    },
    IfElse {
        condition: Box<TypedExprDraft>,
        then_branch: Box<TypedExprDraft>,
        else_branch: Box<TypedExprDraft>,
    },
    Unknown,
}

struct TypedEffectCallDraft {
    effect: Ident,
    argument: Box<TypedExprDraft>,
}

struct DictRefDraft {
    impl_id: String,
    span: Span,
    requirements: Vec<String>,
    ty: Type,
}

struct ParamBinding {
    name: String,
    span: Span,
    ty: Type,
}

fn make_typed(
    expr: &Expr,
    kind: TypedExprKindDraft,
    ty: Type,
    dict_ref_ids: Vec<typed::DictRefId>,
) -> TypedExprDraft {
    TypedExprDraft {
        span: expr.span,
        kind,
        ty,
        dict_ref_ids,
    }
}

fn finalize_typed_expr(expr: TypedExprDraft, substitution: &Substitution) -> typed::TypedExpr {
    let ty = substitution.apply(&expr.ty);
    let kind = match expr.kind {
        TypedExprKindDraft::Literal(literal) => typed::TypedExprKind::Literal(literal),
        TypedExprKindDraft::Identifier { ident } => typed::TypedExprKind::Identifier { ident },
        TypedExprKindDraft::Binary {
            operator,
            left,
            right,
        } => typed::TypedExprKind::Binary {
            operator,
            left: Box::new(finalize_typed_expr(*left, substitution)),
            right: Box::new(finalize_typed_expr(*right, substitution)),
        },
        TypedExprKindDraft::Call { callee, args } => typed::TypedExprKind::Call {
            callee: Box::new(finalize_typed_expr(*callee, substitution)),
            args: args
                .into_iter()
                .map(|arg| finalize_typed_expr(arg, substitution))
                .collect(),
        },
        TypedExprKindDraft::PerformCall { call } => typed::TypedExprKind::PerformCall {
            call: typed::TypedEffectCall {
                effect: call.effect,
                argument: Box::new(finalize_typed_expr(*call.argument, substitution)),
            },
        },
        TypedExprKindDraft::IfElse {
            condition,
            then_branch,
            else_branch,
        } => typed::TypedExprKind::IfElse {
            condition: Box::new(finalize_typed_expr(*condition, substitution)),
            then_branch: Box::new(finalize_typed_expr(*then_branch, substitution)),
            else_branch: Box::new(finalize_typed_expr(*else_branch, substitution)),
        },
        TypedExprKindDraft::Unknown => typed::TypedExprKind::Unknown,
    };
    typed::TypedExpr {
        span: expr.span,
        kind,
        ty: ty.label(),
        dict_ref_ids: expr.dict_ref_ids,
    }
}

fn register_dict_ref(
    dict_refs: &mut Vec<DictRefDraft>,
    span: Span,
    impl_id: String,
    ty: &Type,
) -> typed::DictRefId {
    let id = dict_refs.len();
    dict_refs.push(DictRefDraft {
        impl_id,
        span,
        requirements: Vec::new(),
        ty: ty.clone(),
    });
    id
}

fn generalize_function_type(env: &TypeEnv, ty: Type) -> Scheme {
    let env_vars = env.free_type_variables();
    let mut quantifiers = ty
        .free_type_variables()
        .into_iter()
        .filter(|variable| !env_vars.contains(variable))
        .collect::<Vec<_>>();
    quantifiers.sort_unstable_by_key(|variable| variable.id());
    let mut scheme = Scheme::generalize(ty);
    scheme.quantifiers = quantifiers;
    scheme
}

fn build_scheme_info(id: usize, scheme: &Scheme, substitution: &Substitution) -> typed::SchemeInfo {
    let quantifiers = scheme
        .quantifiers
        .iter()
        .map(|variable| variable.to_string())
        .collect::<Vec<_>>();
    let constraints = scheme
        .constraints
        .iter()
        .map(|(name, ty)| format!("{}: {}", name, substitution.apply(ty).label()))
        .collect::<Vec<_>>();
    typed::SchemeInfo {
        id,
        quantifiers,
        constraints,
        ty: substitution.apply(&scheme.ty).label(),
    }
}

fn check_bool_condition(
    span: Span,
    ty: &Type,
    violations: &mut Vec<TypecheckViolation>,
    function_name: Option<&str>,
) {
    if matches!(ty, Type::Builtin(BuiltinType::Bool)) {
        return;
    }
    violations.push(TypecheckViolation::condition_literal_bool(
        span,
        ty.clone(),
        function_name.map(|name| name.to_string()),
    ));
}

fn detect_capability_violations(
    module: &Module,
    config: &TypecheckConfig,
) -> Vec<TypecheckViolation> {
    let mut usages = Vec::new();
    for function in &module.functions {
        collect_perform_effects(&function.body, &mut usages);
    }
    if usages.is_empty() {
        return Vec::new();
    }
    let provided_capabilities = config
        .runtime_capabilities
        .iter()
        .map(|cap| cap.id().clone())
        .collect::<HashSet<_>>();
    let runtime_stage = config.effect_context.runtime.clone();
    let capability_requirement = config.effect_context.capability.clone();
    let mut violations = Vec::new();
    for usage in usages {
        let descriptor = CapabilityDescriptor::resolve(&usage.effect_name);
        let descriptor_requirement = StageRequirement::AtLeast(descriptor.stage().clone());
        let required_stage =
            StageRequirement::merged_with(&descriptor_requirement, &capability_requirement);
        if !runtime_stage.satisfies(&required_stage) {
            violations.push(TypecheckViolation::stage_mismatch(
                Some(usage.span),
                descriptor.id().to_string(),
                required_stage.clone(),
                runtime_stage.clone(),
            ));
            continue;
        }
        if !provided_capabilities.contains(descriptor.id()) {
            violations.push(TypecheckViolation::residual_leak(
                Some(usage.span),
                Some(descriptor.id().to_string()),
            ));
        }
    }
    violations
}

fn detect_iterator_stage_mismatches(
    dict_refs: &[DictRefDraft],
    substitution: &Substitution,
    config: &TypecheckConfig,
) -> Vec<TypecheckViolation> {
    let mut violations = Vec::new();
    let runtime_stage = config.effect_context.runtime.clone();
    for draft in dict_refs {
        let ty = substitution.apply(&draft.ty);
        if let Some(info) = iterator::solve_iterator(&ty) {
            let snapshot = info.stage_snapshot();
            if !runtime_stage.satisfies(&snapshot.required) {
                violations.push(TypecheckViolation::iterator_stage_mismatch(
                    Some(draft.span),
                    snapshot,
                    runtime_stage.clone(),
                ));
            }
        }
    }
    violations
}

fn collect_perform_effects(expr: &Expr, usages: &mut Vec<EffectUsage>) {
    match &expr.kind {
        ExprKind::PerformCall { call } => {
            usages.push(EffectUsage::new(call.effect.name.clone(), expr.span()));
            collect_perform_effects(&call.argument, usages);
        }
        ExprKind::IfElse {
            condition,
            then_branch,
            else_branch,
        } => {
            collect_perform_effects(condition, usages);
            collect_perform_effects(then_branch, usages);
            if let Some(else_branch_expr) = else_branch {
                collect_perform_effects(&else_branch_expr, usages);
            } else {
                collect_perform_effects(then_branch, usages);
            }
        }
        ExprKind::Binary { left, right, .. } => {
            collect_perform_effects(left, usages);
            collect_perform_effects(right, usages);
        }
        ExprKind::Call { callee, args, .. } => {
            collect_perform_effects(callee, usages);
            for arg in args {
                collect_perform_effects(arg, usages);
            }
        }
        ExprKind::Literal(_) | ExprKind::Identifier(_) => {}
        _ => {}
    }
}

fn compress_typecheck_violations(violations: Vec<TypecheckViolation>) -> Vec<TypecheckViolation> {
    if violations.is_empty() {
        return violations;
    }
    let mut residual = ResidualLeakAccumulator::default();
    let mut others = Vec::new();
    for violation in violations.into_iter() {
        if matches!(violation.kind, TypecheckViolationKind::ResidualLeak) {
            residual.ingest(&violation);
        } else {
            others.push(violation);
        }
    }
    if let Some(merged) = residual.finish() {
        others.push(merged);
    }
    others
}

#[derive(Default)]
struct ResidualLeakAccumulator {
    span: Option<Span>,
    tokens: ExpectedTokenCollector,
    notes: Vec<ViolationNote>,
    seen_capabilities: HashSet<String>,
    has_generic: bool,
}

impl ResidualLeakAccumulator {
    fn ingest(&mut self, violation: &TypecheckViolation) {
        if self.span.is_none() {
            self.span = violation.span;
        }
        if let Some(capability) = violation.capability.clone() {
            if self.seen_capabilities.insert(capability.clone()) {
                self.tokens.push_custom(capability);
                self.notes.extend(violation.notes.clone());
            }
        } else if !self.has_generic {
            self.has_generic = true;
            self.tokens.push_custom("residual.effect");
            self.notes.extend(violation.notes.clone());
        }
    }

    fn finish(self) -> Option<TypecheckViolation> {
        if self.span.is_none() && !self.has_generic && self.seen_capabilities.is_empty() {
            return None;
        }
        let mut violation = TypecheckViolation::residual_leak(self.span, None);
        if !self.notes.is_empty() {
            violation.notes = self.notes;
        }
        let summary = if self.tokens.is_empty() {
            let mut collector = ExpectedTokenCollector::new();
            collector.push_custom("residual.effect");
            collector.summarize_with_context(Some(
                "不足している Capability を Runtime Registry へ登録してください".to_string(),
            ))
        } else {
            self.tokens.summarize_with_context(Some(
                "不足している Capability を Runtime Registry へ登録してください".to_string(),
            ))
        };
        Some(violation.with_expected_summary(summary))
    }
}
