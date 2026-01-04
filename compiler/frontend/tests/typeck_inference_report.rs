use reml_frontend::parser::ast::Module;
use reml_frontend::parser::ParserDriver;
use reml_frontend::typeck::{Constraint, TypecheckConfig, TypecheckDriver, TypecheckReport};

use serde_json::Value;

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
fn typecheck_report_emits_typed_function_summary() {
    let report = typecheck_source("fn sum(x_int, y_int) = x_int + y_int");
    let typed_module = &report.typed_module;
    assert_eq!(typed_module.functions.len(), 1);
    let function = &typed_module.functions[0];
    assert_eq!(function.name, "sum");
    assert_eq!(function.params.len(), 2);
    let param_types = function
        .params
        .iter()
        .map(|param| param.ty.as_str())
        .collect::<Vec<_>>();
    assert_eq!(param_types.len(), 2);
    assert_eq!(
        param_types[0], param_types[1],
        "第一引数と第二引数は同じ型変数を共有"
    );
    assert!(
        param_types[0].starts_with("'"),
        "型は未解決型変数であるべき"
    );
    assert!(
        !function.return_type.is_empty(),
        "戻り型ラベルが記録されている"
    );
    assert_eq!(typed_module.schemes.len(), 1);
    assert!(typed_module.dict_refs.is_empty());
}

#[test]
fn typecheck_report_constraints_store_binary_equal() {
    let report = typecheck_source("fn double(x_int) = x_int + x_int");
    assert!(
        report
            .constraints
            .iter()
            .any(|constraint| matches!(constraint, Constraint::Equal { .. })),
        "Addition では何らかの Equal 制約が生成されている"
    );
}

#[test]
fn typecheck_report_serializes_typed_module_and_metrics() {
    let report = typecheck_source("fn pair(x_int, y_int) = x_int + y_int");
    assert_eq!(report.metrics.typed_functions, 1);
    assert!(report.metrics.constraints_total >= 1);
    let serialized =
        serde_json::to_value(&report).expect("TypecheckReport は serde_json で直列化可能");
    assert!(
        matches!(serialized.get("typed_module"), Some(Value::Object(_))),
        "typed_module が JSON オブジェクトとして出力される"
    );
    assert!(
        serialized
            .get("constraints")
            .and_then(|value| value.as_array())
            .map_or(false, |arr| !arr.is_empty()),
        "constraints 配列が空でない"
    );
}
