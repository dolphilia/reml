use reml_frontend::parser::ast::Module;
use reml_frontend::parser::ParserDriver;
use reml_frontend::typeck::{TypecheckConfig, TypecheckDriver, TypecheckReport};
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
    TypecheckDriver::infer_module(Some(&module), &config)
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

#[test]
fn intrinsic_missing_effect_is_reported() {
    let report = typecheck_source(r#"@intrinsic("llvm.ctpop.i64") fn pop(x: Int) = x"#);
    assert!(
        report
            .violations
            .iter()
            .any(|entry| entry.code == "native.intrinsic.missing_effect"),
        "native.intrinsic.missing_effect が報告される"
    );
}

#[test]
fn intrinsic_invalid_type_is_reported() {
    let report = typecheck_source(r#"@intrinsic("llvm.ctpop.i64") fn pop(x: Str) !{native} = x"#);
    assert!(
        report
            .violations
            .iter()
            .any(|entry| entry.code == "native.intrinsic.invalid_type"),
        "native.intrinsic.invalid_type が報告される"
    );
}

#[test]
fn inline_asm_missing_effect_is_reported() {
    let report = typecheck_source(r#"fn read() = unsafe { inline_asm("rdtsc") }"#);
    assert!(
        report
            .violations
            .iter()
            .any(|entry| entry.code == "native.inline_asm.missing_effect"),
        "native.inline_asm.missing_effect が報告される"
    );
}

#[test]
fn inline_asm_missing_cfg_is_reported() {
    let report = typecheck_source(r#"fn read() !{native} = unsafe { inline_asm("rdtsc") }"#);
    assert!(
        report
            .violations
            .iter()
            .any(|entry| entry.code == "native.inline_asm.missing_cfg"),
        "native.inline_asm.missing_cfg が報告される"
    );
}

#[test]
fn inline_asm_invalid_type_is_reported() {
    let report = typecheck_source(
        r#"
@cfg(target_arch = "x86_64")
fn read(s: Str) !{native} = unsafe { inline_asm("nop", inputs("r": s)) }
"#,
    );
    assert!(
        report
            .violations
            .iter()
            .any(|entry| entry.code == "native.inline_asm.invalid_type"),
        "native.inline_asm.invalid_type が報告される"
    );
}

#[test]
fn llvm_ir_missing_effect_is_reported() {
    let report = typecheck_source(
        r#"fn add(a: Int, b: Int) = unsafe { llvm_ir!(Int) { "%0 = add i32 $0, $1", inputs(a, b) } }"#,
    );
    assert!(
        report
            .violations
            .iter()
            .any(|entry| entry.code == "native.llvm_ir.missing_effect"),
        "native.llvm_ir.missing_effect が報告される"
    );
}

#[test]
fn llvm_ir_missing_cfg_is_reported() {
    let report = typecheck_source(
        r#"fn add(a: Int, b: Int) !{native} = unsafe { llvm_ir!(Int) { "%0 = add i32 $0, $1", inputs(a, b) } }"#,
    );
    assert!(
        report
            .violations
            .iter()
            .any(|entry| entry.code == "native.llvm_ir.missing_cfg"),
        "native.llvm_ir.missing_cfg が報告される"
    );
}

#[test]
fn llvm_ir_invalid_type_is_reported() {
    let report = typecheck_source(
        r#"
@cfg(target_arch = "x86_64")
fn add(s: Str) !{native} = unsafe { llvm_ir!(Str) { "%0 = add i32 $0, $1", inputs(s) } }
"#,
    );
    assert!(
        report
            .violations
            .iter()
            .any(|entry| entry.code == "native.llvm_ir.invalid_type"),
        "native.llvm_ir.invalid_type が報告される"
    );
}
