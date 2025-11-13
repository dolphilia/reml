use std::collections::HashSet;

use serde::Serialize;

use super::capability::{CapabilityDescriptor, EffectUsage, RuntimeCapability};
use super::constraint::Constraint;
use super::constraint::ConstraintSolver;
use super::env::{StageRequirement, TypeEnv, TypeRowMode, TypecheckConfig};
use super::metrics::TypecheckMetrics;
use super::scheme::Scheme;
use super::types::{BuiltinType, Type, TypeVarGen};
use crate::diagnostic::{ExpectedTokenCollector, ExpectedTokensSummary};
use crate::parser::ast::{Expr, ExprKind, Function, Literal, Module};
use crate::semantics::typed;
use crate::span::Span;

/// 型推論の簡易ドライバ。現時点では AST を走査して
/// メトリクスとサマリ情報のみを生成する。
pub struct TypecheckDriver;

impl TypecheckDriver {
    pub fn infer_module(module: &Module, config: &TypecheckConfig) -> TypecheckReport {
        let mut metrics = TypecheckMetrics::default();
        let mut functions = Vec::new();
        let mut violations = Vec::new();
        let mut typed_functions = Vec::new();
        let mut all_constraints = Vec::new();

        if config.trace_enabled {
            eprintln!(
                "[TRACE] typecheck.start functions={}",
                module.functions.len()
            );
        }

        let solver = ConstraintSolver::new();
        let mut var_gen = TypeVarGen::default();
        for function in &module.functions {
            metrics.record_function();
            let mut stats = FunctionStats::default();
            let mut constraints = Vec::new();
            let mut env = TypeEnv::new();

            let param_bindings: Vec<(String, Type)> = function
                .params
                .iter()
                .map(|param| {
                    (
                        param.name.name.clone(),
                        type_for_param(param.name.name.as_str()),
                    )
                })
                .collect();
            for (name, ty) in &param_bindings {
                env.insert(name.clone(), Scheme::simple(ty.clone()));
            }

            let typed_body = infer_function(
                function,
                function.name.name.as_str(),
                &mut env,
                &mut var_gen,
                &mut constraints,
                &mut stats,
                &mut metrics,
                &mut violations,
            );
            let _ = solver.solve(&constraints);
            all_constraints.extend(constraints.into_iter());

            let param_type_labels: Vec<String> =
                param_bindings.iter().map(|(_, ty)| ty.label()).collect();

            functions.push(TypedFunctionSummary {
                name: function.name.name.clone(),
                param_types: param_type_labels,
                return_type: typed_body.ty.label(),
                typed_exprs: stats.typed_exprs,
                constraints: stats.constraints,
                unresolved_identifiers: stats.unresolved_identifiers,
            });

            let typed_params = function
                .params
                .iter()
                .zip(param_bindings.iter())
                .map(|(param, (_, param_ty))| typed::TypedParam {
                    name: param.name.name.clone(),
                    span: param.name.span,
                    ty: param_ty.label(),
                })
                .collect();

            typed_functions.push(typed::TypedFunction {
                name: function.name.name.clone(),
                span: function.span,
                params: typed_params,
                return_type: typed_body.ty.label(),
                body: typed_body.node,
            });
        }

        if config.trace_enabled {
            eprintln!("[TRACE] typecheck.finish");
        }

        violations.extend(detect_capability_violations(module, config));
        let violations = compress_typecheck_violations(violations);

        let used_impls = all_constraints
            .iter()
            .filter_map(|constraint| match constraint {
                Constraint::ImplBound { implementation, .. } => Some(implementation.to_string()),
                _ => None,
            })
            .collect();

        TypecheckReport {
            metrics,
            functions,
            violations,
            typed_module: typed::TypedModule {
                functions: typed_functions,
            },
            constraints: all_constraints,
            used_impls,
        }
    }

    pub fn infer_fallback_from_source(source: &str, config: &TypecheckConfig) -> TypecheckReport {
        let mut metrics = TypecheckMetrics::default();
        let mut functions = Vec::new();

        if config.trace_enabled {
            eprintln!("[TRACE] typecheck.fallback");
        }

        for name in extract_top_level_functions(source) {
            metrics.record_function();
            metrics.record_expr();
            functions.push(TypedFunctionSummary {
                name,
                param_types: Vec::new(),
                return_type: Type::builtin(BuiltinType::Unknown).label(),
                typed_exprs: 0,
                constraints: 0,
                unresolved_identifiers: 0,
            });
        }

        let violations = detect_residual_leaks_from_source(source, config);
        let violations = compress_typecheck_violations(violations);

        TypecheckReport {
            metrics,
            functions,
            violations,
            typed_module: typed::TypedModule::default(),
            constraints: Vec::new(),
            used_impls: Vec::new(),
        }
    }
}

fn extract_top_level_functions(source: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut extern_depth: i32 = 0;
    let mut pending_extern = false;

    for line in source.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("extern") {
            pending_extern = true;
        }

        if extern_depth == 0 && !pending_extern {
            let mut candidate = trimmed;
            if candidate.starts_with("pub ") {
                candidate = candidate[4..].trim_start();
            }
            if let Some(rest) = candidate.strip_prefix("fn ") {
                let mut name = String::new();
                for ch in rest.chars() {
                    if ch.is_alphanumeric() || ch == '_' {
                        name.push(ch);
                    } else {
                        break;
                    }
                }
                if !name.is_empty() {
                    let remainder = &rest[name.len()..];
                    let next_sig_char = remainder.chars().find(|c| !c.is_whitespace());
                    if next_sig_char != Some(';') {
                        names.push(name);
                    }
                }
            }
        }

        for ch in trimmed.chars() {
            match ch {
                '{' => {
                    if pending_extern {
                        extern_depth += 1;
                        pending_extern = false;
                    }
                }
                '}' => {
                    if extern_depth > 0 {
                        extern_depth -= 1;
                    }
                }
                _ => {}
            }
        }

        if pending_extern && !trimmed.contains('{') {
            // keep pending flag until opening brace appears
        } else {
            pending_extern = false;
        }
    }

    names
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
}

#[derive(Debug, Serialize, Clone)]
pub enum TypecheckViolationKind {
    ConditionLiteralBool,
    ResidualLeak,
    StageMismatch,
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
        }
    }

    pub fn domain(&self) -> &'static str {
        match self.kind {
            TypecheckViolationKind::ConditionLiteralBool => "type",
            TypecheckViolationKind::ResidualLeak | TypecheckViolationKind::StageMismatch => {
                "effects"
            }
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
    constraints: &mut Vec<Constraint>,
    stats: &mut FunctionStats,
    metrics: &mut TypecheckMetrics,
    violations: &mut Vec<TypecheckViolation>,
) -> TypedExprResult {
    infer_expr(
        &function.body,
        env,
        var_gen,
        constraints,
        stats,
        metrics,
        violations,
        Some(function_name),
    )
}

fn infer_expr(
    expr: &Expr,
    env: &TypeEnv,
    var_gen: &mut TypeVarGen,
    constraints: &mut Vec<Constraint>,
    stats: &mut FunctionStats,
    metrics: &mut TypecheckMetrics,
    violations: &mut Vec<TypecheckViolation>,
    function_name: Option<&str>,
) -> TypedExprResult {
    stats.typed_exprs += 1;
    metrics.record_expr();
    match &expr.kind {
        ExprKind::Literal(literal) => {
            let ty = type_for_literal(literal);
            make_typed(expr, typed::TypedExprKind::Literal(literal.clone()), ty)
        }
        ExprKind::Identifier(ident) => {
            let ty = match env.lookup(ident.name.as_str()) {
                Some(binding) => binding.scheme.instantiate(var_gen),
                None => {
                    stats.unresolved_identifiers += 1;
                    metrics.record_unresolved_identifier();
                    Type::builtin(BuiltinType::Unknown)
                }
            };
            make_typed(
                expr,
                typed::TypedExprKind::Identifier {
                    ident: ident.clone(),
                },
                ty,
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
                constraints,
                stats,
                metrics,
                violations,
                function_name,
            );
            let right_result = infer_expr(
                right,
                env,
                var_gen,
                constraints,
                stats,
                metrics,
                violations,
                function_name,
            );
            stats.constraints += 1;
            metrics.record_constraint("binary.operands");
            constraints.push(Constraint::equal(
                left_result.ty.clone(),
                right_result.ty.clone(),
            ));
            let ty = combine_numeric_types(&left_result.ty, &right_result.ty);
            make_typed(
                expr,
                typed::TypedExprKind::Binary {
                    operator: operator.clone(),
                    left: Box::new(left_result.node),
                    right: Box::new(right_result.node),
                },
                ty,
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
                constraints,
                stats,
                metrics,
                violations,
                function_name,
            );
            let mut typed_args = Vec::new();
            for arg in args {
                let arg_result = infer_expr(
                    arg,
                    env,
                    var_gen,
                    constraints,
                    stats,
                    metrics,
                    violations,
                    function_name,
                );
                typed_args.push(arg_result.node);
            }
            make_typed(
                expr,
                typed::TypedExprKind::Call {
                    callee: Box::new(callee_result.node),
                    args: typed_args,
                },
                Type::builtin(BuiltinType::Unknown),
            )
        }
        ExprKind::PerformCall { call } => {
            let argument_result = infer_expr(
                &call.argument,
                env,
                var_gen,
                constraints,
                stats,
                metrics,
                violations,
                function_name,
            );
            constraints.push(Constraint::has_capability(
                Type::builtin(BuiltinType::Unknown),
                call.effect.name.clone(),
            ));
            make_typed(
                expr,
                typed::TypedExprKind::PerformCall {
                    call: typed::TypedEffectCall {
                        effect: call.effect.clone(),
                        argument: Box::new(argument_result.node),
                    },
                },
                Type::builtin(BuiltinType::Unknown),
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
                constraints,
                stats,
                metrics,
                violations,
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
                constraints,
                stats,
                metrics,
                violations,
                function_name,
            );
            let else_result = infer_expr(
                else_branch,
                env,
                var_gen,
                constraints,
                stats,
                metrics,
                violations,
                function_name,
            );
            stats.constraints += 1;
            metrics.record_constraint("conditional");
            constraints.push(Constraint::equal(
                then_result.ty.clone(),
                else_result.ty.clone(),
            ));
            let ty = if then_result.ty == else_result.ty {
                then_result.ty.clone()
            } else {
                Type::builtin(BuiltinType::Unknown)
            };
            make_typed(
                expr,
                typed::TypedExprKind::IfElse {
                    condition: Box::new(condition_result.node),
                    then_branch: Box::new(then_result.node),
                    else_branch: Box::new(else_result.node),
                },
                ty,
            )
        }
    }
}

fn type_for_literal(literal: &Literal) -> Type {
    match literal {
        Literal::Int { .. } => Type::builtin(BuiltinType::Int),
        Literal::Bool { .. } => Type::builtin(BuiltinType::Bool),
        _ => Type::builtin(BuiltinType::Unknown),
    }
}

fn type_for_param(name: &str) -> Type {
    if name.ends_with("_int") {
        Type::builtin(BuiltinType::Int)
    } else if name.ends_with("_bool") {
        Type::builtin(BuiltinType::Bool)
    } else {
        Type::builtin(BuiltinType::Unknown)
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

struct TypedExprResult {
    ty: Type,
    node: typed::TypedExpr,
}

fn make_typed(expr: &Expr, kind: typed::TypedExprKind, ty: Type) -> TypedExprResult {
    let label = ty.label();
    TypedExprResult {
        ty,
        node: typed::TypedExpr {
            span: expr.span,
            kind,
            ty: label,
        },
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
        .filter_map(|value| RuntimeCapability::parse(value))
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
            collect_perform_effects(else_branch, usages);
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

fn detect_residual_leaks_from_source(
    source: &str,
    config: &TypecheckConfig,
) -> Vec<TypecheckViolation> {
    if !matches!(config.type_row_mode, TypeRowMode::DualWrite) {
        return Vec::new();
    }
    let mut leaks = Vec::new();
    let mut seen_capabilities: HashSet<String> = HashSet::new();
    let mut seen_generic = false;
    let mut offset: u32 = 0;
    for line in source.lines() {
        let mut local_matches = find_perform_matches(line);
        if local_matches.is_empty() {
            offset = offset.saturating_add(line.len() as u32 + 1);
            continue;
        }
        for (byte_index, capability) in local_matches.drain(..) {
            if let Some(cap) = capability.clone() {
                if !seen_capabilities.insert(cap.clone()) {
                    continue;
                }
            } else if seen_generic {
                continue;
            } else {
                seen_generic = true;
            }
            let span = Span::new(
                offset.saturating_add(byte_index),
                offset.saturating_add(byte_index + "perform".len() as u32),
            );
            leaks.push(TypecheckViolation::residual_leak(Some(span), capability));
        }
        offset = offset.saturating_add(line.len() as u32 + 1);
    }
    leaks
}

fn find_perform_matches(line: &str) -> Vec<(u32, Option<String>)> {
    let mut matches = Vec::new();
    let keyword = "perform";
    let mut search_start = 0;
    while let Some(idx) = line[search_start..].find(keyword) {
        let absolute = search_start + idx;
        let before = line[..absolute].chars().last();
        let after_index = absolute + keyword.len();
        let after_char = line[after_index..].chars().next();
        let is_identifier_char = |ch: char| ch.is_ascii_alphanumeric() || ch == '_';
        let boundary_before = before.map_or(true, |ch| !is_identifier_char(ch));
        let boundary_after = after_char.map_or(true, |ch| !is_identifier_char(ch));
        if boundary_before && boundary_after {
            let rest = line[after_index..].trim_start();
            let capability = rest
                .split_whitespace()
                .next()
                .map(|token| {
                    token.trim_matches(|c: char| c == '(' || c == ')' || c == ',' || c == ';')
                })
                .filter(|token| !token.is_empty())
                .map(|token| token.to_string());
            matches.push((absolute as u32, capability));
        }
        search_start = absolute + keyword.len();
        if search_start >= line.len() {
            break;
        }
    }
    matches
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
