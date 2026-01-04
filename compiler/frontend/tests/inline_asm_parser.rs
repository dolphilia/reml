use reml_frontend::parser::ast::{DeclKind, Expr, ExprKind, StmtKind};
use reml_frontend::parser::ParserDriver;

fn any_expr(expr: &Expr, predicate: &mut impl FnMut(&Expr) -> bool) -> bool {
    if predicate(expr) {
        return true;
    }
    match &expr.kind {
        ExprKind::Call { callee, args } => {
            any_expr(callee, predicate) || args.iter().any(|arg| any_expr(arg, predicate))
        }
        ExprKind::PerformCall { call } => any_expr(&call.argument, predicate),
        ExprKind::InlineAsm(asm) => {
            asm.outputs
                .iter()
                .any(|output| any_expr(&output.target, predicate))
                || asm
                    .inputs
                    .iter()
                    .any(|input| any_expr(&input.expr, predicate))
        }
        ExprKind::LlvmIr(ir) => ir.inputs.iter().any(|input| any_expr(input, predicate)),
        ExprKind::Lambda { body, .. }
        | ExprKind::Loop { body }
        | ExprKind::Unsafe { body }
        | ExprKind::Defer { body }
        | ExprKind::EffectBlock { body }
        | ExprKind::Async { body, .. } => any_expr(body, predicate),
        ExprKind::Pipe { left, right } | ExprKind::Binary { left, right, .. } => {
            any_expr(left, predicate) || any_expr(right, predicate)
        }
        ExprKind::Unary { expr: inner, .. }
        | ExprKind::Rec { expr: inner }
        | ExprKind::Propagate { expr: inner }
        | ExprKind::Await { expr: inner } => any_expr(inner, predicate),
        ExprKind::Break { value } | ExprKind::Return { value } => value
            .as_deref()
            .map_or(false, |inner| any_expr(inner, predicate)),
        ExprKind::FieldAccess { target, .. } | ExprKind::TupleAccess { target, .. } => {
            any_expr(target, predicate)
        }
        ExprKind::Index { target, index } => {
            any_expr(target, predicate) || any_expr(index, predicate)
        }
        ExprKind::IfElse {
            condition,
            then_branch,
            else_branch,
        } => {
            any_expr(condition, predicate)
                || any_expr(then_branch, predicate)
                || else_branch
                    .as_deref()
                    .map_or(false, |branch| any_expr(branch, predicate))
        }
        ExprKind::Match { target, arms } => {
            any_expr(target, predicate)
                || arms.iter().any(|arm| {
                    arm.guard
                        .as_ref()
                        .map_or(false, |guard| any_expr(guard, predicate))
                        || any_expr(&arm.body, predicate)
                })
        }
        ExprKind::While { condition, body } => {
            any_expr(condition, predicate) || any_expr(body, predicate)
        }
        ExprKind::For { start, end, .. } => any_expr(start, predicate) || any_expr(end, predicate),
        ExprKind::Handle { handle } => any_expr(&handle.target, predicate),
        ExprKind::Block { statements, .. } => statements.iter().any(|stmt| match &stmt.kind {
            StmtKind::Decl { decl } => match &decl.kind {
                DeclKind::Let { value, .. }
                | DeclKind::Var { value, .. }
                | DeclKind::Const { value, .. } => any_expr(value, predicate),
                _ => false,
            },
            StmtKind::Expr { expr } | StmtKind::Defer { expr } => any_expr(expr, predicate),
            StmtKind::Assign { target, value } => {
                any_expr(target, predicate) || any_expr(value, predicate)
            }
        }),
        ExprKind::Literal(_)
        | ExprKind::FixityLiteral(_)
        | ExprKind::Identifier(_)
        | ExprKind::ModulePath(_)
        | ExprKind::Continue => false,
        ExprKind::Assign { target, value } => {
            any_expr(target, predicate) || any_expr(value, predicate)
        }
    }
}

#[test]
fn parses_inline_asm_expr() {
    let source = r#"
fn main() = unsafe {
  inline_asm(
    "rdtsc",
    outputs("=a": lo),
    inputs("=d": hi),
    clobbers("rcx"),
    options("volatile")
  )
}
"#;
    let parsed = ParserDriver::parse(source);
    assert!(
        parsed.diagnostics.is_empty(),
        "diagnostics: {:?}",
        parsed.diagnostics
    );
    let module = parsed.value.expect("module");
    let body = &module.functions[0].body;
    let found = any_expr(body, &mut |expr| {
        matches!(expr.kind, ExprKind::InlineAsm(_))
    });
    assert!(found, "inline_asm 式が検出できません");
}

#[test]
fn parses_llvm_ir_expr() {
    let source = r#"
fn main(a: Int, b: Int) = unsafe {
  llvm_ir!(Int) {
    "%0 = add i32 $0, $1",
    inputs(a, b)
  }
}
"#;
    let parsed = ParserDriver::parse(source);
    assert!(
        parsed.diagnostics.is_empty(),
        "diagnostics: {:?}",
        parsed.diagnostics
    );
    let module = parsed.value.expect("module");
    let body = &module.functions[0].body;
    let found = any_expr(body, &mut |expr| matches!(expr.kind, ExprKind::LlvmIr(_)));
    assert!(found, "llvm_ir 式が検出できません");
}
