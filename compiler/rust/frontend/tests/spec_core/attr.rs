use reml_frontend::parser::ast::{ExprKind, LiteralKind, StmtKind};

mod common;

use common::{parse_example_module, parse_example_module_with_diagnostics};

#[test]
fn ch1_attr_101_records_block_attributes() {
    let module = parse_example_module(
        "examples/spec_core/chapter1/attributes/bnf-attr-cfg-let-gate-ok.reml",
    );
    let select_fn = module
        .functions
        .iter()
        .find(|function| function.name.name == "select_message")
        .expect("select_message should exist");
    let statements = match &select_fn.body.kind {
        ExprKind::Block { statements, .. } => statements,
        other => panic!(
            "expected select_message body to be a block, got {:?}",
            other
        ),
    };
    let attr_block = statements.iter().find_map(|stmt| {
        if let StmtKind::Expr { expr } = &stmt.kind {
            if let ExprKind::Block { attrs, .. } = &expr.kind {
                if !attrs.is_empty() {
                    return Some(attrs);
                }
            }
        }
        None
    });
    let attrs = attr_block.expect("expected block expression with attributes");
    assert_eq!(attrs.len(), 1, "expected a single @cfg attribute");
    assert_eq!(attrs[0].name.name, "cfg");
    assert_eq!(
        attrs[0].args.len(),
        1,
        "expected cfg attribute to retain its predicate expression"
    );
}

#[test]
fn ch1_attr_102_attaches_attributes_to_functions() {
    let (module, diagnostics) = parse_example_module_with_diagnostics(
        "examples/spec_core/chapter1/attributes/bnf-attr-cfg-missing-flag-error.reml",
    );
    assert!(
        diagnostics.iter().any(|message| {
            message.contains("未定義ターゲット") && message.contains("quantum")
        }),
        "expected @cfg to report missing target, got {:?}",
        diagnostics
    );
    let hidden_fn = module
        .functions
        .iter()
        .find(|function| function.name.name == "hidden")
        .expect("hidden function should be parsed");
    assert_eq!(
        hidden_fn.attrs.len(),
        1,
        "expected @cfg attribute on hidden function"
    );
    assert_eq!(hidden_fn.attrs[0].name.name, "cfg");
}

#[test]
fn ch1_eff_701_records_pure_attribute_and_perform_path() {
    let module = parse_example_module(
        "examples/spec_core/chapter1/effects/bnf-attr-pure-perform-error.reml",
    );
    let announce_fn = module
        .functions
        .iter()
        .find(|function| function.name.name == "announce")
        .expect("announce function should be parsed");
    assert!(
        announce_fn
            .attrs
            .iter()
            .any(|attr| attr.name.name == "pure"),
        "expected announce to have @pure attribute"
    );
    let perform_call = match &announce_fn.body.kind {
        ExprKind::Block { statements, .. } => statements
            .iter()
            .find_map(|stmt| {
                if let StmtKind::Expr { expr } = &stmt.kind {
                    if let ExprKind::PerformCall { call } = &expr.kind {
                        return Some(call);
                    }
                }
                None
            })
            .expect("announce block should contain a perform call"),
        other => panic!("expected announce body to be a block, got {:?}", other),
    };
    assert_eq!(
        perform_call.effect.name, "Console::log",
        "perform should preserve qualified effect path"
    );
    match &perform_call.argument.kind {
        ExprKind::Literal(literal) => match &literal.value {
            LiteralKind::String { value, .. } => {
                assert_eq!(value, "hi", "perform argument should keep string literal")
            }
            other => panic!(
                "expected perform argument to be a string literal, got {:?}",
                other
            ),
        },
        other => panic!("expected perform argument literal, got {:?}", other),
    }
}
