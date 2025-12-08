use reml_frontend::parser::ast::{
    Decl, DeclKind, ExprKind, Module, Pattern, PatternKind, StmtKind, UseTree,
};
use reml_frontend::parser::ParserDriver;
use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

fn parse_example_module(relative_path: &str) -> Module {
    let source_path = repo_root().join(relative_path);
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));
    let result = ParserDriver::parse(&source);
    if !result.diagnostics.is_empty() {
        let messages = result
            .diagnostics
            .iter()
            .map(|diag| diag.message.clone())
            .collect::<Vec<_>>();
        panic!(
            "unexpected parser diagnostics for {}: {:?}",
            relative_path, messages
        );
    }
    result
        .value
        .unwrap_or_else(|| panic!("parser did not return a module for {relative_path}"))
}

#[test]
fn ch1_mod_003_accepts_module_and_use_prefix() {
    let module = parse_example_module(
        "examples/spec_core/chapter1/module_use/bnf-compilationunit-module-use-alias-ok.reml",
    );
    let header = module
        .header
        .as_ref()
        .expect("module header should be present");
    assert_eq!(header.path.render(), "spec_core.match_guard");
    assert_eq!(module.uses.len(), 2, "expected two use declarations");
    assert!(
        module
            .uses
            .iter()
            .any(|decl| matches!(decl.tree, UseTree::Brace { .. })),
        "expected a brace-style use tree to be parsed"
    );
}

#[test]
fn ch1_let_001_accepts_top_level_let_binding() {
    let module = parse_example_module(
        "examples/spec_core/chapter1/let_binding/bnf-valdecl-let-simple-ok.reml",
    );
    let has_top_level_let = module.decls.iter().any(|decl| {
        matches!(
            decl.kind,
            DeclKind::Let {
                pattern: Pattern {
                    kind: PatternKind::Var(ref ident),
                    ..
                },
                ..
            } if ident.name == "greeting_prefix"
        )
    });
    assert!(
        has_top_level_let,
        "expected greeting_prefix to be recorded as a top-level let binding"
    );
}

#[test]
fn ch1_let_002_supports_tuple_pattern_bindings() {
    let module = parse_example_module(
        "examples/spec_core/chapter1/let_binding/bnf-valdecl-let-pattern-tuple.reml",
    );
    let tuple_binding_present = module.functions.iter().any(|function| {
        function.name.name == "sum_pair"
            && matches!(
                function.body.kind,
                ExprKind::Block { ref statements } if statements.iter().any(|stmt| {
                    matches!(
                        stmt.kind,
                        StmtKind::Decl {
                            decl:
                                Decl {
                                    kind: DeclKind::Let { ref pattern, .. },
                                    ..
                                },
                        } if matches!(pattern.kind, PatternKind::Tuple { .. })
                    )
                })
            )
    });
    assert!(
        tuple_binding_present,
        "expected sum_pair to contain a tuple-pattern let binding"
    );
}

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
        other => panic!("expected select_message body to be a block, got {:?}", other),
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
    let module = parse_example_module(
        "examples/spec_core/chapter1/attributes/bnf-attr-cfg-missing-flag-error.reml",
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
