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
    BinaryOp, Decl, DeclKind, Expr, ExprKind, FixityKind, Function, HandlerEntry, Ident, Literal,
    LiteralKind, MatchArm, Module, ModulePath, Pattern, PatternKind, RelativeHead,
    SlicePatternItem, Stmt, StmtKind, TypeKind,
};
use crate::semantics::{mir, typed};
use crate::span::Span;

/// 型推論の簡易ドライバ。現時点では AST を走査して
/// メトリクスとサマリ情報のみを生成する。
pub struct TypecheckDriver;

#[derive(Default)]
struct UnicodeShadowTracker {
    seen: HashMap<String, Span>,
}

impl UnicodeShadowTracker {
    fn observe_pattern(
        &mut self,
        pattern: &Pattern,
        span: Span,
        violations: &mut Vec<TypecheckViolation>,
    ) {
        if let Some(name) = pattern_binding_name(pattern) {
            if name.is_ascii() {
                return;
            }
            if self.seen.contains_key(&name) {
                violations.push(TypecheckViolation::unicode_shadowing(span, &name));
            } else {
                self.seen.insert(name, span);
            }
        }
    }
}

#[derive(Clone, Copy)]
enum ContextKind {
    Function,
    ActivePattern,
    Module,
}

#[derive(Clone, Copy)]
struct FunctionContext<'a> {
    name: Option<&'a str>,
    is_pure: bool,
    kind: ContextKind,
}

impl<'a> FunctionContext<'a> {
    fn function(name: &'a str, is_pure: bool) -> Self {
        Self {
            name: Some(name),
            is_pure,
            kind: ContextKind::Function,
        }
    }

    fn active_pattern(name: &'a str, is_pure: bool) -> Self {
        Self {
            name: Some(name),
            is_pure,
            kind: ContextKind::ActivePattern,
        }
    }

    fn module() -> Self {
        Self {
            name: None,
            is_pure: false,
            kind: ContextKind::Module,
        }
    }

    fn purity_violation(&self, span: Span, effect: String) -> TypecheckViolation {
        match self.kind {
            ContextKind::ActivePattern => TypecheckViolation::active_pattern_effect_violation(
                span,
                self.name.map(|name| name.to_string()),
                effect,
            ),
            _ => TypecheckViolation::purity_violation(
                span,
                self.name.map(|name| name.to_string()),
                effect,
            ),
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
        let mut unicode_shadow_tracker = UnicodeShadowTracker::default();

        collect_opbuilder_violations(module, &mut violations);
        violations.extend(detect_active_pattern_conflicts(module));

        if !module.decls.is_empty() {
            let mut module_decl_stats = FunctionStats::default();
            let mut module_decl_constraints = Vec::new();
            let module_context = FunctionContext::module();
            let mut module_loop_context = LoopContextStack::default();
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
                    Some(&mut unicode_shadow_tracker),
                    module_context,
                    &mut module_loop_context,
                );
            }
            all_constraints.extend(module_decl_constraints.drain(..));
        }

        for active in &module.active_patterns {
            let mut stats = FunctionStats::default();
            let mut constraints = Vec::new();
            let mut env = module_env.clone();
            let mut param_bindings = Vec::new();
            let is_pure = active.attrs.iter().any(|attr| attr.name.name == "pure");
            let context = FunctionContext::active_pattern(active.name.name.as_str(), is_pure);
            for param in &active.params {
                let ty = param
                    .type_annotation
                    .as_ref()
                    .and_then(|annot| type_from_annotation_kind(&annot.kind))
                    .unwrap_or_else(|| var_gen.fresh_type());
                let scheme = Scheme::simple(ty.clone());
                bind_pattern_to_env(&param.pattern, &scheme, &mut env, &mut var_gen);
                param_bindings.push(ParamBinding {
                    display: param.pattern.render(),
                    span: param.span,
                    ty,
                });
            }
            let mut loop_context = LoopContextStack::default();
            let typed_body_draft = infer_expr(
                &active.body,
                &mut env,
                &mut var_gen,
                &mut solver,
                &mut constraints,
                &mut stats,
                &mut metrics,
                &mut violations,
                &mut dict_ref_drafts,
                &mut loop_context,
                context,
            );
            all_constraints.extend(constraints.drain(..));
            let return_kind = classify_active_pattern_return(&active.body);
            let substitution = solver.substitution().clone();
            let typed_params = param_bindings
                .into_iter()
                .map(|binding| typed::TypedParam {
                    name: binding.display,
                    span: binding.span,
                    ty: substitution.apply(&binding.ty).label(),
                })
                .collect::<Vec<_>>();
            let typed_body = finalize_typed_expr(typed_body_draft, &substitution);
            let dict_ref_ids = typed_body.dict_ref_ids.clone();
            let is_valid = if active.is_partial {
                matches!(return_kind, ActiveReturnKind::Option)
            } else {
                !matches!(
                    return_kind,
                    ActiveReturnKind::Option | ActiveReturnKind::Result
                )
            };
            if !is_valid {
                violations.push(TypecheckViolation::active_pattern_return_contract(
                    active.span,
                    active.name.name.as_str(),
                    active.is_partial,
                    return_kind,
                ));
            }

            typed_module
                .active_patterns
                .push(typed::TypedActivePattern {
                    name: active.name.name.clone(),
                    span: active.span,
                    kind: if active.is_partial {
                        typed::ActivePatternKind::Partial
                    } else {
                        typed::ActivePatternKind::Total
                    },
                    return_carrier: active_return_carrier(return_kind),
                    has_miss_path: matches!(return_kind, ActiveReturnKind::Option),
                    params: typed_params,
                    body: typed_body,
                    dict_ref_ids,
                });
        }

        for function in &module.functions {
            metrics.record_function();
            let mut stats = FunctionStats::default();
            let mut constraints = Vec::new();
            let mut env = module_env.clone();
            let mut param_bindings = Vec::new();
            let is_pure = function.attrs.iter().any(|attr| attr.name.name == "pure");
            let function_context = FunctionContext::function(function.name.name.as_str(), is_pure);

            for param in &function.params {
                let ty = param
                    .type_annotation
                    .as_ref()
                    .and_then(|annot| type_from_annotation_kind(&annot.kind))
                    .unwrap_or_else(|| var_gen.fresh_type());
                let scheme = Scheme::simple(ty.clone());
                bind_pattern_to_env(&param.pattern, &scheme, &mut env, &mut var_gen);
                param_bindings.push(ParamBinding {
                    display: param.pattern.render(),
                    span: param.span,
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
                    name: binding.display,
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
        let mir_module = mir::MirModule::from_typed_module(&typed_module);

        TypecheckReport {
            metrics,
            functions,
            violations,
            typed_module,
            mir: mir_module,
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
    pub mir: mir::MirModule,
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
    UnicodeShadowing,
    ResidualLeak,
    ActivePatternReturnContract,
    ActivePatternEffectViolation,
    ActivePatternNameConflict,
    PatternExhaustivenessMissing,
    PatternUnreachableArm,
    PatternBindingDuplicate,
    PatternRegexUnsupportedTarget,
    PatternRangeTypeMismatch,
    PatternRangeBoundInverted,
    PatternSliceTypeMismatch,
    PatternSliceMultipleRest,
    StageMismatch,
    IteratorStageMismatch,
    ValueRestriction,
    PurityViolation,
    ImplDuplicate,
    CoreParseRecoverBranch,
    RuntimeBridgeStageMismatch,
    IteratorExpected,
    ControlFlowUnreachable,
    OpBuilderLevelConflict,
    OpBuilderFixityMissing,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ActiveReturnKind {
    Option,
    Result,
    Value,
}

impl ActiveReturnKind {
    fn label(&self) -> &'static str {
        match self {
            ActiveReturnKind::Option => "Option",
            ActiveReturnKind::Result => "Result",
            ActiveReturnKind::Value => "値",
        }
    }
}

fn active_return_carrier(kind: ActiveReturnKind) -> typed::ActiveReturnCarrier {
    match kind {
        ActiveReturnKind::Option | ActiveReturnKind::Result => {
            typed::ActiveReturnCarrier::OptionLike
        }
        ActiveReturnKind::Value => typed::ActiveReturnCarrier::Value,
    }
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

    fn unicode_shadowing(span: Span, name: &str) -> Self {
        Self {
            kind: TypecheckViolationKind::UnicodeShadowing,
            code: "language.shadowing.unicode",
            message: "Unicode 識別子の再束縛は `let` セクションの警告ポリシーにより拒否されます。"
                .to_string(),
            span: Some(span),
            notes: vec![ViolationNote::plain(format!(
                "`{name}` を 1 回のみ束縛するか、別名に変更してください。"
            ))],
            capability: None,
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

    fn for_iterator_expected(span: Span, actual: Type) -> Self {
        Self {
            kind: TypecheckViolationKind::IteratorExpected,
            code: "language.iterator.expected",
            message: "for 式の `in` 右辺は Array<T> などのイテレータである必要があります"
                .to_string(),
            span: Some(span),
            notes: vec![ViolationNote::plain(format!(
                "実際の型: {}",
                actual.label()
            ))],
            capability: None,
            function: None,
            expected: None,
            iterator_stage: None,
            capability_mismatch: None,
        }
    }

    fn control_flow_unreachable(span: Span) -> Self {
        Self {
            kind: TypecheckViolationKind::ControlFlowUnreachable,
            code: "language.control_flow.unreachable",
            message: "このコードには制御フロー上到達できません".to_string(),
            span: Some(span),
            notes: vec![ViolationNote::plain(
                "`return` や `break` の後に続くコードを削除するか、条件分岐を見直してください。",
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

    fn active_pattern_effect_violation(span: Span, name: Option<String>, effect: String) -> Self {
        let message = match name.as_ref() {
            Some(label) => format!(
                "`@pure` Active Pattern `{}` で副作用 `{}` が検出されました。",
                label, effect
            ),
            None => format!(
                "`@pure` Active Pattern 本体で副作用 `{}` が検出されました。",
                effect
            ),
        };
        Self {
            kind: TypecheckViolationKind::ActivePatternEffectViolation,
            code: "pattern.active.effect_violation",
            message,
            span: Some(span),
            notes: vec![ViolationNote::plain(
                "`@pure` を外すか副作用を伴う処理を通常の関数へ移動してください。",
            )],
            capability: None,
            function: name,
            expected: None,
            iterator_stage: None,
            capability_mismatch: None,
        }
    }

    fn active_pattern_return_contract(
        span: Span,
        name: &str,
        is_partial: bool,
        actual: ActiveReturnKind,
    ) -> Self {
        let message = "Active Pattern の戻り値は Option<T>（部分）または T（完全）のみ許可されます。Result など別の型は使用できません。".to_string();
        let mut notes = Vec::new();
        let expected_label = if is_partial { "Option<T>" } else { "T" };
        notes.push(ViolationNote::plain(format!(
            "期待される戻り値: {expected_label}"
        )));
        let actual_label = actual.label();
        notes.push(ViolationNote::plain(format!(
            "検出した戻り値の形: {}",
            actual_label
        )));
        if matches!(actual, ActiveReturnKind::Result) {
            notes.push(ViolationNote::plain(
                "Result を返す場合は Option へ変換するか通常の関数として呼び出してください",
            ));
        }
        Self {
            kind: TypecheckViolationKind::ActivePatternReturnContract,
            code: "pattern.active.return_contract_invalid",
            message,
            span: Some(span),
            notes,
            capability: None,
            function: Some(name.to_string()),
            expected: None,
            iterator_stage: None,
            capability_mismatch: None,
        }
    }

    fn active_pattern_name_conflict(
        span: Span,
        name: &str,
        other_span: Span,
        other_kind: &str,
    ) -> Self {
        Self {
            kind: TypecheckViolationKind::ActivePatternNameConflict,
            code: "pattern.active.name_conflict",
            message: format!(
                "Active Pattern `{}` は既存の {} と同名のため利用できません。",
                name, other_kind
            ),
            span: Some(span),
            notes: vec![ViolationNote::plain(format!(
                "最初に定義された {} は {} にあります",
                other_kind, other_span
            ))],
            capability: None,
            function: Some(name.to_string()),
            expected: None,
            iterator_stage: None,
            capability_mismatch: None,
        }
    }

    fn pattern_binding_duplicate(span: Span, name: &str) -> Self {
        Self {
            kind: TypecheckViolationKind::PatternBindingDuplicate,
            code: "pattern.binding.duplicate_name",
            message: format!("パターン内で識別子 `{}` が重複しています。", name),
            span: Some(span),
            notes: vec![ViolationNote::plain(
                "`as` と `@` の併用や同名の束縛を見直してください。",
            )],
            capability: None,
            function: None,
            expected: None,
            iterator_stage: None,
            capability_mismatch: None,
        }
    }

    fn pattern_regex_unsupported_target(span: Span, actual: String) -> Self {
        Self {
            kind: TypecheckViolationKind::PatternRegexUnsupportedTarget,
            code: "pattern.regex.unsupported_target",
            message: "正規表現パターンは文字列またはバイト列にのみ適用できます。".to_string(),
            span: Some(span),
            notes: vec![ViolationNote::plain(format!("対象の型: {}", actual))],
            capability: None,
            function: None,
            expected: None,
            iterator_stage: None,
            capability_mismatch: None,
        }
    }

    fn pattern_range_type_mismatch(
        span: Span,
        target: Option<Type>,
        start: Option<Type>,
        end: Option<Type>,
    ) -> Self {
        let mut notes = Vec::new();
        if let Some(target) = target {
            notes.push(ViolationNote::plain(format!(
                "対象の型: {}",
                target.label()
            )));
        }
        if let Some(start) = start {
            notes.push(ViolationNote::plain(format!(
                "開始境界の型: {}",
                start.label()
            )));
        }
        if let Some(end) = end {
            notes.push(ViolationNote::plain(format!(
                "終了境界の型: {}",
                end.label()
            )));
        }
        Self {
            kind: TypecheckViolationKind::PatternRangeTypeMismatch,
            code: "pattern.range.type_mismatch",
            message: "範囲パターンの型が一致しません。数値など比較可能な同一型を使用してください。"
                .to_string(),
            span: Some(span),
            notes,
            capability: None,
            function: None,
            expected: None,
            iterator_stage: None,
            capability_mismatch: None,
        }
    }

    fn pattern_range_bound_inverted(span: Span, start: i64, end: i64) -> Self {
        Self {
            kind: TypecheckViolationKind::PatternRangeBoundInverted,
            code: "pattern.range.bound_inverted",
            message: "範囲パターンの下限と上限が逆転しています。".to_string(),
            span: Some(span),
            notes: vec![ViolationNote::plain(format!("開始: {start} / 終了: {end}"))],
            capability: None,
            function: None,
            expected: None,
            iterator_stage: None,
            capability_mismatch: None,
        }
    }

    fn pattern_slice_type_mismatch(span: Span, actual: String) -> Self {
        Self {
            kind: TypecheckViolationKind::PatternSliceTypeMismatch,
            code: "pattern.slice.type_mismatch",
            message: "スライスパターンは Array など反復可能な型にのみ適用できます。".to_string(),
            span: Some(span),
            notes: vec![ViolationNote::plain(format!("対象の型: {actual}"))],
            capability: None,
            function: None,
            expected: None,
            iterator_stage: None,
            capability_mismatch: None,
        }
    }

    fn pattern_slice_multiple_rest(span: Span) -> Self {
        Self {
            kind: TypecheckViolationKind::PatternSliceMultipleRest,
            code: "pattern.slice.multiple_rest",
            message: "スライスパターンで `..` を複数回使用することはできません。".to_string(),
            span: Some(span),
            notes: vec![ViolationNote::plain(
                "`[head, ..tail]` のように `..` は 1 回に絞ってください。",
            )],
            capability: None,
            function: None,
            expected: None,
            iterator_stage: None,
            capability_mismatch: None,
        }
    }

    fn pattern_exhaustiveness_missing(span: Span) -> Self {
        Self {
            kind: TypecheckViolationKind::PatternExhaustivenessMissing,
            code: "pattern.exhaustiveness.missing",
            message: "この match はすべての入力を網羅していません".to_string(),
            span: Some(span),
            notes: vec![ViolationNote::plain(
                "`_` もしくは完全一致するパターンを追加してください。",
            )],
            capability: None,
            function: None,
            expected: None,
            iterator_stage: None,
            capability_mismatch: None,
        }
    }

    fn pattern_unreachable_arm(span: Span) -> Self {
        Self {
            kind: TypecheckViolationKind::PatternUnreachableArm,
            code: "pattern.unreachable_arm",
            message: "このパターンには到達できません".to_string(),
            span: Some(span),
            notes: vec![ViolationNote::plain(
                "前段で常にマッチするパターンが存在するか、guard が不要か確認してください。",
            )],
            capability: None,
            function: None,
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

    fn opbuilder_level_conflict(
        span: Span,
        priority: i64,
        existing: FixityKind,
        next: FixityKind,
    ) -> Self {
        let message = format!(
            "優先度 {priority} に複数の fixity (`{}` と `{}`) が登録されました。",
            existing.label(),
            next.label()
        );
        Self {
            kind: TypecheckViolationKind::OpBuilderLevelConflict,
            code: "core.parse.opbuilder.level_conflict",
            message,
            span: Some(span),
            notes: vec![ViolationNote::plain(
                "各レベルにつき 1 種類の fixity のみを指定してください。",
            )],
            capability: None,
            function: None,
            expected: None,
            iterator_stage: None,
            capability_mismatch: None,
        }
    }

    fn opbuilder_fixity_missing(span: Span, fixity: FixityKind, reason: impl Into<String>) -> Self {
        let message = format!(
            "`{}` fixity のトークン定義が無効です: {}",
            fixity.keyword(),
            reason.into()
        );
        Self {
            kind: TypecheckViolationKind::OpBuilderFixityMissing,
            code: "core.parse.opbuilder.fixity_missing",
            message,
            span: Some(span),
            notes: vec![ViolationNote::plain(
                "`builder.level(priority, :fixity, [\"token\"])` の形式でトークンを指定してください。",
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
            | TypecheckViolationKind::UnicodeShadowing
            | TypecheckViolationKind::ActivePatternReturnContract
            | TypecheckViolationKind::ActivePatternNameConflict
            | TypecheckViolationKind::PatternExhaustivenessMissing
            | TypecheckViolationKind::PatternUnreachableArm
            | TypecheckViolationKind::PatternBindingDuplicate
            | TypecheckViolationKind::PatternRegexUnsupportedTarget
            | TypecheckViolationKind::PatternRangeTypeMismatch
            | TypecheckViolationKind::PatternRangeBoundInverted
            | TypecheckViolationKind::PatternSliceTypeMismatch
            | TypecheckViolationKind::PatternSliceMultipleRest
            | TypecheckViolationKind::ValueRestriction
            | TypecheckViolationKind::ImplDuplicate
            | TypecheckViolationKind::IteratorExpected
            | TypecheckViolationKind::ControlFlowUnreachable => "type",
            TypecheckViolationKind::ResidualLeak
            | TypecheckViolationKind::StageMismatch
            | TypecheckViolationKind::IteratorStageMismatch
            | TypecheckViolationKind::PurityViolation
            | TypecheckViolationKind::ActivePatternEffectViolation => "effects",
            TypecheckViolationKind::CoreParseRecoverBranch
            | TypecheckViolationKind::OpBuilderLevelConflict
            | TypecheckViolationKind::OpBuilderFixityMissing => "parser",
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

fn collect_opbuilder_violations(module: &Module, violations: &mut Vec<TypecheckViolation>) {
    for decl in &module.decls {
        let mut tracker = OpBuilderTracker::new(None);
        visit_decl_for_opbuilder(decl, &mut tracker, violations);
    }
    for function in &module.functions {
        let mut tracker = OpBuilderTracker::new(Some(function.name.name.as_str()));
        visit_expr_for_opbuilder(&function.body, &mut tracker, violations);
    }
}

#[derive(Default)]
struct OpBuilderTracker {
    scope: String,
    builders: HashMap<String, HashMap<i64, FixityKind>>,
}

impl OpBuilderTracker {
    fn new(scope: Option<&str>) -> Self {
        Self {
            scope: scope.unwrap_or("<module>").to_string(),
            builders: HashMap::new(),
        }
    }

    fn record_level_call(
        &mut self,
        builder: String,
        priority: i64,
        fixity: FixityKind,
        span: Span,
        violations: &mut Vec<TypecheckViolation>,
    ) {
        let key = format!("{}::{}", self.scope, builder);
        let entry = self.builders.entry(key).or_default();
        if let Some(existing) = entry.get(&priority).copied() {
            if existing != fixity {
                violations.push(TypecheckViolation::opbuilder_level_conflict(
                    span, priority, existing, fixity,
                ));
            }
        } else {
            entry.insert(priority, fixity);
        }
    }
}

fn visit_decl_for_opbuilder(
    decl: &Decl,
    tracker: &mut OpBuilderTracker,
    violations: &mut Vec<TypecheckViolation>,
) {
    match &decl.kind {
        DeclKind::Let { value, .. } | DeclKind::Var { value, .. } => {
            visit_expr_for_opbuilder(value, tracker, violations);
        }
        DeclKind::Effect(effect) => {
            for op in &effect.operations {
                for attr in &op.attrs {
                    for arg in &attr.args {
                        visit_expr_for_opbuilder(arg, tracker, violations);
                    }
                }
            }
        }
        DeclKind::Conductor(conductor) => {
            if let Some(exec) = &conductor.execution {
                visit_expr_for_opbuilder(&exec.body, tracker, violations);
            }
            if let Some(monitor) = &conductor.monitoring {
                visit_expr_for_opbuilder(&monitor.body, tracker, violations);
            }
        }
        _ => {}
    }
}

fn visit_stmt_for_opbuilder(
    stmt: &Stmt,
    tracker: &mut OpBuilderTracker,
    violations: &mut Vec<TypecheckViolation>,
) {
    match &stmt.kind {
        StmtKind::Decl { decl } => visit_decl_for_opbuilder(decl, tracker, violations),
        StmtKind::Expr { expr } | StmtKind::Defer { expr } => {
            visit_expr_for_opbuilder(expr, tracker, violations)
        }
        StmtKind::Assign { target, value } => {
            visit_expr_for_opbuilder(target, tracker, violations);
            visit_expr_for_opbuilder(value, tracker, violations);
        }
    }
}

fn visit_literal_for_opbuilder(
    literal: &Literal,
    tracker: &mut OpBuilderTracker,
    violations: &mut Vec<TypecheckViolation>,
) {
    match &literal.value {
        LiteralKind::Tuple { elements } | LiteralKind::Array { elements } => {
            for element in elements {
                visit_expr_for_opbuilder(element, tracker, violations);
            }
        }
        LiteralKind::Record { fields } => {
            for field in fields {
                visit_expr_for_opbuilder(&field.value, tracker, violations);
            }
        }
        _ => {}
    }
}

fn visit_expr_for_opbuilder(
    expr: &Expr,
    tracker: &mut OpBuilderTracker,
    violations: &mut Vec<TypecheckViolation>,
) {
    if let Some(call) = extract_opbuilder_call(expr) {
        if let Err(reason) = validate_opbuilder_tokens(call.tokens_expr, call.fixity) {
            violations.push(TypecheckViolation::opbuilder_fixity_missing(
                call.tokens_expr.span(),
                call.fixity,
                reason,
            ));
        } else {
            tracker.record_level_call(
                call.builder_key,
                call.priority,
                call.fixity,
                call.span,
                violations,
            );
        }
    }

    match &expr.kind {
        ExprKind::Literal(literal) => visit_literal_for_opbuilder(literal, tracker, violations),
        ExprKind::FixityLiteral(_) | ExprKind::Identifier(_) | ExprKind::ModulePath(_) => {}
        ExprKind::Call { callee, args } => {
            visit_expr_for_opbuilder(callee, tracker, violations);
            for arg in args {
                visit_expr_for_opbuilder(arg, tracker, violations);
            }
        }
        ExprKind::PerformCall { call } => {
            visit_expr_for_opbuilder(&call.argument, tracker, violations);
        }
        ExprKind::Lambda { body, .. } => visit_expr_for_opbuilder(body, tracker, violations),
        ExprKind::Pipe { left, right } | ExprKind::Binary { left, right, .. } => {
            visit_expr_for_opbuilder(left, tracker, violations);
            visit_expr_for_opbuilder(right, tracker, violations);
        }
        ExprKind::Unary { expr: body, .. }
        | ExprKind::Propagate { expr: body }
        | ExprKind::Loop { body }
        | ExprKind::Defer { body }
        | ExprKind::Assign { value: body, .. } => {
            visit_expr_for_opbuilder(body, tracker, violations)
        }
        ExprKind::FieldAccess { target, .. } | ExprKind::TupleAccess { target, .. } => {
            visit_expr_for_opbuilder(target, tracker, violations)
        }
        ExprKind::Index { target, index } => {
            visit_expr_for_opbuilder(target, tracker, violations);
            visit_expr_for_opbuilder(index, tracker, violations);
        }
        ExprKind::Return { value } | ExprKind::Break { value } => {
            if let Some(value) = value {
                visit_expr_for_opbuilder(value, tracker, violations);
            }
        }
        ExprKind::IfElse {
            condition,
            then_branch,
            else_branch,
        } => {
            visit_expr_for_opbuilder(condition, tracker, violations);
            visit_expr_for_opbuilder(then_branch, tracker, violations);
            if let Some(else_branch) = else_branch {
                visit_expr_for_opbuilder(else_branch, tracker, violations);
            }
        }
        ExprKind::Match { target, arms } => {
            visit_expr_for_opbuilder(target, tracker, violations);
            for arm in arms {
                if let Some(guard) = &arm.guard {
                    visit_expr_for_opbuilder(guard, tracker, violations);
                }
                visit_expr_for_opbuilder(&arm.body, tracker, violations);
            }
        }
        ExprKind::While { condition, body } => {
            visit_expr_for_opbuilder(condition, tracker, violations);
            visit_expr_for_opbuilder(body, tracker, violations);
        }
        ExprKind::For { start, end, .. } => {
            visit_expr_for_opbuilder(start, tracker, violations);
            visit_expr_for_opbuilder(end, tracker, violations);
        }
        ExprKind::Handle { handle } => {
            visit_expr_for_opbuilder(&handle.target, tracker, violations);
            for entry in &handle.handler.entries {
                match entry {
                    HandlerEntry::Operation { body, .. } | HandlerEntry::Return { body, .. } => {
                        visit_expr_for_opbuilder(body, tracker, violations)
                    }
                }
            }
        }
        ExprKind::Block { statements, .. } => {
            for stmt in statements {
                visit_stmt_for_opbuilder(stmt, tracker, violations);
            }
        }
        ExprKind::Unsafe { body } => visit_expr_for_opbuilder(body, tracker, violations),
        ExprKind::Continue => {}
    }
}

struct OpBuilderCall<'a> {
    builder_key: String,
    priority: i64,
    fixity: FixityKind,
    span: Span,
    tokens_expr: &'a Expr,
}

fn extract_opbuilder_call(expr: &Expr) -> Option<OpBuilderCall<'_>> {
    if let ExprKind::Call { callee, args } = &expr.kind {
        if args.len() < 3 {
            return None;
        }
        if let ExprKind::FieldAccess { target, field } = &callee.kind {
            if field.name != "level" {
                return None;
            }
            let builder_key = render_opbuilder_target(target)?;
            let priority = extract_priority(&args[0])?;
            let fixity = match args[1].kind {
                ExprKind::FixityLiteral(kind) => kind,
                _ => return None,
            };
            return Some(OpBuilderCall {
                builder_key,
                priority,
                fixity,
                span: expr.span(),
                tokens_expr: &args[2],
            });
        }
    }
    None
}

fn render_opbuilder_target(expr: &Expr) -> Option<String> {
    match &expr.kind {
        ExprKind::Identifier(ident) => Some(ident.name.clone()),
        ExprKind::FieldAccess { target, field } => {
            render_opbuilder_target(target).map(|base| format!("{base}.{}", field.name))
        }
        ExprKind::ModulePath(path) => Some(path.render()),
        _ => None,
    }
}

fn extract_priority(expr: &Expr) -> Option<i64> {
    match &expr.kind {
        ExprKind::Literal(Literal {
            value: LiteralKind::Int { value, .. },
        }) => Some(*value),
        _ => None,
    }
}

fn validate_opbuilder_tokens(expr: &Expr, fixity: FixityKind) -> Result<(), String> {
    match &expr.kind {
        ExprKind::Literal(Literal {
            value: LiteralKind::Array { elements },
        }) => {
            if elements.is_empty() {
                return Err("トークン配列が空です".to_string());
            }
            if fixity == FixityKind::Ternary && elements.len() < 2 {
                return Err("`:ternary` には head/mid の 2 トークンが必要です".to_string());
            }
            for element in elements {
                match &element.kind {
                    ExprKind::Literal(Literal {
                        value: LiteralKind::String { .. },
                    }) => {}
                    _ => {
                        return Err("レベルのトークンは文字列リテラルで指定してください".to_string())
                    }
                }
            }
            Ok(())
        }
        _ => Err("レベルのトークンは配列リテラルで指定してください".to_string()),
    }
}

#[derive(Default)]
struct LoopContextStack {
    frames: Vec<LoopFrame>,
}

struct LoopFrame {
    result_ty: Type,
    has_result: bool,
}

impl LoopContextStack {
    fn push(&mut self, result_ty: Type) {
        self.frames.push(LoopFrame {
            result_ty,
            has_result: false,
        });
    }

    fn pop(&mut self) -> Option<LoopFrame> {
        self.frames.pop()
    }

    fn current_mut(&mut self) -> Option<&mut LoopFrame> {
        self.frames.last_mut()
    }
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
    let mut loop_context = LoopContextStack::default();
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
        &mut loop_context,
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
    loop_context: &mut LoopContextStack,
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
                loop_context,
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
                loop_context,
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
            if matches!(operator, BinaryOp::And | BinaryOp::Or) {
                let bool_ty = Type::builtin(BuiltinType::Bool);
                stats.constraints += 2;
                metrics.record_constraint("binary.logical");
                constraints.push(Constraint::equal(left_result.ty.clone(), bool_ty.clone()));
                constraints.push(Constraint::equal(right_result.ty.clone(), bool_ty.clone()));
                metrics.record_unify_call();
                let _ = solver.unify(left_result.ty.clone(), bool_ty.clone());
                metrics.record_unify_call();
                let _ = solver.unify(right_result.ty.clone(), bool_ty);
            }
            let ty = match operator {
                BinaryOp::Add
                | BinaryOp::Sub
                | BinaryOp::Mul
                | BinaryOp::Div
                | BinaryOp::Mod
                | BinaryOp::Pow => combine_numeric_types(&left_result.ty, &right_result.ty),
                BinaryOp::And | BinaryOp::Or => Type::builtin(BuiltinType::Bool),
                BinaryOp::Eq
                | BinaryOp::Ne
                | BinaryOp::Lt
                | BinaryOp::Le
                | BinaryOp::Gt
                | BinaryOp::Ge => Type::builtin(BuiltinType::Bool),
                _ => combine_numeric_types(&left_result.ty, &right_result.ty),
            };
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
                loop_context,
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
                        loop_context,
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
                loop_context,
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
                violations.push(context.purity_violation(expr.span(), call.effect.name.clone()));
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
        ExprKind::Loop { body } => {
            let loop_ty = var_gen.fresh_type();
            loop_context.push(loop_ty.clone());
            let body_result = infer_expr(
                body,
                env,
                var_gen,
                solver,
                constraints,
                stats,
                metrics,
                violations,
                dict_refs,
                loop_context,
                context,
            );
            let frame = loop_context.pop().unwrap_or(LoopFrame {
                result_ty: loop_ty.clone(),
                has_result: false,
            });
            let ty = if frame.has_result {
                solver.substitution().apply(&frame.result_ty)
            } else {
                Type::builtin(BuiltinType::Unknown)
            };
            make_typed(
                expr,
                TypedExprKindDraft::Unknown,
                ty,
                body_result.dict_ref_ids,
            )
        }
        ExprKind::Break { value } => {
            let mut dict_ids = Vec::new();
            let mut break_ty = Type::builtin(BuiltinType::Unit);
            if let Some(break_expr) = value.as_deref() {
                let value_result = infer_expr(
                    break_expr,
                    env,
                    var_gen,
                    solver,
                    constraints,
                    stats,
                    metrics,
                    violations,
                    dict_refs,
                    loop_context,
                    context,
                );
                break_ty = value_result.ty.clone();
                dict_ids.extend(value_result.dict_ref_ids);
            }
            if let Some(frame) = loop_context.current_mut() {
                stats.constraints += 1;
                metrics.record_constraint("loop.break");
                constraints.push(Constraint::equal(frame.result_ty.clone(), break_ty.clone()));
                metrics.record_unify_call();
                let _ = solver.unify(frame.result_ty.clone(), break_ty);
                frame.has_result = true;
            }
            make_typed(
                expr,
                TypedExprKindDraft::Unknown,
                Type::builtin(BuiltinType::Unknown),
                dict_ids,
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
                loop_context,
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
                loop_context,
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
                loop_context,
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
        ExprKind::While { condition, body } => {
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
                loop_context,
                context,
            );
            check_bool_condition(condition.span(), &condition_result.ty, violations, context);
            let body_result = infer_expr(
                body,
                env,
                var_gen,
                solver,
                constraints,
                stats,
                metrics,
                violations,
                dict_refs,
                loop_context,
                context,
            );
            let mut dicts = condition_result.dict_ref_ids;
            dicts.extend(body_result.dict_ref_ids);
            make_typed(
                expr,
                TypedExprKindDraft::Unknown,
                Type::builtin(BuiltinType::Unknown),
                dicts,
            )
        }
        ExprKind::For {
            pattern,
            start,
            end,
        } => {
            let start_result = infer_expr(
                start,
                env,
                var_gen,
                solver,
                constraints,
                stats,
                metrics,
                violations,
                dict_refs,
                loop_context,
                context,
            );
            let element_ty = var_gen.fresh_type();
            let array_ty = Type::app("Array", vec![element_ty.clone()]);
            stats.constraints += 1;
            metrics.record_constraint("for.iterator");
            constraints.push(Constraint::equal(start_result.ty.clone(), array_ty.clone()));
            metrics.record_unify_call();
            if let Err(_) = solver.unify(start_result.ty.clone(), array_ty.clone()) {
                violations.push(TypecheckViolation::for_iterator_expected(
                    start.span(),
                    start_result.ty.clone(),
                ));
            }
            let mut loop_env = env.enter_scope();
            let element_scheme = Scheme::simple(solver.substitution().apply(&element_ty.clone()));
            bind_pattern_to_env(pattern, &element_scheme, &mut loop_env, var_gen);
            let body_result = infer_expr(
                end,
                &mut loop_env,
                var_gen,
                solver,
                constraints,
                stats,
                metrics,
                violations,
                dict_refs,
                loop_context,
                context,
            );
            let mut dicts = start_result.dict_ref_ids;
            dicts.extend(body_result.dict_ref_ids);
            make_typed(
                expr,
                TypedExprKindDraft::Unknown,
                Type::builtin(BuiltinType::Unknown),
                dicts,
            )
        }
        ExprKind::Match { target, arms } => {
            let target_result = infer_expr(
                target,
                env,
                var_gen,
                solver,
                constraints,
                stats,
                metrics,
                violations,
                dict_refs,
                loop_context,
                context,
            );
            let mut dicts = target_result.dict_ref_ids.clone();
            let coverage = analyze_match_exhaustiveness(arms);
            let target_ty = solver.substitution().apply(&target_result.ty);
            let mut arm_type: Option<Type> = None;
            let unreachable_indices: HashSet<usize> =
                coverage.unreachable_arm_indices.iter().copied().collect();
            let mut typed_arms = Vec::new();
            for (arm_index, arm) in arms.iter().enumerate() {
                if unreachable_indices.contains(&arm_index) {
                    violations.push(TypecheckViolation::pattern_unreachable_arm(arm.span));
                }
                let mut arm_env = env.enter_scope();
                detect_duplicate_bindings(&arm.pattern, violations);
                validate_pattern_against_type(&arm.pattern, &target_ty, violations);
                detect_regex_target_mismatch(&arm.pattern, &target_ty, violations);
                let pattern_scheme = Scheme::simple(var_gen.fresh_type());
                bind_pattern_to_env(&arm.pattern, &pattern_scheme, &mut arm_env, var_gen);
                if let Some(alias) = &arm.alias {
                    arm_env.insert(alias.name.clone(), Scheme::simple(var_gen.fresh_type()));
                }
                let typed_guard_draft = if let Some(guard) = &arm.guard {
                    let guard_result = infer_expr(
                        guard,
                        &mut arm_env,
                        var_gen,
                        solver,
                        constraints,
                        stats,
                        metrics,
                        violations,
                        dict_refs,
                        loop_context,
                        context,
                    );
                    check_bool_condition(guard.span(), &guard_result.ty, violations, context);
                    dicts.extend(guard_result.dict_ref_ids.clone());
                    Some(guard_result)
                } else {
                    None
                };
                let body_result = infer_expr(
                    &arm.body,
                    &mut arm_env,
                    var_gen,
                    solver,
                    constraints,
                    stats,
                    metrics,
                    violations,
                    dict_refs,
                    loop_context,
                    context,
                );
                if let Some(existing) = arm_type.as_ref() {
                    stats.constraints += 1;
                    metrics.record_constraint("match.arm");
                    constraints.push(Constraint::equal(existing.clone(), body_result.ty.clone()));
                    metrics.record_unify_call();
                    let _ = solver.unify(existing.clone(), body_result.ty.clone());
                } else {
                    arm_type = Some(body_result.ty.clone());
                }
                dicts.extend(body_result.dict_ref_ids.clone());
                typed_arms.push(TypedMatchArmDraft {
                    pattern: lower_typed_pattern(&arm.pattern),
                    guard: typed_guard_draft,
                    alias: arm.alias.as_ref().map(|ident| ident.name.clone()),
                    body: body_result,
                });
            }
            if !coverage.coverage_reached {
                violations.push(TypecheckViolation::pattern_exhaustiveness_missing(
                    expr.span(),
                ));
            }
            make_typed(
                expr,
                TypedExprKindDraft::Match {
                    target: Box::new(target_result),
                    arms: typed_arms,
                },
                arm_type.unwrap_or_else(|| Type::builtin(BuiltinType::Unknown)),
                dicts,
            )
        }
        ExprKind::Block { statements, .. } => {
            let block_result = infer_block(
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
                loop_context,
            );
            if let Some(tail_expr) = block_result.tail_expr {
                TypedExprDraft {
                    span: expr.span,
                    kind: tail_expr.kind,
                    ty: block_result.ty,
                    dict_ref_ids: block_result.dict_ref_ids,
                }
            } else {
                make_typed(
                    expr,
                    TypedExprKindDraft::Unknown,
                    block_result.ty,
                    block_result.dict_ref_ids,
                )
            }
        }
        ExprKind::Lambda { params, body, .. } => {
            let mut lambda_env = env.enter_scope();
            let mut param_types = Vec::new();
            for param in params {
                let ty = param
                    .type_annotation
                    .as_ref()
                    .and_then(|annot| type_from_annotation_kind(&annot.kind))
                    .unwrap_or_else(|| var_gen.fresh_type());
                let scheme = Scheme::simple(ty.clone());
                bind_pattern_to_env(&param.pattern, &scheme, &mut lambda_env, var_gen);
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
                loop_context,
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

struct BlockInferenceResult {
    ty: Type,
    dict_ref_ids: Vec<typed::DictRefId>,
    tail_expr: Option<TypedExprDraft>,
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
    loop_context: &mut LoopContextStack,
) -> BlockInferenceResult {
    let mut block_env = parent_env.enter_scope();
    let mut last_ty = Type::builtin(BuiltinType::Unknown);
    let mut block_dict_refs = Vec::new();
    let mut tail_expr = None;
    let mut terminated = false;
    for stmt in statements {
        if terminated {
            violations.push(TypecheckViolation::control_flow_unreachable(stmt.span));
        }
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
                    None,
                    context,
                    loop_context,
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
                    loop_context,
                    context,
                );
                last_ty = expr_result.ty.clone();
                block_dict_refs.extend(expr_result.dict_ref_ids.clone());
                tail_expr = Some(expr_result);
                if matches!(expr.kind, ExprKind::Return { .. } | ExprKind::Break { .. }) {
                    terminated = true;
                }
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
                    loop_context,
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
                    loop_context,
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
                    loop_context,
                    context,
                );
                block_dict_refs.extend(defer_result.dict_ref_ids);
            }
        }
    }
    BlockInferenceResult {
        ty: last_ty,
        dict_ref_ids: block_dict_refs,
        tail_expr,
    }
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
    unicode_tracker: Option<&mut UnicodeShadowTracker>,
    context: FunctionContext<'_>,
    loop_context: &mut LoopContextStack,
) -> Vec<typed::DictRefId> {
    match &decl.kind {
        DeclKind::Let {
            pattern,
            value,
            type_annotation: _,
        } => {
            if let Some(tracker) = unicode_tracker {
                tracker.observe_pattern(pattern, decl.span, violations);
            }
            infer_binding(
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
                loop_context,
            )
        }
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
                loop_context,
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
    loop_context: &mut LoopContextStack,
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
        loop_context,
        context,
    );
    let substitution = solver.substitution().clone();
    let resolved_ty = substitution.apply(&value_result.ty);
    detect_duplicate_bindings(pattern, violations);
    validate_pattern_against_type(pattern, &resolved_ty, violations);
    detect_regex_target_mismatch(pattern, &resolved_ty, violations);
    let scheme = generalize_type(env, resolved_ty.clone());
    bind_pattern_to_env(pattern, &scheme, env, var_gen);
    value_result.dict_ref_ids
}

fn bind_pattern_to_env(
    pattern: &Pattern,
    scheme: &Scheme,
    env: &mut TypeEnv,
    var_gen: &mut TypeVarGen,
) {
    match &pattern.kind {
        PatternKind::Var(ident) => {
            env.insert(ident.name.clone(), scheme.clone());
        }
        PatternKind::Binding {
            name,
            pattern,
            via_at: _,
        } => {
            env.insert(name.name.clone(), scheme.clone());
            bind_pattern_to_env(pattern, scheme, env, var_gen);
        }
        PatternKind::Or { variants } => {
            for variant in variants {
                bind_pattern_to_env(variant, scheme, env, var_gen);
            }
        }
        PatternKind::Tuple { elements } => {
            for element in elements {
                let element_scheme = Scheme::simple(var_gen.fresh_type());
                bind_pattern_to_env(element, &element_scheme, env, var_gen);
            }
        }
        PatternKind::Record { fields, .. } => {
            for field in fields {
                if let Some(value) = &field.value {
                    let field_scheme = Scheme::simple(var_gen.fresh_type());
                    bind_pattern_to_env(value, &field_scheme, env, var_gen);
                } else {
                    env.insert(field.key.name.clone(), Scheme::simple(var_gen.fresh_type()));
                }
            }
        }
        PatternKind::Slice { elements } => {
            for element in elements {
                match element {
                    SlicePatternItem::Element(pat) => {
                        let element_scheme = Scheme::simple(var_gen.fresh_type());
                        bind_pattern_to_env(pat, &element_scheme, env, var_gen);
                    }
                    SlicePatternItem::Rest { ident: Some(ident) } => {
                        env.insert(ident.name.clone(), Scheme::simple(var_gen.fresh_type()));
                    }
                    SlicePatternItem::Rest { ident: None } => {}
                }
            }
        }
        PatternKind::Range { .. } => {}
        PatternKind::Regex { .. } => {}
        PatternKind::Constructor { args, .. } => {
            for arg in args {
                let arg_scheme = Scheme::simple(var_gen.fresh_type());
                bind_pattern_to_env(arg, &arg_scheme, env, var_gen);
            }
        }
        PatternKind::Guard { pattern: inner, .. } => {
            bind_pattern_to_env(inner, scheme, env, var_gen);
        }
        PatternKind::ActivePattern { argument, .. } => {
            if let Some(argument) = argument {
                let arg_scheme = Scheme::simple(var_gen.fresh_type());
                bind_pattern_to_env(argument, &arg_scheme, env, var_gen);
            }
        }
        PatternKind::Literal(_) | PatternKind::Wildcard => {}
    }
}

fn lower_typed_pattern(pattern: &Pattern) -> typed::TypedPattern {
    use typed::{TypedPatternKind, TypedPatternRecordField, TypedSlicePatternItem};

    let kind = match &pattern.kind {
        PatternKind::Wildcard => TypedPatternKind::Wildcard,
        PatternKind::Var(ident) => TypedPatternKind::Var {
            name: ident.name.clone(),
        },
        PatternKind::Literal(literal) => TypedPatternKind::Literal(literal.clone()),
        PatternKind::Tuple { elements } => TypedPatternKind::Tuple {
            elements: elements.iter().map(lower_typed_pattern).collect(),
        },
        PatternKind::Record { fields, has_rest } => TypedPatternKind::Record {
            fields: fields
                .iter()
                .map(|field| TypedPatternRecordField {
                    key: field.key.name.clone(),
                    value: field.value.as_deref().map(lower_typed_pattern).map(Box::new),
                })
                .collect(),
            has_rest: *has_rest,
        },
        PatternKind::Constructor { name, args } => TypedPatternKind::Constructor {
            name: name.name.clone(),
            args: args.iter().map(lower_typed_pattern).collect(),
        },
        PatternKind::Guard { pattern: inner, .. } => {
            return lower_typed_pattern(inner);
        }
        PatternKind::Binding {
            name,
            pattern: inner,
            via_at,
        } => TypedPatternKind::Binding {
            name: name.name.clone(),
            pattern: Box::new(lower_typed_pattern(inner)),
            via_at: *via_at,
        },
        PatternKind::Or { variants } => TypedPatternKind::Or {
            variants: variants.iter().map(lower_typed_pattern).collect(),
        },
        PatternKind::Slice { elements } => TypedPatternKind::Slice {
            elements: elements
                .iter()
                .map(|elem| match elem {
                    SlicePatternItem::Element(pat) => {
                        TypedSlicePatternItem::Element(lower_typed_pattern(pat))
                    }
                    SlicePatternItem::Rest { ident } => TypedSlicePatternItem::Rest {
                        ident: ident.as_ref().map(|id| id.name.clone()),
                    },
                })
                .collect(),
        },
        PatternKind::Range {
            start,
            end,
            inclusive,
        } => TypedPatternKind::Range {
            start: start.as_deref().map(lower_typed_pattern).map(Box::new),
            end: end.as_deref().map(lower_typed_pattern).map(Box::new),
            inclusive: *inclusive,
        },
        PatternKind::Regex { pattern, .. } => TypedPatternKind::Regex {
            pattern: pattern.clone(),
        },
        PatternKind::ActivePattern {
            name,
            is_partial,
            argument,
        } => TypedPatternKind::ActivePattern {
            name: name.name.clone(),
            is_partial: *is_partial,
            argument: argument
                .as_deref()
                .map(lower_typed_pattern)
                .map(Box::new),
        },
    };

    typed::TypedPattern {
        span: pattern.span,
        kind,
    }
}

fn pattern_binding_name(pattern: &Pattern) -> Option<String> {
    match &pattern.kind {
        PatternKind::Var(ident) => Some(ident.name.clone()),
        _ => None,
    }
}

fn classify_active_pattern_return(expr: &Expr) -> ActiveReturnKind {
    match &expr.kind {
        ExprKind::IfElse {
            then_branch,
            else_branch,
            ..
        } => {
            let then_kind = classify_active_pattern_return(then_branch);
            let else_kind = else_branch
                .as_deref()
                .map(classify_active_pattern_return)
                .unwrap_or(ActiveReturnKind::Value);
            if then_kind == else_kind {
                then_kind
            } else {
                ActiveReturnKind::Value
            }
        }
        ExprKind::Call { callee, .. } => {
            classify_constructor(callee).unwrap_or(ActiveReturnKind::Value)
        }
        ExprKind::Identifier(ident) => {
            classify_constructor_name(ident.name.as_str()).unwrap_or(ActiveReturnKind::Value)
        }
        ExprKind::ModulePath(path) => {
            classify_constructor_path(path).unwrap_or(ActiveReturnKind::Value)
        }
        ExprKind::Match { arms, .. } => classify_match_return(arms),
        ExprKind::Block { statements, .. } => classify_block_return(statements),
        _ => ActiveReturnKind::Value,
    }
}

fn classify_block_return(statements: &[Stmt]) -> ActiveReturnKind {
    for stmt in statements.iter().rev() {
        match &stmt.kind {
            StmtKind::Expr { expr } | StmtKind::Defer { expr } => {
                return classify_active_pattern_return(expr)
            }
            _ => continue,
        }
    }
    ActiveReturnKind::Value
}

fn classify_match_return(arms: &[MatchArm]) -> ActiveReturnKind {
    let mut iter = arms.iter();
    if let Some(first) = iter.next() {
        let mut kind = classify_active_pattern_return(&first.body);
        for arm in iter {
            let next = classify_active_pattern_return(&arm.body);
            if next != kind {
                kind = ActiveReturnKind::Value;
                break;
            }
        }
        kind
    } else {
        ActiveReturnKind::Value
    }
}

fn classify_constructor(expr: &Expr) -> Option<ActiveReturnKind> {
    match &expr.kind {
        ExprKind::Identifier(ident) => classify_constructor_name(ident.name.as_str()),
        ExprKind::ModulePath(path) => classify_constructor_path(path),
        _ => None,
    }
}

fn classify_constructor_path(path: &ModulePath) -> Option<ActiveReturnKind> {
    module_path_last_segment(path)
        .and_then(|segment| classify_constructor_name(segment.name.as_str()))
}

fn classify_constructor_name(name: &str) -> Option<ActiveReturnKind> {
    match name {
        "Some" | "None" => Some(ActiveReturnKind::Option),
        "Ok" | "Err" => Some(ActiveReturnKind::Result),
        _ => None,
    }
}

fn detect_duplicate_bindings(pattern: &Pattern, violations: &mut Vec<TypecheckViolation>) {
    fn walk(
        pattern: &Pattern,
        seen: &mut HashSet<String>,
        violations: &mut Vec<TypecheckViolation>,
    ) {
        match &pattern.kind {
            PatternKind::Var(ident) => {
                if !seen.insert(ident.name.clone()) {
                    violations.push(TypecheckViolation::pattern_binding_duplicate(
                        pattern.span,
                        ident.name.as_str(),
                    ));
                }
            }
            PatternKind::Binding { name, pattern, .. } => {
                if !seen.insert(name.name.clone()) {
                    violations.push(TypecheckViolation::pattern_binding_duplicate(
                        pattern.span,
                        name.name.as_str(),
                    ));
                }
                walk(pattern, seen, violations);
            }
            PatternKind::Tuple { elements } => {
                for element in elements {
                    walk(element, seen, violations);
                }
            }
            PatternKind::Record { fields, .. } => {
                for field in fields {
                    if let Some(value) = &field.value {
                        walk(value, seen, violations);
                    } else if !seen.insert(field.key.name.clone()) {
                        violations.push(TypecheckViolation::pattern_binding_duplicate(
                            pattern.span,
                            field.key.name.as_str(),
                        ));
                    }
                }
            }
            PatternKind::Constructor { args, .. } => {
                for arg in args {
                    walk(arg, seen, violations);
                }
            }
            PatternKind::Guard { pattern: inner, .. } => walk(inner, seen, violations),
            PatternKind::ActivePattern { argument, .. } => {
                if let Some(arg) = argument {
                    walk(arg, seen, violations);
                }
            }
            PatternKind::Or { variants } => {
                for variant in variants {
                    let mut branch_seen = seen.clone();
                    walk(variant, &mut branch_seen, violations);
                }
            }
            PatternKind::Slice { elements } => {
                for element in elements {
                    match element {
                        SlicePatternItem::Element(pat) => walk(pat, seen, violations),
                        SlicePatternItem::Rest { ident: Some(ident) } => {
                            if !seen.insert(ident.name.clone()) {
                                violations.push(TypecheckViolation::pattern_binding_duplicate(
                                    pattern.span,
                                    ident.name.as_str(),
                                ));
                            }
                        }
                        SlicePatternItem::Rest { ident: None } => {}
                    }
                }
            }
            PatternKind::Range { start, end, .. } => {
                if let Some(start) = start {
                    walk(start, seen, violations);
                }
                if let Some(end) = end {
                    walk(end, seen, violations);
                }
            }
            PatternKind::Regex { .. } | PatternKind::Literal(_) | PatternKind::Wildcard => {}
        }
    }

    let mut seen = HashSet::new();
    walk(pattern, &mut seen, violations);
}

fn is_string_like(ty: &Type) -> bool {
    matches!(ty, Type::Builtin(BuiltinType::Str))
        || matches!(ty, Type::Builtin(BuiltinType::Bytes))
        || matches!(
            ty,
            Type::App {
                constructor,
                arguments
            } if (constructor == "Str" || constructor == "Bytes") && arguments.is_empty()
        )
}

fn detect_regex_target_mismatch(
    pattern: &Pattern,
    target_ty: &Type,
    violations: &mut Vec<TypecheckViolation>,
) {
    fn walk(pattern: &Pattern, target_ty: &Type, violations: &mut Vec<TypecheckViolation>) {
        match &pattern.kind {
            PatternKind::Regex { .. } => {
                if !is_string_like(target_ty)
                    && !matches!(
                        target_ty,
                        Type::Builtin(BuiltinType::Unknown) | Type::Var(_)
                    )
                {
                    violations.push(TypecheckViolation::pattern_regex_unsupported_target(
                        pattern.span,
                        target_ty.label(),
                    ));
                }
            }
            PatternKind::Tuple { elements } => {
                for element in elements {
                    walk(element, target_ty, violations);
                }
            }
            PatternKind::Record { fields, .. } => {
                for field in fields {
                    if let Some(value) = &field.value {
                        walk(value, target_ty, violations);
                    }
                }
            }
            PatternKind::Constructor { args, .. } => {
                for arg in args {
                    walk(arg, target_ty, violations);
                }
            }
            PatternKind::Guard { pattern: inner, .. } => walk(inner, target_ty, violations),
            PatternKind::ActivePattern { argument, .. } => {
                if let Some(arg) = argument {
                    walk(arg, target_ty, violations);
                }
            }
            PatternKind::Or { variants } => {
                for variant in variants {
                    walk(variant, target_ty, violations);
                }
            }
            PatternKind::Slice { elements } => {
                for element in elements {
                    if let SlicePatternItem::Element(inner) = element {
                        walk(inner, target_ty, violations);
                    }
                }
            }
            PatternKind::Range { start, end, .. } => {
                if let Some(start) = start {
                    walk(start, target_ty, violations);
                }
                if let Some(end) = end {
                    walk(end, target_ty, violations);
                }
            }
            PatternKind::Binding { pattern: inner, .. } => {
                walk(inner, target_ty, violations);
            }
            PatternKind::Literal(_) | PatternKind::Wildcard | PatternKind::Var(_) => {}
        }
    }

    walk(pattern, target_ty, violations);
}

fn is_unknown_type(ty: &Type) -> bool {
    matches!(ty, Type::Builtin(BuiltinType::Unknown) | Type::Var(_))
}

fn types_compatible(left: &Type, right: &Type) -> bool {
    left == right || is_unknown_type(left) || is_unknown_type(right)
}

fn is_range_compatible_type(ty: &Type) -> bool {
    matches!(ty, Type::Builtin(BuiltinType::Int))
        || matches!(
            ty,
            Type::App {
                constructor,
                arguments: _
            } if constructor == "Int"
        )
        || is_unknown_type(ty)
}

fn array_element_type(target_ty: &Type) -> Option<Type> {
    match target_ty {
        Type::App {
            constructor,
            arguments,
        } if constructor == "Array" && arguments.len() == 1 => Some(arguments[0].clone()),
        _ => None,
    }
}

fn option_inner_type(target_ty: &Type) -> Option<Type> {
    match target_ty {
        Type::App {
            constructor,
            arguments,
        } if constructor == "Option" && arguments.len() == 1 => Some(arguments[0].clone()),
        _ => None,
    }
}

fn result_inner_types(target_ty: &Type) -> Option<(Type, Type)> {
    match target_ty {
        Type::App {
            constructor,
            arguments,
        } if constructor == "Result" && arguments.len() == 2 => {
            Some((arguments[0].clone(), arguments[1].clone()))
        }
        _ => None,
    }
}

fn pattern_hint_type(pattern: &Pattern) -> Option<Type> {
    match &pattern.kind {
        PatternKind::Literal(literal) => Some(type_for_literal(literal)),
        _ => None,
    }
}

fn int_literal_value(pattern: Option<&Pattern>) -> Option<i64> {
    match pattern {
        Some(Pattern {
            kind:
                PatternKind::Literal(Literal {
                    value: LiteralKind::Int { value, .. },
                }),
            ..
        }) => Some(*value),
        _ => None,
    }
}

fn validate_range_pattern(
    pattern: &Pattern,
    target_ty: &Type,
    violations: &mut Vec<TypecheckViolation>,
) {
    let (start, end) = match &pattern.kind {
        PatternKind::Range { start, end, .. } => (start.as_deref(), end.as_deref()),
        _ => return,
    };
    let start_ty = start.and_then(pattern_hint_type);
    let end_ty = end.and_then(pattern_hint_type);
    let mut type_mismatch = false;
    if !is_range_compatible_type(target_ty) && !is_unknown_type(target_ty) {
        type_mismatch = true;
    }
    if let (Some(start_ty), Some(end_ty)) = (&start_ty, &end_ty) {
        if !types_compatible(start_ty, end_ty)
            || (!is_unknown_type(target_ty) && !types_compatible(start_ty, target_ty))
        {
            type_mismatch = true;
        }
    } else if let Some(start_ty) = &start_ty {
        if !is_unknown_type(target_ty) && !types_compatible(start_ty, target_ty) {
            type_mismatch = true;
        }
    } else if let Some(end_ty) = &end_ty {
        if !is_unknown_type(target_ty) && !types_compatible(end_ty, target_ty) {
            type_mismatch = true;
        }
    }
    if type_mismatch {
        violations.push(TypecheckViolation::pattern_range_type_mismatch(
            pattern.span,
            Some(target_ty.clone()),
            start_ty.clone(),
            end_ty.clone(),
        ));
    }
    if let (Some(start_val), Some(end_val)) = (int_literal_value(start), int_literal_value(end)) {
        if start_val > end_val {
            violations.push(TypecheckViolation::pattern_range_bound_inverted(
                pattern.span,
                start_val,
                end_val,
            ));
        }
    }
}

fn validate_slice_pattern(
    pattern: &Pattern,
    elements: &[SlicePatternItem],
    target_ty: &Type,
    violations: &mut Vec<TypecheckViolation>,
) {
    let rest_count = elements
        .iter()
        .filter(|item| matches!(item, SlicePatternItem::Rest { .. }))
        .count();
    if rest_count > 1 {
        violations.push(TypecheckViolation::pattern_slice_multiple_rest(
            pattern.span,
        ));
    }
    let Some(element_ty) = array_element_type(target_ty) else {
        if !is_unknown_type(target_ty) {
            violations.push(TypecheckViolation::pattern_slice_type_mismatch(
                pattern.span,
                target_ty.label(),
            ));
        }
        return;
    };
    for element in elements {
        if let SlicePatternItem::Element(inner) = element {
            validate_pattern_against_type(inner, &element_ty, violations);
        }
    }
}

fn validate_pattern_against_type(
    pattern: &Pattern,
    target_ty: &Type,
    violations: &mut Vec<TypecheckViolation>,
) {
    match &pattern.kind {
        PatternKind::Or { variants } => {
            for variant in variants {
                validate_pattern_against_type(variant, target_ty, violations);
            }
        }
        PatternKind::Binding { pattern: inner, .. } => {
            validate_pattern_against_type(inner, target_ty, violations);
        }
        PatternKind::Guard { pattern: inner, .. } => {
            validate_pattern_against_type(inner, target_ty, violations);
        }
        PatternKind::Slice { elements } => {
            validate_slice_pattern(pattern, elements, target_ty, violations);
        }
        PatternKind::Range { .. } => {
            validate_range_pattern(pattern, target_ty, violations);
        }
        PatternKind::Constructor { name, args, .. } => {
            if let Some(inner_ty) = option_inner_type(target_ty) {
                match name.name.as_str() {
                    "Some" => {
                        if let Some(arg) = args.get(0) {
                            validate_pattern_against_type(arg, &inner_ty, violations);
                        }
                        return;
                    }
                    "None" => return,
                    _ => {}
                }
            }
            if let Some((ok_ty, err_ty)) = result_inner_types(target_ty) {
                match name.name.as_str() {
                    "Ok" => {
                        if let Some(arg) = args.get(0) {
                            validate_pattern_against_type(arg, &ok_ty, violations);
                        }
                        return;
                    }
                    "Err" => {
                        if let Some(arg) = args.get(0) {
                            validate_pattern_against_type(arg, &err_ty, violations);
                        }
                        return;
                    }
                    _ => {}
                }
            }
            for arg in args {
                validate_pattern_against_type(
                    arg,
                    &Type::builtin(BuiltinType::Unknown),
                    violations,
                );
            }
        }
        PatternKind::Tuple { elements } => {
            for element in elements {
                validate_pattern_against_type(
                    element,
                    &Type::builtin(BuiltinType::Unknown),
                    violations,
                );
            }
        }
        PatternKind::Record { fields, .. } => {
            for field in fields {
                if let Some(value) = &field.value {
                    validate_pattern_against_type(
                        value,
                        &Type::builtin(BuiltinType::Unknown),
                        violations,
                    );
                }
            }
        }
        PatternKind::ActivePattern { argument, .. } => {
            if let Some(argument) = argument {
                validate_pattern_against_type(
                    argument,
                    &Type::builtin(BuiltinType::Unknown),
                    violations,
                );
            }
        }
        PatternKind::Regex { .. }
        | PatternKind::Literal(_)
        | PatternKind::Wildcard
        | PatternKind::Var(_) => {}
    }
}

#[derive(Default)]
struct ExhaustivenessResult {
    coverage_reached: bool,
    unreachable_arm_indices: Vec<usize>,
}

fn analyze_match_exhaustiveness(arms: &[MatchArm]) -> ExhaustivenessResult {
    let mut tracker = ExhaustivenessTracker::default();
    let mut unreachable_arm_indices = Vec::new();
    for (idx, arm) in arms.iter().enumerate() {
        if tracker.coverage_reached() {
            unreachable_arm_indices.push(idx);
            continue;
        }
        tracker.observe_arm(arm);
    }
    ExhaustivenessResult {
        coverage_reached: tracker.coverage_reached(),
        unreachable_arm_indices,
    }
}

#[derive(Default)]
struct ExhaustivenessTracker {
    wildcard_covered: bool,
    bool_true_seen: bool,
    bool_false_seen: bool,
    option_some_seen: bool,
    option_none_seen: bool,
    slice_empty_seen: bool,
    slice_rest_seen: bool,
}

impl ExhaustivenessTracker {
    fn observe_arm(&mut self, arm: &MatchArm) {
        if arm.guard.is_some() {
            return;
        }
        self.observe_pattern(&arm.pattern, false);
    }

    fn observe_pattern(&mut self, pattern: &Pattern, has_guard: bool) {
        if has_guard || self.wildcard_covered {
            return;
        }
        match &pattern.kind {
            PatternKind::Wildcard | PatternKind::Var(_) => {
                self.wildcard_covered = true;
            }
            PatternKind::Binding { pattern: inner, .. } => {
                self.observe_pattern(inner, has_guard);
            }
            PatternKind::Or { variants } => {
                for variant in variants {
                    self.observe_pattern(variant, has_guard);
                }
            }
            PatternKind::Literal(Literal {
                value: LiteralKind::Bool { value },
            }) => {
                if *value {
                    self.bool_true_seen = true;
                } else {
                    self.bool_false_seen = true;
                }
            }
            PatternKind::Constructor { name, .. } => match name.name.as_str() {
                "Some" | "Ok" => self.option_some_seen = true,
                "None" | "Err" => self.option_none_seen = true,
                _ => {}
            },
            PatternKind::ActivePattern { is_partial, .. } => {
                if !*is_partial {
                    self.wildcard_covered = true;
                }
            }
            PatternKind::Guard { pattern, .. } => {
                self.observe_pattern(pattern, true);
            }
            PatternKind::Slice { elements } => {
                let has_rest = elements
                    .iter()
                    .any(|element| matches!(element, SlicePatternItem::Rest { .. }));
                if elements.is_empty() && !has_rest {
                    self.slice_empty_seen = true;
                }
                if has_rest {
                    self.slice_rest_seen = true;
                }
                if self.slice_empty_seen && self.slice_rest_seen {
                    self.wildcard_covered = true;
                }
            }
            PatternKind::Range { start, end, .. } => {
                if start.is_none() && end.is_none() {
                    self.wildcard_covered = true;
                }
            }
            PatternKind::Regex { .. } => {}
            _ => {}
        }
    }

    fn coverage_reached(&self) -> bool {
        self.wildcard_covered
            || (self.bool_true_seen && self.bool_false_seen)
            || (self.option_some_seen && self.option_none_seen)
            || (self.slice_empty_seen && self.slice_rest_seen)
    }
}

fn type_from_annotation_kind(kind: &TypeKind) -> Option<Type> {
    match kind {
        TypeKind::Ident { name } => match name.name.as_str() {
            "Int" => Some(Type::builtin(BuiltinType::Int)),
            "Bool" => Some(Type::builtin(BuiltinType::Bool)),
            "Str" => Some(Type::builtin(BuiltinType::Str)),
            "Bytes" => Some(Type::builtin(BuiltinType::Bytes)),
            _ => None,
        },
        TypeKind::App { callee, args } => {
            let mut resolved_args = Vec::new();
            for arg in args {
                if let Some(arg_ty) = type_from_annotation_kind(&arg.kind) {
                    resolved_args.push(arg_ty);
                } else {
                    return None;
                }
            }
            Some(Type::app(callee.name.clone(), resolved_args))
        }
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
        Literal {
            value: LiteralKind::String { .. },
        } => Type::builtin(BuiltinType::Str),
        Literal {
            value: LiteralKind::Char { .. },
        } => Type::builtin(BuiltinType::Unknown),
        Literal {
            value: LiteralKind::Unit,
        } => Type::builtin(BuiltinType::Unit),
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
    Match {
        target: Box<TypedExprDraft>,
        arms: Vec<TypedMatchArmDraft>,
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

struct TypedMatchArmDraft {
    pattern: typed::TypedPattern,
    guard: Option<TypedExprDraft>,
    alias: Option<String>,
    body: TypedExprDraft,
}

struct DictRefDraft {
    impl_id: String,
    span: Span,
    requirements: Vec<String>,
    ty: Type,
}

struct ParamBinding {
    display: String,
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
        TypedExprKindDraft::Match { target, arms } => typed::TypedExprKind::Match {
            target: Box::new(finalize_typed_expr(*target, substitution)),
            arms: arms
                .into_iter()
                .map(|arm| typed::TypedMatchArm {
                    pattern: arm.pattern,
                    guard: arm
                        .guard
                        .map(|guard| finalize_typed_expr(guard, substitution)),
                    alias: arm.alias,
                    body: finalize_typed_expr(arm.body, substitution),
                })
                .collect(),
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
    if matches!(ty, Type::Builtin(BuiltinType::Bool))
        || matches!(ty, Type::Builtin(BuiltinType::Unknown) | Type::Var(_))
    {
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

fn detect_active_pattern_conflicts(module: &Module) -> Vec<TypecheckViolation> {
    #[derive(Clone, Copy)]
    enum SymbolKind {
        Function,
        ActivePattern,
    }

    impl SymbolKind {
        fn label(&self) -> &'static str {
            match self {
                SymbolKind::Function => "関数",
                SymbolKind::ActivePattern => "Active Pattern",
            }
        }
    }

    let mut registry: HashMap<String, (Span, SymbolKind)> = HashMap::new();
    for function in &module.functions {
        registry.insert(
            function.name.name.clone(),
            (function.span, SymbolKind::Function),
        );
    }
    let mut violations = Vec::new();
    for active in &module.active_patterns {
        if let Some((other_span, kind)) = registry.get(&active.name.name).copied() {
            violations.push(TypecheckViolation::active_pattern_name_conflict(
                active.span,
                active.name.name.as_str(),
                other_span,
                kind.label(),
            ));
        } else {
            registry.insert(
                active.name.name.clone(),
                (active.span, SymbolKind::ActivePattern),
            );
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
        ExprKind::FixityLiteral(_)
        | ExprKind::Identifier(_)
        | ExprKind::ModulePath(_)
        | ExprKind::Continue => {}
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
        ExprKind::Break { value } => {
            if let Some(inner) = value {
                visit_expr(inner, visitor);
            }
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
