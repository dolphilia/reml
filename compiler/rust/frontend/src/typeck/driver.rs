use std::collections::{HashMap, HashSet};

use serde::Serialize;

use super::env::{TypeRowMode, TypecheckConfig};
use super::metrics::TypecheckMetrics;
use crate::diagnostic::{ExpectedTokenCollector, ExpectedTokensSummary};
use crate::parser::ast::{Expr, Function, Module};
use crate::span::Span;

/// 型推論の簡易ドライバ。現時点では AST を走査して
/// メトリクスとサマリ情報のみを生成する。
pub struct TypecheckDriver;

impl TypecheckDriver {
    pub fn infer_module(module: &Module, config: &TypecheckConfig) -> TypecheckReport {
        let mut metrics = TypecheckMetrics::default();
        let mut functions = Vec::new();
        let mut violations = Vec::new();

        if config.trace_enabled {
            eprintln!(
                "[TRACE] typecheck.start functions={}",
                module.functions.len()
            );
        }

        for function in &module.functions {
            metrics.record_function();
            let mut stats = FunctionStats::default();
            let typed_return = infer_function(
                function,
                &function.name,
                &mut stats,
                &mut metrics,
                &mut violations,
            );
            functions.push(TypedFunctionSummary {
                name: function.name.clone(),
                param_types: function
                    .params
                    .iter()
                    .map(|param| SimpleType::from_param(param.name.as_str()).label())
                    .collect(),
                return_type: typed_return.label(),
                typed_exprs: stats.typed_exprs,
                constraints: stats.constraints,
                unresolved_identifiers: stats.unresolved_identifiers,
            });
        }

        if config.trace_enabled {
            eprintln!("[TRACE] typecheck.finish");
        }

        violations.extend(detect_residual_leaks_from_module(module, config));
        let violations = compress_typecheck_violations(violations);

        TypecheckReport {
            metrics,
            functions,
            violations,
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
                return_type: SimpleType::Unknown.label(),
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
}

#[derive(Debug, Serialize, Clone)]
pub struct TypedFunctionSummary {
    pub name: String,
    pub param_types: Vec<&'static str>,
    pub return_type: &'static str,
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
    fn condition_literal_bool(span: Span, actual: SimpleType, function: Option<String>) -> Self {
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

    pub fn domain(&self) -> &'static str {
        match self.kind {
            TypecheckViolationKind::ConditionLiteralBool => "type",
            TypecheckViolationKind::ResidualLeak => "effects",
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
    stats: &mut FunctionStats,
    metrics: &mut TypecheckMetrics,
    violations: &mut Vec<TypecheckViolation>,
) -> SimpleType {
    let mut env = HashMap::new();
    for param in &function.params {
        env.insert(param.name.clone(), SimpleType::from_param(&param.name));
    }
    infer_expr(
        &function.body,
        &env,
        stats,
        metrics,
        violations,
        Some(function_name),
    )
}

fn infer_expr(
    expr: &Expr,
    env: &HashMap<String, SimpleType>,
    stats: &mut FunctionStats,
    metrics: &mut TypecheckMetrics,
    violations: &mut Vec<TypecheckViolation>,
    function_name: Option<&str>,
) -> SimpleType {
    stats.typed_exprs += 1;
    metrics.record_expr();
    match expr {
        Expr::Int { .. } => SimpleType::Int,
        Expr::Bool { .. } => SimpleType::Bool,
        Expr::String { .. } => SimpleType::Unknown,
        Expr::Identifier { name, .. } => match env.get(name) {
            Some(ty) => *ty,
            None => {
                stats.unresolved_identifiers += 1;
                metrics.record_unresolved_identifier();
                SimpleType::Unknown
            }
        },
        Expr::Binary { left, right, .. } => {
            metrics.record_binary_expr();
            let left_ty = infer_expr(left, env, stats, metrics, violations, function_name);
            let right_ty = infer_expr(right, env, stats, metrics, violations, function_name);
            stats.constraints += 1;
            metrics.record_constraint("binary.operands");
            combine_numeric_types(left_ty, right_ty)
        }
        Expr::Call { callee, args, .. } => {
            metrics.record_call_site();
            stats.constraints += 1;
            metrics.record_constraint("call.arity");
            let _callee_ty = infer_expr(callee, env, stats, metrics, violations, function_name);
            for arg in args {
                let _ = infer_expr(arg, env, stats, metrics, violations, function_name);
            }
            SimpleType::Unknown
        }
        Expr::Perform { argument, .. } => {
            let _ = infer_expr(argument, env, stats, metrics, violations, function_name);
            SimpleType::Unknown
        }
        Expr::IfElse {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            let condition_ty =
                infer_expr(condition, env, stats, metrics, violations, function_name);
            check_bool_condition(condition.span(), condition_ty, violations, function_name);
            let then_ty = infer_expr(then_branch, env, stats, metrics, violations, function_name);
            let else_ty = infer_expr(else_branch, env, stats, metrics, violations, function_name);
            if then_ty == else_ty {
                then_ty
            } else {
                SimpleType::Unknown
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SimpleType {
    Int,
    Bool,
    Unknown,
}

impl SimpleType {
    fn label(self) -> &'static str {
        match self {
            SimpleType::Int => "Int",
            SimpleType::Bool => "Bool",
            SimpleType::Unknown => "Unknown",
        }
    }

    fn from_param(name: &str) -> Self {
        if name.ends_with("_int") {
            SimpleType::Int
        } else if name.ends_with("_bool") {
            SimpleType::Bool
        } else {
            SimpleType::Unknown
        }
    }
}

fn combine_numeric_types(left: SimpleType, right: SimpleType) -> SimpleType {
    match (left, right) {
        (SimpleType::Int, SimpleType::Int) => SimpleType::Int,
        _ => SimpleType::Unknown,
    }
}

fn check_bool_condition(
    span: Span,
    ty: SimpleType,
    violations: &mut Vec<TypecheckViolation>,
    function_name: Option<&str>,
) {
    if ty == SimpleType::Bool {
        return;
    }
    violations.push(TypecheckViolation::condition_literal_bool(
        span,
        ty,
        function_name.map(|name| name.to_string()),
    ));
}

fn detect_residual_leaks_from_module(
    module: &Module,
    config: &TypecheckConfig,
) -> Vec<TypecheckViolation> {
    if !matches!(config.type_row_mode, TypeRowMode::DualWrite) {
        return Vec::new();
    }
    let mut usages = Vec::new();
    for function in &module.functions {
        collect_perform_effects(&function.body, &mut usages);
    }
    let mut seen = HashSet::new();
    usages
        .into_iter()
        .filter_map(|(effect, span)| {
            if seen.insert(effect.clone()) {
                Some(TypecheckViolation::residual_leak(Some(span), Some(effect)))
            } else {
                None
            }
        })
        .collect()
}

fn collect_perform_effects(expr: &Expr, usages: &mut Vec<(String, Span)>) {
    match expr {
        &Expr::Perform {
            ref effect,
            ref argument,
            ref span,
        } => {
            usages.push((effect.clone(), *span));
            collect_perform_effects(argument.as_ref(), usages);
        }
        &Expr::IfElse {
            ref condition,
            ref then_branch,
            ref else_branch,
            ..
        } => {
            collect_perform_effects(condition, usages);
            collect_perform_effects(then_branch, usages);
            collect_perform_effects(else_branch, usages);
        }
        &Expr::Binary {
            ref left,
            ref right,
            ..
        } => {
            collect_perform_effects(left, usages);
            collect_perform_effects(right, usages);
        }
        &Expr::Call {
            ref callee,
            ref args,
            ..
        } => {
            collect_perform_effects(callee, usages);
            for arg in args {
                collect_perform_effects(arg, usages);
            }
        }
        &Expr::Int { .. }
        | &Expr::Bool { .. }
        | &Expr::String { .. }
        | &Expr::Identifier { .. } => {}
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
