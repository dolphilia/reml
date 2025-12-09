use std::collections::{HashMap, HashSet};

use once_cell::sync::Lazy;
use serde::Serialize;

use super::capability::{CapabilityDescriptor, EffectUsage};
use super::constraint::{
    iterator, Constraint, ConstraintSolver, ConstraintSolverError, Substitution,
};
use super::env::{StageRequirement, TypeEnv, TypecheckConfig};
use super::metrics::TypecheckMetrics;
use super::scheme::Scheme;
use super::types::{BuiltinType, Type, TypeVarGen};
use crate::diagnostic::{ExpectedToken, ExpectedTokenCollector, ExpectedTokensSummary};
use crate::effects::diagnostics::CapabilityMismatch;
use crate::parser::ast::{
    Decl, DeclKind, Expr, ExprKind, Function, Ident, Literal, LiteralKind, Module, ModulePath,
    Pattern, PatternKind, RelativeHead, Stmt, StmtKind,
};
use crate::semantics::typed;
use crate::span::Span;

/// 型推論の簡易ドライバ。現時点では AST を走査して
/// メトリクスとサマリ情報のみを生成する。
pub struct TypecheckDriver;

#[derive(Clone, Copy)]
struct FunctionContext<'a> {
    name: Option<&'a str>,
    is_pure: bool,
}

impl<'a> FunctionContext<'a> {
    fn new(name: &'a str, is_pure: bool) -> Self {
        Self {
            name: Some(name),
            is_pure,
        }
    }
}

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

        if !module.decls.is_empty() {
            let mut module_decl_stats = FunctionStats::default();
            let mut module_decl_constraints = Vec::new();
            let module_context = FunctionContext {
                name: None,
                is_pure: false,
            };
            for decl in &module.decls {
                infer_decl(
                    decl,
                    &mut module_env,
                    &mut var_gen,
                    &mut solver,
                    &mut module_decl_constraints,
                    &mut module_decl_stats,
                    &mut metrics,
                    &mut violations,
                    &mut dict_ref_drafts,
                    module_context,
                );
            }
            all_constraints.extend(module_decl_constraints.drain(..));
        }

        for function in &module.functions {
            metrics.record_function();
            let mut stats = FunctionStats::default();
            let mut constraints = Vec::new();
            let mut env = module_env.clone();
            let mut param_bindings = Vec::new();
            let is_pure = function.attrs.iter().any(|attr| attr.name.name == "pure");
            let function_context = FunctionContext::new(function.name.name.as_str(), is_pure);

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
                &mut env,
                &mut var_gen,
                &mut solver,
                &mut constraints,
                &mut stats,
                &mut metrics,
                &mut violations,
                &mut dict_ref_drafts,
                function_context,
            );

            all_constraints.extend(constraints.drain(..));

            let substitution = solver.substitution().clone();
            let resolved_return = substitution.apply(&typed_body.ty);
            let param_types = param_bindings
                .iter()
                .map(|binding| substitution.apply(&binding.ty))
                .collect::<Vec<_>>();
            let function_type = Type::arrow(param_types.clone(), resolved_return.clone());
            let scheme = generalize_type(&module_env, function_type);
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
        violations.extend(detect_duplicate_impls(module));
        violations.extend(detect_spec_core_runtime_violations(module));
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
    #[serde(skip_serializing)]
    pub capability_mismatch: Option<CapabilityMismatch>,
}

#[derive(Debug, Serialize, Clone)]
pub enum TypecheckViolationKind {
    ConditionLiteralBool,
    AstUnavailable,
    ReturnConflict,
    ResidualLeak,
    StageMismatch,
    IteratorStageMismatch,
    ValueRestriction,
    PurityViolation,
    ImplDuplicate,
    CoreParseRecoverBranch,
    RuntimeBridgeStageMismatch,
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
            capability_mismatch: None,
        }
    }

    fn return_conflict(span: Span, function: Option<String>, then_ty: Type, else_ty: Type) -> Self {
        Self {
            kind: TypecheckViolationKind::ReturnConflict,
            code: "language.inference.return_conflict",
            message: "if 式の then/else の戻り値型が一致しません".to_string(),
            span: Some(span),
            notes: vec![ViolationNote::plain(format!(
                "then: {} / else: {}",
                then_ty.label(),
                else_ty.label()
            ))],
            capability: None,
            function,
            expected: None,
            iterator_stage: None,
            capability_mismatch: None,
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
            capability_mismatch: None,
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
        let capability_label = capability.clone();
        Self {
            kind: TypecheckViolationKind::StageMismatch,
            code: "effects.contract.stage_mismatch",
            message,
            span,
            notes: vec![ViolationNote::plain(note_message)],
            capability: Some(capability_label.clone()),
            function: None,
            expected: None,
            iterator_stage: None,
            capability_mismatch: Some(CapabilityMismatch::new(
                capability_label,
                required.clone(),
                actual.clone(),
            )),
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
            capability_mismatch: None,
        }
        .with_expected_summary(top_level_declaration_summary())
    }

    fn value_restriction(span: Span, binding: String) -> Self {
        Self {
            kind: TypecheckViolationKind::ValueRestriction,
            code: "language.inference.value_restriction",
            message: format!("`var {binding}` は汎化できない式を共有しようとしました。"),
            span: Some(span),
            notes: vec![ViolationNote::plain(
                "可変セルを共有する場合は `fn` で包むか、明示的な型を指定してください。",
            )],
            capability: None,
            function: None,
            expected: None,
            iterator_stage: None,
            capability_mismatch: None,
        }
    }

    fn purity_violation(span: Span, function: Option<String>, effect: String) -> Self {
        let message = match function.as_ref() {
            Some(name) => {
                format!(
                    "`@pure` 関数 `{}` で `perform {}` が検出されました。",
                    name, effect
                )
            }
            None => format!("`@pure` ブロックで `perform {}` が検出されました。", effect),
        };
        Self {
            kind: TypecheckViolationKind::PurityViolation,
            code: "effects.purity.violated",
            message,
            span: Some(span),
            notes: vec![ViolationNote::plain(format!(
                "`@pure` を外すか `{}` をハンドラで捕捉してください。",
                effect
            ))],
            capability: None,
            function,
            expected: None,
            iterator_stage: None,
            capability_mismatch: None,
        }
    }

    fn impl_duplicate(span: Span, trait_name: String, target: String, previous_span: Span) -> Self {
        let message = format!(
            "`{}` への `{}` impl が重複定義されています。",
            target, trait_name
        );
        Self {
            kind: TypecheckViolationKind::ImplDuplicate,
            code: "typeclass.impl.duplicate",
            message,
            span: Some(span),
            notes: vec![ViolationNote::plain(format!(
                "最初の impl は {previous_span} にあります"
            ))],
            capability: None,
            function: None,
            expected: None,
            iterator_stage: None,
            capability_mismatch: None,
        }
    }

    fn core_parse_recover_branch(span: Span) -> Self {
        Self {
            kind: TypecheckViolationKind::CoreParseRecoverBranch,
            code: "core.parse.recover.branch",
            message: "`Parse.recover` が `let` 文の識別子位置で分岐回復を実施しました。"
                .to_string(),
            span: Some(span),
            notes: vec![ViolationNote::plain(
                "`let name = value` の `name` を補完するか、`run_with_recovery` のログを確認してください。",
            )],
            capability: None,
            function: None,
            expected: None,
            iterator_stage: None,
            capability_mismatch: None,
        }
    }

    fn runtime_bridge_stage_mismatch(
        span: Span,
        bridge: Option<String>,
        required: String,
        provided: String,
    ) -> Self {
        let bridge_label = bridge.unwrap_or_else(|| "bridge".to_string());
        let message = format!(
            "Bridge `{}` は Stage::{} で、要求 Stage::{} を満たしていません。",
            bridge_label, provided, required
        );
        Self {
            kind: TypecheckViolationKind::RuntimeBridgeStageMismatch,
            code: "runtime.bridge.stage_mismatch",
            message,
            span: Some(span),
            notes: vec![ViolationNote::plain(
                "`reml.toml` の stage_bounds を更新するか、要求レベルを下げてください。",
            )],
            capability: None,
            function: None,
            expected: None,
            iterator_stage: None,
            capability_mismatch: None,
        }
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
            capability_mismatch: None,
        }
    }

    pub fn domain(&self) -> &'static str {
        match self.kind {
            TypecheckViolationKind::ConditionLiteralBool
            | TypecheckViolationKind::AstUnavailable
            | TypecheckViolationKind::ReturnConflict
            | TypecheckViolationKind::ValueRestriction
            | TypecheckViolationKind::ImplDuplicate => "type",
            TypecheckViolationKind::ResidualLeak
            | TypecheckViolationKind::StageMismatch
            | TypecheckViolationKind::IteratorStageMismatch
            | TypecheckViolationKind::PurityViolation => "effects",
            TypecheckViolationKind::CoreParseRecoverBranch => "parser",
            TypecheckViolationKind::RuntimeBridgeStageMismatch => "runtime",
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
    env: &mut TypeEnv,
    var_gen: &mut TypeVarGen,
    solver: &mut ConstraintSolver,
    constraints: &mut Vec<Constraint>,
    stats: &mut FunctionStats,
    metrics: &mut TypecheckMetrics,
    violations: &mut Vec<TypecheckViolation>,
    dict_refs: &mut Vec<DictRefDraft>,
    context: FunctionContext<'_>,
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
        context,
    )
}

fn infer_expr(
    expr: &Expr,
    env: &mut TypeEnv,
    var_gen: &mut TypeVarGen,
    solver: &mut ConstraintSolver,
    constraints: &mut Vec<Constraint>,
    stats: &mut FunctionStats,
    metrics: &mut TypecheckMetrics,
    violations: &mut Vec<TypecheckViolation>,
    dict_refs: &mut Vec<DictRefDraft>,
    context: FunctionContext<'_>,
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
                context,
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
                context,
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
                context,
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
                        context,
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
                context,
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
            if context.is_pure {
                violations.push(TypecheckViolation::purity_violation(
                    expr.span(),
                    context.name.map(|name| name.to_string()),
                    call.effect.name.clone(),
                ));
            }
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
                context,
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
                context,
            );
            let has_explicit_else = else_branch.is_some();
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
                context,
            );
            stats.constraints += 1;
            metrics.record_constraint("conditional");
            constraints.push(Constraint::equal(
                then_result.ty.clone(),
                else_result.ty.clone(),
            ));
            metrics.record_unify_call();
            let unify_error = solver
                .unify(then_result.ty.clone(), else_result.ty.clone())
                .err();
            if let (true, Some(error)) = (has_explicit_else, unify_error) {
                if let Some((left, right)) = match error {
                    ConstraintSolverError::Mismatch(left, right) => Some((left, right)),
                    ConstraintSolverError::Occurs(variable, ty) => Some((Type::Var(variable), ty)),
                    ConstraintSolverError::NotImplemented(_) => None,
                } {
                    violations.push(TypecheckViolation::return_conflict(
                        expr.span(),
                        context.name.map(|name| name.to_string()),
                        left,
                        right,
                    ));
                }
            }
            check_bool_condition(condition.span(), &condition_result.ty, violations, context);
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
        ExprKind::Block { statements, .. } => {
            let (block_ty, block_dict_refs) = infer_block(
                statements,
                env,
                var_gen,
                solver,
                constraints,
                stats,
                metrics,
                violations,
                dict_refs,
                context,
            );
            make_typed(expr, TypedExprKindDraft::Unknown, block_ty, block_dict_refs)
        }
        ExprKind::Lambda { params, body, .. } => {
            let mut lambda_env = env.enter_scope();
            let mut param_types = Vec::new();
            for param in params {
                let ty = var_gen.fresh_type();
                lambda_env.insert(param.name.name.clone(), Scheme::simple(ty.clone()));
                param_types.push(ty);
            }
            let body_result = infer_expr(
                body,
                &mut lambda_env,
                var_gen,
                solver,
                constraints,
                stats,
                metrics,
                violations,
                dict_refs,
                context,
            );
            let lambda_ty = Type::arrow(param_types, body_result.ty.clone());
            make_typed(
                expr,
                TypedExprKindDraft::Unknown,
                lambda_ty,
                body_result.dict_ref_ids,
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

fn infer_block(
    statements: &[Stmt],
    parent_env: &TypeEnv,
    var_gen: &mut TypeVarGen,
    solver: &mut ConstraintSolver,
    constraints: &mut Vec<Constraint>,
    stats: &mut FunctionStats,
    metrics: &mut TypecheckMetrics,
    violations: &mut Vec<TypecheckViolation>,
    dict_refs: &mut Vec<DictRefDraft>,
    context: FunctionContext<'_>,
) -> (Type, Vec<typed::DictRefId>) {
    let mut block_env = parent_env.enter_scope();
    let mut last_ty = Type::builtin(BuiltinType::Unknown);
    let mut block_dict_refs = Vec::new();
    for stmt in statements {
        match &stmt.kind {
            StmtKind::Decl { decl } => {
                let stmt_refs = infer_decl(
                    decl,
                    &mut block_env,
                    var_gen,
                    solver,
                    constraints,
                    stats,
                    metrics,
                    violations,
                    dict_refs,
                    context,
                );
                block_dict_refs.extend(stmt_refs);
            }
            StmtKind::Expr { expr } => {
                let expr_result = infer_expr(
                    expr,
                    &mut block_env,
                    var_gen,
                    solver,
                    constraints,
                    stats,
                    metrics,
                    violations,
                    dict_refs,
                    context,
                );
                last_ty = expr_result.ty.clone();
                block_dict_refs.extend(expr_result.dict_ref_ids);
            }
            StmtKind::Assign { target, value } => {
                let target_result = infer_expr(
                    target,
                    &mut block_env,
                    var_gen,
                    solver,
                    constraints,
                    stats,
                    metrics,
                    violations,
                    dict_refs,
                    context,
                );
                block_dict_refs.extend(target_result.dict_ref_ids);
                let value_result = infer_expr(
                    value,
                    &mut block_env,
                    var_gen,
                    solver,
                    constraints,
                    stats,
                    metrics,
                    violations,
                    dict_refs,
                    context,
                );
                block_dict_refs.extend(value_result.dict_ref_ids);
            }
            StmtKind::Defer { expr } => {
                let defer_result = infer_expr(
                    expr,
                    &mut block_env,
                    var_gen,
                    solver,
                    constraints,
                    stats,
                    metrics,
                    violations,
                    dict_refs,
                    context,
                );
                block_dict_refs.extend(defer_result.dict_ref_ids);
            }
        }
    }
    (last_ty, block_dict_refs)
}

fn infer_decl(
    decl: &Decl,
    env: &mut TypeEnv,
    var_gen: &mut TypeVarGen,
    solver: &mut ConstraintSolver,
    constraints: &mut Vec<Constraint>,
    stats: &mut FunctionStats,
    metrics: &mut TypecheckMetrics,
    violations: &mut Vec<TypecheckViolation>,
    dict_refs: &mut Vec<DictRefDraft>,
    context: FunctionContext<'_>,
) -> Vec<typed::DictRefId> {
    match &decl.kind {
        DeclKind::Let {
            pattern,
            value,
            type_annotation: _,
        } => infer_binding(
            pattern,
            value,
            env,
            var_gen,
            solver,
            constraints,
            stats,
            metrics,
            violations,
            dict_refs,
            context,
        ),
        DeclKind::Var {
            pattern,
            value,
            type_annotation,
        } => {
            let dicts = infer_binding(
                pattern,
                value,
                env,
                var_gen,
                solver,
                constraints,
                stats,
                metrics,
                violations,
                dict_refs,
                context,
            );
            if type_annotation.is_none() {
                if let Some(name) = pattern_binding_name(pattern) {
                    violations.push(TypecheckViolation::value_restriction(decl.span, name));
                }
            }
            dicts
        }
        _ => Vec::new(),
    }
}

fn infer_binding(
    pattern: &Pattern,
    value: &Expr,
    env: &mut TypeEnv,
    var_gen: &mut TypeVarGen,
    solver: &mut ConstraintSolver,
    constraints: &mut Vec<Constraint>,
    stats: &mut FunctionStats,
    metrics: &mut TypecheckMetrics,
    violations: &mut Vec<TypecheckViolation>,
    dict_refs: &mut Vec<DictRefDraft>,
    context: FunctionContext<'_>,
) -> Vec<typed::DictRefId> {
    let value_result = infer_expr(
        value,
        env,
        var_gen,
        solver,
        constraints,
        stats,
        metrics,
        violations,
        dict_refs,
        context,
    );
    let substitution = solver.substitution().clone();
    let resolved_ty = substitution.apply(&value_result.ty);
    let scheme = generalize_type(env, resolved_ty.clone());
    bind_pattern_to_env(pattern, &scheme, env);
    value_result.dict_ref_ids
}

fn bind_pattern_to_env(pattern: &Pattern, scheme: &Scheme, env: &mut TypeEnv) {
    match &pattern.kind {
        PatternKind::Var(ident) => {
            env.insert(ident.name.clone(), scheme.clone());
        }
        _ => {}
    }
}

fn pattern_binding_name(pattern: &Pattern) -> Option<String> {
    match &pattern.kind {
        PatternKind::Var(ident) => Some(ident.name.clone()),
        _ => None,
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

fn generalize_type(env: &TypeEnv, ty: Type) -> Scheme {
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
    context: FunctionContext<'_>,
) {
    if matches!(ty, Type::Builtin(BuiltinType::Bool)) {
        return;
    }
    violations.push(TypecheckViolation::condition_literal_bool(
        span,
        ty.clone(),
        context.name.map(|name| name.to_string()),
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
        if descriptor.is_user_defined() {
            continue;
        }
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

fn detect_duplicate_impls(module: &Module) -> Vec<TypecheckViolation> {
    let mut seen: HashMap<String, Span> = HashMap::new();
    let mut violations = Vec::new();
    for decl in &module.decls {
        if let DeclKind::Impl(impl_decl) = &decl.kind {
            if let Some(trait_ref) = &impl_decl.trait_ref {
                let trait_name = trait_ref.render();
                let target = impl_decl.target.render();
                let key = format!("{}::{}", trait_name, target);
                if let Some(previous_span) = seen.get(&key) {
                    violations.push(TypecheckViolation::impl_duplicate(
                        decl.span,
                        trait_name,
                        target,
                        *previous_span,
                    ));
                } else {
                    seen.insert(key, decl.span);
                }
            }
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

fn detect_spec_core_runtime_violations(module: &Module) -> Vec<TypecheckViolation> {
    let mut violations = Vec::new();
    if let Some(span) = find_parse_run_with_recovery_call(module) {
        violations.push(TypecheckViolation::core_parse_recover_branch(span));
    }
    if let Some(call) = find_runtime_bridge_call(module) {
        if let (Some(required), Some(provided)) = (call.required_stage, call.provided_stage) {
            if required != provided {
                violations.push(TypecheckViolation::runtime_bridge_stage_mismatch(
                    call.span,
                    call.bridge_name,
                    required,
                    provided,
                ));
            }
        }
    }
    violations
}

fn find_parse_run_with_recovery_call(module: &Module) -> Option<Span> {
    let mut span = None;
    visit_module_exprs(module, &mut |expr| {
        if span.is_some() {
            return;
        }
        if let ExprKind::Call { callee, .. } = &expr.kind {
            if matches_module_member(callee, "Parse", "run_with_recovery") {
                span = Some(expr.span);
            }
        }
    });
    span
}

struct RuntimeBridgeCallInfo {
    span: Span,
    bridge_name: Option<String>,
    required_stage: Option<String>,
    provided_stage: Option<String>,
}

fn find_runtime_bridge_call(module: &Module) -> Option<RuntimeBridgeCallInfo> {
    let mut result = None;
    visit_module_exprs(module, &mut |expr| {
        if result.is_some() {
            return;
        }
        if let ExprKind::Call { callee, args } = &expr.kind {
            if matches_module_member(callee, "RuntimeBridge", "verify_stage") {
                let mut call = RuntimeBridgeCallInfo {
                    span: expr.span,
                    bridge_name: None,
                    required_stage: None,
                    provided_stage: None,
                };
                for arg in args {
                    if let ExprKind::Assign { target, value } = &arg.kind {
                        if let Some(name) = expr_ident_name(target) {
                            match name {
                                "bridge" => {
                                    call.bridge_name = extract_string_literal(value);
                                }
                                "required" => {
                                    call.required_stage = extract_stage_identifier(value);
                                }
                                "provided" => {
                                    call.provided_stage = extract_stage_identifier(value);
                                }
                                _ => {}
                            }
                        }
                    }
                }
                result = Some(call);
            }
        }
    });
    result
}

fn extract_string_literal(expr: &Expr) -> Option<String> {
    match &expr.kind {
        ExprKind::Literal(Literal {
            value: LiteralKind::String { value, .. },
        }) => Some(value.clone()),
        _ => None,
    }
}

fn extract_stage_identifier(expr: &Expr) -> Option<String> {
    match &expr.kind {
        ExprKind::FieldAccess { target, field } => {
            if matches_identifier(target, "Stage") {
                Some(field.name.clone())
            } else {
                None
            }
        }
        ExprKind::ModulePath(path) => {
            if module_path_head_is(path, "Stage") {
                module_path_last_segment(path)
                    .map(|segment| segment.name.clone())
                    .or_else(|| {
                        // path が head のみの場合は head を返す
                        module_path_head_name(path).map(|name| name.to_string())
                    })
            } else {
                None
            }
        }
        _ => None,
    }
}

fn module_path_head_is(path: &ModulePath, expected: &str) -> bool {
    match path {
        ModulePath::Root { segments } => segments
            .first()
            .map(|segment| segment.name.as_str())
            .map(|name| name == expected)
            .unwrap_or(false),
        ModulePath::Relative { head, .. } => match head {
            RelativeHead::PlainIdent(ident) => ident.name == expected,
            _ => false,
        },
    }
}

fn module_path_head_name(path: &ModulePath) -> Option<&str> {
    match path {
        ModulePath::Root { segments } => segments.first().map(|segment| segment.name.as_str()),
        ModulePath::Relative { head, .. } => match head {
            RelativeHead::PlainIdent(ident) => Some(ident.name.as_str()),
            _ => None,
        },
    }
}

fn module_path_last_segment(path: &ModulePath) -> Option<&Ident> {
    match path {
        ModulePath::Root { segments } => segments.last(),
        ModulePath::Relative { segments, .. } => segments.last(),
    }
}

fn visit_module_exprs(module: &Module, visitor: &mut impl FnMut(&Expr)) {
    for function in &module.functions {
        visit_expr(&function.body, visitor);
    }
    for decl in &module.decls {
        visit_decl(decl, visitor);
    }
}

fn visit_decl(decl: &Decl, visitor: &mut impl FnMut(&Expr)) {
    match &decl.kind {
        DeclKind::Let { value, .. } | DeclKind::Var { value, .. } => visit_expr(value, visitor),
        _ => {}
    }
}

fn visit_stmt(stmt: &Stmt, visitor: &mut impl FnMut(&Expr)) {
    match &stmt.kind {
        StmtKind::Decl { decl } => visit_decl(decl, visitor),
        StmtKind::Expr { expr } | StmtKind::Defer { expr } => visit_expr(expr, visitor),
        StmtKind::Assign { target, value } => {
            visit_expr(target, visitor);
            visit_expr(value, visitor);
        }
    }
}

fn visit_expr(expr: &Expr, visitor: &mut impl FnMut(&Expr)) {
    visitor(expr);
    match &expr.kind {
        ExprKind::Literal(literal) => visit_literal(literal, visitor),
        ExprKind::Identifier(_) | ExprKind::ModulePath(_) | ExprKind::Continue => {}
        ExprKind::Call { callee, args } => {
            visit_expr(callee, visitor);
            for arg in args {
                visit_expr(arg, visitor);
            }
        }
        ExprKind::PerformCall { call } => {
            visit_expr(&call.argument, visitor);
        }
        ExprKind::Lambda { body, .. }
        | ExprKind::Loop { body }
        | ExprKind::Unsafe { body }
        | ExprKind::Defer { body } => visit_expr(body, visitor),
        ExprKind::Pipe { left, right } | ExprKind::Binary { left, right, .. } => {
            visit_expr(left, visitor);
            visit_expr(right, visitor);
        }
        ExprKind::Unary { expr: inner, .. }
        | ExprKind::Propagate { expr: inner }
        | ExprKind::Return { value: Some(inner) } => {
            visit_expr(inner, visitor);
        }
        ExprKind::Return { value: None } => {}
        ExprKind::FieldAccess { target, .. } | ExprKind::TupleAccess { target, .. } => {
            visit_expr(target, visitor);
        }
        ExprKind::Handle { handle } => {
            visit_expr(&handle.target, visitor);
        }
        ExprKind::Index { target, index } => {
            visit_expr(target, visitor);
            visit_expr(index, visitor);
        }
        ExprKind::IfElse {
            condition,
            then_branch,
            else_branch,
        } => {
            visit_expr(condition, visitor);
            visit_expr(then_branch, visitor);
            if let Some(else_branch_expr) = else_branch {
                visit_expr(else_branch_expr, visitor);
            }
        }
        ExprKind::Match { target, arms } => {
            visit_expr(target, visitor);
            for arm in arms {
                if let Some(guard) = &arm.guard {
                    visit_expr(guard, visitor);
                }
                visit_expr(&arm.body, visitor);
            }
        }
        ExprKind::While { condition, body } => {
            visit_expr(condition, visitor);
            visit_expr(body, visitor);
        }
        ExprKind::For { start, end, .. } => {
            visit_expr(start, visitor);
            visit_expr(end, visitor);
        }
        ExprKind::Block { statements, .. } => {
            for stmt in statements {
                visit_stmt(stmt, visitor);
            }
        }
        ExprKind::Assign { target, value } => {
            visit_expr(target, visitor);
            visit_expr(value, visitor);
        }
    }
}

fn visit_literal(literal: &Literal, visitor: &mut impl FnMut(&Expr)) {
    match &literal.value {
        LiteralKind::Tuple { elements } | LiteralKind::Array { elements } => {
            for element in elements {
                visit_expr(element, visitor);
            }
        }
        _ => {}
    }
}

fn matches_module_member(expr: &Expr, module_name: &str, member_name: &str) -> bool {
    match &expr.kind {
        ExprKind::ModulePath(path) => module_path_matches_member(path, module_name, member_name),
        ExprKind::FieldAccess { target, field } => {
            if field.name != member_name {
                return false;
            }
            if let Some(name) = expr_ident_name(target) {
                name == module_name
            } else {
                false
            }
        }
        _ => false,
    }
}

fn module_path_matches_member(path: &ModulePath, module_name: &str, member_name: &str) -> bool {
    match path {
        ModulePath::Root { segments } => {
            if segments.is_empty() {
                return false;
            }
            let first = &segments[0].name;
            let last = &segments[segments.len() - 1].name;
            first == module_name && last == member_name
        }
        ModulePath::Relative { head, segments } => {
            let head_name = match head {
                RelativeHead::PlainIdent(ident) => &ident.name,
                _ => return false,
            };
            if segments.is_empty() {
                return false;
            }
            head_name == module_name
                && segments.last().map(|segment| segment.name.as_str()) == Some(member_name)
        }
    }
}

fn expr_ident_name(expr: &Expr) -> Option<&str> {
    match &expr.kind {
        ExprKind::Identifier(ident) => Some(ident.name.as_str()),
        _ => None,
    }
}

fn matches_identifier(expr: &Expr, expected: &str) -> bool {
    expr_ident_name(expr)
        .map(|name| name == expected)
        .unwrap_or(false)
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
        ExprKind::Block { statements, .. } => {
            for stmt in statements {
                collect_perform_effects_in_stmt(stmt, usages);
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
        ExprKind::Lambda { body, .. } => {
            collect_perform_effects(body, usages);
        }
        ExprKind::Literal(_) | ExprKind::Identifier(_) => {}
        _ => {}
    }
}

fn collect_perform_effects_in_stmt(stmt: &Stmt, usages: &mut Vec<EffectUsage>) {
    match &stmt.kind {
        StmtKind::Decl { decl } => collect_perform_effects_in_decl(decl, usages),
        StmtKind::Expr { expr } => collect_perform_effects(expr, usages),
        StmtKind::Assign { target, value } => {
            collect_perform_effects(target, usages);
            collect_perform_effects(value, usages);
        }
        StmtKind::Defer { expr } => collect_perform_effects(expr, usages),
    }
}

fn collect_perform_effects_in_decl(decl: &Decl, usages: &mut Vec<EffectUsage>) {
    match &decl.kind {
        DeclKind::Let { value, .. } | DeclKind::Var { value, .. } => {
            collect_perform_effects(value, usages);
        }
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
