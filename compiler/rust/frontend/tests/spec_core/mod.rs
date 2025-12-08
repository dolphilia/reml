use reml_frontend::parser::ast::{
    ConductorMonitorTarget, Decl, DeclKind, Expr, ExprKind, LiteralKind, Pattern, PatternKind, Stmt,
    StmtKind, TypeKind, UseTree,
};

mod common;

use common::parse_example_module;

#[test]
fn ch1_mod_003_accepts_module_and_use_prefix() {
    let module = parse_example_module(
        "examples/spec_core/chapter1/module_use/bnf-compilationunit-module-use-alias-ok.reml",
    );
    let header = module
        .header
        .as_ref()
        .expect("module header should be present");
    assert_eq!(header.path.render(), "spec_core.match_guard");
    assert_eq!(module.uses.len(), 2, "expected two use declarations");
    assert!(
        module
            .uses
            .iter()
            .any(|decl| matches!(decl.tree, UseTree::Brace { .. })),
        "expected a brace-style use tree to be parsed"
    );
}

#[test]
fn ch1_let_001_accepts_top_level_let_binding() {
    let module = parse_example_module(
        "examples/spec_core/chapter1/let_binding/bnf-valdecl-let-simple-ok.reml",
    );
    let has_top_level_let = module.decls.iter().any(|decl| {
        matches!(
            decl.kind,
            DeclKind::Let {
                pattern: Pattern {
                    kind: PatternKind::Var(ref ident),
                    ..
                },
                ..
            } if ident.name == "greeting_prefix"
        )
    });
    assert!(
        has_top_level_let,
        "expected greeting_prefix to be recorded as a top-level let binding"
    );
}

#[test]
fn ch1_let_002_supports_tuple_pattern_bindings() {
    let module = parse_example_module(
        "examples/spec_core/chapter1/let_binding/bnf-valdecl-let-pattern-tuple.reml",
    );
    let tuple_binding_present = module.functions.iter().any(|function| {
        function.name.name == "sum_pair"
            && matches!(
                function.body.kind,
                ExprKind::Block { ref statements, .. } if statements.iter().any(|stmt| {
                    matches!(
                        stmt.kind,
                        StmtKind::Decl {
                            decl:
                                Decl {
                                    kind: DeclKind::Let { ref pattern, .. },
                                    ..
                                },
                        } if matches!(pattern.kind, PatternKind::Tuple { .. })
                    )
                })
            )
    });
    assert!(
        tuple_binding_present,
        "expected sum_pair to contain a tuple-pattern let binding"
    );
}

#[test]
fn ch1_dsl_801_parses_conductor_sections() {
    let module = parse_example_module(
        "examples/spec_core/chapter1/conductor/bnf-conductor-basic-pipeline-ok.reml",
    );
    let conductor = module
        .decls
        .iter()
        .find_map(|decl| match &decl.kind {
            DeclKind::Conductor(decl) => Some(decl),
            _ => None,
        })
        .expect("expected conductor declaration");
    assert_eq!(conductor.name.name, "telemetry");
    assert_eq!(
        conductor.channels.len(),
        1,
        "expected a single channel route"
    );
    let route = &conductor.channels[0];
    assert_eq!(route.source.path.name, "source.metrics");
    assert_eq!(route.target.path.name, "sink.dashboard");
    match &route.payload.kind {
        TypeKind::App { callee, args } => {
            assert_eq!(callee.name, "Stream");
            assert_eq!(args.len(), 1, "expected Stream payload to be generic");
            match &args[0].kind {
                TypeKind::Ident { name } => assert_eq!(name.name, "Int"),
                other => panic!("expected payload Int type, got {:?}", other),
            }
        }
        other => panic!("expected Stream<Int> payload, got {:?}", other),
    }
    assert!(
        conductor.execution.is_some(),
        "execution block should be preserved"
    );
    let monitoring = conductor
        .monitoring
        .as_ref()
        .expect("monitoring block should be present");
    match &monitoring.target {
        Some(ConductorMonitorTarget::Module(target)) => {
            assert_eq!(target.name, "Telemetry::Observer");
        }
        other => panic!("expected monitoring target module, got {:?}", other),
    }
}

#[test]
fn ch2_stream_301_parses_streaming_example() {
    let module = parse_example_module(
        "examples/spec_core/chapter2/streaming/core-parse-runstream-demandhint-ok.reml",
    );
    let digits = module
        .functions
        .iter()
        .find(|function| function.name.name == "digits")
        .expect("digits function should be parsed");
    let ret_type = digits
        .ret_type
        .as_ref()
        .expect("digits should retain an explicit return type");
    match &ret_type.kind {
        TypeKind::App { callee, args } => {
            assert_eq!(callee.name, "Parse::Parser");
            assert!(
                !args.is_empty(),
                "Parse::Parser should keep its payload type parameter"
            );
        }
        other => panic!("expected Parse::Parser return type, got {:?}", other),
    }
    let main_fn = module
        .functions
        .iter()
        .find(|function| function.name.name == "main")
        .expect("main function should exist");
    assert!(
        expr_contains_array(&main_fn.body),
        "stream chunks array literal should survive parsing"
    );
}

fn expr_contains_array(expr: &Expr) -> bool {
    match &expr.kind {
        ExprKind::Literal(literal) => match &literal.value {
            LiteralKind::Array { .. } => true,
            LiteralKind::Tuple { elements } => elements.iter().any(expr_contains_array),
            LiteralKind::Record { fields } => {
                fields.iter().any(|field| expr_contains_array(&field.value))
            }
            _ => false,
        },
        ExprKind::Call { callee, args } => {
            expr_contains_array(callee) || args.iter().any(expr_contains_array)
        }
        ExprKind::PerformCall { call } => expr_contains_array(&call.argument),
        ExprKind::Pipe { left, right }
        | ExprKind::Binary { left, right, .. } => {
            expr_contains_array(left) || expr_contains_array(right)
        }
        ExprKind::Unary { expr: inner, .. } => expr_contains_array(inner),
        ExprKind::FieldAccess { target, .. }
        | ExprKind::TupleAccess { target, .. } => expr_contains_array(target),
        ExprKind::Index { target, index } => {
            expr_contains_array(target) || expr_contains_array(index)
        }
        ExprKind::Propagate { expr: inner }
        | ExprKind::Loop { body: inner }
        | ExprKind::Unsafe { body: inner }
        | ExprKind::Defer { body: inner } => expr_contains_array(inner),
        ExprKind::Handle { handle } => expr_contains_array(&handle.target),
        ExprKind::Lambda { body, .. } => expr_contains_array(body),
        ExprKind::IfElse {
            condition,
            then_branch,
            else_branch,
        } => {
            expr_contains_array(condition)
                || expr_contains_array(then_branch)
                || else_branch
                    .as_ref()
                    .map_or(false, |branch| expr_contains_array(branch))
        }
        ExprKind::Match { target, arms } => {
            expr_contains_array(target)
                || arms
                    .iter()
                    .any(|arm| expr_contains_array(&arm.body))
        }
        ExprKind::While { condition, body } => {
            expr_contains_array(condition) || expr_contains_array(body)
        }
        ExprKind::For { start, end, .. } => {
            expr_contains_array(start) || expr_contains_array(end)
        }
        ExprKind::Block { statements, .. } => statements.iter().any(stmt_contains_array),
        ExprKind::Return { value } => value
            .as_ref()
            .map_or(false, |expr| expr_contains_array(expr)),
        ExprKind::Assign { target, value } => {
            expr_contains_array(target) || expr_contains_array(value)
        }
        ExprKind::Identifier(_)
        | ExprKind::ModulePath(_)
        | ExprKind::Continue => false,
    }
}

fn stmt_contains_array(stmt: &Stmt) -> bool {
    match &stmt.kind {
        StmtKind::Decl { decl } => decl_contains_array(decl),
        StmtKind::Expr { expr } | StmtKind::Defer { expr } => expr_contains_array(expr),
        StmtKind::Assign { target, value } => {
            expr_contains_array(target) || expr_contains_array(value)
        }
    }
}

fn decl_contains_array(decl: &Decl) -> bool {
    match &decl.kind {
        DeclKind::Let { value, .. } | DeclKind::Var { value, .. } => expr_contains_array(value),
        _ => false,
    }
}
