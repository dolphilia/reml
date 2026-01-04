use reml_frontend::parser::{ast::ExprKind, ParserDriver};

#[test]
fn block_collects_defers_in_source_order() {
    let source = r#"
fn sample() = {
  defer cleanup("a")
  defer cleanup("b")
  0
}
"#;
    let parsed = ParserDriver::parse(source);
    assert!(
        parsed.diagnostics.is_empty(),
        "diagnostics: {:?}",
        parsed.diagnostics
    );
    let module = parsed.value.expect("module");
    let function = module.functions.first().expect("function");
    let ExprKind::Block { defers, .. } = &function.body.kind else {
        panic!("function body should be block");
    };
    let rendered = defers.iter().map(|expr| expr.render()).collect::<Vec<_>>();
    assert_eq!(
        rendered,
        vec![
            "call(var(cleanup))[str(\"a\")]".to_string(),
            "call(var(cleanup))[str(\"b\")]".to_string(),
        ]
    );
}
