use reml_frontend::diagnostic::FrontendDiagnostic;
use reml_frontend::parser::ast::{
    Attribute, ConductorMonitorTarget, Decl, DeclKind, EffectCall, Expr, ExprKind, Function, Ident,
    ImplDecl, ImplItem, LiteralKind, Module, Param, Pattern, PatternKind, Stmt, StmtKind,
    TraitItemKind, TypeKind, UseTree, Visibility,
};
use reml_frontend::parser::{ParserDriver, ParserOptions, RunConfig};
use reml_frontend::span::Span;
use reml_frontend::typeck::{TypecheckConfig, TypecheckDriver, TypecheckReport};

mod common;

use common::{parse_example_module, repo_root};

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
fn ch1_mod_004_reports_invalid_super_use() {
    let input_path = repo_root()
        .join("examples/spec_core/chapter1/module_use/bnf-usedecl-super-root-invalid.reml");
    let source = std::fs::read_to_string(&input_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", input_path.display()));
    let run_config = RunConfig::default();
    let parser_options = ParserOptions::from_run_config(&run_config);
    let result =
        ParserDriver::parse_with_options_and_run_config(&source, parser_options, run_config);
    let codes = result
        .diagnostics
        .iter()
        .filter_map(|diag| diag.code.as_deref())
        .collect::<Vec<_>>();
    assert!(
        codes.contains(&"language.use.invalid_super"),
        "expected language.use.invalid_super, got {:?}",
        codes
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
fn ch1_let_003_reports_unicode_shadowing_violation() {
    let module = parse_example_module(
        "examples/spec_core/chapter1/let_binding/bnf-valdecl-let-shadow-unicode.reml",
    );
    let report = TypecheckDriver::infer_module(Some(&module), &TypecheckConfig::default());
    assert!(
        has_violation(&report, "language.shadowing.unicode"),
        "Unicode let shadowing should be rejected, got {:?}",
        report
            .violations
            .iter()
            .map(|violation| violation.code)
            .collect::<Vec<_>>()
    );
}

#[test]
fn ch1_lit_202_parses_float_literal_forms() {
    let module =
        parse_example_module("examples/spec_core/chapter1/literals/bnf-literal-float-forms.reml");
    let mut raws = Vec::new();
    for decl in &module.decls {
        if let DeclKind::Let { value, .. } = &decl.kind {
            if let ExprKind::Literal(literal) = &value.kind {
                if let LiteralKind::Float { raw } = &literal.value {
                    raws.push(raw.clone());
                }
            }
        }
    }
    assert!(
        raws.contains(&"3.141_592".to_string())
            && raws.contains(&"1.25e-3".to_string())
            && raws.contains(&"2E+2".to_string()),
        "expected float literals to be preserved, got {:?}",
        raws
    );
}

#[test]
fn ch1_attr_102_reports_cfg_unsatisfied_branch() {
    let input_path = repo_root()
        .join("examples/spec_core/chapter1/attributes/bnf-attr-cfg-missing-flag-error.reml");
    let source = std::fs::read_to_string(&input_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", input_path.display()));
    let run_config = RunConfig::default();
    let parser_options = ParserOptions::from_run_config(&run_config);
    let result =
        ParserDriver::parse_with_options_and_run_config(&source, parser_options, run_config);
    let codes = result
        .diagnostics
        .iter()
        .filter_map(|diag| diag.code.as_deref())
        .collect::<Vec<_>>();
    assert!(
        codes.contains(&"language.cfg.unsatisfied_branch"),
        "expected language.cfg.unsatisfied_branch, got {:?}",
        codes
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
fn ch2_op_401_reports_opbuilder_level_conflict() {
    let module = parse_example_module(
        "examples/spec_core/chapter2/op_builder/core-opbuilder-level-conflict-error.reml",
    );
    let report = TypecheckDriver::infer_module(Some(&module), &TypecheckConfig::default());
    assert!(
        has_violation(&report, "core.parse.opbuilder.level_conflict"),
        "expected core.parse.opbuilder.level_conflict, got {:?}",
        report
            .violations
            .iter()
            .map(|violation| violation.code)
            .collect::<Vec<_>>()
    );
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
fn ch1_match_002_accepts_tuple_literal_pattern() {
    let module = parse_example_module(
        "examples/spec_core/chapter1/match_expr/bnf-matchexpr-tuple-alternate.reml",
    );
    let describe_fn = module
        .functions
        .iter()
        .find(|function| function.name.name == "describe")
        .expect("describe function should exist");
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
            let tuple_arm = arms
                .first()
                .expect("tuple match should contain at least one arm");
            match &tuple_arm.pattern.kind {
                PatternKind::Tuple { elements } => {
                    assert_eq!(
                        elements.len(),
                        2,
                        "tuple pattern should contain two elements"
                    );
                    match &elements[0].kind {
                        PatternKind::Literal(literal) => match &literal.value {
                            LiteralKind::Int { value, .. } => {
                                assert_eq!(*value, 0, "first element should be literal 0")
                            }
                            other => panic!("expected int literal in tuple pattern, got {other:?}"),
                        },
                        other => panic!("expected literal pattern, got {other:?}"),
                    }
                    match &elements[1].kind {
                        PatternKind::Var(ident) => assert_eq!(
                            ident.name, "y",
                            "second element should bind the `y` identifier"
                        ),
                        other => panic!("expected identifier binding, got {other:?}"),
                    }
                }
                other => panic!("expected tuple pattern, got {:?}", other),
            }
        }
        other => panic!("expected match expression, got {:?}", other),
    }
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

    #[test]
    fn ch1_act_001_parses_partial_active_pattern_definition() {
        let module = parse_example_module(
            "examples/spec_core/chapter1/active_patterns/bnf-activepattern-partial-ok.reml",
        );
        assert_eq!(
            module.active_patterns.len(),
            1,
            "expected one active pattern declaration"
        );
        let active = &module.active_patterns[0];
        assert!(active.is_partial, "active pattern should be partial");
        assert_eq!(active.name.name, "IsFoo");
        assert_eq!(active.params.len(), 1, "expected one parameter");

        let main_fn = module
            .functions
            .iter()
            .find(|function| function.name.name == "main")
            .expect("main function should exist");
        let match_expr = match &main_fn.body.kind {
            ExprKind::Block { statements, .. } => statements
                .iter()
                .find_map(|stmt| match &stmt.kind {
                    StmtKind::Expr { expr } => Some(expr),
                    _ => None,
                })
                .expect("match expression should be present"),
            other => panic!("expected block body, got {:?}", other),
        };
        match &match_expr.kind {
            ExprKind::Match { arms, .. } => {
                let first_arm = arms.first().expect("expected a match arm");
                match &first_arm.pattern.kind {
                    PatternKind::ActivePattern {
                        name,
                        is_partial,
                        argument,
                    } => {
                        assert_eq!(name.name, "IsFoo");
                        assert!(*is_partial, "match arm should treat pattern as partial");
                        assert!(
                            argument.is_some(),
                            "active pattern application should capture the argument"
                        );
                    }
                    other => panic!("expected active pattern application, got {:?}", other),
                }
            }
            other => panic!("expected match expression, got {:?}", other),
        }
    }

    #[test]
    fn match_guard_accepts_alias_before_guard() {
        let source = r#"
module Spec.Core.Chapter1.ActivePatterns.AliasOrder

fn demo(n: Int) -> Int = {
  match n with
  | _ as value when value > 0 -> value
  | _ -> 0
}
"#;
        let result = ParserDriver::parse(source);
        assert!(
            result.diagnostics.is_empty(),
            "expected parser diagnostics to be empty, got {:?}",
            result
                .diagnostics
                .iter()
                .map(|diag| diag.code.clone())
                .collect::<Vec<_>>()
        );
        let module = result.value.expect("module should parse");
        let demo_fn = module
            .functions
            .iter()
            .find(|function| function.name.name == "demo")
            .expect("demo function should exist");
        let match_expr = match &demo_fn.body.kind {
            ExprKind::Block { statements, .. } => statements
                .iter()
                .find_map(|stmt| match &stmt.kind {
                    StmtKind::Expr { expr } => Some(expr),
                    _ => None,
                })
                .expect("match expression should be present"),
            other => panic!("expected block body, got {:?}", other),
        };
        match &match_expr.kind {
            ExprKind::Match { arms, .. } => {
                let first_arm = arms.first().expect("expected a match arm");
                let alias = first_arm
                    .alias
                    .as_ref()
                    .map(|ident| ident.name.as_str())
                    .unwrap_or("missing");
                assert_eq!(
                    alias, "value",
                    "alias should be parsed from alias-first form"
                );
                assert!(
                    first_arm.guard.is_some(),
                    "guard should be preserved after alias"
                );
            }
            other => panic!("expected match expression, got {:?}", other),
        }
    }

    #[test]
    fn match_guard_with_if_emits_deprecation_warning() {
        let source = r#"
module Spec.Core.Chapter1.ActivePatterns.IfGuard

fn demo(n: Int) -> Int = {
  match n with
  | _ if n > 0 -> n
  | _ -> 0
}
"#;
        let result = ParserDriver::parse(source);
        let codes = result
            .diagnostics
            .iter()
            .filter_map(|diag| diag.code.as_deref())
            .collect::<Vec<_>>();
        assert!(
            codes.contains(&"pattern.guard.if_deprecated"),
            "expected pattern.guard.if_deprecated, got {:?}",
            codes
        );
        let module = result.value.expect("module should parse");
        let demo_fn = module
            .functions
            .iter()
            .find(|function| function.name.name == "demo")
            .expect("demo function should exist");
        let has_if_guard = match &demo_fn.body.kind {
            ExprKind::Block { statements, .. } => statements.iter().any(|stmt| {
                if let StmtKind::Expr { expr } = &stmt.kind {
                    if let ExprKind::Match { arms, .. } = &expr.kind {
                        return arms
                            .iter()
                            .any(|arm| arm.guard.is_some() && arm.guard_used_if);
                    }
                }
                false
            }),
            _ => false,
        };
        assert!(has_if_guard, "match arm should record use of `if` guard");
    }
}

#[test]
fn ch1_act_003_reports_return_contract_violation_from_typeck() {
    let module = parse_example_module(
        "examples/spec_core/chapter1/active_patterns/bnf-activepattern-return-contract-error.reml",
    );
    let report = TypecheckDriver::infer_module(Some(&module), &TypecheckConfig::default());
    assert!(
        has_violation(&report, "pattern.active.return_contract_invalid"),
        "expected pattern.active.return_contract_invalid, got {:?}",
        report
            .violations
            .iter()
            .map(|violation| violation.code)
            .collect::<Vec<_>>()
    );
}

#[test]
fn active_pattern_perform_in_pure_context_reports_violation() {
    let source = r#"
module Spec.Core.Chapter1.ActivePatterns.PureEffect

use Core.Prelude

@pure
pattern (|Logger|_|)(n: Int) = perform Console("ping")

fn main() -> Int = {
  match 1 with
  | (|Logger|_|) v -> v
  | _ -> 0
}
"#;
    let result = ParserDriver::parse(source);
    let module = result.value.expect("module should parse");
    let report = TypecheckDriver::infer_module(Some(&module), &TypecheckConfig::default());
    assert!(
        has_violation(&report, "pattern.active.effect_violation"),
        "expected pattern.active.effect_violation, got {:?}",
        report
            .violations
            .iter()
            .map(|violation| violation.code)
            .collect::<Vec<_>>()
    );
}

#[test]
fn active_pattern_name_conflicts_with_function_symbol() {
    let source = r#"
module Spec.Core.Chapter1.ActivePatterns.NameConflict

pattern (|Demo|)(n: Int) = n

fn Demo(n: Int) -> Int = n
"#;
    let module = ParserDriver::parse(source)
        .value
        .expect("module should parse");
    let report = TypecheckDriver::infer_module(Some(&module), &TypecheckConfig::default());
    assert!(
        has_violation(&report, "pattern.active.name_conflict"),
        "expected pattern.active.name_conflict, got {:?}",
        report
            .violations
            .iter()
            .map(|violation| violation.code)
            .collect::<Vec<_>>()
    );
}

#[test]
fn match_with_partial_active_pattern_requires_fallback() {
    let source = r#"
module Spec.Core.Chapter1.ActivePatterns.Exhaustiveness

use Core.Prelude

pattern (|IsFoo|_|)(s: String) = if s == "foo" then Some(()) else None

fn main() -> Int = {
  match "foo" with
  | (|IsFoo|_|) () -> 1
}
"#;
    let result = ParserDriver::parse(source);
    let module = result.value.expect("module should parse");
    let report = TypecheckDriver::infer_module(Some(&module), &TypecheckConfig::default());
    assert!(
        has_violation(&report, "pattern.exhaustiveness.missing"),
        "expected pattern.exhaustiveness.missing, got {:?}",
        report
            .violations
            .iter()
            .map(|violation| violation.code)
            .collect::<Vec<_>>()
    );
}

#[test]
fn typed_active_pattern_carries_miss_path_flag() {
    let module = parse_example_module(
        "examples/spec_core/chapter1/active_patterns/bnf-activepattern-partial-ok.reml",
    );
    let report = TypecheckDriver::infer_module(Some(&module), &TypecheckConfig::default());
    let partial = report
        .typed_module
        .active_patterns
        .iter()
        .find(|pattern| pattern.name == "IsFoo")
        .expect("partial active pattern should be present");
    assert!(
        partial.has_miss_path,
        "partial active pattern should require miss path (None -> next arm)"
    );

    let total_source = r#"
module Spec.Core.Chapter1.ActivePatterns.TotalCoverage

use Core.Prelude

pattern (|Total|)(n: Int) = n

fn main() -> Int = {
  match 1 with
  | (|Total|) v -> v
}
"#;
    let total_module = ParserDriver::parse(total_source)
        .value
        .expect("total active pattern module should parse");
    let total_report =
        TypecheckDriver::infer_module(Some(&total_module), &TypecheckConfig::default());
    let total = total_report
        .typed_module
        .active_patterns
        .iter()
        .find(|pattern| pattern.name == "Total")
        .expect("total active pattern should be present");
    assert!(
        !total.has_miss_path,
        "total active pattern should not have a miss path"
    );
}

#[test]
fn active_pattern_with_guard_does_not_trigger_unreachable() {
    let source = r#"
module Spec.Core.Chapter1.ActivePatterns.GuardedTotal

use Core.Prelude

pattern (|Always|)(n: Int) = n

fn eval(n: Int) -> Int = {
  match n with
  | (|Always|) v when v > 0 -> v
  | (|Always|) v -> v + 1
}
"#;
    let module = ParserDriver::parse(source)
        .value
        .expect("guarded active pattern module should parse");
    let report = TypecheckDriver::infer_module(Some(&module), &TypecheckConfig::default());
    assert!(
        !has_violation(&report, "pattern.unreachable_arm"),
        "guarded total active pattern should not make following arms unreachable"
    );
}

#[test]
fn ch1_match_014_reports_binding_duplicate_name() {
    let module = parse_example_module(
        "examples/spec_core/chapter1/match_expr/bnf-match-binding-duplicate.reml",
    );
    let report = TypecheckDriver::infer_module(Some(&module), &TypecheckConfig::default());
    let codes = report.violations.iter().map(|v| v.code).collect::<Vec<_>>();
    assert!(
        has_violation(&report, "pattern.binding.duplicate_name"),
        "expected pattern.binding.duplicate_name, got {:?}",
        codes
    );
}

#[test]
fn ch1_match_016_reports_regex_unsupported_target() {
    let module = parse_example_module(
        "examples/spec_core/chapter1/match_expr/bnf-match-regex-unsupported-target.reml",
    );
    let report = TypecheckDriver::infer_module(Some(&module), &TypecheckConfig::default());
    let codes = report.violations.iter().map(|v| v.code).collect::<Vec<_>>();
    assert!(
        has_violation(&report, "pattern.regex.unsupported_target"),
        "expected pattern.regex.unsupported_target, got {:?}",
        codes
    );
}

#[test]
fn ch1_match_008_reports_unreachable_or_arm_after_wildcard() {
    let module = parse_example_module(
        "examples/spec_core/chapter1/match_expr/bnf-match-or-pattern-unreachable.reml",
    );
    let report = TypecheckDriver::infer_module(Some(&module), &TypecheckConfig::default());
    assert!(
        has_violation(&report, "pattern.unreachable_arm"),
        "expected pattern.unreachable_arm for trailing or-pattern arm"
    );
}

#[test]
fn ch1_match_009_accepts_slice_head_tail() {
    let module = parse_example_module(
        "examples/spec_core/chapter1/match_expr/bnf-match-slice-head-tail-ok.reml",
    );
    let report = TypecheckDriver::infer_module(Some(&module), &TypecheckConfig::default());
    assert!(
        report.violations.is_empty(),
        "expected no diagnostics, got {:?}",
        report
            .violations
            .iter()
            .map(|violation| violation.code)
            .collect::<Vec<_>>()
    );
}

#[test]
fn ch1_match_010_reports_slice_multiple_rest() {
    let module = parse_example_module(
        "examples/spec_core/chapter1/match_expr/bnf-match-slice-multiple-rest.reml",
    );
    let report = TypecheckDriver::infer_module(Some(&module), &TypecheckConfig::default());
    assert!(
        has_violation(&report, "pattern.slice.multiple_rest"),
        "expected pattern.slice.multiple_rest, got {:?}",
        report
            .violations
            .iter()
            .map(|violation| violation.code)
            .collect::<Vec<_>>()
    );
}

#[test]
fn ch1_match_011_accepts_range_inclusive() {
    let module = parse_example_module(
        "examples/spec_core/chapter1/match_expr/bnf-match-range-inclusive-ok.reml",
    );
    let report = TypecheckDriver::infer_module(Some(&module), &TypecheckConfig::default());
    assert!(
        report.violations.is_empty(),
        "expected no diagnostics, got {:?}",
        report
            .violations
            .iter()
            .map(|violation| violation.code)
            .collect::<Vec<_>>()
    );
}

#[test]
fn ch1_match_012_reports_range_bound_inverted() {
    let module = parse_example_module(
        "examples/spec_core/chapter1/match_expr/bnf-match-range-bound-inverted.reml",
    );
    let report = TypecheckDriver::infer_module(Some(&module), &TypecheckConfig::default());
    assert!(
        has_violation(&report, "pattern.range.bound_inverted"),
        "expected pattern.range.bound_inverted, got {:?}",
        report
            .violations
            .iter()
            .map(|violation| violation.code)
            .collect::<Vec<_>>()
    );
}

#[test]
fn ch1_match_013_accepts_binding_alias() {
    let module =
        parse_example_module("examples/spec_core/chapter1/match_expr/bnf-match-binding-as-ok.reml");
    let report = TypecheckDriver::infer_module(Some(&module), &TypecheckConfig::default());
    assert!(
        report.violations.is_empty(),
        "expected no diagnostics, got {:?}",
        report
            .violations
            .iter()
            .map(|violation| violation.code)
            .collect::<Vec<_>>()
    );
}

#[test]
fn ch1_match_017_accepts_active_pattern_or_combination() {
    let module = parse_example_module(
        "examples/spec_core/chapter1/match_expr/bnf-match-active-or-combined.reml",
    );
    let report = TypecheckDriver::infer_module(Some(&module), &TypecheckConfig::default());
    assert!(
        report.violations.is_empty(),
        "expected no diagnostics, got {:?}",
        report
            .violations
            .iter()
            .map(|violation| violation.code)
            .collect::<Vec<_>>()
    );
}

#[test]
fn match_reports_unreachable_arm_after_wildcard() {
    let source = r#"
module Spec.Core.Chapter1.ActivePatterns.Unreachable

fn demo(n: Int) -> Int = {
  match n with
  | _ -> 0
  | 1 -> 1
}
"#;
    let result = ParserDriver::parse(source);
    let module = result.value.expect("module should parse");
    let report = TypecheckDriver::infer_module(Some(&module), &TypecheckConfig::default());
    assert!(
        has_violation(&report, "pattern.unreachable_arm"),
        "expected pattern.unreachable_arm, got {:?}",
        report
            .violations
            .iter()
            .map(|violation| violation.code)
            .collect::<Vec<_>>()
    );
}

#[test]
fn ch1_effects_201_parses_handle_expr_perform_counter() {
    let module = parse_example_module(
        "examples/spec_core/chapter1/effect_handlers/bnf-handleexpr-perform-counter.reml",
    );
    let sum_pair = module
        .functions
        .iter()
        .find(|function| function.name.name == "sum_pair")
        .expect("sum_pair function should exist");
    let statements = match &sum_pair.body.kind {
        ExprKind::Block { statements, .. } => statements,
        other => panic!("sum_pair body should be a block, got {:?}", other),
    };
    let perform_units: Vec<String> = statements
        .iter()
        .filter_map(|stmt| match &stmt.kind {
            StmtKind::Decl { decl } => match &decl.kind {
                DeclKind::Let { value, .. } => match &value.kind {
                    ExprKind::PerformCall { call } => match &call.argument.kind {
                        ExprKind::Literal(literal) => match literal.value {
                            LiteralKind::Unit => Some(call.effect.name.clone()),
                            ref other => {
                                panic!("perform argument should be unit literal, got {:?}", other)
                            }
                        },
                        other => panic!(
                            "perform argument should remain a literal expression, got {:?}",
                            other
                        ),
                    },
                    _ => None,
                },
                _ => None,
            },
            _ => None,
        })
        .collect();
    assert_eq!(
        perform_units.len(),
        2,
        "two perform calls should be captured in sum_pair"
    );
    assert!(
        perform_units.iter().all(|effect| effect == "Counter::next"),
        "perform targets should resolve to Counter::next: {:?}",
        perform_units
    );

    let main_fn = module
        .functions
        .iter()
        .find(|function| function.name.name == "main")
        .expect("main function should exist");
    let handle_present = match &main_fn.body.kind {
        ExprKind::Block { statements, .. } => statements.iter().any(|stmt| match &stmt.kind {
            StmtKind::Expr { expr } => matches!(expr.kind, ExprKind::Handle { .. }),
            _ => false,
        }),
        other => panic!("main body should be a block, got {:?}", other),
    };
    assert!(
        handle_present,
        "handle expression should survive parsing inside fn main"
    );
}

#[test]
fn ch1_effects_missing_with_reports_single_diagnostic() {
    let source_path = common::repo_root()
        .join("examples/spec_core/chapter1/effect_handlers/bnf-handleexpr-missing-with.reml");
    let source =
        std::fs::read_to_string(&source_path).expect("failed to read missing_with scenario");
    let result = ParserDriver::parse(&source);
    let missing_with_count = result
        .diagnostics
        .iter()
        .filter(|diag| diag_has_code(diag, "effects.handler.missing_with"))
        .count();
    assert_eq!(
        missing_with_count, 1,
        "missing-with scenario should emit exactly one parser diagnostic"
    );
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
    match &first_item.kind {
        TraitItemKind::Function {
            signature,
            default_body,
        } => {
            assert_eq!(signature.name.name, "show");
            assert!(
                default_body.is_none(),
                "trait method without body should not synthesize a block"
            );
        }
        other => panic!("expected function trait item, got {:?}", other),
    }
    let second_item = &trait_decl.items[1];
    match &second_item.kind {
        TraitItemKind::Function {
            signature,
            default_body,
        } => {
            assert_eq!(signature.name.name, "show_with_label");
            assert!(
                matches!(
                    default_body.as_ref().map(|expr| &expr.kind),
                    Some(ExprKind::Block { .. })
                ),
                "default implementation should be parsed as a block expression"
            );
        }
        other => panic!("expected function trait item, got {:?}", other),
    }
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
            LiteralKind::Record { fields, .. } => {
                fields.iter().any(|field| expr_contains_array(&field.value))
            }
            _ => false,
        },
        ExprKind::FixityLiteral(_)
        | ExprKind::Identifier(_)
        | ExprKind::ModulePath(_)
        | ExprKind::Continue => false,
        ExprKind::Call { callee, args } => {
            expr_contains_array(callee) || args.iter().any(expr_contains_array)
        }
        ExprKind::PerformCall { call } => expr_contains_array(&call.argument),
        ExprKind::Pipe { left, right } | ExprKind::Binary { left, right, .. } => {
            expr_contains_array(left) || expr_contains_array(right)
        }
        ExprKind::Unary { expr: inner, .. } => expr_contains_array(inner),
        ExprKind::Rec { expr: inner } => expr_contains_array(inner),
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
        ExprKind::EffectBlock { body } => expr_contains_array(body),
        ExprKind::Async { body, .. } => expr_contains_array(body),
        ExprKind::Await { expr: inner } => expr_contains_array(inner),
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
        ExprKind::Block { statements, defers, .. } => {
            statements.iter().any(stmt_contains_array) || defers.iter().any(expr_contains_array)
        }
        ExprKind::Return { value } => value
            .as_ref()
            .map_or(false, |expr| expr_contains_array(expr)),
        ExprKind::Assign { target, value } => {
            expr_contains_array(target) || expr_contains_array(value)
        }
        ExprKind::Break { value } => value
            .as_ref()
            .map_or(false, |expr| expr_contains_array(expr)),
        ExprKind::InlineAsm(_) | ExprKind::LlvmIr(_) => false,
    }
}

fn has_violation(report: &TypecheckReport, code: &str) -> bool {
    report
        .violations
        .iter()
        .any(|violation| violation.code == code)
}

fn diag_has_code(diag: &FrontendDiagnostic, code: &str) -> bool {
    diag.code.as_deref() == Some(code) || diag.codes.iter().any(|existing| existing == code)
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
        pattern: make_pattern(param),
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
        qualified_name: None,
        visibility: Visibility::Private,
        generics: Vec::new(),
        params: Vec::new(),
        body,
        ret_type: None,
        where_clause: Vec::new(),
        effect: None,
        is_async: false,
        is_unsafe: false,
        span: dummy_span(),
        attrs,
    }
}

fn run_typecheck(function: Function) -> TypecheckReport {
    let module = Module {
        header: None,
        uses: Vec::new(),
        effects: Vec::new(),
        active_patterns: Vec::new(),
        functions: vec![function],
        decls: Vec::new(),
        exprs: Vec::new(),
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
fn ch1_fn_103_reports_return_mismatch_before_condition_error() {
    let module = parse_example_module(
        "examples/spec_core/chapter1/fn_decl/bnf-fndecl-return-inference-error.reml",
    );
    let report = TypecheckDriver::infer_module(Some(&module), &TypecheckConfig::default());
    let first_code = report
        .violations
        .first()
        .map(|violation| violation.code)
        .expect("戻り値型診断が最初に出力されるはずです");
    assert_eq!(
        first_code, "language.inference.return_conflict",
        "戻り値型診断が先頭に並ぶ必要があります"
    );
    if let Some(index) = report
        .violations
        .iter()
        .position(|violation| violation.code == "E7006")
    {
        assert!(
            index > 0,
            "Bool 条件診断は戻り値型診断より前へ出るべきではありません"
        );
    }
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
