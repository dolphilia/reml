use std::collections::{BTreeMap, HashMap, HashSet};

use once_cell::sync::Lazy;
use serde::Serialize;

use super::capability::{CapabilityDescriptor, EffectUsage};
use super::constraint::{
    iterator, Constraint, ConstraintSolver, ConstraintSolverError, Substitution,
};
use super::env::{
    StageRequirement, TypeConstructorBinding, TypeDeclBinding, TypeDeclKind, TypeEnv,
    TypecheckConfig,
};
use super::metrics::TypecheckMetrics;
use super::scheme::Scheme;
use super::types::{BuiltinType, Type, TypeVarGen, TypeVariable};
use crate::diagnostic::{ExpectedToken, ExpectedTokenCollector, ExpectedTokensSummary};
use crate::effects::diagnostics::CapabilityMismatch;
use crate::parser::ast::{
    ActorSpecDecl, Attribute, BinaryOp, ConductorDecl, ConductorMonitorTarget, Decl, DeclKind,
    EffectAnnotation, EffectDecl, EnumDecl, Expr, ExprKind, FixityKind, Function,
    FunctionSignature, HandlerDecl, HandlerEntry, Ident, ImplItem, Literal, LiteralKind, MacroDecl,
    MatchArm, Module, ModuleBody, ModulePath, Param, Pattern, PatternKind, RelativeHead,
    SlicePatternItem, Stmt, StmtKind, StructDecl, TraitDecl, TypeAnnot, TypeDecl, TypeDeclBody,
    TypeDeclVariant, TypeDeclVariantPayload, TypeKind, TypeLiteral, TypeUnionVariant, UnaryOp,
    VariantPayload,
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
    trait_names: &'a HashSet<String>,
}

impl<'a> FunctionContext<'a> {
    fn function(name: &'a str, is_pure: bool, trait_names: &'a HashSet<String>) -> Self {
        Self {
            name: Some(name),
            is_pure,
            kind: ContextKind::Function,
            trait_names,
        }
    }

    fn active_pattern(name: &'a str, is_pure: bool, trait_names: &'a HashSet<String>) -> Self {
        Self {
            name: Some(name),
            is_pure,
            kind: ContextKind::ActivePattern,
            trait_names,
        }
    }

    fn module(trait_names: &'a HashSet<String>) -> Self {
        Self {
            name: None,
            is_pure: false,
            kind: ContextKind::Module,
            trait_names,
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
        let trait_names = collect_trait_names(module);

        register_prelude_type_decls(&mut module_env);
        register_type_decls(&module.decls, &mut module_env);
        validate_type_decl_bodies(&module.decls, &module_env, &mut violations);
        register_function_decls(
            &module.decls,
            &mut module_env,
            &mut var_gen,
            &mut violations,
        );
        let effect_names = collect_effect_names(module);
        validate_handles_attrs(module, &effect_names, &mut violations);
        let (impls, impl_registry_duplicates, impl_registry_unresolved) =
            collect_impl_specs(module);
        collect_opbuilder_violations(module, &mut violations);
        violations.extend(detect_active_pattern_conflicts(module));

        if !module.decls.is_empty() {
            let mut module_decl_stats = FunctionStats::default();
            let mut module_decl_constraints = Vec::new();
            let module_context = FunctionContext::module(&trait_names);
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
            let context =
                FunctionContext::active_pattern(active.name.name.as_str(), is_pure, &trait_names);
            for param in &active.params {
                let ty = param
                    .type_annotation
                    .as_ref()
                    .and_then(|annot| {
                        type_from_annotation(annot, None, &module_env, &mut violations)
                    })
                    .unwrap_or_else(|| var_gen.fresh_type());
                let scheme = Scheme::simple(ty.clone());
                bind_pattern_to_env(&param.pattern, &scheme, &mut env, &mut var_gen);
                param_bindings.push(ParamBinding {
                    display: param.pattern.render(),
                    span: param.span,
                    ty,
                    annotation: param.type_annotation.as_ref().map(|annot| annot.render()),
                });
            }
            stats.local_bindings = collect_function_bindings(&active.params, &active.body);
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
                    annotation: binding.annotation,
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
            let generic_map = build_generic_map(&function.generics, &mut var_gen);
            let generic_map_ref = if generic_map.is_empty() {
                None
            } else {
                Some(&generic_map)
            };
            let is_pure = function.attrs.iter().any(|attr| attr.name.name == "pure");
            let function_context =
                FunctionContext::function(function.name.name.as_str(), is_pure, &trait_names);

            for param in &function.params {
                let ty = param
                    .type_annotation
                    .as_ref()
                    .and_then(|annot| {
                        type_from_annotation(annot, generic_map_ref, &env, &mut violations)
                    })
                    .unwrap_or_else(|| var_gen.fresh_type());
                let scheme = Scheme::simple(ty.clone());
                bind_pattern_to_env(&param.pattern, &scheme, &mut env, &mut var_gen);
                param_bindings.push(ParamBinding {
                    display: param.pattern.render(),
                    span: param.span,
                    ty,
                    annotation: param.type_annotation.as_ref().map(|annot| annot.render()),
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
            if let Some(intrinsic_attr) = extract_intrinsic_attr(&function.attrs) {
                let function_label = Some(function.name.name.clone());
                if !effect_has_native(&function.effect) {
                    violations.push(TypecheckViolation::intrinsic_missing_effect(
                        intrinsic_attr.span,
                        Some(intrinsic_attr.name.clone()),
                        function_label.clone(),
                    ));
                }
                if let Some(invalid_label) =
                    first_intrinsic_invalid_type(&param_bindings, &resolved_return, &substitution)
                {
                    violations.push(TypecheckViolation::intrinsic_invalid_type(
                        intrinsic_attr.span,
                        Some(intrinsic_attr.name),
                        invalid_label,
                        function_label,
                    ));
                }
            }
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
            module_env.insert(function.binding_key(), scheme.clone());

            let typed_params = param_bindings
                .into_iter()
                .map(|binding| typed::TypedParam {
                    name: binding.display,
                    span: binding.span,
                    ty: substitution.apply(&binding.ty).label(),
                    annotation: binding.annotation,
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
                attributes: function_attribute_strings(&function.attrs),
                params: typed_params,
                varargs: false,
                return_type: return_label,
                return_annotation: function.ret_type.as_ref().map(|ty| ty.render()),
                is_async: function.is_async,
                is_unsafe: function.is_unsafe,
                body: typed_body,
                dict_ref_ids,
                scheme_id: Some(scheme_id),
            });
        }

        for decl in &module.decls {
            let DeclKind::Impl(impl_decl) = &decl.kind else {
                continue;
            };
            for item in &impl_decl.items {
                let ImplItem::Function(function) = item else {
                    continue;
                };
                metrics.record_function();
                let mut stats = FunctionStats::default();
                let mut constraints = Vec::new();
                let mut env = module_env.clone();
                let is_pure = function.attrs.iter().any(|attr| attr.name.name == "pure");
                let receiver_generics = collect_type_param_names_from_annotation(&impl_decl.target);
                let mut generic_map = build_generic_map(&function.generics, &mut var_gen);
                for name in receiver_generics {
                    insert_generic(&mut generic_map, name.as_str(), &mut var_gen);
                }
                let generic_map_ref = if generic_map.is_empty() {
                    None
                } else {
                    Some(&generic_map)
                };
                let mut param_bindings = Vec::new();
                for param in &function.params {
                    let ty = param
                        .type_annotation
                        .as_ref()
                        .and_then(|annot| {
                            type_from_annotation(annot, generic_map_ref, &env, &mut violations)
                        })
                        .unwrap_or_else(|| var_gen.fresh_type());
                    let scheme = Scheme::simple(ty.clone());
                    bind_pattern_to_env(&param.pattern, &scheme, &mut env, &mut var_gen);
                    param_bindings.push(ParamBinding {
                        display: param.pattern.render(),
                        span: param.span,
                        ty,
                        annotation: param.type_annotation.as_ref().map(|annot| annot.render()),
                    });
                }
                let method_label = format!("{}.{}", impl_decl.target.render(), function.name.name);
                let function_name =
                    format!("{}__{}", impl_decl.target.render(), function.name.name);
                let function_context =
                    FunctionContext::function(method_label.as_str(), is_pure, &trait_names);
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
                if let Some(intrinsic_attr) = extract_intrinsic_attr(&function.attrs) {
                    let function_label = Some(method_label.clone());
                    if !effect_has_native(&function.effect) {
                        violations.push(TypecheckViolation::intrinsic_missing_effect(
                            intrinsic_attr.span,
                            Some(intrinsic_attr.name.clone()),
                            function_label.clone(),
                        ));
                    }
                    if let Some(invalid_label) = first_intrinsic_invalid_type(
                        &param_bindings,
                        &resolved_return,
                        &substitution,
                    ) {
                        violations.push(TypecheckViolation::intrinsic_invalid_type(
                            intrinsic_attr.span,
                            Some(intrinsic_attr.name),
                            invalid_label,
                            function_label,
                        ));
                    }
                }

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
                module_env.insert(function_name.clone(), scheme.clone());

                let typed_params = param_bindings
                    .into_iter()
                    .map(|binding| typed::TypedParam {
                        name: binding.display,
                        span: binding.span,
                        ty: substitution.apply(&binding.ty).label(),
                        annotation: binding.annotation,
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
                    name: function_name.clone(),
                    param_types: param_type_labels,
                    return_type: return_label.clone(),
                    typed_exprs: stats.typed_exprs,
                    constraints: stats.constraints,
                    unresolved_identifiers: stats.unresolved_identifiers,
                });

                typed_module.functions.push(typed::TypedFunction {
                    name: function_name,
                    span: function.span,
                    attributes: function_attribute_strings(&function.attrs),
                    params: typed_params,
                    varargs: false,
                    return_type: return_label,
                    return_annotation: function.ret_type.as_ref().map(|ty| ty.render()),
                    is_async: function.is_async,
                    is_unsafe: function.is_unsafe,
                    body: typed_body,
                    dict_ref_ids,
                    scheme_id: Some(scheme_id),
                });
            }
        }

        for decl in &module.decls {
            if let DeclKind::Conductor(conductor) = &decl.kind {
                let mut stats = FunctionStats::default();
                let mut constraints = Vec::new();
                let mut env = module_env.clone();
                let mut loop_context = LoopContextStack::default();
                let context = FunctionContext::module(&trait_names);
                let typed_conductor = infer_conductor(
                    conductor,
                    &mut env,
                    &mut var_gen,
                    &mut solver,
                    &mut constraints,
                    &mut stats,
                    &mut metrics,
                    &mut violations,
                    &mut dict_ref_drafts,
                    context,
                    &mut loop_context,
                );
                all_constraints.extend(constraints.drain(..));
                typed_module.conductors.push(typed_conductor);
            }
        }

        let mut actor_specs = Vec::new();
        visit_actor_specs(module, &mut |actor_spec| {
            metrics.record_function();
            let mut stats = FunctionStats::default();
            let mut constraints = Vec::new();
            let mut env = module_env.clone();
            let mut param_bindings = Vec::new();
            let mut loop_context = LoopContextStack::default();
            let context =
                FunctionContext::function(actor_spec.name.name.as_str(), false, &trait_names);
            for param in &actor_spec.params {
                let ty = param
                    .type_annotation
                    .as_ref()
                    .and_then(|annot| type_from_annotation(annot, None, &env, &mut violations))
                    .unwrap_or_else(|| var_gen.fresh_type());
                let scheme = Scheme::simple(ty.clone());
                bind_pattern_to_env(&param.pattern, &scheme, &mut env, &mut var_gen);
                param_bindings.push(ParamBinding {
                    display: param.pattern.render(),
                    span: param.span,
                    ty,
                    annotation: param.type_annotation.as_ref().map(|annot| annot.render()),
                });
            }
            stats.local_bindings = collect_function_bindings(&actor_spec.params, &actor_spec.body);
            let typed_body = infer_expr(
                &actor_spec.body,
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
            let substitution = solver.substitution().clone();
            let resolved_return = substitution.apply(&typed_body.ty);
            let typed_params = param_bindings
                .into_iter()
                .map(|binding| typed::TypedParam {
                    name: binding.display,
                    span: binding.span,
                    ty: substitution.apply(&binding.ty).label(),
                    annotation: binding.annotation,
                })
                .collect::<Vec<_>>();
            let typed_body = finalize_typed_expr(typed_body, &substitution);
            let dict_ref_ids = typed_body.dict_ref_ids.clone();
            actor_specs.push(typed::TypedActorSpec {
                name: actor_spec.name.name.clone(),
                span: actor_spec.span,
                params: typed_params,
                return_type: resolved_return.label(),
                body: typed_body,
                dict_ref_ids,
            });
        });
        typed_module.actor_specs = actor_specs;
        typed_module.externs = collect_externs(module);

        for expr in &module.exprs {
            let mut stats = FunctionStats::default();
            let mut constraints = Vec::new();
            let mut env = module_env.clone();
            let mut loop_context = LoopContextStack::default();
            let context = FunctionContext::module(&trait_names);
            let _ = infer_expr(
                expr,
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
        violations.extend(detect_varargs_violations(module));
        violations.extend(detect_spec_core_runtime_violations(module));
        violations.extend(detect_native_escape_hatch_violations(module));
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
        let mut mir_module = mir::MirModule::from_typed_module(&typed_module);
        mir_module.impls = impls;
        mir_module.impl_registry_duplicates = impl_registry_duplicates;
        mir_module.impl_registry_unresolved = impl_registry_unresolved;
        populate_qualified_call_candidates(&mut mir_module);
        let qualified_call_table = mir_module.qualified_calls.clone();

        TypecheckReport {
            metrics,
            functions,
            violations,
            typed_module,
            mir: mir_module,
            constraints: all_constraints,
            used_impls,
            qualified_call_table,
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
    pub qualified_call_table: BTreeMap<String, mir::MirQualifiedCall>,
}

static TOP_LEVEL_DECLARATION_SUMMARY: Lazy<ExpectedTokensSummary> = Lazy::new(|| {
    let mut collector = ExpectedTokenCollector::new();
    collector.extend([
        ExpectedToken::keyword("async"),
        ExpectedToken::keyword("conductor"),
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
    pub recover: Option<TypecheckRecoverHint>,
    #[serde(skip_serializing)]
    pub iterator_stage: Option<IteratorStageViolationInfo>,
    #[serde(skip_serializing)]
    pub capability_mismatch: Option<CapabilityMismatch>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern_missing_variants: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern_missing_ranges: Option<Vec<PatternRangeInfo>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern_range: Option<PatternRangeInfo>,
}

#[derive(Debug, Serialize, Clone)]
pub struct PatternRangeInfo {
    pub start: String,
    pub end: String,
    pub inclusive: bool,
}

#[derive(Debug, Serialize, Clone)]
pub struct TypecheckRecoverHint {
    pub mode: Option<String>,
    pub action: Option<String>,
    pub sync: Option<String>,
    pub context: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub enum TypecheckViolationKind {
    ConditionLiteralBool,
    AstUnavailable,
    ReturnConflict,
    UnicodeShadowing,
    ResidualLeak,
    HandlesUnknownEffect,
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
    RecursionInfinite,
    PurityViolation,
    ImplDuplicate,
    CoreParseRecoverBranch,
    RuntimeBridgeStageMismatch,
    IteratorExpected,
    ControlFlowUnreachable,
    OpBuilderLevelConflict,
    OpBuilderFixityMissing,
    IntrinsicMissingEffect,
    IntrinsicInvalidType,
    ConductorDslIdDuplicate,
    VarargsInvalidAbi,
    VarargsMissingFixedParam,
    UnsafeInPureContext,
    NativeInlineAsmMissingEffect,
    NativeInlineAsmMissingCfg,
    NativeInlineAsmInvalidType,
    NativeLlvmIrMissingEffect,
    NativeLlvmIrMissingCfg,
    NativeLlvmIrInvalidType,
    LambdaCaptureUnsupported,
    LambdaCaptureMutUnsupported,
    RecUnresolvedIdent,
    TypeUnresolvedIdent,
    TypeAliasCycle,
    TypeAliasExpansionLimit,
    ConstructorArityMismatch,
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
            recover: None,
            iterator_stage: None,
            capability_mismatch: None,
            pattern_missing_variants: None,
            pattern_missing_ranges: None,
            pattern_range: None,
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
            recover: None,
            iterator_stage: None,
            capability_mismatch: None,
            pattern_missing_variants: None,
            pattern_missing_ranges: None,
            pattern_range: None,
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
            recover: None,
            iterator_stage: None,
            capability_mismatch: None,
            pattern_missing_variants: None,
            pattern_missing_ranges: None,
            pattern_range: None,
        }
    }

    fn handles_unknown_effect(span: Span, effect: String, owner: Option<String>) -> Self {
        let mut notes = vec![ViolationNote::plain(format!(
            "`effect {}` を宣言するか、既存の効果名を指定してください。",
            effect
        ))];
        if let Some(owner) = owner {
            notes.push(ViolationNote::plain(format!("対象: {owner}")));
        }
        Self {
            kind: TypecheckViolationKind::HandlesUnknownEffect,
            code: "effects.handles.undefined",
            message: format!("`@handles` で指定された効果 `{effect}` が見つかりません。"),
            span: Some(span),
            notes,
            capability: None,
            function: None,
            expected: None,
            recover: None,
            iterator_stage: None,
            capability_mismatch: None,
            pattern_missing_variants: None,
            pattern_missing_ranges: None,
            pattern_range: None,
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
            recover: None,
            iterator_stage: None,
            capability_mismatch: None,
            pattern_missing_variants: None,
            pattern_missing_ranges: None,
            pattern_range: None,
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
            recover: None,
            iterator_stage: None,
            capability_mismatch: Some(CapabilityMismatch::new(
                capability_label,
                required.clone(),
                actual.clone(),
            )),
            pattern_missing_variants: None,
            pattern_missing_ranges: None,
            pattern_range: None,
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
            recover: None,
            iterator_stage: Some(IteratorStageViolationInfo {
                required: snapshot.required.clone(),
                actual,
                capability: Some(capability),
                kind: kind_label.clone(),
                source: snapshot.source,
            }),
            capability_mismatch: None,
            pattern_missing_variants: None,
            pattern_missing_ranges: None,
            pattern_range: None,
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
            recover: None,
            iterator_stage: None,
            capability_mismatch: None,
            pattern_missing_variants: None,
            pattern_missing_ranges: None,
            pattern_range: None,
        }
    }

    fn recursion_infinite(span: Span, binding: &str) -> Self {
        Self {
            kind: TypecheckViolationKind::RecursionInfinite,
            code: "core.parse.recursion.infinite",
            message: "rec による再帰参照が無限再帰になります".to_string(),
            span: Some(span),
            notes: vec![ViolationNote::plain(format!(
                "`{binding}` は rec で自身を直接参照しています"
            ))],
            capability: None,
            function: None,
            expected: None,
            recover: None,
            iterator_stage: None,
            capability_mismatch: None,
            pattern_missing_variants: None,
            pattern_missing_ranges: None,
            pattern_range: None,
        }
    }

    fn for_iterator_expected(span: Span, actual: Type) -> Self {
        Self {
            kind: TypecheckViolationKind::IteratorExpected,
            code: "language.iterator.expected",
            message: "for 式の `in` 右辺は [T] などのイテレータである必要があります".to_string(),
            span: Some(span),
            notes: vec![ViolationNote::plain(format!(
                "実際の型: {}",
                actual.label()
            ))],
            capability: None,
            function: None,
            expected: None,
            recover: None,
            iterator_stage: None,
            capability_mismatch: None,
            pattern_missing_variants: None,
            pattern_missing_ranges: None,
            pattern_range: None,
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
            recover: None,
            iterator_stage: None,
            capability_mismatch: None,
            pattern_missing_variants: None,
            pattern_missing_ranges: None,
            pattern_range: None,
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
            recover: None,
            iterator_stage: None,
            capability_mismatch: None,
            pattern_missing_variants: None,
            pattern_missing_ranges: None,
            pattern_range: None,
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
            recover: None,
            iterator_stage: None,
            capability_mismatch: None,
            pattern_missing_variants: None,
            pattern_missing_ranges: None,
            pattern_range: None,
        }
    }

    fn intrinsic_missing_effect(
        span: Span,
        name: Option<String>,
        function: Option<String>,
    ) -> Self {
        let label = name.unwrap_or_else(|| "unknown".to_string());
        let message = format!(
            "`@intrinsic` 関数 `{}` は `!{{native}}` を必ず指定する必要があります。",
            label
        );
        Self {
            kind: TypecheckViolationKind::IntrinsicMissingEffect,
            code: "native.intrinsic.missing_effect",
            message,
            span: Some(span),
            notes: vec![ViolationNote::plain(
                "例: `fn sqrt(x: Int) !{native} = ...` のように効果注釈を追加してください。",
            )],
            capability: None,
            function,
            expected: None,
            recover: None,
            iterator_stage: None,
            capability_mismatch: None,
            pattern_missing_variants: None,
            pattern_missing_ranges: None,
            pattern_range: None,
        }
    }

    fn intrinsic_invalid_type(
        span: Span,
        name: Option<String>,
        ty_label: String,
        function: Option<String>,
    ) -> Self {
        let label = name.unwrap_or_else(|| "unknown".to_string());
        let message = format!(
            "`@intrinsic` 関数 `{}` の型 `{}` は ABI 安全/Copy 制約に違反しています。",
            label, ty_label
        );
        Self {
            kind: TypecheckViolationKind::IntrinsicInvalidType,
            code: "native.intrinsic.invalid_type",
            message,
            span: Some(span),
            notes: vec![ViolationNote::plain(
                "Int/Bool/Unit およびそれらのみで構成される Tuple のみ許可されます。",
            )],
            capability: None,
            function,
            expected: None,
            recover: None,
            iterator_stage: None,
            capability_mismatch: None,
            pattern_missing_variants: None,
            pattern_missing_ranges: None,
            pattern_range: None,
        }
    }

    fn inline_asm_missing_effect(span: Span, function: Option<String>) -> Self {
        let label = function.unwrap_or_else(|| "unknown".to_string());
        let message = format!(
            "`inline_asm` を含む関数 `{}` は `!{{native}}` を必ず指定する必要があります。",
            label
        );
        Self {
            kind: TypecheckViolationKind::NativeInlineAsmMissingEffect,
            code: "native.inline_asm.missing_effect",
            message,
            span: Some(span),
            notes: vec![ViolationNote::plain(
                "例: `fn read() !{native} = unsafe { inline_asm(...) }` のように効果注釈を追加してください。",
            )],
            capability: None,
            function: Some(label),
            expected: None,
            recover: None,
            iterator_stage: None,
            capability_mismatch: None,
            pattern_missing_variants: None,
            pattern_missing_ranges: None,
            pattern_range: None,
        }
    }

    fn llvm_ir_missing_effect(span: Span, function: Option<String>) -> Self {
        let label = function.unwrap_or_else(|| "unknown".to_string());
        let message = format!(
            "`llvm_ir!` を含む関数 `{}` は `!{{native}}` を必ず指定する必要があります。",
            label
        );
        Self {
            kind: TypecheckViolationKind::NativeLlvmIrMissingEffect,
            code: "native.llvm_ir.missing_effect",
            message,
            span: Some(span),
            notes: vec![ViolationNote::plain(
                "例: `fn add() !{native} = unsafe { llvm_ir!(Int) { ... } }` のように効果注釈を追加してください。",
            )],
            capability: None,
            function: Some(label),
            expected: None,
            recover: None,
            iterator_stage: None,
            capability_mismatch: None,
            pattern_missing_variants: None,
            pattern_missing_ranges: None,
            pattern_range: None,
        }
    }

    fn inline_asm_missing_cfg(span: Span, function: Option<String>) -> Self {
        let message =
            "`inline_asm` は `@cfg(target_...)` によるターゲット限定が必須です。".to_string();
        Self {
            kind: TypecheckViolationKind::NativeInlineAsmMissingCfg,
            code: "native.inline_asm.missing_cfg",
            message,
            span: Some(span),
            notes: vec![ViolationNote::plain(
                "例: `@cfg(target_arch = \"x86_64\")` を併用してください。",
            )],
            capability: None,
            function,
            expected: None,
            recover: None,
            iterator_stage: None,
            capability_mismatch: None,
            pattern_missing_variants: None,
            pattern_missing_ranges: None,
            pattern_range: None,
        }
    }

    fn llvm_ir_missing_cfg(span: Span, function: Option<String>) -> Self {
        let message =
            "`llvm_ir!` は `@cfg(target_...)` によるターゲット限定が必須です。".to_string();
        Self {
            kind: TypecheckViolationKind::NativeLlvmIrMissingCfg,
            code: "native.llvm_ir.missing_cfg",
            message,
            span: Some(span),
            notes: vec![ViolationNote::plain(
                "例: `@cfg(target_arch = \"x86_64\")` を併用してください。",
            )],
            capability: None,
            function,
            expected: None,
            recover: None,
            iterator_stage: None,
            capability_mismatch: None,
            pattern_missing_variants: None,
            pattern_missing_ranges: None,
            pattern_range: None,
        }
    }

    fn inline_asm_invalid_type(span: Span, ty_label: String, function: Option<String>) -> Self {
        let message = format!(
            "`inline_asm` の型 `{}` は ABI 安全/Copy 制約に違反しています。",
            ty_label
        );
        Self {
            kind: TypecheckViolationKind::NativeInlineAsmInvalidType,
            code: "native.inline_asm.invalid_type",
            message,
            span: Some(span),
            notes: vec![ViolationNote::plain(
                "Int/UInt/Float/Bool/Char/Unit と Ptr/&T、それらのみで構成される Tuple のみ許可されます。",
            )],
            capability: None,
            function,
            expected: None,
            recover: None,
            iterator_stage: None,
            capability_mismatch: None,
            pattern_missing_variants: None,
            pattern_missing_ranges: None,
            pattern_range: None,
        }
    }

    fn llvm_ir_invalid_type(span: Span, ty_label: String, function: Option<String>) -> Self {
        let message = format!(
            "`llvm_ir!` の型 `{}` は ABI 安全/Copy 制約に違反しています。",
            ty_label
        );
        Self {
            kind: TypecheckViolationKind::NativeLlvmIrInvalidType,
            code: "native.llvm_ir.invalid_type",
            message,
            span: Some(span),
            notes: vec![ViolationNote::plain(
                "Int/UInt/Float/Bool/Char/Unit と Ptr/&T、それらのみで構成される Tuple のみ許可されます。",
            )],
            capability: None,
            function,
            expected: None,
            recover: None,
            iterator_stage: None,
            capability_mismatch: None,
            pattern_missing_variants: None,
            pattern_missing_ranges: None,
            pattern_range: None,
        }
    }

    fn conductor_dsl_id_duplicate(span: Span, name: &str) -> Self {
        Self {
            kind: TypecheckViolationKind::ConductorDslIdDuplicate,
            code: "conductor.dsl_id.duplicate",
            message: "conductor 内で dsl_id が重複しています".to_string(),
            span: Some(span),
            notes: vec![ViolationNote::plain(format!(
                "`{name}` が複数回宣言されています。dsl_id を一意にしてください。"
            ))],
            capability: None,
            function: None,
            expected: None,
            recover: None,
            iterator_stage: None,
            capability_mismatch: None,
            pattern_missing_variants: None,
            pattern_missing_ranges: None,
            pattern_range: None,
        }
    }

    fn varargs_invalid_abi(span: Span, function: &str, abi: &str) -> Self {
        Self {
            kind: TypecheckViolationKind::VarargsInvalidAbi,
            code: "ffi.varargs.invalid_abi",
            message: "可変長引数は extern \"C\" でのみ使用できます".to_string(),
            span: Some(span),
            notes: vec![ViolationNote::plain(format!(
                "`{function}` は ABI \"{abi}\" で宣言されています。"
            ))],
            capability: None,
            function: Some(function.to_string()),
            expected: None,
            recover: None,
            iterator_stage: None,
            capability_mismatch: None,
            pattern_missing_variants: None,
            pattern_missing_ranges: None,
            pattern_range: None,
        }
    }

    fn varargs_missing_fixed_param(span: Span, function: &str) -> Self {
        Self {
            kind: TypecheckViolationKind::VarargsMissingFixedParam,
            code: "ffi.varargs.missing_fixed_param",
            message: "可変長引数の前に固定引数が必要です".to_string(),
            span: Some(span),
            notes: vec![ViolationNote::plain(format!(
                "`{function}` の可変長引数に最低 1 つの固定引数を指定してください。"
            ))],
            capability: None,
            function: Some(function.to_string()),
            expected: None,
            recover: None,
            iterator_stage: None,
            capability_mismatch: None,
            pattern_missing_variants: None,
            pattern_missing_ranges: None,
            pattern_range: None,
        }
    }

    fn unsafe_in_pure_context(span: Span, function: Option<String>) -> Self {
        let message = match function.as_ref() {
            Some(name) => format!("`@pure` 関数 `{name}` で `unsafe` が検出されました。"),
            None => "`@pure` ブロックで `unsafe` が検出されました。".to_string(),
        };
        Self {
            kind: TypecheckViolationKind::UnsafeInPureContext,
            code: "effects.unsafe.pure_violation",
            message,
            span: Some(span),
            notes: vec![ViolationNote::plain(
                "`unsafe` を取り除くか、`@pure` を外してください。",
            )],
            capability: None,
            function,
            expected: None,
            recover: None,
            iterator_stage: None,
            capability_mismatch: None,
            pattern_missing_variants: None,
            pattern_missing_ranges: None,
            pattern_range: None,
        }
    }

    fn lambda_capture_unsupported(span: Span, name: &str) -> Self {
        Self {
            kind: TypecheckViolationKind::LambdaCaptureUnsupported,
            code: "typeck.lambda.capture_unsupported",
            message: "キャプチャ付きラムダは未実装です".to_string(),
            span: Some(span),
            notes: vec![ViolationNote::plain(format!(
                "`{name}` は外側の束縛です。引数として渡してください。"
            ))],
            capability: None,
            function: None,
            expected: None,
            recover: None,
            iterator_stage: None,
            capability_mismatch: None,
            pattern_missing_variants: None,
            pattern_missing_ranges: None,
            pattern_range: None,
        }
    }

    fn lambda_capture_mut_unsupported(span: Span, name: &str) -> Self {
        Self {
            kind: TypecheckViolationKind::LambdaCaptureMutUnsupported,
            code: "typeck.lambda.capture_mut_unsupported",
            message: "可変キャプチャは未実装です".to_string(),
            span: Some(span),
            notes: vec![ViolationNote::plain(format!(
                "`{name}` を更新する場合は明示的に引数で渡してください。"
            ))],
            capability: None,
            function: None,
            expected: None,
            recover: None,
            iterator_stage: None,
            capability_mismatch: None,
            pattern_missing_variants: None,
            pattern_missing_ranges: None,
            pattern_range: None,
        }
    }

    fn rec_unresolved_ident(span: Span, name: &str) -> Self {
        Self {
            kind: TypecheckViolationKind::RecUnresolvedIdent,
            code: "typeck.rec.unresolved_ident",
            message: "`rec` 参照が未解決です".to_string(),
            span: Some(span),
            notes: vec![ViolationNote::plain(format!(
                "`rec {name}` の参照先が見つかりません。"
            ))],
            capability: None,
            function: None,
            expected: None,
            recover: None,
            iterator_stage: None,
            capability_mismatch: None,
            pattern_missing_variants: None,
            pattern_missing_ranges: None,
            pattern_range: None,
        }
    }

    fn type_unresolved_ident(span: Span, name: &str) -> Self {
        Self {
            kind: TypecheckViolationKind::TypeUnresolvedIdent,
            code: "type.unresolved_ident",
            message: "型参照が未解決です".to_string(),
            span: Some(span),
            notes: vec![ViolationNote::plain(format!(
                "`{name}` に対応する型宣言が見つかりません。"
            ))],
            capability: None,
            function: None,
            expected: None,
            recover: None,
            iterator_stage: None,
            capability_mismatch: None,
            pattern_missing_variants: None,
            pattern_missing_ranges: None,
            pattern_range: None,
        }
    }

    fn type_alias_cycle(span: Span, chain: Vec<String>) -> Self {
        let chain_label = chain.join(" -> ");
        Self {
            kind: TypecheckViolationKind::TypeAliasCycle,
            code: "type.alias.cycle",
            message: "型エイリアスが循環参照しています".to_string(),
            span: Some(span),
            notes: vec![ViolationNote::plain(format!("循環経路: {chain_label}"))],
            capability: None,
            function: None,
            expected: None,
            recover: None,
            iterator_stage: None,
            capability_mismatch: None,
            pattern_missing_variants: None,
            pattern_missing_ranges: None,
            pattern_range: None,
        }
    }

    fn type_alias_expansion_limit(span: Span, name: &str, limit: usize) -> Self {
        Self {
            kind: TypecheckViolationKind::TypeAliasExpansionLimit,
            code: "type.alias.expansion_limit",
            message: "型エイリアスの展開が上限に達しました".to_string(),
            span: Some(span),
            notes: vec![ViolationNote::plain(format!(
                "`{name}` の展開が {limit} 回を超えています"
            ))],
            capability: None,
            function: None,
            expected: None,
            recover: None,
            iterator_stage: None,
            capability_mismatch: None,
            pattern_missing_variants: None,
            pattern_missing_ranges: None,
            pattern_range: None,
        }
    }

    fn constructor_arity_mismatch(span: Span, name: &str, expected: usize, actual: usize) -> Self {
        Self {
            kind: TypecheckViolationKind::ConstructorArityMismatch,
            code: "type.sum.constructor_arity_mismatch",
            message: "合成型コンストラクタの引数数が一致しません".to_string(),
            span: Some(span),
            notes: vec![ViolationNote::plain(format!(
                "`{name}` は {expected} 個の引数を受け取りますが、{actual} 個が渡されました"
            ))],
            capability: None,
            function: None,
            expected: None,
            recover: None,
            iterator_stage: None,
            capability_mismatch: None,
            pattern_missing_variants: None,
            pattern_missing_ranges: None,
            pattern_range: None,
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
            recover: None,
            iterator_stage: None,
            capability_mismatch: None,
            pattern_missing_variants: None,
            pattern_missing_ranges: None,
            pattern_range: None,
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
            recover: None,
            iterator_stage: None,
            capability_mismatch: None,
            pattern_missing_variants: None,
            pattern_missing_ranges: None,
            pattern_range: None,
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
            recover: None,
            iterator_stage: None,
            capability_mismatch: None,
            pattern_missing_variants: None,
            pattern_missing_ranges: None,
            pattern_range: None,
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
            recover: None,
            iterator_stage: None,
            capability_mismatch: None,
            pattern_missing_variants: None,
            pattern_missing_ranges: None,
            pattern_range: None,
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
            recover: None,
            iterator_stage: None,
            capability_mismatch: None,
            pattern_missing_variants: None,
            pattern_missing_ranges: None,
            pattern_range: None,
        }
    }

    fn pattern_range_bound_inverted(
        span: Span,
        start: IntLiteralInfo,
        end: IntLiteralInfo,
        inclusive: bool,
    ) -> Self {
        let pattern_range = PatternRangeInfo {
            start: start.raw,
            end: end.raw,
            inclusive,
        };
        Self {
            kind: TypecheckViolationKind::PatternRangeBoundInverted,
            code: "pattern.range.bound_inverted",
            message: "範囲パターンの下限と上限が逆転しています。".to_string(),
            span: Some(span),
            notes: vec![ViolationNote::plain(format!(
                "開始: {} / 終了: {}",
                pattern_range.start, pattern_range.end
            ))],
            capability: None,
            function: None,
            expected: None,
            recover: None,
            iterator_stage: None,
            capability_mismatch: None,
            pattern_missing_variants: None,
            pattern_missing_ranges: None,
            pattern_range: Some(pattern_range),
        }
    }

    fn pattern_slice_type_mismatch(span: Span, actual: String) -> Self {
        Self {
            kind: TypecheckViolationKind::PatternSliceTypeMismatch,
            code: "pattern.slice.type_mismatch",
            message: "スライスパターンは [T] など反復可能な型にのみ適用できます。".to_string(),
            span: Some(span),
            notes: vec![ViolationNote::plain(format!("対象の型: {actual}"))],
            capability: None,
            function: None,
            expected: None,
            recover: None,
            iterator_stage: None,
            capability_mismatch: None,
            pattern_missing_variants: None,
            pattern_missing_ranges: None,
            pattern_range: None,
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
            recover: None,
            iterator_stage: None,
            capability_mismatch: None,
            pattern_missing_variants: None,
            pattern_missing_ranges: None,
            pattern_range: None,
        }
    }

    fn pattern_exhaustiveness_missing(
        span: Span,
        missing_variants: Option<Vec<String>>,
        missing_ranges: Option<Vec<PatternRangeInfo>>,
    ) -> Self {
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
            recover: None,
            iterator_stage: None,
            capability_mismatch: None,
            pattern_missing_variants: missing_variants,
            pattern_missing_ranges: missing_ranges,
            pattern_range: None,
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
            recover: None,
            iterator_stage: None,
            capability_mismatch: None,
            pattern_missing_variants: None,
            pattern_missing_ranges: None,
            pattern_range: None,
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
            recover: None,
            iterator_stage: None,
            capability_mismatch: None,
            pattern_missing_variants: None,
            pattern_missing_ranges: None,
            pattern_range: None,
        }
    }

    fn core_parse_recover_branch(span: Span, recover: Option<TypecheckRecoverHint>) -> Self {
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
            recover,
            iterator_stage: None,
            capability_mismatch: None,
            pattern_missing_variants: None,
            pattern_missing_ranges: None,
            pattern_range: None,
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
            recover: None,
            iterator_stage: None,
            capability_mismatch: None,
            pattern_missing_variants: None,
            pattern_missing_ranges: None,
            pattern_range: None,
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
            recover: None,
            iterator_stage: None,
            capability_mismatch: None,
            pattern_missing_variants: None,
            pattern_missing_ranges: None,
            pattern_range: None,
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
            recover: None,
            iterator_stage: None,
            capability_mismatch: None,
            pattern_missing_variants: None,
            pattern_missing_ranges: None,
            pattern_range: None,
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
            recover: None,
            iterator_stage: None,
            capability_mismatch: None,
            pattern_missing_variants: None,
            pattern_missing_ranges: None,
            pattern_range: None,
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
            | TypecheckViolationKind::PatternBindingDuplicate
            | TypecheckViolationKind::PatternRegexUnsupportedTarget
            | TypecheckViolationKind::PatternSliceTypeMismatch
            | TypecheckViolationKind::PatternSliceMultipleRest
            | TypecheckViolationKind::ValueRestriction
            | TypecheckViolationKind::ImplDuplicate
            | TypecheckViolationKind::IteratorExpected
            | TypecheckViolationKind::ControlFlowUnreachable
            | TypecheckViolationKind::IntrinsicInvalidType
            | TypecheckViolationKind::NativeInlineAsmInvalidType
            | TypecheckViolationKind::NativeLlvmIrInvalidType
            | TypecheckViolationKind::ConductorDslIdDuplicate
            | TypecheckViolationKind::VarargsInvalidAbi
            | TypecheckViolationKind::VarargsMissingFixedParam
            | TypecheckViolationKind::RecursionInfinite
            | TypecheckViolationKind::LambdaCaptureUnsupported
            | TypecheckViolationKind::LambdaCaptureMutUnsupported
            | TypecheckViolationKind::RecUnresolvedIdent
            | TypecheckViolationKind::TypeUnresolvedIdent
            | TypecheckViolationKind::TypeAliasCycle
            | TypecheckViolationKind::TypeAliasExpansionLimit
            | TypecheckViolationKind::ConstructorArityMismatch => "type",
            TypecheckViolationKind::ResidualLeak
            | TypecheckViolationKind::StageMismatch
            | TypecheckViolationKind::IteratorStageMismatch
            | TypecheckViolationKind::PurityViolation
            | TypecheckViolationKind::HandlesUnknownEffect
            | TypecheckViolationKind::ActivePatternEffectViolation
            | TypecheckViolationKind::IntrinsicMissingEffect
            | TypecheckViolationKind::NativeInlineAsmMissingEffect
            | TypecheckViolationKind::NativeInlineAsmMissingCfg
            | TypecheckViolationKind::NativeLlvmIrMissingEffect
            | TypecheckViolationKind::NativeLlvmIrMissingCfg
            | TypecheckViolationKind::UnsafeInPureContext => "effects",
            TypecheckViolationKind::CoreParseRecoverBranch
            | TypecheckViolationKind::PatternExhaustivenessMissing
            | TypecheckViolationKind::PatternUnreachableArm
            | TypecheckViolationKind::PatternRangeTypeMismatch
            | TypecheckViolationKind::PatternRangeBoundInverted
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

    pub fn recover_hint(&self) -> Option<&TypecheckRecoverHint> {
        self.recover.as_ref()
    }
}

#[derive(Default)]
struct FunctionStats {
    typed_exprs: usize,
    constraints: usize,
    unresolved_identifiers: usize,
    local_bindings: HashSet<String>,
}

fn register_prelude_type_decls(env: &mut TypeEnv) {
    let span = Span::default();
    let entries: &[(&str, &[&str])] = &[
        ("Option", &["T"]),
        ("Result", &["T", "E"]),
        ("List", &["T"]),
        ("Iter", &["T"]),
        ("Vec", &["T"]),
        ("Map", &["K", "V"]),
        ("Set", &["T"]),
        ("String", &[]),
        ("Diagnostic", &[]),
        ("MemoryError", &[]),
        ("CollectError", &[]),
        ("StringError", &[]),
        ("AsyncStream", &["T"]),
        ("Future", &["T"]),
        ("Range", &["T"]),
        ("Histogram", &[]),
        ("Collector", &["T", "C"]),
    ];
    for (name, generics) in entries {
        let generics = generics.iter().map(|value| (*value).to_string()).collect();
        env.insert_type_decl(TypeDeclBinding::new(
            *name,
            generics,
            TypeDeclKind::Opaque,
            None,
            span,
            None,
        ));
    }
}

fn register_type_decls(decls: &[Decl], env: &mut TypeEnv) {
    for decl in decls {
        match &decl.kind {
            DeclKind::Type { decl } => {
                let kind = match decl.body {
                    Some(TypeDeclBody::Alias { .. }) => TypeDeclKind::Alias,
                    Some(TypeDeclBody::Newtype { .. }) => TypeDeclKind::Newtype,
                    Some(TypeDeclBody::Sum { .. }) => TypeDeclKind::Sum,
                    None => TypeDeclKind::Opaque,
                };
                let generics = decl
                    .generics
                    .iter()
                    .map(|ident| ident.name.clone())
                    .collect();
                let binding = TypeDeclBinding::new(
                    decl.name.name.clone(),
                    generics,
                    kind,
                    decl.body.clone(),
                    decl.span,
                    decl.body_span,
                );
                env.insert_type_decl(binding);
                if let Some(TypeDeclBody::Sum { variants }) = &decl.body {
                    let generics = decl
                        .generics
                        .iter()
                        .map(|ident| ident.name.clone())
                        .collect::<Vec<_>>();
                    for variant in variants {
                        let ctor_binding = TypeConstructorBinding::new(
                            variant.name.name.clone(),
                            decl.name.name.clone(),
                            generics.clone(),
                            variant.payload.clone(),
                            variant.span,
                        );
                        env.insert_type_constructor(ctor_binding);
                    }
                }
            }
            DeclKind::Struct(struct_decl) => {
                let generics = struct_decl
                    .generics
                    .iter()
                    .map(|ident| ident.name.clone())
                    .collect();
                let binding = TypeDeclBinding::new(
                    struct_decl.name.name.clone(),
                    generics,
                    TypeDeclKind::Opaque,
                    None,
                    struct_decl.span,
                    None,
                );
                env.insert_type_decl(binding);
            }
            DeclKind::Enum(enum_decl) => register_enum_decl(enum_decl, env),
            _ => continue,
        }
    }
}

fn register_enum_decl(enum_decl: &EnumDecl, env: &mut TypeEnv) {
    let generics = enum_decl
        .generics
        .iter()
        .map(|ident| ident.name.clone())
        .collect::<Vec<_>>();
    let variants = enum_decl_to_sum_variants(enum_decl);
    let binding = TypeDeclBinding::new(
        enum_decl.name.name.clone(),
        generics.clone(),
        TypeDeclKind::Sum,
        Some(TypeDeclBody::Sum {
            variants: variants.clone(),
        }),
        enum_decl.span,
        None,
    );
    env.insert_type_decl(binding);
    for variant in &variants {
        let ctor_binding = TypeConstructorBinding::new(
            variant.name.name.clone(),
            enum_decl.name.name.clone(),
            generics.clone(),
            variant.payload.clone(),
            variant.span,
        );
        env.insert_type_constructor(ctor_binding);
    }
}

fn enum_decl_to_sum_variants(enum_decl: &EnumDecl) -> Vec<TypeDeclVariant> {
    enum_decl
        .variants
        .iter()
        .map(|variant| TypeDeclVariant {
            name: variant.name.clone(),
            payload: variant
                .payload
                .as_ref()
                .map(enum_variant_payload_to_type_decl_payload),
            span: variant.span,
        })
        .collect()
}

fn enum_variant_payload_to_type_decl_payload(payload: &VariantPayload) -> TypeDeclVariantPayload {
    match payload {
        VariantPayload::Record { fields } => TypeDeclVariantPayload::Record {
            fields: fields.clone(),
            has_rest: false,
        },
        VariantPayload::Tuple { elements } => TypeDeclVariantPayload::Tuple {
            elements: elements.clone(),
        },
    }
}

fn register_function_decls(
    decls: &[Decl],
    env: &mut TypeEnv,
    var_gen: &mut TypeVarGen,
    violations: &mut Vec<TypecheckViolation>,
) {
    for decl in decls {
        let DeclKind::Fn { signature } = &decl.kind else {
            continue;
        };
        let generic_map = build_generic_map(&signature.generics, var_gen);
        let generic_map_ref = if generic_map.is_empty() {
            None
        } else {
            Some(&generic_map)
        };
        let mut param_types = Vec::new();
        for param in &signature.params {
            let ty = param
                .type_annotation
                .as_ref()
                .and_then(|annot| type_from_annotation(annot, generic_map_ref, env, violations))
                .unwrap_or_else(|| var_gen.fresh_type());
            param_types.push(ty);
        }
        let mut ret_type = signature
            .ret_type
            .as_ref()
            .and_then(|annot| type_from_annotation(annot, generic_map_ref, env, violations))
            .unwrap_or_else(|| var_gen.fresh_type());
        if signature.is_async {
            ret_type = future_type(ret_type);
        }
        let function_type = Type::arrow(param_types, ret_type);
        let scheme = generalize_type(env, function_type);
        env.insert(signature.binding_key(), scheme);
    }
}

fn collect_impl_specs(
    module: &Module,
) -> (BTreeMap<String, mir::MirImplSpec>, Vec<String>, Vec<String>) {
    let mut impls = BTreeMap::new();
    let mut duplicates = Vec::new();
    let mut unresolved = Vec::new();
    for decl in &module.decls {
        let DeclKind::Impl(impl_decl) = &decl.kind else {
            continue;
        };
        let trait_name = impl_decl
            .trait_ref
            .as_ref()
            .map(|trait_ref| trait_ref.name.name.clone());
        let target = impl_decl.target.render();
        let resolved_target = if target.trim().is_empty() {
            unresolved.push("<unknown>".to_string());
            "<unknown>".to_string()
        } else {
            target.clone()
        };
        let impl_id = if let Some(name) = &trait_name {
            format!("{name}::{resolved_target}")
        } else {
            resolved_target.clone()
        };
        let mut associated_types = Vec::new();
        let mut methods = Vec::new();
        for item in &impl_decl.items {
            match item {
                ImplItem::Function(function) => {
                    methods.push(function.name.name.clone());
                }
                ImplItem::Decl(decl) => match &decl.kind {
                    DeclKind::Type { decl } => {
                        if let Some(assoc) = associated_type_from_decl(decl) {
                            associated_types.push(assoc);
                        }
                    }
                    DeclKind::Fn { signature } => {
                        methods.push(signature.name.name.clone());
                    }
                    _ => {}
                },
            }
        }
        let entry = mir::MirImplSpec {
            trait_name: trait_name.clone(),
            target,
            associated_types,
            methods,
            span: Some(impl_decl.span),
        };
        if impls.insert(impl_id.clone(), entry).is_some() {
            duplicates.push(impl_id);
        }
    }
    (impls, duplicates, unresolved)
}

fn populate_qualified_call_candidates(mir_module: &mut mir::MirModule) {
    for call in mir_module.qualified_calls.values_mut() {
        if call.kind != mir::MirQualifiedCallKind::TraitMethod {
            continue;
        }
        let receiver_ty = match call.receiver_ty.as_ref() {
            Some(ty) => ty,
            None => continue,
        };
        let trait_name = match call
            .owner
            .as_ref()
            .and_then(|owner| owner.split("::").last())
        {
            Some(name) => name,
            None => continue,
        };
        let mut candidates = Vec::new();
        for (impl_id, spec) in &mir_module.impls {
            let normalized_target = normalize_impl_target_for_match(&spec.target);
            if spec
                .trait_name
                .as_ref()
                .map(|name| name == trait_name)
                .unwrap_or(false)
                && normalized_target == *receiver_ty
            {
                candidates.push(impl_id.clone());
            }
        }
        if !candidates.is_empty() {
            call.impl_candidates = candidates.clone();
        }
        if candidates.len() == 1 {
            call.impl_id = Some(candidates[0].clone());
        } else if call.impl_candidates.is_empty() {
            call.impl_id = None;
        }
    }
}

fn normalize_impl_target_for_match(target: &str) -> String {
    match target {
        "Int" => "i64".to_string(),
        "Unit" => "()".to_string(),
        _ => target.to_string(),
    }
}

fn collect_trait_names(module: &Module) -> HashSet<String> {
    let mut names = HashSet::new();
    for decl in &module.decls {
        let DeclKind::Trait(trait_decl) = &decl.kind else {
            continue;
        };
        names.insert(trait_decl.name.name.clone());
    }
    names
}

fn collect_effect_names(module: &Module) -> HashSet<String> {
    let mut names = HashSet::new();
    for effect in &module.effects {
        names.insert(effect.name.name.clone());
    }
    for decl in &module.decls {
        collect_effect_names_from_decl(decl, &mut names);
    }
    names
}

fn collect_effect_names_from_body(body: &ModuleBody, names: &mut HashSet<String>) {
    for effect in &body.effects {
        names.insert(effect.name.name.clone());
    }
    for decl in &body.decls {
        collect_effect_names_from_decl(decl, names);
    }
}

fn collect_effect_names_from_decl(decl: &Decl, names: &mut HashSet<String>) {
    match &decl.kind {
        DeclKind::Effect(effect) => {
            names.insert(effect.name.name.clone());
        }
        DeclKind::Module(module_decl) => {
            collect_effect_names_from_body(&module_decl.body, names);
        }
        _ => {}
    }
}

fn validate_handles_attrs(
    module: &Module,
    effect_names: &HashSet<String>,
    violations: &mut Vec<TypecheckViolation>,
) {
    for function in &module.functions {
        check_handles_attrs(
            &function.attrs,
            Some(function.name.name.clone()),
            effect_names,
            violations,
        );
    }
    for active in &module.active_patterns {
        check_handles_attrs(
            &active.attrs,
            Some(active.name.name.clone()),
            effect_names,
            violations,
        );
    }
    for decl in &module.decls {
        validate_handles_in_decl(decl, effect_names, violations);
    }
}

fn validate_handles_in_body(
    body: &ModuleBody,
    effect_names: &HashSet<String>,
    violations: &mut Vec<TypecheckViolation>,
) {
    for function in &body.functions {
        check_handles_attrs(
            &function.attrs,
            Some(function.name.name.clone()),
            effect_names,
            violations,
        );
    }
    for active in &body.active_patterns {
        check_handles_attrs(
            &active.attrs,
            Some(active.name.name.clone()),
            effect_names,
            violations,
        );
    }
    for decl in &body.decls {
        validate_handles_in_decl(decl, effect_names, violations);
    }
}

fn validate_handles_in_decl(
    decl: &Decl,
    effect_names: &HashSet<String>,
    violations: &mut Vec<TypecheckViolation>,
) {
    match &decl.kind {
        DeclKind::Fn { signature } => {
            check_handles_attrs(
                &decl.attrs,
                Some(signature.name.name.clone()),
                effect_names,
                violations,
            );
        }
        DeclKind::Impl(impl_decl) => {
            let target = impl_decl.target.render();
            for item in &impl_decl.items {
                if let ImplItem::Function(function) = item {
                    let owner = format!("{}::{}", target, function.name.name);
                    check_handles_attrs(&function.attrs, Some(owner), effect_names, violations);
                }
            }
        }
        DeclKind::Module(module_decl) => {
            validate_handles_in_body(&module_decl.body, effect_names, violations);
        }
        _ => {}
    }
}

fn check_handles_attrs(
    attrs: &[Attribute],
    owner: Option<String>,
    effect_names: &HashSet<String>,
    violations: &mut Vec<TypecheckViolation>,
) {
    for attr in attrs {
        if attr.name.name != "handles" {
            continue;
        }
        for arg in &attr.args {
            let Some(effect_name) = render_qualified_access(arg) else {
                continue;
            };
            if !effect_names.contains(&effect_name) {
                violations.push(TypecheckViolation::handles_unknown_effect(
                    attr.span,
                    effect_name,
                    owner.clone(),
                ));
            }
        }
    }
}

fn collect_externs(module: &Module) -> Vec<typed::TypedExtern> {
    let mut externs = Vec::new();
    for decl in &module.decls {
        collect_externs_from_decl(decl, &mut externs);
    }
    externs
}

fn collect_externs_from_body(body: &ModuleBody, externs: &mut Vec<typed::TypedExtern>) {
    for decl in &body.decls {
        collect_externs_from_decl(decl, externs);
    }
}

fn collect_externs_from_decl(decl: &Decl, externs: &mut Vec<typed::TypedExtern>) {
    match &decl.kind {
        DeclKind::Extern { abi, functions, .. } => {
            for item in functions {
                let name = item.signature.binding_key();
                let symbol = extract_extern_symbol(&item.attrs, &name);
                externs.push(typed::TypedExtern {
                    name,
                    span: item.span,
                    abi: abi.clone(),
                    symbol,
                });
            }
        }
        DeclKind::Module(module_decl) => {
            collect_externs_from_body(&module_decl.body, externs);
        }
        _ => {}
    }
}

fn extract_extern_symbol(attrs: &[Attribute], fallback: &str) -> String {
    for attr in attrs {
        if attr.name.name != "link_name" && attr.name.name != "ffi_link_name" {
            continue;
        }
        if let Some(symbol) = attribute_string_arg(attr) {
            return symbol;
        }
    }
    fallback.to_string()
}

fn attribute_string_arg(attr: &Attribute) -> Option<String> {
    let first = attr.args.first()?;
    match &first.kind {
        ExprKind::Literal(Literal {
            value: LiteralKind::String { value, .. },
        }) => Some(value.clone()),
        _ => None,
    }
}

fn visit_actor_specs(module: &Module, visitor: &mut impl FnMut(&ActorSpecDecl)) {
    for decl in &module.decls {
        visit_actor_specs_in_decl(decl, visitor);
    }
}

fn visit_actor_specs_in_body(body: &ModuleBody, visitor: &mut impl FnMut(&ActorSpecDecl)) {
    for decl in &body.decls {
        visit_actor_specs_in_decl(decl, visitor);
    }
}

fn visit_actor_specs_in_decl(decl: &Decl, visitor: &mut impl FnMut(&ActorSpecDecl)) {
    match &decl.kind {
        DeclKind::ActorSpec(actor_spec) => visitor(actor_spec),
        DeclKind::Module(module_decl) => visit_actor_specs_in_body(&module_decl.body, visitor),
        _ => {}
    }
}

fn associated_type_from_decl(decl: &TypeDecl) -> Option<mir::MirAssociatedType> {
    match decl.body.as_ref() {
        Some(TypeDeclBody::Alias { ty }) | Some(TypeDeclBody::Newtype { ty }) => {
            Some(mir::MirAssociatedType {
                name: decl.name.name.clone(),
                ty: ty.render(),
            })
        }
        _ => None,
    }
}

fn validate_type_decl_bodies(
    decls: &[Decl],
    env: &TypeEnv,
    violations: &mut Vec<TypecheckViolation>,
) {
    let mut var_gen = TypeVarGen::default();
    for decl in decls {
        match &decl.kind {
            DeclKind::Type { decl } => {
                let generic_map = build_generic_map(&decl.generics, &mut var_gen);
                let generic_map_ref = if generic_map.is_empty() {
                    None
                } else {
                    Some(&generic_map)
                };
                match decl.body.as_ref() {
                    Some(TypeDeclBody::Alias { ty }) | Some(TypeDeclBody::Newtype { ty }) => {
                        let mut resolver = TypeAliasResolver::new(env, violations);
                        let _ = type_from_annotation_kind_with_generics(
                            &ty.kind,
                            ty.span,
                            generic_map_ref,
                            None,
                            &mut resolver,
                        );
                    }
                    Some(TypeDeclBody::Sum { variants }) => {
                        for variant in variants {
                            if let Some(payload) = &variant.payload {
                                validate_type_decl_payload(
                                    payload,
                                    generic_map_ref,
                                    env,
                                    violations,
                                );
                            }
                        }
                    }
                    None => {}
                }
            }
            DeclKind::Enum(enum_decl) => {
                let generic_map = build_generic_map(&enum_decl.generics, &mut var_gen);
                let generic_map_ref = if generic_map.is_empty() {
                    None
                } else {
                    Some(&generic_map)
                };
                for variant in &enum_decl.variants {
                    if let Some(payload) = &variant.payload {
                        let converted = enum_variant_payload_to_type_decl_payload(payload);
                        validate_type_decl_payload(&converted, generic_map_ref, env, violations);
                    }
                }
            }
            _ => continue,
        }
    }
}

fn validate_type_decl_payload(
    payload: &TypeDeclVariantPayload,
    generics: Option<&HashMap<String, TypeVariable>>,
    env: &TypeEnv,
    violations: &mut Vec<TypecheckViolation>,
) {
    match payload {
        TypeDeclVariantPayload::Tuple { elements } => {
            for element in elements {
                let mut resolver = TypeAliasResolver::new(env, violations);
                let _ = type_from_annotation_kind_with_generics(
                    &element.ty.kind,
                    element.ty.span,
                    generics,
                    None,
                    &mut resolver,
                );
            }
        }
        TypeDeclVariantPayload::Record { fields, .. } => {
            for field in fields {
                let mut resolver = TypeAliasResolver::new(env, violations);
                let _ = type_from_annotation_kind_with_generics(
                    &field.ty.kind,
                    field.ty.span,
                    generics,
                    None,
                    &mut resolver,
                );
            }
        }
    }
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
    for expr in &module.exprs {
        let mut tracker = OpBuilderTracker::new(None);
        visit_expr_for_opbuilder(expr, &mut tracker, violations);
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
        DeclKind::Let { value, .. }
        | DeclKind::Var { value, .. }
        | DeclKind::Const { value, .. } => {
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
        DeclKind::Module(module_decl) => {
            for function in &module_decl.body.functions {
                visit_expr_for_opbuilder(&function.body, tracker, violations);
            }
            for active in &module_decl.body.active_patterns {
                visit_expr_for_opbuilder(&active.body, tracker, violations);
            }
            for decl in &module_decl.body.decls {
                visit_decl_for_opbuilder(decl, tracker, violations);
            }
            for expr in &module_decl.body.exprs {
                visit_expr_for_opbuilder(expr, tracker, violations);
            }
        }
        DeclKind::Macro(macro_decl) => {
            visit_expr_for_opbuilder(&macro_decl.body, tracker, violations);
        }
        DeclKind::ActorSpec(actor_spec) => {
            visit_expr_for_opbuilder(&actor_spec.body, tracker, violations);
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
        LiteralKind::Tuple { elements }
        | LiteralKind::Array { elements }
        | LiteralKind::Set { elements } => {
            for element in elements {
                visit_expr_for_opbuilder(element, tracker, violations);
            }
        }
        LiteralKind::Record { fields, .. } => {
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
        ExprKind::FieldAccess { target, .. } | ExprKind::TupleAccess { target, .. } => {
            visit_expr_for_opbuilder(target, tracker, violations);
        }
        ExprKind::Index { target, index } => {
            visit_expr_for_opbuilder(target, tracker, violations);
            visit_expr_for_opbuilder(index, tracker, violations);
        }
        ExprKind::PerformCall { call } => {
            visit_expr_for_opbuilder(&call.argument, tracker, violations);
        }
        ExprKind::InlineAsm(asm) => {
            for output in &asm.outputs {
                visit_expr_for_opbuilder(&output.target, tracker, violations);
            }
            for input in &asm.inputs {
                visit_expr_for_opbuilder(&input.expr, tracker, violations);
            }
        }
        ExprKind::LlvmIr(ir) => {
            for input in &ir.inputs {
                visit_expr_for_opbuilder(input, tracker, violations);
            }
        }
        ExprKind::Lambda { body, .. } => visit_expr_for_opbuilder(body, tracker, violations),
        ExprKind::Pipe { left, right } | ExprKind::Binary { left, right, .. } => {
            visit_expr_for_opbuilder(left, tracker, violations);
            visit_expr_for_opbuilder(right, tracker, violations);
        }
        ExprKind::Unary { expr: body, .. }
        | ExprKind::Rec { expr: body }
        | ExprKind::Propagate { expr: body }
        | ExprKind::Loop { body }
        | ExprKind::Defer { body }
        | ExprKind::Assign { value: body, .. }
        | ExprKind::EffectBlock { body }
        | ExprKind::Async { body, .. } => visit_expr_for_opbuilder(body, tracker, violations),
        ExprKind::Await { expr } => visit_expr_for_opbuilder(expr, tracker, violations),
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
    render_qualified_access(expr)
}

fn render_qualified_access(expr: &Expr) -> Option<String> {
    match &expr.kind {
        ExprKind::Identifier(ident) => Some(ident.name.clone()),
        ExprKind::FieldAccess { target, field } => {
            render_qualified_access(target).map(|base| format!("{base}.{}", field.name))
        }
        ExprKind::ModulePath(path) => Some(path.render()),
        _ => None,
    }
}

fn is_type_like_ident(name: &str) -> bool {
    name.chars()
        .next()
        .map(|ch| ch.is_ascii_uppercase())
        .unwrap_or(false)
}

fn collect_type_path_parts(expr: &Expr, parts: &mut Vec<String>) -> bool {
    match &expr.kind {
        ExprKind::Identifier(ident) => {
            parts.push(ident.name.clone());
            true
        }
        ExprKind::ModulePath(path) => match path {
            ModulePath::Root { segments } => {
                for segment in segments {
                    parts.push(segment.name.clone());
                }
                !segments.is_empty()
            }
            ModulePath::Relative { head, segments } => match head {
                RelativeHead::PlainIdent(ident) => {
                    parts.push(ident.name.clone());
                    for segment in segments {
                        parts.push(segment.name.clone());
                    }
                    true
                }
                RelativeHead::Self_ | RelativeHead::Super(_) => false,
            },
        },
        ExprKind::FieldAccess { target, field } => {
            if collect_type_path_parts(target, parts) {
                parts.push(field.name.clone());
                true
            } else {
                false
            }
        }
        _ => false,
    }
}

fn type_method_target_name(expr: &Expr) -> Option<String> {
    let mut parts = Vec::new();
    if !collect_type_path_parts(expr, &mut parts) {
        return None;
    }
    if parts.is_empty() {
        return None;
    }
    if !parts.iter().all(|part| is_type_like_ident(part)) {
        return None;
    }
    Some(parts.join("__"))
}

fn render_type_owner(expr: &Expr) -> Option<String> {
    let mut parts = Vec::new();
    if !collect_type_path_parts(expr, &mut parts) {
        return None;
    }
    if parts.is_empty() || !parts.iter().all(|part| is_type_like_ident(part)) {
        return None;
    }
    Some(parts.join("::"))
}

fn future_type(inner: Type) -> Type {
    Type::app("Future", vec![inner])
}

fn module_path_parts(path: &ModulePath) -> Option<Vec<String>> {
    match path {
        ModulePath::Root { segments } => {
            if segments.is_empty() {
                None
            } else {
                Some(
                    segments
                        .iter()
                        .map(|segment| segment.name.clone())
                        .collect(),
                )
            }
        }
        ModulePath::Relative { head, segments } => match head {
            RelativeHead::PlainIdent(ident) => {
                let mut parts = Vec::with_capacity(1 + segments.len());
                parts.push(ident.name.clone());
                parts.extend(segments.iter().map(|segment| segment.name.clone()));
                Some(parts)
            }
            RelativeHead::Self_ | RelativeHead::Super(_) => None,
        },
    }
}

fn resolve_qualified_call(
    callee: &Expr,
    trait_names: &HashSet<String>,
) -> Option<typed::QualifiedCall> {
    match &callee.kind {
        ExprKind::FieldAccess { target, field } => {
            if type_method_target_name(target).is_some() {
                let owner = render_type_owner(target)?;
                let owner_last = owner.split("::").last().unwrap_or(owner.as_str());
                let kind = if trait_names.contains(owner_last) {
                    typed::QualifiedCallKind::TraitMethod
                } else {
                    typed::QualifiedCallKind::TypeMethod
                };
                let impl_id = if matches!(kind, typed::QualifiedCallKind::TypeMethod) {
                    Some(owner.clone())
                } else {
                    None
                };
                Some(typed::QualifiedCall {
                    kind,
                    owner: Some(owner.clone()),
                    name: Some(field.name.clone()),
                    impl_id,
                })
            } else {
                None
            }
        }
        ExprKind::ModulePath(path) => {
            let parts = module_path_parts(path)?;
            if parts.len() < 2 {
                return None;
            }
            // `ModulePath` は `foo::Bar::baz` のような構造を想定し、
            // owner は `foo::Bar`、name は `baz` として解決する。
            let mut owner_parts = parts.clone();
            let name = owner_parts.pop().unwrap();
            let owner = owner_parts.join("::");
            let trait_match = owner_parts
                .last()
                .map(|part| trait_names.contains(part))
                .unwrap_or(false);
            let kind = if trait_match {
                typed::QualifiedCallKind::TraitMethod
            } else if owner_parts.iter().all(|part| is_type_like_ident(part)) {
                typed::QualifiedCallKind::TypeAssoc
            } else {
                typed::QualifiedCallKind::Unknown
            };
            let impl_id = match kind {
                typed::QualifiedCallKind::TypeAssoc => Some(owner.clone()),
                _ => None,
            };
            Some(typed::QualifiedCall {
                kind,
                owner: Some(owner),
                name: Some(name),
                impl_id,
            })
        }
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
    stats.local_bindings = collect_function_bindings(&function.params, &function.body);
    let body_result = infer_expr(
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
    );
    if function.is_async {
        let async_ty = future_type(body_result.ty.clone());
        let dict_ids = body_result.dict_ref_ids.clone();
        TypedExprDraft {
            span: body_result.span,
            kind: TypedExprKindDraft::Async {
                body: Box::new(body_result),
                is_move: false,
            },
            ty: async_ty,
            dict_ref_ids: dict_ids,
        }
    } else {
        body_result
    }
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
        ExprKind::Literal(literal) => match &literal.value {
            LiteralKind::Tuple { elements } => {
                let mut dicts = Vec::new();
                let mut element_types = Vec::new();
                for element in elements {
                    let result = infer_expr(
                        element,
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
                    dicts.extend(result.dict_ref_ids);
                    element_types.push(solver.substitution().apply(&result.ty));
                }
                let ty = Type::app("Tuple", element_types);
                make_typed(
                    expr,
                    TypedExprKindDraft::Literal(literal.clone()),
                    ty,
                    dicts,
                )
            }
            LiteralKind::Array { elements } => {
                let mut dicts = Vec::new();
                let element_ty = var_gen.fresh_type();
                for element in elements {
                    let result = infer_expr(
                        element,
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
                    dicts.extend(result.dict_ref_ids);
                    stats.constraints += 1;
                    metrics.record_constraint("literal.array.element");
                    constraints.push(Constraint::equal(result.ty.clone(), element_ty.clone()));
                    metrics.record_unify_call();
                    let _ = solver.unify(result.ty.clone(), element_ty.clone());
                }
                let ty = Type::slice(solver.substitution().apply(&element_ty));
                make_typed(
                    expr,
                    TypedExprKindDraft::Literal(literal.clone()),
                    ty,
                    dicts,
                )
            }
            LiteralKind::Set { elements } => {
                let mut dicts = Vec::new();
                let element_ty = var_gen.fresh_type();
                for element in elements {
                    let result = infer_expr(
                        element,
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
                    dicts.extend(result.dict_ref_ids);
                    stats.constraints += 1;
                    metrics.record_constraint("literal.set.element");
                    constraints.push(Constraint::equal(result.ty.clone(), element_ty.clone()));
                    metrics.record_unify_call();
                    let _ = solver.unify(result.ty.clone(), element_ty.clone());
                }
                let ty = Type::app("Set", vec![solver.substitution().apply(&element_ty)]);
                make_typed(
                    expr,
                    TypedExprKindDraft::Literal(literal.clone()),
                    ty,
                    dicts,
                )
            }
            LiteralKind::Record { fields, .. } => {
                let mut dicts = Vec::new();
                let mut field_types = Vec::new();
                for field in fields {
                    let result = infer_expr(
                        &field.value,
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
                    dicts.extend(result.dict_ref_ids);
                    field_types.push(solver.substitution().apply(&result.ty));
                }
                let ty = Type::app("Record", field_types);
                make_typed(
                    expr,
                    TypedExprKindDraft::Literal(literal.clone()),
                    ty,
                    dicts,
                )
            }
            _ => {
                let ty = type_for_literal(literal);
                make_typed(
                    expr,
                    TypedExprKindDraft::Literal(literal.clone()),
                    ty,
                    Vec::new(),
                )
            }
        },
        ExprKind::Identifier(ident) => {
            let mut ty = match env.lookup(ident.name.as_str()) {
                Some(binding) => binding.scheme.instantiate(var_gen),
                None => match ident.name.as_str() {
                    "Some" => {
                        let t = var_gen.fresh_type();
                        Type::arrow(vec![t.clone()], Type::app("Option", vec![t]))
                    }
                    "None" => {
                        let t = var_gen.fresh_type();
                        Type::app("Option", vec![t])
                    }
                    "format" => {
                        let t = var_gen.fresh_type();
                        let arg = Type::slice(t);
                        Type::arrow(vec![arg], Type::builtin(BuiltinType::Str))
                    }
                    "Ok" => {
                        let ok_ty = var_gen.fresh_type();
                        let err_ty = var_gen.fresh_type();
                        Type::arrow(
                            vec![ok_ty.clone()],
                            Type::app("Result", vec![ok_ty, err_ty]),
                        )
                    }
                    "Err" => {
                        let ok_ty = var_gen.fresh_type();
                        let err_ty = var_gen.fresh_type();
                        Type::arrow(
                            vec![err_ty.clone()],
                            Type::app("Result", vec![ok_ty, err_ty]),
                        )
                    }
                    other => {
                        if let Some(binding) = env.lookup_type_constructor(other) {
                            constructor_type_from_binding(binding, var_gen, env, violations)
                        } else {
                            let _ = other; // 未解決識別子として扱う
                            stats.unresolved_identifiers += 1;
                            metrics.record_unresolved_identifier();
                            Type::builtin(BuiltinType::Unknown)
                        }
                    }
                },
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
        ExprKind::FieldAccess { target, field } => {
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
            let target_ty = solver.substitution().apply(&target_result.ty);
            let result_ty = match field.name.as_str() {
                "to_string" => Type::arrow(vec![], Type::builtin(BuiltinType::Str)),
                "len" => {
                    let int_ty = Type::builtin(BuiltinType::Int);
                    match target_ty {
                        Type::Builtin(BuiltinType::Str) => Type::arrow(vec![], int_ty),
                        _ => {
                            let elem = var_gen.fresh_type();
                            let array_ty = Type::slice(elem);
                            stats.constraints += 1;
                            metrics.record_constraint("method.len.array");
                            constraints.push(Constraint::equal(
                                target_result.ty.clone(),
                                array_ty.clone(),
                            ));
                            metrics.record_unify_call();
                            let _ = solver.unify(target_result.ty.clone(), array_ty);
                            Type::arrow(vec![], int_ty)
                        }
                    }
                }
                "is_empty" => {
                    let bool_ty = Type::builtin(BuiltinType::Bool);
                    match target_ty {
                        Type::Builtin(BuiltinType::Str) => Type::arrow(vec![], bool_ty),
                        _ => {
                            let elem = var_gen.fresh_type();
                            let array_ty = Type::slice(elem);
                            stats.constraints += 1;
                            metrics.record_constraint("method.is_empty.array");
                            constraints.push(Constraint::equal(
                                target_result.ty.clone(),
                                array_ty.clone(),
                            ));
                            metrics.record_unify_call();
                            let _ = solver.unify(target_result.ty.clone(), array_ty);
                            Type::arrow(vec![], bool_ty)
                        }
                    }
                }
                "starts_with" => {
                    if matches!(target_ty, Type::Builtin(BuiltinType::Str)) {
                        Type::arrow(
                            vec![Type::builtin(BuiltinType::Str)],
                            Type::builtin(BuiltinType::Bool),
                        )
                    } else {
                        var_gen.fresh_type()
                    }
                }
                "push" => {
                    let elem = var_gen.fresh_type();
                    let array_ty = Type::slice(elem.clone());
                    stats.constraints += 1;
                    metrics.record_constraint("method.push.array");
                    constraints.push(Constraint::equal(
                        target_result.ty.clone(),
                        array_ty.clone(),
                    ));
                    metrics.record_unify_call();
                    let _ = solver.unify(target_result.ty.clone(), array_ty);
                    Type::arrow(vec![elem], Type::builtin(BuiltinType::Unit))
                }
                "pop" => {
                    let elem = var_gen.fresh_type();
                    let array_ty = Type::slice(elem.clone());
                    stats.constraints += 1;
                    metrics.record_constraint("method.pop.array");
                    constraints.push(Constraint::equal(
                        target_result.ty.clone(),
                        array_ty.clone(),
                    ));
                    metrics.record_unify_call();
                    let _ = solver.unify(target_result.ty.clone(), array_ty);
                    Type::arrow(vec![], Type::app("Option", vec![elem]))
                }
                _ => var_gen.fresh_type(),
            };
            make_typed(
                expr,
                TypedExprKindDraft::FieldAccess {
                    target: Box::new(target_result),
                    field: field.clone(),
                },
                solver.substitution().apply(&result_ty),
                Vec::new(),
            )
        }
        ExprKind::TupleAccess { target, index } => {
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
            let mut elements = Vec::new();
            for _ in 0..=*index {
                elements.push(var_gen.fresh_type());
            }
            let tuple_ty = Type::app("Tuple", elements.clone());
            stats.constraints += 1;
            metrics.record_constraint("tuple.access");
            constraints.push(Constraint::equal(
                target_result.ty.clone(),
                tuple_ty.clone(),
            ));
            metrics.record_unify_call();
            let _ = solver.unify(target_result.ty.clone(), tuple_ty.clone());
            let elem_ty = elements
                .get(*index as usize)
                .cloned()
                .unwrap_or_else(|| var_gen.fresh_type());
            make_typed(
                expr,
                TypedExprKindDraft::TupleAccess {
                    target: Box::new(target_result),
                    index: *index,
                },
                solver.substitution().apply(&elem_ty),
                Vec::new(),
            )
        }
        ExprKind::Index { target, index } => {
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
            let index_result = infer_expr(
                index,
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
            let array_ty = Type::slice(element_ty.clone());
            stats.constraints += 1;
            metrics.record_constraint("index.target");
            constraints.push(Constraint::equal(
                target_result.ty.clone(),
                array_ty.clone(),
            ));
            metrics.record_unify_call();
            let _ = solver.unify(target_result.ty.clone(), array_ty);
            let int_ty = Type::builtin(BuiltinType::Int);
            stats.constraints += 1;
            metrics.record_constraint("index.type");
            constraints.push(Constraint::equal(index_result.ty.clone(), int_ty.clone()));
            metrics.record_unify_call();
            let _ = solver.unify(index_result.ty.clone(), int_ty);
            let mut dicts = target_result.dict_ref_ids.clone();
            dicts.extend(index_result.dict_ref_ids.clone());
            make_typed(
                expr,
                TypedExprKindDraft::Index {
                    target: Box::new(target_result),
                    index: Box::new(index_result),
                },
                solver.substitution().apply(&element_ty),
                dicts,
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
            let left_ty = solver.substitution().apply(&left_result.ty);
            let right_ty = solver.substitution().apply(&right_result.ty);
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
                    if matches!(left_ty, Type::Builtin(BuiltinType::Str))
                        && matches!(right_ty, Type::Builtin(BuiltinType::Str)) =>
                {
                    Type::builtin(BuiltinType::Str)
                }
                BinaryOp::Add
                | BinaryOp::Sub
                | BinaryOp::Mul
                | BinaryOp::Div
                | BinaryOp::Mod
                | BinaryOp::Pow => combine_numeric_types(&left_ty, &right_ty),
                BinaryOp::And | BinaryOp::Or => Type::builtin(BuiltinType::Bool),
                BinaryOp::Eq
                | BinaryOp::Ne
                | BinaryOp::Lt
                | BinaryOp::Le
                | BinaryOp::Gt
                | BinaryOp::Ge => Type::builtin(BuiltinType::Bool),
                _ => combine_numeric_types(&left_ty, &right_ty),
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
        ExprKind::Pipe { left, right } => {
            let desugared = match &right.kind {
                ExprKind::Call { callee, args } => {
                    let mut new_args = Vec::with_capacity(args.len() + 1);
                    new_args.push((**left).clone());
                    new_args.extend(args.iter().cloned());
                    Expr::call((**callee).clone(), new_args, expr.span)
                }
                _ => Expr::call((**right).clone(), vec![(**left).clone()], expr.span),
            };
            infer_expr(
                &desugared,
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
        }
        ExprKind::Call { callee, args } => {
            metrics.record_call_site();
            let qualified_call = resolve_qualified_call(callee, context.trait_names);
            let mut desugared_callee = None;
            if let ExprKind::FieldAccess { target, field } = &callee.kind {
                if let Some(qualified_name) = render_qualified_access(callee) {
                    if env.lookup(qualified_name.as_str()).is_some() {
                        desugared_callee = Some(Expr::identifier(Ident {
                            name: qualified_name,
                            span: callee.span(),
                        }));
                    }
                }
                if let Some(target_name) = type_method_target_name(target) {
                    if desugared_callee.is_none() {
                        desugared_callee = Some(Expr::identifier(Ident {
                            name: format!("{target_name}__{}", field.name),
                            span: field.span,
                        }));
                    }
                }
            }
            let callee_expr = desugared_callee.as_ref().unwrap_or(callee);
            if let ExprKind::Identifier(ident) = &callee_expr.kind {
                if let Some(binding) = env.lookup_type_constructor(ident.name.as_str()) {
                    let expected = constructor_expected_arity(binding.payload.as_ref());
                    if args.len() != expected {
                        violations.push(TypecheckViolation::constructor_arity_mismatch(
                            expr.span(),
                            ident.name.as_str(),
                            expected,
                            args.len(),
                        ));
                    }
                }
            }
            let callee_result = infer_expr(
                callee_expr,
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
            let typed_args: Vec<_> = args
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
            let param_types: Vec<Type> = typed_args.iter().map(|_| var_gen.fresh_type()).collect();
            let result_type = var_gen.fresh_type();

            // callee が関数型であることを期待して矢印型と一致させる
            stats.constraints += 1;
            metrics.record_constraint("call.signature");
            let callee_arrow = Type::arrow(param_types.clone(), result_type.clone());
            constraints.push(Constraint::equal(
                callee_result.ty.clone(),
                callee_arrow.clone(),
            ));
            metrics.record_unify_call();
            let _ = solver.unify(callee_result.ty.clone(), callee_arrow);

            // 引数とパラメータ型を対応付ける
            for (arg_result, param_ty) in typed_args.iter().zip(param_types.iter()) {
                stats.constraints += 1;
                metrics.record_constraint("call.param");
                constraints.push(Constraint::equal(arg_result.ty.clone(), param_ty.clone()));
                metrics.record_unify_call();
                let _ = solver.unify(arg_result.ty.clone(), param_ty.clone());
            }

            // dict refs を集約
            let mut dict_ids = callee_result.dict_ref_ids.clone();
            for arg in &typed_args {
                dict_ids.extend(arg.dict_ref_ids.clone());
            }

            make_typed(
                expr,
                TypedExprKindDraft::Call {
                    callee: Box::new(callee_result),
                    args: typed_args,
                    qualified: qualified_call,
                },
                solver.substitution().apply(&result_type),
                dict_ids,
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
        ExprKind::InlineAsm(asm) => {
            let mut outputs = Vec::new();
            let mut inputs = Vec::new();
            let mut dicts = Vec::new();
            let mut invalid_label = None;
            for output in &asm.outputs {
                let result = infer_expr(
                    &output.target,
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
                dicts.extend(result.dict_ref_ids.clone());
                let resolved = solver.substitution().apply(&result.ty);
                if invalid_label.is_none() && !native_abi_type_allowed(&resolved) {
                    invalid_label = Some(resolved.label());
                }
                outputs.push(InlineAsmOutputDraft {
                    constraint: output.constraint.clone(),
                    target: Box::new(result),
                });
            }
            for input in &asm.inputs {
                let result = infer_expr(
                    &input.expr,
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
                dicts.extend(result.dict_ref_ids.clone());
                let resolved = solver.substitution().apply(&result.ty);
                if invalid_label.is_none() && !native_abi_type_allowed(&resolved) {
                    invalid_label = Some(resolved.label());
                }
                inputs.push(InlineAsmInputDraft {
                    constraint: input.constraint.clone(),
                    expr: Box::new(result),
                });
            }
            if let Some(label) = invalid_label {
                violations.push(TypecheckViolation::inline_asm_invalid_type(
                    expr.span(),
                    label,
                    context.name.map(|name| name.to_string()),
                ));
            }
            make_typed(
                expr,
                TypedExprKindDraft::InlineAsm {
                    template: asm.template.clone(),
                    outputs,
                    inputs,
                    clobbers: asm.clobbers.clone(),
                    options: asm.options.clone(),
                },
                Type::builtin(BuiltinType::Unit),
                dicts,
            )
        }
        ExprKind::LlvmIr(ir) => {
            let mut inputs = Vec::new();
            let mut dicts = Vec::new();
            let mut invalid_label = None;
            for input in &ir.inputs {
                let result = infer_expr(
                    input,
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
                dicts.extend(result.dict_ref_ids.clone());
                let resolved = solver.substitution().apply(&result.ty);
                if invalid_label.is_none() && !native_abi_type_allowed(&resolved) {
                    invalid_label = Some(resolved.label());
                }
                inputs.push(result);
            }
            let result_ty = type_from_annotation(&ir.result_type, None, env, violations)
                .unwrap_or_else(|| Type::builtin(BuiltinType::Unknown));
            let resolved_result_ty = solver.substitution().apply(&result_ty);
            if invalid_label.is_none() && !native_abi_type_allowed(&resolved_result_ty) {
                invalid_label = Some(resolved_result_ty.label());
            }
            if let Some(label) = invalid_label {
                violations.push(TypecheckViolation::llvm_ir_invalid_type(
                    expr.span(),
                    label,
                    context.name.map(|name| name.to_string()),
                ));
            }
            make_typed(
                expr,
                TypedExprKindDraft::LlvmIr {
                    result_type: resolved_result_ty.label(),
                    template: ir.template.clone(),
                    inputs,
                },
                result_ty,
                dicts,
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
        ExprKind::Unsafe { body } => {
            if context.is_pure {
                violations.push(TypecheckViolation::unsafe_in_pure_context(
                    expr.span(),
                    context.name.map(|name| name.to_string()),
                ));
            }
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
            make_typed(
                expr,
                TypedExprKindDraft::Unsafe {
                    body: Box::new(body_result.clone()),
                },
                body_result.ty.clone(),
                body_result.dict_ref_ids,
            )
        }
        ExprKind::EffectBlock { body } => {
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
            make_typed(
                expr,
                TypedExprKindDraft::EffectBlock {
                    body: Box::new(body_result.clone()),
                },
                body_result.ty.clone(),
                body_result.dict_ref_ids,
            )
        }
        ExprKind::Async { body, is_move } => {
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
            let async_ty = future_type(body_result.ty.clone());
            make_typed(
                expr,
                TypedExprKindDraft::Async {
                    body: Box::new(body_result.clone()),
                    is_move: *is_move,
                },
                async_ty,
                body_result.dict_ref_ids,
            )
        }
        ExprKind::Await { expr: inner } => {
            let inner_result = infer_expr(
                inner,
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
            let awaited_ty = var_gen.fresh_type();
            let expected_future = future_type(awaited_ty.clone());
            stats.constraints += 1;
            metrics.record_constraint("await.future");
            constraints.push(Constraint::equal(
                inner_result.ty.clone(),
                expected_future.clone(),
            ));
            metrics.record_unify_call();
            let _ = solver.unify(inner_result.ty.clone(), expected_future);
            make_typed(
                expr,
                TypedExprKindDraft::Await {
                    expr: Box::new(inner_result.clone()),
                },
                solver.substitution().apply(&awaited_ty),
                inner_result.dict_ref_ids,
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
            let array_ty = Type::slice(element_ty.clone());
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
            let target_ty = solver.substitution().apply(&target_result.ty);
            let coverage = analyze_match_exhaustiveness(arms, &target_ty, env);
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
                validate_pattern_against_type(&arm.pattern, &target_ty, env, violations);
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
            if coverage.should_report_missing && !coverage.coverage_reached {
                violations.push(TypecheckViolation::pattern_exhaustiveness_missing(
                    expr.span(),
                    coverage.missing_variants,
                    coverage.missing_ranges,
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
            make_typed(
                expr,
                TypedExprKindDraft::Block {
                    statements: block_result.statements,
                    tail: block_result.tail_expr.map(Box::new),
                    defers: block_result.defer_exprs,
                },
                block_result.ty,
                block_result.dict_ref_ids,
            )
        }
        ExprKind::Return { value } => {
            let (typed_value, dicts, ty) = if let Some(inner) = value {
                let result = infer_expr(
                    inner,
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
                let dicts = result.dict_ref_ids.clone();
                let ty = result.ty.clone();
                (Some(Box::new(result)), dicts, ty)
            } else {
                (None, Vec::new(), Type::builtin(BuiltinType::Unit))
            };
            make_typed(
                expr,
                TypedExprKindDraft::Return { value: typed_value },
                ty,
                dicts,
            )
        }
        ExprKind::Propagate { expr: inner } => {
            let result = infer_expr(
                inner,
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
            let dicts = result.dict_ref_ids.clone();
            let ty = result.ty.clone();
            make_typed(
                expr,
                TypedExprKindDraft::Propagate {
                    expr: Box::new(result),
                },
                ty,
                dicts,
            )
        }
        ExprKind::Unary {
            operator,
            expr: inner,
        } => {
            let result = infer_expr(
                inner,
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
            let expected = match operator {
                UnaryOp::Not => Type::builtin(BuiltinType::Bool),
                UnaryOp::Neg => Type::builtin(BuiltinType::Int),
                UnaryOp::Custom(_) => result.ty.clone(),
            };
            stats.constraints += 1;
            metrics.record_constraint("unary.operand");
            constraints.push(Constraint::equal(result.ty.clone(), expected.clone()));
            metrics.record_unify_call();
            let _ = solver.unify(result.ty.clone(), expected.clone());
            make_typed(
                expr,
                TypedExprKindDraft::Unknown,
                solver.substitution().apply(&expected),
                result.dict_ref_ids.clone(),
            )
        }
        ExprKind::Rec { expr: inner } => {
            let ident = if let ExprKind::Identifier(ident) = &inner.kind {
                if env.lookup(ident.name.as_str()).is_none() {
                    violations.push(TypecheckViolation::rec_unresolved_ident(
                        ident.span,
                        ident.name.as_str(),
                    ));
                }
                Some(ident.clone())
            } else {
                None
            };
            let result = infer_expr(
                inner,
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
            let dicts = result.dict_ref_ids.clone();
            let ty = result.ty.clone();
            make_typed(
                expr,
                TypedExprKindDraft::Rec {
                    target: Box::new(result),
                    ident,
                },
                ty,
                dicts,
            )
        }
        ExprKind::Lambda {
            params,
            ret_type,
            body,
            ..
        } => {
            let capture_report = if stats.local_bindings.is_empty() {
                None
            } else {
                Some(detect_lambda_captures(body, params, &stats.local_bindings))
            };
            let mut captures = Vec::new();
            if let Some(report) = capture_report {
                for (name, span) in report.captures {
                    violations.push(TypecheckViolation::lambda_capture_unsupported(span, &name));
                    captures.push(typed::TypedLambdaCapture {
                        name,
                        span,
                        mutable: false,
                    });
                }
                for (name, span) in report.mut_captures {
                    violations.push(TypecheckViolation::lambda_capture_mut_unsupported(
                        span, &name,
                    ));
                    captures.push(typed::TypedLambdaCapture {
                        name,
                        span,
                        mutable: true,
                    });
                }
            }
            let mut lambda_env = env.enter_scope();
            let mut param_types = Vec::new();
            let mut param_bindings = Vec::new();
            for param in params {
                let ty = param
                    .type_annotation
                    .as_ref()
                    .and_then(|annot| type_from_annotation(annot, None, env, violations))
                    .unwrap_or_else(|| var_gen.fresh_type());
                let scheme = Scheme::simple(ty.clone());
                bind_pattern_to_env(&param.pattern, &scheme, &mut lambda_env, var_gen);
                param_types.push(ty.clone());
                param_bindings.push(ParamBinding {
                    display: param.pattern.render(),
                    span: param.span,
                    ty,
                    annotation: param.type_annotation.as_ref().map(|annot| annot.render()),
                });
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
            let dicts = body_result.dict_ref_ids.clone();
            make_typed(
                expr,
                TypedExprKindDraft::Lambda {
                    params: param_bindings,
                    return_annotation: ret_type.as_ref().map(|ty| ty.render()),
                    body: Box::new(body_result),
                    captures,
                },
                lambda_ty,
                dicts,
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
    defer_exprs: Vec<TypedExprDraft>,
    statements: Vec<TypedStmtDraft>,
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
    let mut defer_exprs = Vec::new();
    let mut typed_statements = Vec::new();
    let mut terminated = false;
    for stmt in statements {
        if terminated {
            violations.push(TypecheckViolation::control_flow_unreachable(stmt.span));
        }
        match &stmt.kind {
            StmtKind::Decl { decl } => match &decl.kind {
                DeclKind::Let { pattern, value, .. } => {
                    let (value_result, stmt_refs) = infer_binding_with_value(
                        pattern,
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
                        loop_context,
                    );
                    block_dict_refs.extend(stmt_refs.clone());
                    typed_statements.push(TypedStmtDraft {
                        span: stmt.span,
                        kind: TypedStmtKindDraft::Let {
                            pattern: lower_typed_pattern(pattern),
                            value: Box::new(value_result),
                        },
                    });
                }
                DeclKind::Const {
                    name,
                    value,
                    type_annotation: _,
                } => {
                    let pattern = Pattern {
                        span: name.span,
                        kind: PatternKind::Var(name.clone()),
                    };
                    let (value_result, stmt_refs) = infer_binding_with_value(
                        &pattern,
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
                        loop_context,
                    );
                    block_dict_refs.extend(stmt_refs.clone());
                    typed_statements.push(TypedStmtDraft {
                        span: stmt.span,
                        kind: TypedStmtKindDraft::Let {
                            pattern: lower_typed_pattern(&pattern),
                            value: Box::new(value_result),
                        },
                    });
                }
                DeclKind::Var {
                    pattern,
                    value,
                    type_annotation,
                } => {
                    let (value_result, stmt_refs) = infer_binding_with_value(
                        pattern,
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
                        loop_context,
                    );
                    if type_annotation.is_none() {
                        if let Some(name) = pattern_binding_name(pattern) {
                            violations.push(TypecheckViolation::value_restriction(decl.span, name));
                        }
                    }
                    block_dict_refs.extend(stmt_refs.clone());
                    typed_statements.push(TypedStmtDraft {
                        span: stmt.span,
                        kind: TypedStmtKindDraft::Var {
                            pattern: lower_typed_pattern(pattern),
                            value: Box::new(value_result),
                        },
                    });
                }
                _ => {
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
            },
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
                typed_statements.push(TypedStmtDraft {
                    span: stmt.span,
                    kind: TypedStmtKindDraft::Expr {
                        expr: Box::new(expr_result.clone()),
                    },
                });
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
                block_dict_refs.extend(target_result.dict_ref_ids.clone());
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
                block_dict_refs.extend(value_result.dict_ref_ids.clone());
                typed_statements.push(TypedStmtDraft {
                    span: stmt.span,
                    kind: TypedStmtKindDraft::Assign {
                        target: Box::new(target_result),
                        value: Box::new(value_result),
                    },
                });
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
                block_dict_refs.extend(defer_result.dict_ref_ids.clone());
                defer_exprs.push(defer_result.clone());
                typed_statements.push(TypedStmtDraft {
                    span: stmt.span,
                    kind: TypedStmtKindDraft::Defer {
                        expr: Box::new(defer_result.clone()),
                    },
                });
            }
        }
    }
    BlockInferenceResult {
        ty: last_ty,
        dict_ref_ids: block_dict_refs,
        tail_expr,
        defer_exprs,
        statements: typed_statements,
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
        DeclKind::Const {
            name,
            value,
            type_annotation: _,
        } => {
            let pattern = Pattern {
                span: name.span,
                kind: PatternKind::Var(name.clone()),
            };
            if let Some(tracker) = unicode_tracker {
                tracker.observe_pattern(&pattern, decl.span, violations);
            }
            infer_binding(
                &pattern,
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
    if let PatternKind::Var(ident) = &pattern.kind {
        if is_direct_recursion(value, ident.name.as_str()) {
            violations.push(TypecheckViolation::recursion_infinite(
                value.span(),
                ident.name.as_str(),
            ));
        }
    }
    let value_context = match (&pattern.kind, &value.kind) {
        (PatternKind::Var(ident), ExprKind::Lambda { .. }) => {
            FunctionContext::function(ident.name.as_str(), context.is_pure, context.trait_names)
        }
        _ => context,
    };
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
        value_context,
    );
    let substitution = solver.substitution().clone();
    let resolved_ty = substitution.apply(&value_result.ty);
    detect_duplicate_bindings(pattern, violations);
    validate_pattern_against_type(pattern, &resolved_ty, env, violations);
    detect_regex_target_mismatch(pattern, &resolved_ty, violations);
    let scheme = generalize_type(env, resolved_ty.clone());
    bind_pattern_to_env(pattern, &scheme, env, var_gen);
    value_result.dict_ref_ids
}

fn infer_binding_with_value(
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
) -> (TypedExprDraft, Vec<typed::DictRefId>) {
    if let PatternKind::Var(ident) = &pattern.kind {
        if is_direct_recursion(value, ident.name.as_str()) {
            violations.push(TypecheckViolation::recursion_infinite(
                value.span(),
                ident.name.as_str(),
            ));
        }
    }
    let value_context = match (&pattern.kind, &value.kind) {
        (PatternKind::Var(ident), ExprKind::Lambda { .. }) => {
            FunctionContext::function(ident.name.as_str(), context.is_pure, context.trait_names)
        }
        _ => context,
    };
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
        value_context,
    );
    let substitution = solver.substitution().clone();
    let resolved_ty = substitution.apply(&value_result.ty);
    detect_duplicate_bindings(pattern, violations);
    validate_pattern_against_type(pattern, &resolved_ty, env, violations);
    detect_regex_target_mismatch(pattern, &resolved_ty, violations);
    let scheme = generalize_type(env, resolved_ty.clone());
    bind_pattern_to_env(pattern, &scheme, env, var_gen);
    let dicts = value_result.dict_ref_ids.clone();
    (value_result, dicts)
}

fn infer_conductor(
    conductor: &ConductorDecl,
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
) -> typed::TypedConductor {
    let mut seen_dsl_ids: HashMap<String, Span> = HashMap::new();
    let mut dsl_defs = Vec::new();
    for dsl_def in &conductor.dsl_defs {
        let alias = dsl_def.alias.name.clone();
        if seen_dsl_ids.contains_key(&alias) {
            violations.push(TypecheckViolation::conductor_dsl_id_duplicate(
                dsl_def.span,
                alias.as_str(),
            ));
        } else {
            seen_dsl_ids.insert(alias.clone(), dsl_def.span);
        }
        let target = dsl_def.target.name.clone();
        let target_type = env
            .lookup(target.as_str())
            .map(|binding| binding.scheme.instantiate(var_gen))
            .map(|ty| solver.substitution().apply(&ty).label());
        let pipeline_type = dsl_def.pipeline.as_ref().map(|pipeline| {
            let result = infer_expr(
                &pipeline.expr,
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
            solver.substitution().apply(&result.ty).label()
        });
        let tails = dsl_def
            .tails
            .iter()
            .map(|tail| {
                let mut arg_types = Vec::new();
                for arg in &tail.args {
                    let result = infer_expr(
                        &arg.value,
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
                    arg_types.push(solver.substitution().apply(&result.ty).label());
                }
                typed::TypedConductorDslTail {
                    stage: tail.stage.name.clone(),
                    arg_types,
                    span: tail.span,
                }
            })
            .collect::<Vec<_>>();
        dsl_defs.push(typed::TypedConductorDslDef {
            alias,
            target,
            target_type,
            pipeline_type,
            tails,
            span: dsl_def.span,
        });
    }

    let channels = conductor
        .channels
        .iter()
        .map(|route| {
            let payload = type_from_annotation(&route.payload, None, env, violations)
                .map(|ty| solver.substitution().apply(&ty).label())
                .unwrap_or_else(|| route.payload.render());
            typed::TypedConductorChannel {
                source: route.source.path.name.clone(),
                target: route.target.path.name.clone(),
                payload,
                span: route.span,
            }
        })
        .collect::<Vec<_>>();

    let execution = conductor.execution.as_ref().map(|block| {
        let result = infer_expr(
            &block.body,
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
        typed::TypedConductorBlock {
            ty: solver.substitution().apply(&result.ty).label(),
            span: block.span,
        }
    });

    let monitoring = conductor.monitoring.as_ref().map(|block| {
        let result = infer_expr(
            &block.body,
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
        let target = block.target.as_ref().map(|target| match target {
            ConductorMonitorTarget::Module(ident) => ident.name.clone(),
            ConductorMonitorTarget::Endpoint(endpoint) => endpoint.path.name.clone(),
        });
        typed::TypedConductorMonitoringBlock {
            target,
            ty: solver.substitution().apply(&result.ty).label(),
            span: block.span,
        }
    });

    typed::TypedConductor {
        name: conductor.name.name.clone(),
        span: conductor.span,
        dsl_defs,
        channels,
        execution,
        monitoring,
    }
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
                    value: field
                        .value
                        .as_deref()
                        .map(lower_typed_pattern)
                        .map(Box::new),
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
            argument: argument.as_deref().map(lower_typed_pattern).map(Box::new),
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

fn is_direct_recursion(expr: &Expr, name: &str) -> bool {
    match &expr.kind {
        ExprKind::Rec { expr: inner } => match &inner.kind {
            ExprKind::Identifier(ident) => ident.name == name,
            _ => false,
        },
        _ => false,
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
            StmtKind::Expr { expr } => return classify_active_pattern_return(expr),
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

fn collect_pattern_binding_names(pattern: &Pattern, names: &mut HashSet<String>) {
    match &pattern.kind {
        PatternKind::Var(ident) => {
            names.insert(ident.name.clone());
        }
        PatternKind::Binding { name, pattern, .. } => {
            names.insert(name.name.clone());
            collect_pattern_binding_names(pattern, names);
        }
        PatternKind::Tuple { elements } => {
            for element in elements {
                collect_pattern_binding_names(element, names);
            }
        }
        PatternKind::Record { fields, .. } => {
            for field in fields {
                if let Some(value) = &field.value {
                    collect_pattern_binding_names(value, names);
                } else {
                    names.insert(field.key.name.clone());
                }
            }
        }
        PatternKind::Constructor { args, .. } => {
            for arg in args {
                collect_pattern_binding_names(arg, names);
            }
        }
        PatternKind::Guard { pattern: inner, .. } => {
            collect_pattern_binding_names(inner, names);
        }
        PatternKind::ActivePattern { argument, .. } => {
            if let Some(arg) = argument {
                collect_pattern_binding_names(arg, names);
            }
        }
        PatternKind::Or { variants } => {
            for variant in variants {
                collect_pattern_binding_names(variant, names);
            }
        }
        PatternKind::Slice { elements } => {
            for element in elements {
                match element {
                    SlicePatternItem::Element(pat) => collect_pattern_binding_names(pat, names),
                    SlicePatternItem::Rest { ident: Some(ident) } => {
                        names.insert(ident.name.clone());
                    }
                    SlicePatternItem::Rest { ident: None } => {}
                }
            }
        }
        PatternKind::Range { start, end, .. } => {
            if let Some(start) = start {
                collect_pattern_binding_names(start, names);
            }
            if let Some(end) = end {
                collect_pattern_binding_names(end, names);
            }
        }
        PatternKind::Regex { .. } | PatternKind::Literal(_) | PatternKind::Wildcard => {}
    }
}

fn collect_function_bindings(params: &[Param], body: &Expr) -> HashSet<String> {
    fn walk_expr(expr: &Expr, bindings: &mut HashSet<String>) {
        match &expr.kind {
            ExprKind::Lambda { .. } => {}
            ExprKind::Block { statements, .. } => {
                for stmt in statements {
                    walk_stmt(stmt, bindings);
                }
            }
            ExprKind::Call { callee, args } => {
                walk_expr(callee, bindings);
                for arg in args {
                    walk_expr(arg, bindings);
                }
            }
            ExprKind::PerformCall { call } => walk_expr(&call.argument, bindings),
            ExprKind::InlineAsm(asm) => {
                for output in &asm.outputs {
                    walk_expr(&output.target, bindings);
                }
                for input in &asm.inputs {
                    walk_expr(&input.expr, bindings);
                }
            }
            ExprKind::LlvmIr(ir) => {
                for input in &ir.inputs {
                    walk_expr(input, bindings);
                }
            }
            ExprKind::Pipe { left, right } | ExprKind::Binary { left, right, .. } => {
                walk_expr(left, bindings);
                walk_expr(right, bindings);
            }
            ExprKind::Unary { expr: inner, .. }
            | ExprKind::Rec { expr: inner }
            | ExprKind::Propagate { expr: inner }
            | ExprKind::Return { value: Some(inner) } => walk_expr(inner, bindings),
            ExprKind::Await { expr: inner } => walk_expr(inner, bindings),
            ExprKind::Break { value: Some(inner) } => walk_expr(inner, bindings),
            ExprKind::FieldAccess { target, .. }
            | ExprKind::TupleAccess { target, .. }
            | ExprKind::Index { target, .. } => walk_expr(target, bindings),
            ExprKind::IfElse {
                condition,
                then_branch,
                else_branch,
            } => {
                walk_expr(condition, bindings);
                walk_expr(then_branch, bindings);
                if let Some(branch) = else_branch.as_deref() {
                    walk_expr(branch, bindings);
                }
            }
            ExprKind::Match { target, arms } => {
                walk_expr(target, bindings);
                for arm in arms {
                    collect_pattern_binding_names(&arm.pattern, bindings);
                    if let Some(alias) = &arm.alias {
                        bindings.insert(alias.name.clone());
                    }
                    if let Some(guard) = &arm.guard {
                        walk_expr(guard, bindings);
                    }
                    walk_expr(&arm.body, bindings);
                }
            }
            ExprKind::While { condition, body } => {
                walk_expr(condition, bindings);
                walk_expr(body, bindings);
            }
            ExprKind::For {
                pattern,
                start,
                end,
            } => {
                collect_pattern_binding_names(pattern, bindings);
                walk_expr(start, bindings);
                walk_expr(end, bindings);
            }
            ExprKind::Loop { body } | ExprKind::Unsafe { body } | ExprKind::Defer { body } => {
                walk_expr(body, bindings)
            }
            ExprKind::EffectBlock { body } | ExprKind::Async { body, .. } => {
                walk_expr(body, bindings)
            }
            ExprKind::Assign { target, value } => {
                walk_expr(target, bindings);
                walk_expr(value, bindings);
            }
            ExprKind::Literal(_)
            | ExprKind::FixityLiteral(_)
            | ExprKind::Identifier(_)
            | ExprKind::ModulePath(_)
            | ExprKind::Handle { .. }
            | ExprKind::Break { value: None }
            | ExprKind::Return { value: None }
            | ExprKind::Continue => {}
        }
    }

    fn walk_stmt(stmt: &Stmt, bindings: &mut HashSet<String>) {
        match &stmt.kind {
            StmtKind::Decl { decl } => match &decl.kind {
                DeclKind::Let { pattern, value, .. } | DeclKind::Var { pattern, value, .. } => {
                    walk_expr(value, bindings);
                    collect_pattern_binding_names(pattern, bindings);
                }
                DeclKind::Const { name, value, .. } => {
                    walk_expr(value, bindings);
                    bindings.insert(name.name.clone());
                }
                DeclKind::Fn {
                    signature: FunctionSignature { name, .. },
                }
                | DeclKind::Type {
                    decl: TypeDecl { name, .. },
                }
                | DeclKind::Struct(StructDecl { name, .. })
                | DeclKind::Enum(EnumDecl { name, .. })
                | DeclKind::Trait(TraitDecl { name, .. })
                | DeclKind::Effect(EffectDecl { name, .. })
                | DeclKind::Handler(HandlerDecl { name, .. })
                | DeclKind::Conductor(ConductorDecl { name, .. })
                | DeclKind::Macro(MacroDecl { name, .. })
                | DeclKind::ActorSpec(ActorSpecDecl { name, .. }) => {
                    bindings.insert(name.name.clone());
                }
                _ => {}
            },
            StmtKind::Expr { expr } | StmtKind::Defer { expr } => walk_expr(expr, bindings),
            StmtKind::Assign { target, value } => {
                walk_expr(target, bindings);
                walk_expr(value, bindings);
            }
        }
    }

    let mut bindings = HashSet::new();
    for param in params {
        collect_pattern_binding_names(&param.pattern, &mut bindings);
    }
    walk_expr(body, &mut bindings);
    bindings
}

struct LambdaCaptureReport {
    captures: Vec<(String, Span)>,
    mut_captures: Vec<(String, Span)>,
}

fn detect_lambda_captures(
    body: &Expr,
    params: &[Param],
    function_bindings: &HashSet<String>,
) -> LambdaCaptureReport {
    struct CaptureState<'a> {
        function_bindings: &'a HashSet<String>,
        scopes: Vec<HashSet<String>>,
        captures: HashMap<String, Span>,
        mut_captures: HashMap<String, Span>,
    }

    impl<'a> CaptureState<'a> {
        fn new(function_bindings: &'a HashSet<String>, params: &[Param]) -> Self {
            let mut scopes = Vec::new();
            let mut current = HashSet::new();
            for param in params {
                collect_pattern_binding_names(&param.pattern, &mut current);
            }
            scopes.push(current);
            Self {
                function_bindings,
                scopes,
                captures: HashMap::new(),
                mut_captures: HashMap::new(),
            }
        }

        fn push_scope(&mut self) {
            self.scopes.push(HashSet::new());
        }

        fn pop_scope(&mut self) {
            self.scopes.pop();
        }

        fn is_local(&self, name: &str) -> bool {
            self.scopes.iter().rev().any(|scope| scope.contains(name))
        }

        fn insert_binding(&mut self, name: String) {
            if let Some(scope) = self.scopes.last_mut() {
                scope.insert(name);
            }
        }

        fn record_pattern(&mut self, pattern: &Pattern) {
            let mut names = HashSet::new();
            collect_pattern_binding_names(pattern, &mut names);
            for name in names {
                self.insert_binding(name);
            }
        }

        fn consider_capture(&mut self, ident: &Ident) {
            if self.is_local(ident.name.as_str()) {
                return;
            }
            if !self.function_bindings.contains(ident.name.as_str()) {
                return;
            }
            self.captures
                .entry(ident.name.clone())
                .or_insert(ident.span);
        }

        fn consider_mut_capture(&mut self, ident: &Ident) {
            if self.is_local(ident.name.as_str()) {
                return;
            }
            if !self.function_bindings.contains(ident.name.as_str()) {
                return;
            }
            self.mut_captures
                .entry(ident.name.clone())
                .or_insert(ident.span);
        }
    }

    fn walk_expr(expr: &Expr, state: &mut CaptureState<'_>) {
        match &expr.kind {
            ExprKind::Lambda { .. } => {}
            ExprKind::Identifier(ident) => state.consider_capture(ident),
            ExprKind::Assign { target, value } => {
                if let ExprKind::Identifier(ident) = &target.kind {
                    state.consider_mut_capture(ident);
                } else {
                    walk_expr(target, state);
                }
                walk_expr(value, state);
            }
            ExprKind::Block { statements, .. } => {
                state.push_scope();
                for stmt in statements {
                    walk_stmt(stmt, state);
                }
                state.pop_scope();
            }
            ExprKind::Call { callee, args } => {
                walk_expr(callee, state);
                for arg in args {
                    walk_expr(arg, state);
                }
            }
            ExprKind::PerformCall { call } => walk_expr(&call.argument, state),
            ExprKind::InlineAsm(asm) => {
                for output in &asm.outputs {
                    walk_expr(&output.target, state);
                }
                for input in &asm.inputs {
                    walk_expr(&input.expr, state);
                }
            }
            ExprKind::LlvmIr(ir) => {
                for input in &ir.inputs {
                    walk_expr(input, state);
                }
            }
            ExprKind::Pipe { left, right } | ExprKind::Binary { left, right, .. } => {
                walk_expr(left, state);
                walk_expr(right, state);
            }
            ExprKind::Unary { expr: inner, .. }
            | ExprKind::Rec { expr: inner }
            | ExprKind::Propagate { expr: inner }
            | ExprKind::Return { value: Some(inner) } => walk_expr(inner, state),
            ExprKind::Await { expr: inner } => walk_expr(inner, state),
            ExprKind::Break { value: Some(inner) } => walk_expr(inner, state),
            ExprKind::FieldAccess { target, .. }
            | ExprKind::TupleAccess { target, .. }
            | ExprKind::Index { target, .. } => walk_expr(target, state),
            ExprKind::IfElse {
                condition,
                then_branch,
                else_branch,
            } => {
                walk_expr(condition, state);
                walk_expr(then_branch, state);
                if let Some(branch) = else_branch.as_deref() {
                    walk_expr(branch, state);
                }
            }
            ExprKind::Match { target, arms } => {
                walk_expr(target, state);
                for arm in arms {
                    state.push_scope();
                    state.record_pattern(&arm.pattern);
                    if let Some(alias) = &arm.alias {
                        state.insert_binding(alias.name.clone());
                    }
                    if let Some(guard) = &arm.guard {
                        walk_expr(guard, state);
                    }
                    walk_expr(&arm.body, state);
                    state.pop_scope();
                }
            }
            ExprKind::While { condition, body } => {
                walk_expr(condition, state);
                walk_expr(body, state);
            }
            ExprKind::For {
                pattern,
                start,
                end,
            } => {
                state.record_pattern(pattern);
                walk_expr(start, state);
                walk_expr(end, state);
            }
            ExprKind::Loop { body } | ExprKind::Unsafe { body } | ExprKind::Defer { body } => {
                walk_expr(body, state)
            }
            ExprKind::EffectBlock { body } | ExprKind::Async { body, .. } => walk_expr(body, state),
            ExprKind::Break { value: None }
            | ExprKind::Return { value: None }
            | ExprKind::Continue
            | ExprKind::Literal(_)
            | ExprKind::FixityLiteral(_)
            | ExprKind::ModulePath(_)
            | ExprKind::Handle { .. } => {}
        }
    }

    fn walk_stmt(stmt: &Stmt, state: &mut CaptureState<'_>) {
        match &stmt.kind {
            StmtKind::Decl { decl } => match &decl.kind {
                DeclKind::Let { pattern, value, .. } | DeclKind::Var { pattern, value, .. } => {
                    walk_expr(value, state);
                    state.record_pattern(pattern);
                }
                DeclKind::Const { name, value, .. } => {
                    walk_expr(value, state);
                    state.insert_binding(name.name.clone());
                }
                DeclKind::Fn {
                    signature: FunctionSignature { name, .. },
                }
                | DeclKind::Type {
                    decl: TypeDecl { name, .. },
                }
                | DeclKind::Struct(StructDecl { name, .. })
                | DeclKind::Enum(EnumDecl { name, .. })
                | DeclKind::Trait(TraitDecl { name, .. })
                | DeclKind::Effect(EffectDecl { name, .. })
                | DeclKind::Handler(HandlerDecl { name, .. })
                | DeclKind::Conductor(ConductorDecl { name, .. })
                | DeclKind::Macro(MacroDecl { name, .. })
                | DeclKind::ActorSpec(ActorSpecDecl { name, .. }) => {
                    state.insert_binding(name.name.clone());
                }
                _ => {}
            },
            StmtKind::Expr { expr } | StmtKind::Defer { expr } => walk_expr(expr, state),
            StmtKind::Assign { target, value } => {
                if let ExprKind::Identifier(ident) = &target.kind {
                    state.consider_mut_capture(ident);
                } else {
                    walk_expr(target, state);
                }
                walk_expr(value, state);
            }
        }
    }

    let mut state = CaptureState::new(function_bindings, params);
    walk_expr(body, &mut state);
    LambdaCaptureReport {
        captures: state.captures.into_iter().collect(),
        mut_captures: state.mut_captures.into_iter().collect(),
    }
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

fn build_generic_map_from_names(
    names: &[String],
    var_gen: &mut TypeVarGen,
) -> HashMap<String, TypeVariable> {
    let mut map = HashMap::new();
    for name in names {
        insert_generic(&mut map, name.as_str(), var_gen);
    }
    map
}

fn resolve_type_annot_with_args(
    annot: &TypeAnnot,
    alias_args: Option<&HashMap<String, Type>>,
    env: &TypeEnv,
    violations: &mut Vec<TypecheckViolation>,
) -> Type {
    let mut resolver = TypeAliasResolver::new(env, violations);
    type_from_annotation_kind_with_generics(
        &annot.kind,
        annot.span,
        None,
        alias_args,
        &mut resolver,
    )
    .unwrap_or_else(|| Type::builtin(BuiltinType::Unknown))
}

fn resolve_payload_types(
    payload: &TypeDeclVariantPayload,
    alias_args: Option<&HashMap<String, Type>>,
    env: &TypeEnv,
    violations: &mut Vec<TypecheckViolation>,
) -> Vec<Type> {
    match payload {
        TypeDeclVariantPayload::Tuple { elements } => elements
            .iter()
            .map(|element| resolve_type_annot_with_args(&element.ty, alias_args, env, violations))
            .collect(),
        TypeDeclVariantPayload::Record { fields, .. } => {
            let field_types = fields
                .iter()
                .map(|field| resolve_type_annot_with_args(&field.ty, alias_args, env, violations))
                .collect::<Vec<_>>();
            vec![Type::app("Record", field_types)]
        }
    }
}

fn constructor_expected_arity(payload: Option<&TypeDeclVariantPayload>) -> usize {
    match payload {
        None => 0,
        Some(TypeDeclVariantPayload::Tuple { elements }) => elements.len(),
        Some(TypeDeclVariantPayload::Record { .. }) => 1,
    }
}

fn constructor_type_from_binding(
    binding: &TypeConstructorBinding,
    var_gen: &mut TypeVarGen,
    env: &TypeEnv,
    violations: &mut Vec<TypecheckViolation>,
) -> Type {
    let generic_map = build_generic_map_from_names(&binding.generics, var_gen);
    let generic_map_ref = if generic_map.is_empty() {
        None
    } else {
        Some(&generic_map)
    };
    let mut resolver = TypeAliasResolver::new(env, violations);
    let payload_types = match &binding.payload {
        Some(TypeDeclVariantPayload::Tuple { elements }) => elements
            .iter()
            .map(|element| {
                type_from_annotation_kind_with_generics(
                    &element.ty.kind,
                    element.ty.span,
                    generic_map_ref,
                    None,
                    &mut resolver,
                )
                .unwrap_or_else(|| Type::builtin(BuiltinType::Unknown))
            })
            .collect::<Vec<_>>(),
        Some(TypeDeclVariantPayload::Record { fields, .. }) => {
            let field_types = fields
                .iter()
                .map(|field| {
                    type_from_annotation_kind_with_generics(
                        &field.ty.kind,
                        field.ty.span,
                        generic_map_ref,
                        None,
                        &mut resolver,
                    )
                    .unwrap_or_else(|| Type::builtin(BuiltinType::Unknown))
                })
                .collect::<Vec<_>>();
            vec![Type::app("Record", field_types)]
        }
        None => Vec::new(),
    };
    let parent_args = binding
        .generics
        .iter()
        .filter_map(|name| generic_map.get(name))
        .map(|var| Type::var(*var))
        .collect::<Vec<_>>();
    let parent_ty = Type::app(binding.parent.clone(), parent_args);
    if payload_types.is_empty() {
        parent_ty
    } else {
        Type::arrow(payload_types, parent_ty)
    }
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

fn is_exact_int_type(ty: &Type) -> bool {
    matches!(ty, Type::Builtin(BuiltinType::Int))
        || matches!(
            ty,
            Type::App {
                constructor,
                arguments: _
            } if constructor == "Int"
        )
}

fn merge_intervals(intervals: &[RangeInterval]) -> Vec<RangeInterval> {
    let mut merged = intervals.to_vec();
    merged.sort_by(|left, right| {
        let left_start = left.start.unwrap_or(i64::MIN);
        let right_start = right.start.unwrap_or(i64::MIN);
        left_start
            .cmp(&right_start)
            .then_with(|| left.end.unwrap_or(i64::MAX).cmp(&right.end.unwrap_or(i64::MAX)))
    });
    let mut output: Vec<RangeInterval> = Vec::new();
    for interval in merged {
        if output.is_empty() {
            output.push(interval);
            continue;
        }
        let last = output.last_mut().unwrap();
        if ranges_overlap_or_adjacent(last, &interval) {
            last.end = merge_end(last.end, interval.end);
        } else {
            output.push(interval);
        }
    }
    output
}

fn ranges_overlap_or_adjacent(left: &RangeInterval, right: &RangeInterval) -> bool {
    let left_end = match left.end {
        Some(value) => value,
        None => return true,
    };
    let right_start = right.start.unwrap_or(i64::MIN);
    if left_end == i64::MAX {
        return true;
    }
    left_end.saturating_add(1) >= right_start
}

fn merge_end(left: Option<i64>, right: Option<i64>) -> Option<i64> {
    match (left, right) {
        (None, _) | (_, None) => None,
        (Some(left), Some(right)) => Some(left.max(right)),
    }
}

fn range_covers_all(interval: &RangeInterval) -> bool {
    let covers_low = interval.start.is_none() || interval.start == Some(i64::MIN);
    let covers_high = interval.end.is_none() || interval.end == Some(i64::MAX);
    covers_low && covers_high
}

fn compute_missing_ranges(intervals: &[RangeInterval]) -> Vec<PatternRangeInfo> {
    let mut missing = Vec::new();
    if intervals.is_empty() {
        return missing;
    }
    let first = &intervals[0];
    if let Some(start) = first.start {
        if let Some(end) = start.checked_sub(1) {
            missing.push(make_missing_range(None, Some(end)));
        }
    }
    for window in intervals.windows(2) {
        let current = &window[0];
        let next = &window[1];
        let Some(current_end) = current.end else {
            return missing;
        };
        let next_start = next.start.unwrap_or(i64::MIN);
        let gap_start = current_end.checked_add(1);
        let gap_end = next_start.checked_sub(1);
        if let (Some(start), Some(end)) = (gap_start, gap_end) {
            if start <= end {
                missing.push(make_missing_range(Some(start), Some(end)));
            }
        }
    }
    let last = intervals.last().unwrap();
    if let Some(end) = last.end {
        if let Some(start) = end.checked_add(1) {
            missing.push(make_missing_range(Some(start), None));
        }
    }
    missing
}

fn make_missing_range(start: Option<i64>, end: Option<i64>) -> PatternRangeInfo {
    PatternRangeInfo {
        start: start.map(|value| value.to_string()).unwrap_or_else(|| "-inf".to_string()),
        end: end.map(|value| value.to_string()).unwrap_or_else(|| "+inf".to_string()),
        inclusive: true,
    }
}

fn array_element_type(target_ty: &Type) -> Option<Type> {
    match target_ty {
        Type::Slice { element } => Some(element.as_ref().clone()),
        Type::App {
            constructor,
            arguments,
        } if constructor == "Array" && arguments.len() == 1 => Some(arguments[0].clone()),
        Type::App {
            constructor,
            arguments,
        } if constructor == "Slice" && arguments.len() == 1 => Some(arguments[0].clone()),
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

#[derive(Debug, Clone)]
struct IntLiteralInfo {
    value: i64,
    raw: String,
}

fn int_literal_info(pattern: Option<&Pattern>) -> Option<IntLiteralInfo> {
    match pattern {
        Some(Pattern {
            kind:
                PatternKind::Literal(Literal {
                    value: LiteralKind::Int { value, raw, .. },
                }),
            ..
        }) => Some(IntLiteralInfo {
            value: *value,
            raw: raw.clone(),
        }),
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
    if let (Some(start_info), Some(end_info)) =
        (int_literal_info(start), int_literal_info(end))
    {
        if start_info.value > end_info.value {
            if let PatternKind::Range { inclusive, .. } = &pattern.kind {
                violations.push(TypecheckViolation::pattern_range_bound_inverted(
                    pattern.span,
                    start_info,
                    end_info,
                    *inclusive,
                ));
            }
        }
    }
}

fn validate_slice_pattern(
    pattern: &Pattern,
    elements: &[SlicePatternItem],
    target_ty: &Type,
    env: &TypeEnv,
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
            validate_pattern_against_type(inner, &element_ty, env, violations);
        }
    }
}

fn validate_pattern_against_type(
    pattern: &Pattern,
    target_ty: &Type,
    env: &TypeEnv,
    violations: &mut Vec<TypecheckViolation>,
) {
    match &pattern.kind {
        PatternKind::Or { variants } => {
            for variant in variants {
                validate_pattern_against_type(variant, target_ty, env, violations);
            }
        }
        PatternKind::Binding { pattern: inner, .. } => {
            validate_pattern_against_type(inner, target_ty, env, violations);
        }
        PatternKind::Guard { pattern: inner, .. } => {
            validate_pattern_against_type(inner, target_ty, env, violations);
        }
        PatternKind::Slice { elements } => {
            validate_slice_pattern(pattern, elements, target_ty, env, violations);
        }
        PatternKind::Range { .. } => {
            validate_range_pattern(pattern, target_ty, violations);
        }
        PatternKind::Constructor { name, args, .. } => {
            if let Type::App {
                constructor,
                arguments,
            } = target_ty
            {
                if let Some(binding) = env.lookup_type_constructor(name.name.as_str()) {
                    if binding.parent == *constructor {
                        let mut alias_args = HashMap::new();
                        for (param, arg) in binding.generics.iter().zip(arguments.iter()) {
                            alias_args.insert(param.clone(), arg.clone());
                        }
                        let payload_types = binding
                            .payload
                            .as_ref()
                            .map(|payload| {
                                resolve_payload_types(payload, Some(&alias_args), env, violations)
                            })
                            .unwrap_or_default();
                        let expected = payload_types.len();
                        if expected != args.len() {
                            violations.push(TypecheckViolation::constructor_arity_mismatch(
                                pattern.span,
                                name.name.as_str(),
                                expected,
                                args.len(),
                            ));
                        }
                        if expected == args.len() {
                            for (arg, arg_ty) in args.iter().zip(payload_types.iter()) {
                                validate_pattern_against_type(arg, arg_ty, env, violations);
                            }
                        } else {
                            for arg in args {
                                validate_pattern_against_type(
                                    arg,
                                    &Type::builtin(BuiltinType::Unknown),
                                    env,
                                    violations,
                                );
                            }
                        }
                        return;
                    }
                }
            }
            if let Some(inner_ty) = option_inner_type(target_ty) {
                match name.name.as_str() {
                    "Some" => {
                        if let Some(arg) = args.get(0) {
                            validate_pattern_against_type(arg, &inner_ty, env, violations);
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
                            validate_pattern_against_type(arg, &ok_ty, env, violations);
                        }
                        return;
                    }
                    "Err" => {
                        if let Some(arg) = args.get(0) {
                            validate_pattern_against_type(arg, &err_ty, env, violations);
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
                    env,
                    violations,
                );
            }
        }
        PatternKind::Tuple { elements } => {
            for element in elements {
                validate_pattern_against_type(
                    element,
                    &Type::builtin(BuiltinType::Unknown),
                    env,
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
                        env,
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
                    env,
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
    should_report_missing: bool,
    unreachable_arm_indices: Vec<usize>,
    missing_variants: Option<Vec<String>>,
    missing_ranges: Option<Vec<PatternRangeInfo>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ExhaustivenessDomain {
    Unknown,
    Bool,
    OptionLike,
    Slice,
    Sum,
}

#[derive(Debug, Clone)]
struct RangeInterval {
    start: Option<i64>,
    end: Option<i64>,
}

#[derive(Default)]
struct RangeCoverageTracker {
    active: bool,
    has_range_pattern: bool,
    has_unknown_bound: bool,
    has_wildcard: bool,
    intervals: Vec<RangeInterval>,
}

struct RangeCoverageResult {
    has_domain: bool,
    coverage_reached: bool,
    missing_ranges: Option<Vec<PatternRangeInfo>>,
}

impl RangeCoverageTracker {
    fn new(target_ty: &Type) -> Self {
        Self {
            active: is_exact_int_type(target_ty),
            ..Self::default()
        }
    }

    fn observe_pattern(&mut self, pattern: &Pattern) {
        if !self.active {
            return;
        }
        self.collect_from_pattern(pattern);
    }

    fn collect_from_pattern(&mut self, pattern: &Pattern) {
        match &pattern.kind {
            PatternKind::Wildcard | PatternKind::Var(_) => {
                self.has_wildcard = true;
            }
            PatternKind::Binding { pattern: inner, .. } => {
                self.collect_from_pattern(inner);
            }
            PatternKind::Guard { pattern: inner, .. } => {
                self.collect_from_pattern(inner);
            }
            PatternKind::Or { variants } => {
                for variant in variants {
                    self.collect_from_pattern(variant);
                }
            }
            PatternKind::Range {
                start,
                end,
                inclusive,
            } => {
                self.has_range_pattern = true;
                if start.is_none() && end.is_none() {
                    self.has_wildcard = true;
                    return;
                }
                let start_info = int_literal_info(start.as_deref());
                let end_info = int_literal_info(end.as_deref());
                if (start.is_some() && start_info.is_none())
                    || (end.is_some() && end_info.is_none())
                {
                    self.has_unknown_bound = true;
                    return;
                }
                let mut end_value = end_info.as_ref().map(|info| info.value);
                if let (Some(value), false) = (end_value, *inclusive) {
                    end_value = value.checked_sub(1);
                    if end_value.is_none() {
                        return;
                    }
                }
                self.push_interval(start_info.map(|info| info.value), end_value);
            }
            PatternKind::Literal(Literal {
                value: LiteralKind::Int { value, .. },
            }) => {
                self.has_range_pattern = true;
                self.push_interval(Some(*value), Some(*value));
            }
            _ => {}
        }
    }

    fn push_interval(&mut self, start: Option<i64>, end: Option<i64>) {
        if let (Some(start), Some(end)) = (start, end) {
            if start > end {
                return;
            }
        }
        self.intervals.push(RangeInterval { start, end });
    }

    fn result(&self) -> RangeCoverageResult {
        if !self.active || !self.has_range_pattern || self.has_wildcard {
            return RangeCoverageResult {
                has_domain: self.active && self.has_range_pattern,
                coverage_reached: self.has_wildcard,
                missing_ranges: None,
            };
        }
        if self.has_unknown_bound || self.intervals.is_empty() {
            return RangeCoverageResult {
                has_domain: true,
                coverage_reached: false,
                missing_ranges: None,
            };
        }
        let merged = merge_intervals(&self.intervals);
        let coverage_reached = merged.len() == 1 && range_covers_all(&merged[0]);
        let missing_ranges = if coverage_reached {
            None
        } else {
            Some(compute_missing_ranges(&merged))
        };
        RangeCoverageResult {
            has_domain: true,
            coverage_reached,
            missing_ranges,
        }
    }
}

fn sum_constructors_for_type(env: &TypeEnv, target_ty: &Type) -> Option<HashSet<String>> {
    let Type::App { constructor, .. } = target_ty else {
        return None;
    };
    let decl = env.lookup_type_decl(constructor.as_str())?;
    if decl.kind != TypeDeclKind::Sum {
        return None;
    }
    let TypeDeclBody::Sum { variants } = decl.body.as_ref()? else {
        return None;
    };
    let constructors = variants
        .iter()
        .map(|variant| variant.name.name.clone())
        .collect::<HashSet<_>>();
    Some(constructors)
}

fn exhaustiveness_domain_for_type(
    env: &TypeEnv,
    target_ty: &Type,
) -> (ExhaustivenessDomain, Option<HashSet<String>>) {
    if let Some(constructors) = sum_constructors_for_type(env, target_ty) {
        return (ExhaustivenessDomain::Sum, Some(constructors));
    }
    let domain = match target_ty {
        Type::Builtin(BuiltinType::Bool) => ExhaustivenessDomain::Bool,
        Type::App { constructor, .. } if constructor.as_str() == "Option" => {
            ExhaustivenessDomain::OptionLike
        }
        Type::App { constructor, .. } if constructor.as_str() == "Result" => {
            ExhaustivenessDomain::OptionLike
        }
        Type::App { constructor, .. } if constructor.as_str() == "Array" => {
            ExhaustivenessDomain::Slice
        }
        Type::App { constructor, .. } if constructor.as_str() == "Slice" => {
            ExhaustivenessDomain::Slice
        }
        Type::Slice { .. } => ExhaustivenessDomain::Slice,
        _ => ExhaustivenessDomain::Unknown,
    };
    (domain, None)
}

fn analyze_match_exhaustiveness(
    arms: &[MatchArm],
    target_ty: &Type,
    env: &TypeEnv,
) -> ExhaustivenessResult {
    let (domain, sum_constructors) = exhaustiveness_domain_for_type(env, target_ty);
    let mut tracker = ExhaustivenessTracker::new(domain, sum_constructors);
    let mut range_tracker = RangeCoverageTracker::new(target_ty);
    let mut unreachable_arm_indices = Vec::new();
    let mut has_partial_active_pattern = false;
    for (idx, arm) in arms.iter().enumerate() {
        if contains_partial_active_pattern(&arm.pattern) {
            has_partial_active_pattern = true;
        }
        if tracker.coverage_reached() && arm.guard.is_none() {
            unreachable_arm_indices.push(idx);
            continue;
        }
        tracker.observe_arm(arm);
        if arm.guard.is_none() {
            range_tracker.observe_pattern(&arm.pattern);
        }
    }
    let range_result = range_tracker.result();
    let coverage_reached = tracker.coverage_reached() || range_result.coverage_reached;
    let should_report_missing = domain != ExhaustivenessDomain::Unknown
        || has_partial_active_pattern
        || range_result.has_domain;
    let missing_variants = if domain == ExhaustivenessDomain::Sum && !coverage_reached {
        tracker
            .sum_constructors
            .as_ref()
            .map(|constructors| {
                let mut missing = constructors
                    .difference(&tracker.sum_seen)
                    .cloned()
                    .collect::<Vec<_>>();
                missing.sort();
                if missing.is_empty() {
                    None
                } else {
                    Some(missing)
                }
            })
            .unwrap_or(None)
    } else {
        None
    };
    let missing_ranges = if range_result.has_domain && !coverage_reached {
        range_result.missing_ranges
    } else {
        None
    };
    ExhaustivenessResult {
        coverage_reached,
        should_report_missing,
        unreachable_arm_indices,
        missing_variants,
        missing_ranges,
    }
}

fn contains_partial_active_pattern(pattern: &Pattern) -> bool {
    match &pattern.kind {
        PatternKind::ActivePattern {
            is_partial,
            argument,
            ..
        } => {
            if *is_partial {
                true
            } else {
                argument
                    .as_ref()
                    .map(|inner| contains_partial_active_pattern(inner))
                    .unwrap_or(false)
            }
        }
        PatternKind::Binding { pattern, .. } | PatternKind::Guard { pattern, .. } => {
            contains_partial_active_pattern(pattern)
        }
        PatternKind::Tuple { elements } => elements
            .iter()
            .any(|element| contains_partial_active_pattern(element)),
        PatternKind::Record { fields, .. } => fields.iter().any(|field| {
            field
                .value
                .as_ref()
                .map(|value| contains_partial_active_pattern(value))
                .unwrap_or(false)
        }),
        PatternKind::Constructor { args, .. } => {
            args.iter().any(|arg| contains_partial_active_pattern(arg))
        }
        PatternKind::Or { variants } => variants
            .iter()
            .any(|variant| contains_partial_active_pattern(variant)),
        PatternKind::Slice { elements } => elements.iter().any(|element| match element {
            SlicePatternItem::Element(inner) => contains_partial_active_pattern(inner),
            SlicePatternItem::Rest { .. } => false,
        }),
        PatternKind::Range { start, end, .. } => {
            start
                .as_ref()
                .map(|inner| contains_partial_active_pattern(inner))
                .unwrap_or(false)
                || end
                    .as_ref()
                    .map(|inner| contains_partial_active_pattern(inner))
                    .unwrap_or(false)
        }
        PatternKind::Wildcard
        | PatternKind::Var(_)
        | PatternKind::Literal(_)
        | PatternKind::Regex { .. } => false,
    }
}

struct ExhaustivenessTracker {
    domain: ExhaustivenessDomain,
    wildcard_covered: bool,
    bool_true_seen: bool,
    bool_false_seen: bool,
    option_some_seen: bool,
    option_none_seen: bool,
    slice_empty_seen: bool,
    slice_rest_seen: bool,
    sum_constructors: Option<HashSet<String>>,
    sum_seen: HashSet<String>,
}

impl ExhaustivenessTracker {
    fn new(domain: ExhaustivenessDomain, sum_constructors: Option<HashSet<String>>) -> Self {
        Self {
            domain,
            wildcard_covered: false,
            bool_true_seen: false,
            bool_false_seen: false,
            option_some_seen: false,
            option_none_seen: false,
            slice_empty_seen: false,
            slice_rest_seen: false,
            sum_constructors,
            sum_seen: HashSet::new(),
        }
    }

    fn observe_arm(&mut self, arm: &MatchArm) {
        if arm.guard.is_some() {
            return;
        }
        self.observe_pattern(&arm.pattern, false);
    }

    fn is_total_like(&self, pattern: &Pattern) -> bool {
        match &pattern.kind {
            PatternKind::Wildcard | PatternKind::Var(_) => true,
            PatternKind::Binding { pattern: inner, .. } => self.is_total_like(inner),
            PatternKind::ActivePattern { is_partial, .. } => !*is_partial,
            PatternKind::Range { start, end, .. } => start.is_none() && end.is_none(),
            PatternKind::Tuple { elements } => {
                elements.iter().all(|element| self.is_total_like(element))
            }
            _ => false,
        }
    }

    fn observe_pattern(&mut self, pattern: &Pattern, has_guard: bool) {
        if has_guard || self.wildcard_covered {
            return;
        }
        match &pattern.kind {
            PatternKind::Wildcard | PatternKind::Var(_) => {
                self.wildcard_covered = true;
            }
            PatternKind::Tuple { elements } => {
                if elements.iter().all(|element| self.is_total_like(element)) {
                    self.wildcard_covered = true;
                }
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
                if self.domain == ExhaustivenessDomain::Bool {
                    if *value {
                        self.bool_true_seen = true;
                    } else {
                        self.bool_false_seen = true;
                    }
                }
            }
            PatternKind::Constructor { name, .. } => {
                match name.name.as_str() {
                    "Some" | "Ok" => {
                        if self.domain == ExhaustivenessDomain::OptionLike {
                            self.option_some_seen = true;
                        }
                    }
                    "None" | "Err" => {
                        if self.domain == ExhaustivenessDomain::OptionLike {
                            self.option_none_seen = true;
                        }
                    }
                    _ => {}
                }
                if self.domain == ExhaustivenessDomain::Sum {
                    if let Some(constructors) = &self.sum_constructors {
                        if constructors.contains(name.name.as_str()) {
                            self.sum_seen.insert(name.name.clone());
                        }
                    }
                }
            }
            PatternKind::ActivePattern { is_partial, .. } => {
                if !*is_partial {
                    self.wildcard_covered = true;
                }
            }
            PatternKind::Guard { pattern, .. } => {
                self.observe_pattern(pattern, true);
            }
            PatternKind::Slice { elements } => {
                if self.domain != ExhaustivenessDomain::Slice {
                    return;
                }
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
        if self.wildcard_covered {
            return true;
        }
        match self.domain {
            ExhaustivenessDomain::Bool => self.bool_true_seen && self.bool_false_seen,
            ExhaustivenessDomain::OptionLike => self.option_some_seen && self.option_none_seen,
            ExhaustivenessDomain::Slice => self.slice_empty_seen && self.slice_rest_seen,
            ExhaustivenessDomain::Sum => self
                .sum_constructors
                .as_ref()
                .map(|constructors| !constructors.is_empty() && self.sum_seen == *constructors)
                .unwrap_or(false),
            ExhaustivenessDomain::Unknown => false,
        }
    }
}

const TYPE_ALIAS_EXPANSION_LIMIT: usize = 32;

struct TypeAliasResolver<'a> {
    env: &'a TypeEnv,
    stack: Vec<String>,
    max_depth: usize,
    violations: &'a mut Vec<TypecheckViolation>,
}

impl<'a> TypeAliasResolver<'a> {
    fn new(env: &'a TypeEnv, violations: &'a mut Vec<TypecheckViolation>) -> Self {
        Self {
            env,
            stack: Vec::new(),
            max_depth: TYPE_ALIAS_EXPANSION_LIMIT,
            violations,
        }
    }

    fn report_unresolved(&mut self, span: Span, name: &str) {
        self.violations
            .push(TypecheckViolation::type_unresolved_ident(span, name));
    }

    fn enter_alias(&mut self, name: &Ident, span: Span) -> bool {
        if self.stack.iter().any(|entry| entry == name.name.as_str()) {
            if name.name == "Never" {
                return false;
            }
            let mut chain = self.stack.clone();
            chain.push(name.name.clone());
            self.violations
                .push(TypecheckViolation::type_alias_cycle(span, chain));
            return false;
        }
        if self.stack.len() >= self.max_depth {
            self.violations
                .push(TypecheckViolation::type_alias_expansion_limit(
                    span,
                    name.name.as_str(),
                    self.max_depth,
                ));
            return false;
        }
        self.stack.push(name.name.clone());
        true
    }

    fn exit_alias(&mut self) {
        self.stack.pop();
    }
}

fn type_from_annotation(
    annot: &TypeAnnot,
    generics: Option<&HashMap<String, TypeVariable>>,
    env: &TypeEnv,
    violations: &mut Vec<TypecheckViolation>,
) -> Option<Type> {
    let mut resolver = TypeAliasResolver::new(env, violations);
    type_from_annotation_kind_with_generics(&annot.kind, annot.span, generics, None, &mut resolver)
}

fn is_builtin_type_constructor(name: &str) -> bool {
    matches!(
        name,
        "Option" | "Result" | "Array" | "Slice" | "Record" | "Tuple"
    )
}

fn type_from_annotation_kind_with_generics(
    kind: &TypeKind,
    span: Span,
    generics: Option<&HashMap<String, TypeVariable>>,
    alias_args: Option<&HashMap<String, Type>>,
    resolver: &mut TypeAliasResolver<'_>,
) -> Option<Type> {
    match kind {
        TypeKind::Ident { name } => {
            let literal = match name.name.as_str() {
                "Int" => Some(Type::builtin(BuiltinType::Int)),
                "UInt" | "u32" | "usize" => Some(Type::builtin(BuiltinType::UInt)),
                "Float" | "f64" => Some(Type::builtin(BuiltinType::Float)),
                "Bool" => Some(Type::builtin(BuiltinType::Bool)),
                "Char" | "char" => Some(Type::builtin(BuiltinType::Char)),
                "Str" => Some(Type::builtin(BuiltinType::Str)),
                "Bytes" => Some(Type::builtin(BuiltinType::Bytes)),
                _ => None,
            };
            if literal.is_some() {
                return literal;
            }
            if name.name == "Self" || name.name.starts_with("Self::") {
                return Some(Type::app(name.name.clone(), Vec::new()));
            }
            if let Some(map) = alias_args {
                if let Some(ty) = map.get(name.name.as_str()) {
                    return Some(ty.clone());
                }
            }
            if let Some(var) = generics.and_then(|map| map.get(name.name.as_str())) {
                return Some(Type::var(*var));
            }
            if let Some(binding) = resolver.env.lookup_type_decl(name.name.as_str()) {
                return expand_type_alias(name, span, Vec::new(), generics, binding, resolver);
            }
            resolver.report_unresolved(name.span, name.name.as_str());
            Some(Type::app(name.name.clone(), Vec::new()))
        }
        TypeKind::Literal { value } => match value {
            TypeLiteral::String { .. } => Some(Type::builtin(BuiltinType::Str)),
            TypeLiteral::Int { .. } => Some(Type::builtin(BuiltinType::Int)),
        },
        TypeKind::App { callee, args } => {
            let mut resolved_args = Vec::new();
            for arg in args {
                if let Some(arg_ty) = type_from_annotation_kind_with_generics(
                    &arg.kind, arg.span, generics, alias_args, resolver,
                ) {
                    resolved_args.push(arg_ty);
                } else {
                    return None;
                }
            }
            if let Some(binding) = resolver.env.lookup_type_decl(callee.name.as_str()) {
                return expand_type_alias(callee, span, resolved_args, generics, binding, resolver);
            }
            if !is_builtin_type_constructor(callee.name.as_str()) {
                resolver.report_unresolved(callee.span, callee.name.as_str());
            }
            Some(Type::app(callee.name.clone(), resolved_args))
        }
        TypeKind::Union { variants } => {
            let mut resolved = Vec::new();
            for variant in variants {
                let ty = match variant {
                    TypeUnionVariant::Type { ty } => type_from_annotation_kind_with_generics(
                        &ty.kind, ty.span, generics, alias_args, resolver,
                    ),
                    TypeUnionVariant::Variant { .. } => None,
                };
                if let Some(ty) = ty {
                    resolved.push(ty);
                } else {
                    return None;
                }
            }
            let first_builtin = resolved.iter().find_map(|ty| {
                if let Type::Builtin(builtin) = ty {
                    Some(*builtin)
                } else {
                    None
                }
            });
            if let Some(builtin) = first_builtin {
                if resolved
                    .iter()
                    .all(|ty| matches!(ty, Type::Builtin(next) if *next == builtin))
                {
                    Some(Type::builtin(builtin))
                } else {
                    None
                }
            } else {
                None
            }
        }
        TypeKind::Slice { element } => type_from_annotation_kind_with_generics(
            &element.kind,
            element.span,
            generics,
            alias_args,
            resolver,
        )
        .map(Type::slice),
        TypeKind::Array { element, .. } => type_from_annotation_kind_with_generics(
            &element.kind,
            element.span,
            generics,
            alias_args,
            resolver,
        )
        .map(|inner| Type::app("Array", vec![inner])),
        TypeKind::Ref { target, mutable } => type_from_annotation_kind_with_generics(
            &target.kind,
            target.span,
            generics,
            alias_args,
            resolver,
        )
        .map(|inner| Type::reference(inner, *mutable)),
        TypeKind::Fn {
            params,
            param_labels: _,
            ret,
        } => {
            let mut resolved_params = Vec::new();
            for param in params {
                if let Some(param_ty) = type_from_annotation_kind_with_generics(
                    &param.kind,
                    param.span,
                    generics,
                    alias_args,
                    resolver,
                ) {
                    resolved_params.push(param_ty);
                } else {
                    return None;
                }
            }
            let resolved_ret = type_from_annotation_kind_with_generics(
                &ret.kind, ret.span, generics, alias_args, resolver,
            )?;
            Some(Type::arrow(resolved_params, resolved_ret))
        }
        TypeKind::Tuple { elements } => {
            let mut resolved = Vec::new();
            for element in elements {
                if let Some(ty) = type_from_annotation_kind_with_generics(
                    &element.ty.kind,
                    element.ty.span,
                    generics,
                    alias_args,
                    resolver,
                ) {
                    resolved.push(ty);
                } else {
                    return None;
                }
            }
            Some(Type::app("Tuple", resolved))
        }
        TypeKind::Record { fields } => {
            let mut resolved = Vec::new();
            for field in fields {
                if let Some(ty) = type_from_annotation_kind_with_generics(
                    &field.ty.kind,
                    field.ty.span,
                    generics,
                    alias_args,
                    resolver,
                ) {
                    resolved.push(ty);
                } else {
                    return None;
                }
            }
            Some(Type::app("Record", resolved))
        }
    }
}

fn expand_type_alias(
    name: &Ident,
    span: Span,
    args: Vec<Type>,
    generics: Option<&HashMap<String, TypeVariable>>,
    binding: &TypeDeclBinding,
    resolver: &mut TypeAliasResolver<'_>,
) -> Option<Type> {
    if binding.kind != TypeDeclKind::Alias {
        return Some(Type::app(name.name.clone(), args));
    }
    let Some(TypeDeclBody::Alias { ty }) = binding.body.as_ref() else {
        return Some(Type::app(name.name.clone(), args));
    };
    if binding.generics.len() != args.len() {
        return Some(Type::app(name.name.clone(), args));
    }
    if !resolver.enter_alias(name, span) {
        return Some(Type::app(name.name.clone(), args));
    }
    let mut alias_map = HashMap::new();
    for (param, arg) in binding.generics.iter().zip(args.iter()) {
        alias_map.insert(param.clone(), arg.clone());
    }
    let resolved = type_from_annotation_kind_with_generics(
        &ty.kind,
        ty.span,
        generics,
        Some(&alias_map),
        resolver,
    );
    resolver.exit_alias();
    resolved
}

fn insert_generic(map: &mut HashMap<String, TypeVariable>, name: &str, var_gen: &mut TypeVarGen) {
    if map.contains_key(name) {
        return;
    }
    map.insert(name.to_string(), var_gen.next());
}

fn build_generic_map(
    generics: &[Ident],
    var_gen: &mut TypeVarGen,
) -> HashMap<String, TypeVariable> {
    let mut map = HashMap::new();
    for ident in generics {
        insert_generic(&mut map, ident.name.as_str(), var_gen);
    }
    map
}

fn collect_type_param_names_from_annotation(annotation: &TypeAnnot) -> Vec<String> {
    fn visit(kind: &TypeKind, is_root: bool, seen: &mut HashSet<String>, names: &mut Vec<String>) {
        match kind {
            TypeKind::Ident { name } => {
                if is_root {
                    return;
                }
                let name = name.name.as_str();
                if matches!(
                    name,
                    "Int"
                        | "UInt"
                        | "u32"
                        | "usize"
                        | "Float"
                        | "f64"
                        | "Bool"
                        | "Char"
                        | "Str"
                        | "Bytes"
                ) {
                    return;
                }
                if seen.insert(name.to_string()) {
                    names.push(name.to_string());
                }
            }
            TypeKind::App { args, .. } => {
                for arg in args {
                    visit(&arg.kind, false, seen, names);
                }
            }
            TypeKind::Tuple { elements } => {
                for element in elements {
                    visit(&element.ty.kind, false, seen, names);
                }
            }
            TypeKind::Record { fields } => {
                for field in fields {
                    visit(&field.ty.kind, false, seen, names);
                }
            }
            TypeKind::Slice { element } => visit(&element.kind, false, seen, names),
            TypeKind::Array { element, .. } => visit(&element.kind, false, seen, names),
            TypeKind::Ref { target, .. } => visit(&target.kind, false, seen, names),
            TypeKind::Fn {
                params,
                param_labels: _,
                ret,
            } => {
                for param in params {
                    visit(&param.kind, false, seen, names);
                }
                visit(&ret.kind, false, seen, names);
            }
            TypeKind::Union { variants } => {
                for variant in variants {
                    match variant {
                        TypeUnionVariant::Type { ty } => {
                            visit(&ty.kind, false, seen, names);
                        }
                        TypeUnionVariant::Variant {
                            payload: Some(payload),
                            ..
                        } => match payload {
                            VariantPayload::Record { fields } => {
                                for field in fields {
                                    visit(&field.ty.kind, false, seen, names);
                                }
                            }
                            VariantPayload::Tuple { elements } => {
                                for element in elements {
                                    visit(&element.ty.kind, false, seen, names);
                                }
                            }
                        },
                        TypeUnionVariant::Variant { .. } => {}
                    }
                }
            }
            _ => {}
        }
    }

    let mut seen = HashSet::new();
    let mut names = Vec::new();
    visit(&annotation.kind, true, &mut seen, &mut names);
    names
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

#[derive(Clone)]
struct TypedExprDraft {
    span: Span,
    kind: TypedExprKindDraft,
    ty: Type,
    dict_ref_ids: Vec<typed::DictRefId>,
}

#[derive(Clone)]
struct TypedStmtDraft {
    span: Span,
    kind: TypedStmtKindDraft,
}

#[derive(Clone)]
enum TypedStmtKindDraft {
    Let {
        pattern: typed::TypedPattern,
        value: Box<TypedExprDraft>,
    },
    Var {
        pattern: typed::TypedPattern,
        value: Box<TypedExprDraft>,
    },
    Expr {
        expr: Box<TypedExprDraft>,
    },
    Assign {
        target: Box<TypedExprDraft>,
        value: Box<TypedExprDraft>,
    },
    Defer {
        expr: Box<TypedExprDraft>,
    },
}

#[derive(Clone)]
enum TypedExprKindDraft {
    Literal(Literal),
    Identifier {
        ident: Ident,
    },
    FieldAccess {
        target: Box<TypedExprDraft>,
        field: Ident,
    },
    TupleAccess {
        target: Box<TypedExprDraft>,
        index: u32,
    },
    Index {
        target: Box<TypedExprDraft>,
        index: Box<TypedExprDraft>,
    },
    Block {
        statements: Vec<TypedStmtDraft>,
        tail: Option<Box<TypedExprDraft>>,
        defers: Vec<TypedExprDraft>,
    },
    Return {
        value: Option<Box<TypedExprDraft>>,
    },
    Propagate {
        expr: Box<TypedExprDraft>,
    },
    Match {
        target: Box<TypedExprDraft>,
        arms: Vec<TypedMatchArmDraft>,
    },
    Call {
        callee: Box<TypedExprDraft>,
        args: Vec<TypedExprDraft>,
        qualified: Option<typed::QualifiedCall>,
    },
    Lambda {
        params: Vec<ParamBinding>,
        return_annotation: Option<String>,
        body: Box<TypedExprDraft>,
        captures: Vec<typed::TypedLambdaCapture>,
    },
    Binary {
        operator: String,
        left: Box<TypedExprDraft>,
        right: Box<TypedExprDraft>,
    },
    PerformCall {
        call: TypedEffectCallDraft,
    },
    EffectBlock {
        body: Box<TypedExprDraft>,
    },
    Async {
        body: Box<TypedExprDraft>,
        is_move: bool,
    },
    Await {
        expr: Box<TypedExprDraft>,
    },
    Unsafe {
        body: Box<TypedExprDraft>,
    },
    InlineAsm {
        template: String,
        outputs: Vec<InlineAsmOutputDraft>,
        inputs: Vec<InlineAsmInputDraft>,
        clobbers: Vec<String>,
        options: Vec<String>,
    },
    LlvmIr {
        result_type: String,
        template: String,
        inputs: Vec<TypedExprDraft>,
    },
    IfElse {
        condition: Box<TypedExprDraft>,
        then_branch: Box<TypedExprDraft>,
        else_branch: Box<TypedExprDraft>,
    },
    Rec {
        target: Box<TypedExprDraft>,
        ident: Option<Ident>,
    },
    Unknown,
}

#[derive(Clone)]
struct TypedEffectCallDraft {
    effect: Ident,
    argument: Box<TypedExprDraft>,
}

#[derive(Clone)]
struct InlineAsmOutputDraft {
    constraint: String,
    target: Box<TypedExprDraft>,
}

#[derive(Clone)]
struct InlineAsmInputDraft {
    constraint: String,
    expr: Box<TypedExprDraft>,
}

#[derive(Clone)]
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

#[derive(Clone)]
struct ParamBinding {
    display: String,
    span: Span,
    ty: Type,
    annotation: Option<String>,
}

struct IntrinsicAttribute {
    name: String,
    span: Span,
}

fn extract_intrinsic_attr(attrs: &[Attribute]) -> Option<IntrinsicAttribute> {
    for attr in attrs {
        if attr.name.name != "intrinsic" {
            continue;
        }
        if attr.args.len() != 1 {
            continue;
        }
        if let ExprKind::Literal(Literal {
            value: LiteralKind::String { value, .. },
        }) = &attr.args[0].kind
        {
            return Some(IntrinsicAttribute {
                name: value.clone(),
                span: attr.span,
            });
        }
    }
    None
}

fn intrinsic_attribute_strings(attrs: &[Attribute]) -> Vec<String> {
    let mut values = Vec::new();
    for attr in attrs {
        if attr.name.name != "intrinsic" {
            continue;
        }
        if attr.args.len() != 1 {
            continue;
        }
        if let ExprKind::Literal(Literal {
            value: LiteralKind::String { value, .. },
        }) = &attr.args[0].kind
        {
            values.push(format!("intrinsic:{value}"));
        }
    }
    values
}

fn unstable_attribute_strings(attrs: &[Attribute]) -> Vec<String> {
    let mut values = Vec::new();
    for attr in attrs {
        if attr.name.name != "unstable" {
            continue;
        }
        if attr.args.len() != 1 {
            continue;
        }
        if let ExprKind::Literal(Literal {
            value: LiteralKind::String { value, .. },
        }) = &attr.args[0].kind
        {
            match value.as_str() {
                "inline_asm" => values.push("unstable:inline_asm".to_string()),
                "llvm_ir" => values.push("unstable:llvm_ir".to_string()),
                _ => {}
            }
        }
    }
    values
}

fn function_attribute_strings(attrs: &[Attribute]) -> Vec<String> {
    let mut values = intrinsic_attribute_strings(attrs);
    values.extend(unstable_attribute_strings(attrs));
    values
}

fn effect_has_native(effect: &Option<EffectAnnotation>) -> bool {
    match effect {
        Some(annotation) => annotation.tags.iter().any(|tag| tag.name == "native"),
        None => false,
    }
}

fn first_intrinsic_invalid_type(
    params: &[ParamBinding],
    return_ty: &Type,
    substitution: &Substitution,
) -> Option<String> {
    for binding in params {
        let resolved = substitution.apply(&binding.ty);
        if !intrinsic_type_allowed(&resolved) {
            return Some(resolved.label());
        }
    }
    if !intrinsic_type_allowed(return_ty) {
        return Some(return_ty.label());
    }
    None
}

fn intrinsic_type_allowed(ty: &Type) -> bool {
    match ty {
        Type::Builtin(BuiltinType::Int)
        | Type::Builtin(BuiltinType::Bool)
        | Type::Builtin(BuiltinType::Unit) => true,
        Type::App {
            constructor,
            arguments,
        } if constructor.as_str() == "Tuple" => {
            arguments.iter().all(|arg| intrinsic_type_allowed(arg))
        }
        _ => false,
    }
}

fn native_abi_type_allowed(ty: &Type) -> bool {
    match ty {
        Type::Builtin(
            BuiltinType::Int
            | BuiltinType::UInt
            | BuiltinType::Float
            | BuiltinType::Bool
            | BuiltinType::Char
            | BuiltinType::Unit,
        ) => true,
        Type::Ref { .. } => true,
        Type::App {
            constructor,
            arguments,
        } if constructor.as_str() == "Tuple" => {
            arguments.iter().all(|arg| native_abi_type_allowed(arg))
        }
        Type::App { constructor, .. } => matches!(
            constructor.as_str(),
            "Ptr" | "MutPtr" | "ConstPtr" | "NonNullPtr"
        ),
        _ => false,
    }
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
        TypedExprKindDraft::FieldAccess { target, field } => typed::TypedExprKind::FieldAccess {
            target: Box::new(finalize_typed_expr(*target, substitution)),
            field,
        },
        TypedExprKindDraft::TupleAccess { target, index } => typed::TypedExprKind::TupleAccess {
            target: Box::new(finalize_typed_expr(*target, substitution)),
            index,
        },
        TypedExprKindDraft::Index { target, index } => typed::TypedExprKind::Index {
            target: Box::new(finalize_typed_expr(*target, substitution)),
            index: Box::new(finalize_typed_expr(*index, substitution)),
        },
        TypedExprKindDraft::Block {
            statements,
            tail,
            defers,
        } => typed::TypedExprKind::Block {
            statements: statements
                .into_iter()
                .map(|stmt| finalize_typed_stmt(stmt, substitution))
                .collect(),
            tail: tail.map(|tail| Box::new(finalize_typed_expr(*tail, substitution))),
            defers: defers
                .into_iter()
                .map(|defer| finalize_typed_expr(defer, substitution))
                .collect(),
        },
        TypedExprKindDraft::Return { value } => typed::TypedExprKind::Return {
            value: value.map(|value| Box::new(finalize_typed_expr(*value, substitution))),
        },
        TypedExprKindDraft::Propagate { expr } => typed::TypedExprKind::Propagate {
            expr: Box::new(finalize_typed_expr(*expr, substitution)),
        },
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
        TypedExprKindDraft::Call {
            callee,
            args,
            qualified,
        } => typed::TypedExprKind::Call {
            callee: Box::new(finalize_typed_expr(*callee, substitution)),
            args: args
                .into_iter()
                .map(|arg| finalize_typed_expr(arg, substitution))
                .collect(),
            qualified,
        },
        TypedExprKindDraft::Lambda {
            params,
            return_annotation,
            body,
            captures,
        } => typed::TypedExprKind::Lambda {
            params: params
                .into_iter()
                .map(|binding| typed::TypedParam {
                    name: binding.display,
                    span: binding.span,
                    ty: substitution.apply(&binding.ty).label(),
                    annotation: binding.annotation,
                })
                .collect(),
            return_annotation,
            body: Box::new(finalize_typed_expr(*body, substitution)),
            captures,
        },
        TypedExprKindDraft::PerformCall { call } => typed::TypedExprKind::PerformCall {
            call: typed::TypedEffectCall {
                effect: call.effect,
                argument: Box::new(finalize_typed_expr(*call.argument, substitution)),
            },
        },
        TypedExprKindDraft::EffectBlock { body } => typed::TypedExprKind::EffectBlock {
            body: Box::new(finalize_typed_expr(*body, substitution)),
        },
        TypedExprKindDraft::Async { body, is_move } => typed::TypedExprKind::Async {
            body: Box::new(finalize_typed_expr(*body, substitution)),
            is_move,
        },
        TypedExprKindDraft::Await { expr } => typed::TypedExprKind::Await {
            expr: Box::new(finalize_typed_expr(*expr, substitution)),
        },
        TypedExprKindDraft::Unsafe { body } => typed::TypedExprKind::Unsafe {
            body: Box::new(finalize_typed_expr(*body, substitution)),
        },
        TypedExprKindDraft::InlineAsm {
            template,
            outputs,
            inputs,
            clobbers,
            options,
        } => typed::TypedExprKind::InlineAsm {
            template,
            outputs: outputs
                .into_iter()
                .map(|output| typed::TypedInlineAsmOutput {
                    constraint: output.constraint,
                    target: Box::new(finalize_typed_expr(*output.target, substitution)),
                })
                .collect(),
            inputs: inputs
                .into_iter()
                .map(|input| typed::TypedInlineAsmInput {
                    constraint: input.constraint,
                    expr: Box::new(finalize_typed_expr(*input.expr, substitution)),
                })
                .collect(),
            clobbers,
            options,
        },
        TypedExprKindDraft::LlvmIr {
            result_type,
            template,
            inputs,
        } => typed::TypedExprKind::LlvmIr {
            result_type,
            template,
            inputs: inputs
                .into_iter()
                .map(|input| finalize_typed_expr(input, substitution))
                .collect(),
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
        TypedExprKindDraft::Rec { target, ident } => typed::TypedExprKind::Rec {
            target: Box::new(finalize_typed_expr(*target, substitution)),
            ident,
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

fn finalize_typed_stmt(stmt: TypedStmtDraft, substitution: &Substitution) -> typed::TypedStmt {
    let kind = match stmt.kind {
        TypedStmtKindDraft::Let { pattern, value } => typed::TypedStmtKind::Let {
            pattern,
            value: finalize_typed_expr(*value, substitution),
        },
        TypedStmtKindDraft::Var { pattern, value } => typed::TypedStmtKind::Var {
            pattern,
            value: finalize_typed_expr(*value, substitution),
        },
        TypedStmtKindDraft::Expr { expr } => typed::TypedStmtKind::Expr {
            expr: finalize_typed_expr(*expr, substitution),
        },
        TypedStmtKindDraft::Assign { target, value } => typed::TypedStmtKind::Assign {
            target: finalize_typed_expr(*target, substitution),
            value: finalize_typed_expr(*value, substitution),
        },
        TypedStmtKindDraft::Defer { expr } => typed::TypedStmtKind::Defer {
            expr: finalize_typed_expr(*expr, substitution),
        },
    };
    typed::TypedStmt {
        span: stmt.span,
        kind,
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
    for expr in &module.exprs {
        collect_perform_effects(expr, &mut usages);
    }
    for decl in &module.decls {
        if let DeclKind::Conductor(conductor) = &decl.kind {
            for dsl_def in &conductor.dsl_defs {
                if let Some(pipeline) = &dsl_def.pipeline {
                    collect_perform_effects(&pipeline.expr, &mut usages);
                }
                for tail in &dsl_def.tails {
                    for arg in &tail.args {
                        collect_perform_effects(&arg.value, &mut usages);
                    }
                }
            }
            if let Some(execution) = &conductor.execution {
                collect_perform_effects(&execution.body, &mut usages);
            }
            if let Some(monitoring) = &conductor.monitoring {
                collect_perform_effects(&monitoring.body, &mut usages);
            }
        }
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

fn detect_varargs_violations(module: &Module) -> Vec<TypecheckViolation> {
    let mut violations = Vec::new();
    for decl in &module.decls {
        if let DeclKind::Extern { abi, functions, .. } = &decl.kind {
            for item in functions {
                let signature = &item.signature;
                if !signature.varargs {
                    continue;
                }
                if abi != "C" {
                    violations.push(TypecheckViolation::varargs_invalid_abi(
                        signature.span,
                        signature.name.name.as_str(),
                        abi,
                    ));
                }
                if signature.params.is_empty() {
                    violations.push(TypecheckViolation::varargs_missing_fixed_param(
                        signature.span,
                        signature.name.name.as_str(),
                    ));
                }
            }
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
    let recover_hint = find_core_parse_recover_hint(module);
    for span in find_parse_run_with_recovery_calls(module) {
        violations.push(TypecheckViolation::core_parse_recover_branch(
            span,
            recover_hint.clone(),
        ));
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

#[derive(Clone)]
struct NativeEscapeContext {
    has_cfg: bool,
    has_native_effect: bool,
    function_name: Option<String>,
}

impl NativeEscapeContext {
    fn with_cfg(&self, has_cfg: bool) -> Self {
        Self {
            has_cfg,
            has_native_effect: self.has_native_effect,
            function_name: self.function_name.clone(),
        }
    }
}

fn attrs_has_cfg(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| attr.name.name == "cfg")
}

fn detect_native_escape_hatch_violations(module: &Module) -> Vec<TypecheckViolation> {
    let mut violations = Vec::new();

    for function in &module.functions {
        let context = NativeEscapeContext {
            has_cfg: attrs_has_cfg(&function.attrs),
            has_native_effect: effect_has_native(&function.effect),
            function_name: Some(function.name.name.clone()),
        };
        walk_native_escape_expr(&function.body, &context, &mut violations);
    }

    for active_pattern in &module.active_patterns {
        let context = NativeEscapeContext {
            has_cfg: attrs_has_cfg(&active_pattern.attrs),
            has_native_effect: false,
            function_name: Some(active_pattern.name.name.clone()),
        };
        walk_native_escape_expr(&active_pattern.body, &context, &mut violations);
    }

    let module_context = NativeEscapeContext {
        has_cfg: false,
        has_native_effect: false,
        function_name: None,
    };
    for decl in &module.decls {
        walk_native_escape_decl(decl, &module_context, &mut violations);
    }
    for expr in &module.exprs {
        walk_native_escape_expr(expr, &module_context, &mut violations);
    }

    violations
}

fn walk_native_escape_decl(
    decl: &Decl,
    context: &NativeEscapeContext,
    violations: &mut Vec<TypecheckViolation>,
) {
    let next_context = context.with_cfg(context.has_cfg || attrs_has_cfg(&decl.attrs));
    match &decl.kind {
        DeclKind::Let { value, .. }
        | DeclKind::Var { value, .. }
        | DeclKind::Const { value, .. } => {
            walk_native_escape_expr(value, &next_context, violations)
        }
        DeclKind::Module(module_decl) => {
            let nested_context = next_context.clone();
            for function in &module_decl.body.functions {
                let function_context = NativeEscapeContext {
                    has_cfg: nested_context.has_cfg || attrs_has_cfg(&function.attrs),
                    has_native_effect: effect_has_native(&function.effect),
                    function_name: Some(function.name.name.clone()),
                };
                walk_native_escape_expr(&function.body, &function_context, violations);
            }
            for active_pattern in &module_decl.body.active_patterns {
                let active_context = NativeEscapeContext {
                    has_cfg: nested_context.has_cfg || attrs_has_cfg(&active_pattern.attrs),
                    has_native_effect: false,
                    function_name: Some(active_pattern.name.name.clone()),
                };
                walk_native_escape_expr(&active_pattern.body, &active_context, violations);
            }
            for decl in &module_decl.body.decls {
                walk_native_escape_decl(decl, &nested_context, violations);
            }
            for expr in &module_decl.body.exprs {
                walk_native_escape_expr(expr, &nested_context, violations);
            }
        }
        DeclKind::Macro(macro_decl) => {
            walk_native_escape_expr(&macro_decl.body, &next_context, violations);
        }
        DeclKind::ActorSpec(actor_spec) => {
            walk_native_escape_expr(&actor_spec.body, &next_context, violations);
        }
        DeclKind::Handler(handler) => {
            for entry in &handler.entries {
                if let HandlerEntry::Operation { body, .. } = entry {
                    walk_native_escape_expr(body, &next_context, violations);
                }
            }
        }
        DeclKind::Conductor(conductor) => {
            if let Some(exec) = &conductor.execution {
                walk_native_escape_expr(&exec.body, &next_context, violations);
            }
            if let Some(monitor) = &conductor.monitoring {
                walk_native_escape_expr(&monitor.body, &next_context, violations);
            }
        }
        _ => {}
    }
}

fn walk_native_escape_stmt(
    stmt: &Stmt,
    context: &NativeEscapeContext,
    violations: &mut Vec<TypecheckViolation>,
) {
    match &stmt.kind {
        StmtKind::Decl { decl } => walk_native_escape_decl(decl, context, violations),
        StmtKind::Expr { expr } | StmtKind::Defer { expr } => {
            walk_native_escape_expr(expr, context, violations)
        }
        StmtKind::Assign { target, value } => {
            walk_native_escape_expr(target, context, violations);
            walk_native_escape_expr(value, context, violations);
        }
    }
}

fn walk_native_escape_expr(
    expr: &Expr,
    context: &NativeEscapeContext,
    violations: &mut Vec<TypecheckViolation>,
) {
    match &expr.kind {
        ExprKind::InlineAsm(_) => {
            if !context.has_native_effect {
                violations.push(TypecheckViolation::inline_asm_missing_effect(
                    expr.span,
                    context.function_name.clone(),
                ));
            }
            if !context.has_cfg {
                violations.push(TypecheckViolation::inline_asm_missing_cfg(
                    expr.span,
                    context.function_name.clone(),
                ));
            }
        }
        ExprKind::LlvmIr(_) => {
            if !context.has_native_effect {
                violations.push(TypecheckViolation::llvm_ir_missing_effect(
                    expr.span,
                    context.function_name.clone(),
                ));
            }
            if !context.has_cfg {
                violations.push(TypecheckViolation::llvm_ir_missing_cfg(
                    expr.span,
                    context.function_name.clone(),
                ));
            }
        }
        ExprKind::Block {
            attrs, statements, ..
        } => {
            let next_context = context.with_cfg(context.has_cfg || attrs_has_cfg(attrs));
            for stmt in statements {
                walk_native_escape_stmt(stmt, &next_context, violations);
            }
        }
        ExprKind::Call { callee, args } => {
            walk_native_escape_expr(callee, context, violations);
            for arg in args {
                walk_native_escape_expr(arg, context, violations);
            }
        }
        ExprKind::PerformCall { call } => {
            walk_native_escape_expr(&call.argument, context, violations);
        }
        ExprKind::Lambda { body, .. }
        | ExprKind::Loop { body }
        | ExprKind::Unsafe { body }
        | ExprKind::Defer { body }
        | ExprKind::EffectBlock { body }
        | ExprKind::Async { body, .. } => {
            walk_native_escape_expr(body, context, violations);
        }
        ExprKind::Pipe { left, right } | ExprKind::Binary { left, right, .. } => {
            walk_native_escape_expr(left, context, violations);
            walk_native_escape_expr(right, context, violations);
        }
        ExprKind::Unary { expr: inner, .. }
        | ExprKind::Rec { expr: inner }
        | ExprKind::Propagate { expr: inner }
        | ExprKind::Return { value: Some(inner) }
        | ExprKind::Await { expr: inner } => {
            walk_native_escape_expr(inner, context, violations);
        }
        ExprKind::Break { value: Some(inner) } => {
            walk_native_escape_expr(inner, context, violations);
        }
        ExprKind::FieldAccess { target, .. }
        | ExprKind::TupleAccess { target, .. }
        | ExprKind::Index { target, .. } => {
            walk_native_escape_expr(target, context, violations);
        }
        ExprKind::IfElse {
            condition,
            then_branch,
            else_branch,
        } => {
            walk_native_escape_expr(condition, context, violations);
            walk_native_escape_expr(then_branch, context, violations);
            if let Some(branch) = else_branch {
                walk_native_escape_expr(branch, context, violations);
            }
        }
        ExprKind::Match { target, arms } => {
            walk_native_escape_expr(target, context, violations);
            for arm in arms {
                if let Some(guard) = &arm.guard {
                    walk_native_escape_expr(guard, context, violations);
                }
                walk_native_escape_expr(&arm.body, context, violations);
            }
        }
        ExprKind::While { condition, body } => {
            walk_native_escape_expr(condition, context, violations);
            walk_native_escape_expr(body, context, violations);
        }
        ExprKind::For { start, end, .. } => {
            walk_native_escape_expr(start, context, violations);
            walk_native_escape_expr(end, context, violations);
        }
        ExprKind::Handle { handle } => {
            walk_native_escape_expr(&handle.target, context, violations);
        }
        ExprKind::Assign { target, value } => {
            walk_native_escape_expr(target, context, violations);
            walk_native_escape_expr(value, context, violations);
        }
        ExprKind::Literal(_)
        | ExprKind::FixityLiteral(_)
        | ExprKind::Identifier(_)
        | ExprKind::ModulePath(_)
        | ExprKind::Break { value: None }
        | ExprKind::Return { value: None }
        | ExprKind::Continue => {}
    }
}

fn find_parse_run_with_recovery_calls(module: &Module) -> Vec<Span> {
    let mut spans = Vec::new();
    visit_module_exprs(module, &mut |expr| {
        if let ExprKind::Call { callee, .. } = &expr.kind {
            if matches_module_member(callee, "Parse", "run_with_recovery") {
                spans.push(expr.span);
            }
        }
    });
    spans
}

fn find_core_parse_recover_hint(module: &Module) -> Option<TypecheckRecoverHint> {
    if let Some(hint) = find_panic_block_hint(module) {
        return Some(hint);
    }
    if let Some(hint) = find_panic_until_hint(module) {
        return Some(hint);
    }
    if let Some(sync) = find_sync_to_hint(module) {
        return Some(TypecheckRecoverHint {
            mode: Some("collect".to_string()),
            action: Some("skip".to_string()),
            sync: Some(sync),
            context: None,
        });
    }
    None
}

fn find_panic_block_hint(module: &Module) -> Option<TypecheckRecoverHint> {
    let mut hint = None;
    visit_module_exprs(module, &mut |expr| {
        if hint.is_some() {
            return;
        }
        if let ExprKind::Call { callee, args } = &expr.kind {
            if matches_module_member(callee, "Parse", "panic_block") {
                let sync = args.get(2).and_then(extract_sync_token);
                hint = Some(TypecheckRecoverHint {
                    mode: Some("collect".to_string()),
                    action: Some("skip".to_string()),
                    sync,
                    context: Some("panic_block".to_string()),
                });
            }
        }
    });
    hint
}

fn find_panic_until_hint(module: &Module) -> Option<TypecheckRecoverHint> {
    let mut hint = None;
    visit_module_exprs(module, &mut |expr| {
        if hint.is_some() {
            return;
        }
        if let ExprKind::Call { callee, args } = &expr.kind {
            if matches_module_member(callee, "Parse", "panic_until") {
                let sync = args.get(1).and_then(extract_sync_token);
                hint = Some(TypecheckRecoverHint {
                    mode: Some("collect".to_string()),
                    action: Some("skip".to_string()),
                    sync,
                    context: Some("panic".to_string()),
                });
            }
        }
    });
    hint
}

fn find_sync_to_hint(module: &Module) -> Option<String> {
    let mut token = None;
    visit_module_exprs(module, &mut |expr| {
        if token.is_some() {
            return;
        }
        if let ExprKind::Call { callee, args } = &expr.kind {
            if matches_module_member(callee, "Parse", "sync_to") {
                token = args.get(0).and_then(extract_sync_token);
            }
        }
    });
    token
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
    for expr in &module.exprs {
        visit_expr(expr, visitor);
    }
}

fn visit_decl(decl: &Decl, visitor: &mut impl FnMut(&Expr)) {
    match &decl.kind {
        DeclKind::Let { value, .. }
        | DeclKind::Var { value, .. }
        | DeclKind::Const { value, .. } => visit_expr(value, visitor),
        DeclKind::Module(module_decl) => {
            for function in &module_decl.body.functions {
                visit_expr(&function.body, visitor);
            }
            for active in &module_decl.body.active_patterns {
                visit_expr(&active.body, visitor);
            }
            for decl in &module_decl.body.decls {
                visit_decl(decl, visitor);
            }
            for expr in &module_decl.body.exprs {
                visit_expr(expr, visitor);
            }
        }
        DeclKind::Macro(macro_decl) => visit_expr(&macro_decl.body, visitor),
        DeclKind::ActorSpec(actor_spec) => visit_expr(&actor_spec.body, visitor),
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
        ExprKind::InlineAsm(asm) => {
            for output in &asm.outputs {
                visit_expr(&output.target, visitor);
            }
            for input in &asm.inputs {
                visit_expr(&input.expr, visitor);
            }
        }
        ExprKind::LlvmIr(ir) => {
            for input in &ir.inputs {
                visit_expr(input, visitor);
            }
        }
        ExprKind::Lambda { body, .. }
        | ExprKind::Loop { body }
        | ExprKind::Unsafe { body }
        | ExprKind::Defer { body }
        | ExprKind::EffectBlock { body }
        | ExprKind::Async { body, .. } => visit_expr(body, visitor),
        ExprKind::Pipe { left, right } | ExprKind::Binary { left, right, .. } => {
            visit_expr(left, visitor);
            visit_expr(right, visitor);
        }
        ExprKind::Unary { expr: inner, .. }
        | ExprKind::Rec { expr: inner }
        | ExprKind::Propagate { expr: inner }
        | ExprKind::Return { value: Some(inner) } => {
            visit_expr(inner, visitor);
        }
        ExprKind::Await { expr: inner } => {
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
        LiteralKind::Tuple { elements }
        | LiteralKind::Array { elements }
        | LiteralKind::Set { elements } => {
            for element in elements {
                visit_expr(element, visitor);
            }
        }
        LiteralKind::Record { fields, .. } => {
            for field in fields {
                visit_expr(&field.value, visitor);
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

fn extract_sync_token(expr: &Expr) -> Option<String> {
    match &expr.kind {
        ExprKind::Call { callee, args } => {
            if matches_module_member(callee, "Parse", "sync_to") {
                return args.get(0).and_then(extract_sync_token);
            }
            if matches_module_member(callee, "Parse", "expect_symbol")
                || matches_module_member(callee, "Parse", "symbol")
            {
                return args.get(0).and_then(extract_string_literal);
            }
            None
        }
        _ => extract_string_literal(expr),
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
        ExprKind::Loop { body }
        | ExprKind::Unsafe { body }
        | ExprKind::Defer { body }
        | ExprKind::EffectBlock { body }
        | ExprKind::Async { body, .. } => {
            collect_perform_effects(body, usages);
        }
        ExprKind::While { condition, body } => {
            collect_perform_effects(condition, usages);
            collect_perform_effects(body, usages);
        }
        ExprKind::For { start, end, .. } => {
            collect_perform_effects(start, usages);
            collect_perform_effects(end, usages);
        }
        ExprKind::Match { target, arms } => {
            collect_perform_effects(target, usages);
            for arm in arms {
                if let Some(guard) = &arm.guard {
                    collect_perform_effects(guard, usages);
                }
                collect_perform_effects(&arm.body, usages);
            }
        }
        ExprKind::Handle { handle } => {
            collect_perform_effects(&handle.target, usages);
        }
        ExprKind::Pipe { left, right } => {
            collect_perform_effects(left, usages);
            collect_perform_effects(right, usages);
        }
        ExprKind::Unary { expr: inner, .. }
        | ExprKind::Rec { expr: inner }
        | ExprKind::Propagate { expr: inner }
        | ExprKind::Return { value: Some(inner) } => {
            collect_perform_effects(inner, usages);
        }
        ExprKind::Await { expr: inner } => {
            collect_perform_effects(inner, usages);
        }
        ExprKind::Break { value } => {
            if let Some(inner) = value {
                collect_perform_effects(inner, usages);
            }
        }
        ExprKind::FieldAccess { target, .. } | ExprKind::TupleAccess { target, .. } => {
            collect_perform_effects(target, usages);
        }
        ExprKind::Index { target, index } => {
            collect_perform_effects(target, usages);
            collect_perform_effects(index, usages);
        }
        ExprKind::Literal(_)
        | ExprKind::Identifier(_)
        | ExprKind::ModulePath(_)
        | ExprKind::FixityLiteral(_)
        | ExprKind::Continue
        | ExprKind::Return { value: None } => {}
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
        DeclKind::Let { value, .. }
        | DeclKind::Var { value, .. }
        | DeclKind::Const { value, .. } => {
            collect_perform_effects(value, usages);
        }
        DeclKind::Module(module_decl) => {
            for function in &module_decl.body.functions {
                collect_perform_effects(&function.body, usages);
            }
            for active in &module_decl.body.active_patterns {
                collect_perform_effects(&active.body, usages);
            }
            for decl in &module_decl.body.decls {
                collect_perform_effects_in_decl(decl, usages);
            }
            for expr in &module_decl.body.exprs {
                collect_perform_effects(expr, usages);
            }
        }
        DeclKind::Macro(macro_decl) => {
            collect_perform_effects(&macro_decl.body, usages);
        }
        DeclKind::ActorSpec(actor_spec) => {
            collect_perform_effects(&actor_spec.body, usages);
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
