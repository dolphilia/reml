use reml_frontend::typeck::constraint::{ConstraintSolver, ConstraintSolverError};
use reml_frontend::typeck::types::{BuiltinType, Type, TypeVariable};

/// 単純な `ConstraintSolver` に対するユニットテスト群。
#[test]
fn unify_var_with_builtin_updates_substitution() {
    let mut solver = ConstraintSolver::new();
    let variable = TypeVariable::new(1);
    solver
        .unify(Type::var(variable), Type::builtin(BuiltinType::Int))
        .expect("数値型との unify に成功するはず");

    let substitution = solver.solve(&[]).expect("実行時には substitution が返る");
    let applied = substitution.apply(&Type::var(variable));
    assert_eq!(
        applied,
        Type::builtin(BuiltinType::Int),
        "型変数は Int に解決されている"
    );
}

#[test]
fn unify_occurs_check_returns_error() {
    let mut solver = ConstraintSolver::new();
    let variable = TypeVariable::new(2);
    let cyclic_arrow = Type::arrow(vec![], Type::var(variable));
    let result = solver.unify(Type::var(variable), cyclic_arrow);
    assert!(
        matches!(result, Err(ConstraintSolverError::Occurs(_, _))),
        "自己参照する型変数は OccursCheck で拒否される"
    );
}

#[test]
fn unify_mismatch_reports_type_mismatch() {
    let mut solver = ConstraintSolver::new();
    let left = Type::builtin(BuiltinType::Int);
    let right = Type::arrow(vec![], Type::builtin(BuiltinType::Bool));
    let result = solver.unify(left, right);
    assert!(
        matches!(result, Err(ConstraintSolverError::Mismatch(_, _))),
        "Int と Arrow 型は一致しない"
    );
}
