use reml_frontend::parser::ParserDriver;

#[test]
fn intrinsic_attribute_requires_string_literal() {
    let result = ParserDriver::parse(r#"@intrinsic(1) fn sqrt(x: Int) = x"#);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|diag| diag.code.as_deref() == Some("native.intrinsic.invalid_syntax")),
        "native.intrinsic.invalid_syntax が出力されるべきです"
    );
}
