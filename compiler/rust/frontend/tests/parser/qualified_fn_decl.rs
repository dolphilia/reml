use reml_frontend::parser::ParserDriver;

#[test]
fn parses_qualified_function_decl() {
    let source = r#"
fn Core.Dsl.Object.call<Value>(input: Value) = input
"#;
    let parsed = ParserDriver::parse(source);
    assert!(
        parsed.diagnostics.is_empty(),
        "diagnostics: {:?}",
        parsed.diagnostics
    );
    let module = parsed.value.expect("module");
    let function = module.functions.first().expect("function");
    assert_eq!(function.name.name, "Core::Dsl::Object::call");
    assert_eq!(
        function.generics.first().map(|ident| ident.name.as_str()),
        Some("Value")
    );
}
