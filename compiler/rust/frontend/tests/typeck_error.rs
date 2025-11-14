use reml_frontend::parser::ast::Module;
use reml_frontend::parser::ParserDriver;
use reml_frontend::typeck::{
    TypecheckConfig, TypecheckConfigBuilder, TypecheckDriver, TypecheckReport,
};
use serde_json::Value;

fn parse_module(source: &str) -> Module {
    let result = ParserDriver::parse(source);
    assert!(
        result.diagnostics.is_empty(),
        "パーサでエラーが発生しました: {:?}",
        result
            .diagnostics
            .iter()
            .map(|diag| &diag.message)
            .collect::<Vec<_>>()
    );
    result.value.expect("AST を取得できるはず")
}

fn typecheck_with_config(source: &str, config: TypecheckConfig) -> TypecheckReport {
    let module = parse_module(source);
    TypecheckDriver::infer_module(&module, &config)
}

fn typecheck_source(source: &str) -> TypecheckReport {
    typecheck_with_config(source, TypecheckConfig::default())
}

#[test]
fn condition_violation_is_reported_as_json() {
    let report = typecheck_source("fn condition_violation() = if 42 then 1 else 0");
    let violation = report
        .violations
        .iter()
        .find(|entry| entry.code == "E7006")
        .expect("Bool 条件で E7006 が出るはず");
    assert!(
        violation.message.contains("Bool"),
        "メッセージに Bool が含まれる"
    );

    let serialized = serde_json::to_value(&report).expect("シリアライズ可能");
    let violations_array = serialized
        .get("violations")
        .and_then(Value::as_array)
        .expect("violations は配列");
    assert!(
        violations_array.iter().any(|item| {
            item.get("code")
                .and_then(Value::as_str)
                .map(|code| code == "E7006")
                .unwrap_or(false)
        }),
        "JSON にもコードが含まれる"
    );
}

#[test]
fn effect_stage_mismatch_is_serialized() {
    let config = TypecheckConfig::builder()
        .experimental_effects(true)
        .build();
    let report = typecheck_with_config(r#"fn log() = perform Console.log("hello")"#, config);
    assert!(
        report
            .violations
            .iter()
            .any(|entry| entry.code == "effects.contract.stage_mismatch"),
        "効果呼び出しでステージ不一致が記録される"
    );

    let serialized = serde_json::to_value(&report).expect("シリアライズ可能");
    let violations_array = serialized
        .get("violations")
        .and_then(Value::as_array)
        .expect("violations は配列");
    assert!(
        violations_array.iter().any(|item| {
            item.get("code")
                .and_then(Value::as_str)
                .map(|code| code == "effects.contract.stage_mismatch")
                .unwrap_or(false)
        }),
        "JSON にステージ診断が含まれる"
    );
}
