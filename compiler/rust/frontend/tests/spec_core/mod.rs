use reml_frontend::parser::ast::{
    Attribute, ConductorMonitorTarget, Decl, DeclKind, EffectCall, Expr, ExprKind, Function, Ident,
    ImplDecl, ImplItem, LiteralKind, Module, Param, Pattern, PatternKind, Stmt, StmtKind, TypeKind,
    UseTree, Visibility,
};
use reml_frontend::span::Span;
use reml_frontend::typeck::{TypecheckConfig, TypecheckDriver, TypecheckReport};

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

#[test]
fn ch1_match_003_accepts_guard_and_alias() {
    let module = parse_example_module(
        "examples/spec_core/chapter1/match_expr/bnf-matchexpr-when-guard-ok.reml",
    );
    let describe_fn = module
        .functions
        .iter()
        .find(|function| function.name.name == "describe")
        .expect("describe function should be present");
    let match_expr = match &describe_fn.body.kind {
        ExprKind::Block { statements, .. } => statements
            .iter()
            .find_map(|stmt| match &stmt.kind {
                StmtKind::Expr { expr } => Some(expr),
                _ => None,
            })
            .expect("match expression should exist in describe body"),
        other => panic!("expected describe body to be a block, got {:?}", other),
    };
    match &match_expr.kind {
        ExprKind::Match { arms, .. } => {
            let guarded_arm = arms
                .first()
                .expect("match expression should contain at least one arm");
            assert!(
                guarded_arm.guard.is_some(),
                "match guard should be captured on the first arm"
            );
            let alias_name = guarded_arm
                .alias
                .as_ref()
                .map(|ident| ident.name.as_str())
                .unwrap_or("missing");
            assert_eq!(
                alias_name, "large",
                "match arm alias should be parsed as `large`"
            );
        }
        other => panic!("expected match expression, got {:?}", other),
    }
}

#[test]
fn ch1_inf_601_accepts_fn_lambda_in_let_binding() {
    let module = parse_example_module(
        "examples/spec_core/chapter1/type_inference/bnf-inference-let-generalization-ok.reml",
    );
    let main_fn = module
        .functions
        .iter()
        .find(|function| function.name.name == "main")
        .expect("main function should exist");
    let statements = match &main_fn.body.kind {
        ExprKind::Block { statements, .. } => statements,
        other => panic!("main body should be a block, got {:?}", other),
    };
    let lambda_expr = statements.iter().find_map(|stmt| match &stmt.kind {
        StmtKind::Decl { decl } => match &decl.kind {
            DeclKind::Let { pattern, value, .. } => match &pattern.kind {
                PatternKind::Var(ident) if ident.name == "id" => Some(value),
                _ => None,
            },
            _ => None,
        },
        _ => None,
    });
    match lambda_expr.and_then(|expr| match &expr.kind {
        ExprKind::Lambda { .. } => Some(()),
        _ => None,
    }) {
        Some(()) => {}
        None => panic!("expected let id = fn (...) => ... to be parsed as a lambda expression"),
    }
}

#[test]
fn ch1_trait_decl_handles_generics_and_where_clause() {
    let module = parse_example_module(
        "examples/spec_core/chapter1/trait_impl/bnf-traitdecl-default-where-ok.reml",
    );
    let trait_decl = module
        .decls
        .iter()
        .find_map(|decl| match &decl.kind {
            DeclKind::Trait(trait_decl) => Some(trait_decl),
            _ => None,
        })
        .expect("trait declaration should be parsed");
    assert_eq!(trait_decl.name.name, "Show");
    assert_eq!(trait_decl.generics.len(), 1);
    assert_eq!(
        trait_decl.where_clause.len(),
        1,
        "expected trait where clause to be collected"
    );
    assert_eq!(
        trait_decl.items.len(),
        2,
        "two trait methods should be recorded"
    );
    let first_item = &trait_decl.items[0];
    assert_eq!(first_item.signature.name.name, "show");
    assert!(
        first_item.default_body.is_none(),
        "trait method without body should not synthesize a block"
    );
    let second_item = &trait_decl.items[1];
    assert_eq!(second_item.signature.name.name, "show_with_label");
    assert!(
        matches!(
            second_item.default_body.as_ref().map(|expr| &expr.kind),
            Some(ExprKind::Block { .. })
        ),
        "default implementation should be parsed as a block expression"
    );
}

#[test]
fn ch1_impl_decl_supports_trait_impl_items() {
    let module = parse_example_module(
        "examples/spec_core/chapter1/trait_impl/bnf-impldecl-duplicate-error.reml",
    );
    let impls = module
        .decls
        .iter()
        .filter_map(|decl| match &decl.kind {
            DeclKind::Impl(impl_decl) => Some(impl_decl),
            _ => None,
        })
        .collect::<Vec<&ImplDecl>>();
    assert_eq!(impls.len(), 2, "duplicate impls should both be parsed");
    let trait_impl = impls
        .iter()
        .find(|impl_decl| impl_decl.trait_ref.is_some())
        .expect("trait impl should exist in duplicate scenario");
    let trait_ref = trait_impl
        .trait_ref
        .as_ref()
        .expect("trait reference should be recorded");
    assert_eq!(trait_ref.name.name, "MiniDisplay");
    assert_eq!(
        trait_impl.items.len(),
        1,
        "impl block should retain the render method"
    );
    match &trait_impl.items[0] {
        ImplItem::Function(function) => assert_eq!(function.name.name, "render"),
        other => panic!("expected function impl item, got {:?}", other),
    }
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
        ExprKind::Pipe { left, right } | ExprKind::Binary { left, right, .. } => {
            expr_contains_array(left) || expr_contains_array(right)
        }
        ExprKind::Unary { expr: inner, .. } => expr_contains_array(inner),
        ExprKind::FieldAccess { target, .. } | ExprKind::TupleAccess { target, .. } => {
            expr_contains_array(target)
        }
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
            expr_contains_array(target) || arms.iter().any(|arm| expr_contains_array(&arm.body))
        }
        ExprKind::While { condition, body } => {
            expr_contains_array(condition) || expr_contains_array(body)
        }
        ExprKind::For { start, end, .. } => expr_contains_array(start) || expr_contains_array(end),
        ExprKind::Block { statements, .. } => statements.iter().any(stmt_contains_array),
        ExprKind::Return { value } => value
            .as_ref()
            .map_or(false, |expr| expr_contains_array(expr)),
        ExprKind::Assign { target, value } => {
            expr_contains_array(target) || expr_contains_array(value)
        }
        ExprKind::Identifier(_) | ExprKind::ModulePath(_) | ExprKind::Continue => false,
    }
}

fn has_violation(report: &TypecheckReport, code: &str) -> bool {
    report
        .violations
        .iter()
        .any(|violation| violation.code == code)
}

fn dummy_span() -> Span {
    Span::default()
}

fn make_ident(name: &str) -> Ident {
    Ident {
        name: name.to_string(),
        span: dummy_span(),
    }
}

fn make_pattern(name: &str) -> Pattern {
    Pattern {
        kind: PatternKind::Var(make_ident(name)),
        span: dummy_span(),
    }
}

fn make_let_decl(name: &str, value: Expr) -> Decl {
    Decl {
        attrs: Vec::new(),
        visibility: Visibility::Private,
        kind: DeclKind::Let {
            pattern: make_pattern(name),
            value,
            type_annotation: None,
        },
        span: dummy_span(),
    }
}

fn make_var_decl(name: &str, value: Expr) -> Decl {
    Decl {
        attrs: Vec::new(),
        visibility: Visibility::Private,
        kind: DeclKind::Var {
            pattern: make_pattern(name),
            value,
            type_annotation: None,
        },
        span: dummy_span(),
    }
}

fn stmt_from_decl(decl: Decl) -> Stmt {
    Stmt {
        kind: StmtKind::Decl { decl },
        span: dummy_span(),
    }
}

fn stmt_expr(expr: Expr) -> Stmt {
    Stmt {
        kind: StmtKind::Expr {
            expr: Box::new(expr),
        },
        span: dummy_span(),
    }
}

fn make_lambda(param: &str, body: Expr) -> Expr {
    let param = Param {
        name: make_ident(param),
        type_annotation: None,
        default: None,
        span: dummy_span(),
    };
    Expr {
        span: dummy_span(),
        kind: ExprKind::Lambda {
            params: vec![param],
            ret_type: None,
            body: Box::new(body),
        },
    }
}

fn make_perform(effect: &str, argument: Expr) -> Expr {
    Expr {
        span: dummy_span(),
        kind: ExprKind::PerformCall {
            call: EffectCall {
                effect: make_ident(effect),
                argument: Box::new(argument),
            },
        },
    }
}

fn literal_int(value: i64) -> Expr {
    Expr::int(value, value.to_string(), dummy_span())
}

fn ident_expr(name: &str) -> Expr {
    Expr::identifier(make_ident(name))
}

fn block_expr(statements: Vec<Stmt>) -> Expr {
    Expr::block(statements, dummy_span())
}

fn build_function(body: Expr, attrs: Vec<Attribute>) -> Function {
    Function {
        name: make_ident("sample"),
        generics: Vec::new(),
        params: Vec::new(),
        body,
        ret_type: None,
        where_clause: Vec::new(),
        effect: None,
        span: dummy_span(),
        attrs,
    }
}

fn run_typecheck(function: Function) -> TypecheckReport {
    let module = Module {
        header: None,
        uses: Vec::new(),
        effects: Vec::new(),
        functions: vec![function],
        decls: Vec::new(),
    };
    TypecheckDriver::infer_module(Some(&module), &TypecheckConfig::default())
}

#[test]
fn ch1_inf_601_typecheck_runs_without_ast_abort() {
    let id_lambda = make_lambda("x", ident_expr("x"));
    let id_decl = stmt_from_decl(make_let_decl("id", id_lambda));
    let first_decl = stmt_from_decl(make_let_decl("first", ident_expr("id")));
    let statements = vec![id_decl, first_decl, stmt_expr(ident_expr("first"))];
    let body = block_expr(statements);
    let report = run_typecheck(build_function(body, Vec::new()));
    assert!(
        !has_violation(&report, "typeck.aborted.ast_unavailable"),
        "typeck.aborted.ast_unavailable should not appear for CH1-INF-601"
    );
    assert!(
        !report.functions.is_empty(),
        "typed functions should be recorded when type inference succeeds"
    );
}

#[test]
fn ch1_inf_602_reports_value_restriction_violation() {
    let cell_decl = stmt_from_decl(make_var_decl("cell", literal_int(0)));
    let statements = vec![cell_decl];
    let body = block_expr(statements);
    let report = run_typecheck(build_function(body, Vec::new()));
    assert!(
        has_violation(&report, "language.inference.value_restriction"),
        "value restriction diagnostics should surface for CH1-INF-602"
    );
}

#[test]
fn ch1_inf_602_module_example_emits_value_restriction() {
    let module = parse_example_module(
        "examples/spec_core/chapter1/type_inference/bnf-inference-value-restriction-error.reml",
    );
    let report = TypecheckDriver::infer_module(Some(&module), &TypecheckConfig::default());
    assert!(
        has_violation(&report, "language.inference.value_restriction"),
        "example module should emit language.inference.value_restriction at top-level"
    );
}

#[test]
fn ch1_eff_701_reports_purity_violation() {
    let perform = stmt_expr(make_perform("Console::log", literal_int(1)));
    let body = block_expr(vec![perform]);
    let attrs = vec![Attribute {
        name: make_ident("pure"),
        args: Vec::new(),
        span: dummy_span(),
    }];
    let report = run_typecheck(build_function(body, attrs));
    assert!(
        has_violation(&report, "effects.purity.violated"),
        "pure functions performing effects must emit purity violations"
    );
}

#[test]
fn ch1_impl_302_reports_duplicate_impl_violation() {
    let module = parse_example_module(
        "examples/spec_core/chapter1/trait_impl/bnf-impldecl-duplicate-error.reml",
    );
    let report = TypecheckDriver::infer_module(Some(&module), &TypecheckConfig::default());
    let has_duplicate = report
        .violations
        .iter()
        .any(|violation| violation.code == "typeclass.impl.duplicate");
    assert!(
        has_duplicate,
        "duplicate impl scenario should emit typeclass.impl.duplicate diagnostic"
    );
}

#[test]
fn ch2_parse_201_reports_core_parse_recover_branch() {
    let module = parse_example_module(
        "examples/spec_core/chapter2/parser_core/core-parse-recover-diagnostic.reml",
    );
    let report = TypecheckDriver::infer_module(Some(&module), &TypecheckConfig::default());
    assert!(
        has_violation(&report, "core.parse.recover.branch"),
        "Parse.run_with_recovery scenario should emit core.parse.recover.branch diagnostic"
    );
}

#[test]
fn ch3_runtime_601_reports_runtime_bridge_stage_mismatch() {
    let module = parse_example_module(
        "examples/practical/core_runtime/capability/stage_mismatch_runtime_bridge.reml",
    );
    let report = TypecheckDriver::infer_module(Some(&module), &TypecheckConfig::default());
    assert!(
        has_violation(&report, "runtime.bridge.stage_mismatch"),
        "RuntimeBridge.verify_stage mismatch should emit runtime.bridge.stage_mismatch diagnostic"
    );
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
