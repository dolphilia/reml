use reml_frontend::parser::ast::Module;
use reml_frontend::parser::ParserDriver;
use reml_frontend::typeck::{TypecheckConfig, TypecheckDriver, TypecheckReport};

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

fn find_function<'a>(
    report: &'a TypecheckReport,
    name: &str,
) -> &'a reml_frontend::semantics::typed::TypedFunction {
    report
        .typed_module
        .functions
        .iter()
        .find(|func| func.name == name)
        .unwrap_or_else(|| panic!("{name} 関数が存在すること"))
}

#[test]
fn array_literal_infers_slice_types() {
    let report = typecheck_source(
        r#"
fn array_single() = [1]
fn array_multi() = [1, 2, 3]
fn array_nested() = [[1], [2, 3]]
"#,
    );

    let single = find_function(&report, "array_single");
    assert_eq!(single.body.ty, "[Int]", "単一要素配列は [Int] になる");

    let multi = find_function(&report, "array_multi");
    assert_eq!(multi.body.ty, "[Int]", "複数要素配列は [Int] になる");

    let nested = find_function(&report, "array_nested");
    assert_eq!(nested.body.ty, "[[Int]]", "ネスト配列は [[Int]] になる");
}

#[test]
fn empty_array_literal_defaults_to_type_variable_slice() {
    let report = typecheck_source("fn array_empty() = []");
    let empty = find_function(&report, "array_empty");
    assert!(
        empty.body.ty.starts_with("['t") && empty.body.ty.ends_with("]"),
        "空配列は型変数のスライスになる: {}",
        empty.body.ty
    );
}
