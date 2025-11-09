use std::collections::HashMap;

use serde::Serialize;

use crate::parser::ast::{Expr, Function, Module};
use crate::token::TokenKind;

use super::env::TypecheckConfig;
use super::metrics::TypecheckMetrics;

/// 型推論の簡易ドライバ。現時点では AST を走査して
/// メトリクスとサマリ情報のみを生成する。
pub struct TypecheckDriver;

impl TypecheckDriver {
    pub fn infer_module(module: &Module, _config: &TypecheckConfig) -> TypecheckReport {
        let mut metrics = TypecheckMetrics::default();
        let mut functions = Vec::new();

        for function in &module.functions {
            metrics.record_function();
            let mut stats = FunctionStats::default();
            let typed_return = infer_function(function, &mut stats, &mut metrics);
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

        TypecheckReport { metrics, functions }
    }

    pub fn infer_fallback_from_source(source: &str) -> TypecheckReport {
        let mut metrics = TypecheckMetrics::default();
        let mut functions = Vec::new();

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

        TypecheckReport { metrics, functions }
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

#[derive(Default)]
struct FunctionStats {
    typed_exprs: usize,
    constraints: usize,
    unresolved_identifiers: usize,
}

fn infer_function(
    function: &Function,
    stats: &mut FunctionStats,
    metrics: &mut TypecheckMetrics,
) -> SimpleType {
    let mut env = HashMap::new();
    for param in &function.params {
        env.insert(param.name.clone(), SimpleType::from_param(&param.name));
    }
    infer_expr(&function.body, &env, stats, metrics)
}

fn infer_expr(
    expr: &Expr,
    env: &HashMap<String, SimpleType>,
    stats: &mut FunctionStats,
    metrics: &mut TypecheckMetrics,
) -> SimpleType {
    stats.typed_exprs += 1;
    metrics.record_expr();
    match expr {
        Expr::Int { .. } => SimpleType::Int,
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
            let left_ty = infer_expr(left, env, stats, metrics);
            let right_ty = infer_expr(right, env, stats, metrics);
            stats.constraints += 1;
            metrics.record_constraint("binary.operands");
            combine_numeric_types(left_ty, right_ty)
        }
        Expr::Call { callee, args, .. } => {
            metrics.record_call_site();
            stats.constraints += 1;
            metrics.record_constraint("call.arity");
            let _callee_ty = infer_expr(callee, env, stats, metrics);
            for arg in args {
                let _ = infer_expr(arg, env, stats, metrics);
            }
            SimpleType::Unknown
        }
    }
}

#[derive(Clone, Copy)]
enum SimpleType {
    Int,
    Unknown,
}

impl SimpleType {
    fn label(self) -> &'static str {
        match self {
            SimpleType::Int => "Int",
            SimpleType::Unknown => "Unknown",
        }
    }

    fn from_param(name: &str) -> Self {
        if name.ends_with("_int") {
            SimpleType::Int
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
