use reml_frontend::parser::ast::Module;
use reml_frontend::parser::ParserDriver;
use reml_frontend::typeck::{TypecheckConfig, TypecheckDriver, TypecheckReport};

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

fn has_violation(report: &TypecheckReport, code: &str) -> bool {
    report
        .violations
        .iter()
        .any(|violation| violation.code == code)
}

#[test]
fn unreachable_arm_after_total_active_pattern() {
    let source = r#"
pattern (|Total|)(x) = x
fn run(x: Int) -> Int =
  match x with
  | (|Total|) v -> v
  | _ -> 0
"#;
    let module = parse_module(source);
    let report = TypecheckDriver::infer_module(Some(&module), &TypecheckConfig::default());
    assert!(
        has_violation(&report, "pattern.unreachable_arm"),
        "完全 Active Pattern の後続アームは到達不能になるはず"
    );
    assert!(
        !has_violation(&report, "pattern.exhaustiveness.missing"),
        "完全 Active Pattern のみで網羅性は満たされるはず"
    );
}

#[test]
fn slice_without_empty_arm_reports_missing_exhaustiveness() {
    let source = r#"
fn take(xs: Array<Int>) -> Int =
  match xs with
  | [head, ..tail] -> head
"#;
    let module = parse_module(source);
    let report = TypecheckDriver::infer_module(Some(&module), &TypecheckConfig::default());
    assert!(
        has_violation(&report, "pattern.exhaustiveness.missing"),
        "空スライス分岐が無い場合は網羅性警告が出るはず"
    );
}

#[test]
fn wildcard_first_arm_marks_following_range_unreachable() {
    let source = r#"
fn classify(x: Int) -> Int =
  match x with
  | _ -> 0
  | 1..=2 -> 1
"#;
    let module = parse_module(source);
    let report = TypecheckDriver::infer_module(Some(&module), &TypecheckConfig::default());
    assert!(
        has_violation(&report, "pattern.unreachable_arm"),
        "ワイルドカード後の範囲は到達不能として報告されるはず"
    );
}

#[test]
fn sum_type_missing_arm_reports_exhaustiveness() {
    let source = r#"
type Foo = | Bar(Int) | Baz
fn classify(x: Foo) -> Int =
  match x with
  | Bar(_) -> 1
"#;
    let module = parse_module(source);
    let report = TypecheckDriver::infer_module(Some(&module), &TypecheckConfig::default());
    assert!(
        has_violation(&report, "pattern.exhaustiveness.missing"),
        "合成型の分岐が欠ける場合は網羅性診断が必要"
    );
}

#[test]
fn sum_type_all_arms_satisfy_exhaustiveness() {
    let source = r#"
type Foo = | Bar(Int) | Baz
fn classify(x: Foo) -> Int =
  match x with
  | Bar(_) -> 1
  | Baz -> 0
"#;
    let module = parse_module(source);
    let report = TypecheckDriver::infer_module(Some(&module), &TypecheckConfig::default());
    assert!(
        !has_violation(&report, "pattern.exhaustiveness.missing"),
        "合成型の全分岐がある場合は網羅性診断が不要"
    );
}
