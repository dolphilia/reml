use reml_frontend::parser::ast::{Decl, DeclKind, ExprKind, Pattern, PatternKind, StmtKind, UseTree};

mod common;

use common::parse_example_module;

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
