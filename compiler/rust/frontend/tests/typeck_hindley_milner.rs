use std::fmt::Write;

use reml_frontend::parser::ast::Module;
use reml_frontend::parser::ParserDriver;
use reml_frontend::semantics::typed::TypedExprKind;
use reml_frontend::typeck::{
    Constraint, Type, TypecheckConfig, TypecheckDriver, TypecheckReport, TypecheckViolationKind,
};
use serde_json;

fn parse_module(source: &str) -> Module {
    let result = ParserDriver::parse(source);
    assert!(
        result.diagnostics.is_empty(),
        "parser diagnostics: {:?}",
        result
            .diagnostics
            .iter()
            .map(|diag| &diag.message)
            .collect::<Vec<_>>()
    );
    result.value.expect("AST")
}

fn typecheck_source(source: &str) -> TypecheckReport {
    let module = parse_module(source);
    TypecheckDriver::infer_module(Some(&module), &TypecheckConfig::default())
}

#[test]
fn hindley_milner_collects_binary_constraints() {
    let report = typecheck_source("fn sum(x_int, y_int) = x_int + y_int");
    assert_eq!(report.functions.len(), 1);
    assert_eq!(report.metrics.constraints_total, report.constraints.len());
    assert_eq!(report.constraints.len(), 1);
    assert!(matches!(
        &report.constraints[0],
        Constraint::Equal { left, right }
        if matches!(left, Type::Var(_)) && matches!(right, Type::Var(_))
    ));
    assert!(
        report.used_impls.is_empty(),
        "現状では辞書が未使用のため空の Vec を返す"
    );
}

#[test]
fn hindley_milner_flags_non_bool_condition() {
    let report = typecheck_source(
        "fn check(flag_bool: Bool, value_int: Int) = if value_int then flag_bool else flag_bool",
    );
    assert_eq!(report.functions.len(), 1);
    assert!(
        report.violations.iter().any(|violation| {
            matches!(violation.kind, TypecheckViolationKind::ConditionLiteralBool)
        }),
        "非 Bool 条件は diagnostic に記録される"
    );
}

#[test]
fn hindley_milner_constraints_json_roundtrip() {
    let report = typecheck_source("fn pair(x_int, y_int) = x_int + y_int");
    let serialized =
        serde_json::to_value(&report.constraints).expect("constraints should serialize");
    assert!(serialized.is_array());
}

#[test]
fn reports_ast_unavailable_when_module_is_absent() {
    let report = TypecheckDriver::infer_module(None, &TypecheckConfig::default());
    assert!(
        report
            .violations
            .iter()
            .any(|violation| violation.code == "typeck.aborted.ast_unavailable"),
        "AST 不在時の診断が出力される"
    );
    assert!(
        report.typed_module.functions.is_empty(),
        "AST 不在時は typed_module が空のまま"
    );
}

#[test]
fn type_alias_cycle_reports_violation() {
    let report = typecheck_source(
        "type alias A = B\n\
type alias B = A\n\
fn apply(x: A) = x",
    );
    assert!(
        report
            .violations
            .iter()
            .any(|violation| { matches!(violation.kind, TypecheckViolationKind::TypeAliasCycle) }),
        "循環エイリアスは TypeAliasCycle を報告する"
    );
}

#[test]
fn type_alias_expansion_limit_reports_violation() {
    let mut source = String::new();
    for idx in 0..=32 {
        let _ = writeln!(&mut source, "type alias A{idx} = A{}", idx + 1);
    }
    let _ = writeln!(&mut source, "type alias A33 = Int");
    let _ = writeln!(&mut source, "fn apply(x: A0) = x");

    let report = typecheck_source(&source);
    assert!(
        report.violations.iter().any(|violation| {
            matches!(
                violation.kind,
                TypecheckViolationKind::TypeAliasExpansionLimit
            )
        }),
        "展開上限超過は TypeAliasExpansionLimit を報告する"
    );
}

#[test]
fn type_alias_generics_expands_without_violation() {
    let report = typecheck_source(
        "type alias Id<T> = T\n\
fn apply(x: Id<Int>) = x",
    );
    assert!(
        report.violations.iter().all(|violation| {
            !matches!(
                violation.kind,
                TypecheckViolationKind::TypeAliasCycle
                    | TypecheckViolationKind::TypeAliasExpansionLimit
            )
        }),
        "ジェネリクス展開は循環/上限診断を発生させない"
    );
}

#[test]
fn sum_type_constructor_resolves_in_expr() {
    let report = typecheck_source(
        "type Foo = | Bar(Int) | Baz\n\
fn make(x: Int) = Bar(x)",
    );
    assert_eq!(report.functions.len(), 1);
    assert_eq!(
        report.functions[0].unresolved_identifiers, 0,
        "合成型のコンストラクタは未解決識別子として扱わない"
    );
}

#[test]
fn qualified_function_call_keeps_path() {
    let report = typecheck_source(
        "fn Core.Dsl.Object.call(x: Int) = x\n\
fn run() = Core.Dsl.Object.call(1)",
    );
    let run = report
        .typed_module
        .functions
        .iter()
        .find(|function| function.name == "run")
        .expect("run");
    let TypedExprKind::Call { callee, .. } = &run.body.kind else {
        panic!("run body should be call");
    };
    let TypedExprKind::Identifier { ident } = &callee.kind else {
        panic!("callee should be identifier");
    };
    assert_eq!(ident.name, "Core.Dsl.Object.call");
}

#[test]
fn sum_type_record_payload_constructor_and_match() {
    let report = typecheck_source(
        "type Person = | Named { name: Str, age: Int } | Anonymous\n\
fn label(p: Person) = match p with\n\
| Named({ name, age }) -> name\n\
| Anonymous -> \"anon\"",
    );
    assert_eq!(report.functions.len(), 1);
    assert_eq!(
        report.functions[0].return_type, "Str",
        "レコード型ペイロードの束縛が戻り値型の収束に寄与する"
    );
}

#[test]
fn sum_type_constructor_arity_mismatch_is_reported() {
    let report = typecheck_source(
        "type Foo = | Bar(Int, Int) | Baz\n\
fn bad() = Bar(1)",
    );
    assert!(
        report.violations.iter().any(|violation| {
            matches!(
                violation.kind,
                TypecheckViolationKind::ConstructorArityMismatch
            )
        }),
        "コンストラクタの引数数不一致は診断される"
    );
}

#[test]
fn sum_type_record_payload_constructor_zero_args_is_reported() {
    let report = typecheck_source(
        "type Person = | Named { name: Str, age: Int } | Anonymous\n\
fn bad() = Named()",
    );
    assert!(
        report.violations.iter().any(|violation| {
            matches!(
                violation.kind,
                TypecheckViolationKind::ConstructorArityMismatch
            )
        }),
        "レコード型ペイロードの引数 0 個は診断される"
    );
}

#[test]
fn sum_type_record_payload_constructor_two_args_is_reported() {
    let report = typecheck_source(
        "type Person = | Named { name: Str, age: Int } | Anonymous\n\
fn bad() = Named({ name = \"Ada\", age = 36 }, 1)",
    );
    assert!(
        report.violations.iter().any(|violation| {
            matches!(
                violation.kind,
                TypecheckViolationKind::ConstructorArityMismatch
            )
        }),
        "レコード型ペイロードの引数 2 個は診断される"
    );
}

#[test]
fn sum_type_pattern_arity_mismatch_is_reported() {
    let report = typecheck_source(
        "type Foo = | Bar(Int, Int) | Baz\n\
fn bad(x: Foo) = match x with\n\
| Bar(_) -> 0\n\
| Baz -> 1",
    );
    assert!(
        report.violations.iter().any(|violation| {
            matches!(
                violation.kind,
                TypecheckViolationKind::ConstructorArityMismatch
            )
        }),
        "パターンの引数数不一致も診断される"
    );
}
