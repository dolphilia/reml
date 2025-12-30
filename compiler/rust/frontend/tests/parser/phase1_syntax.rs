use reml_frontend::parser::ast::{DeclKind, ExprKind};
use reml_frontend::parser::ParserDriver;

#[test]
fn parses_async_await_effect_and_unsafe_fn() {
    let source = r#"
#[deprecated("use new_main")]
unsafe fn old_main() = 1

fn main() = async move { await old_main() }
"#;
    let parsed = ParserDriver::parse(source);
    assert!(
        parsed.diagnostics.is_empty(),
        "diagnostics: {:?}",
        parsed.diagnostics
    );
    let module = parsed.value.expect("module");
    let unsafe_fn = module
        .functions
        .iter()
        .find(|func| func.name.name == "old_main")
        .expect("old_main");
    assert!(unsafe_fn.is_unsafe, "unsafe fn が検出されていません");
    assert!(
        !unsafe_fn.attrs.is_empty(),
        "#[deprecated] 属性が取得できていません"
    );
    let main_fn = module
        .functions
        .iter()
        .find(|func| func.name.name == "main")
        .expect("main");
    match &main_fn.body.kind {
        ExprKind::Async { is_move, .. } => {
            assert!(*is_move, "async move が検出されていません");
        }
        other => panic!("unexpected main body: {other:?}"),
    }
}

#[test]
fn parses_module_block_macro_and_actor_spec() {
    let source = r#"
module Sample.Core {
  macro build(value) { value }
  actor spec Greeter(name) { name }
  let value = 1
}
"#;
    let parsed = ParserDriver::parse(source);
    assert!(
        parsed.diagnostics.is_empty(),
        "diagnostics: {:?}",
        parsed.diagnostics
    );
    let module = parsed.value.expect("module");
    let module_decl = module
        .decls
        .iter()
        .find_map(|decl| match &decl.kind {
            DeclKind::Module(decl) => Some(decl),
            _ => None,
        })
        .expect("module decl");
    assert!(
        module_decl
            .body
            .decls
            .iter()
            .any(|decl| matches!(decl.kind, DeclKind::Macro(_))),
        "macro 宣言が見つかりません"
    );
    assert!(
        module_decl
            .body
            .decls
            .iter()
            .any(|decl| matches!(decl.kind, DeclKind::ActorSpec(_))),
        "actor spec 宣言が見つかりません"
    );
}
