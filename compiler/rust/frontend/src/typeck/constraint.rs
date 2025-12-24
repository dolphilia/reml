use indexmap::IndexMap;
use serde::Serialize;
use smol_str::SmolStr;
use thiserror::Error;

pub mod iterator;

use super::types::{CapabilityContext, Type, TypeVariable};

/// 型システムの制約。
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Constraint {
    Equal {
        left: Type,
        right: Type,
    },
    HasCapability {
        ty: Type,
        capability: SmolStr,
        context: CapabilityContext,
    },
    ImplBound {
        ty: Type,
        implementation: SmolStr,
    },
}

impl Constraint {
    pub fn equal(left: Type, right: Type) -> Self {
        Self::Equal { left, right }
    }

    pub fn has_capability(ty: Type, capability: impl Into<SmolStr>) -> Self {
        Self::HasCapability {
            ty,
            capability: capability.into(),
            context: CapabilityContext::default(),
        }
    }

    pub fn impl_bound(ty: Type, implementation: impl Into<SmolStr>) -> Self {
        Self::ImplBound {
            ty,
            implementation: implementation.into(),
        }
    }
}

/// 型代入。
#[derive(Debug, Clone, Serialize)]
pub struct Substitution {
    entries: IndexMap<TypeVariable, Type>,
}

impl Default for Substitution {
    fn default() -> Self {
        Self {
            entries: IndexMap::new(),
        }
    }
}

impl Substitution {
    pub fn insert(&mut self, variable: TypeVariable, ty: Type) {
        self.entries.insert(variable, ty);
    }

    pub fn get(&self, variable: &TypeVariable) -> Option<&Type> {
        self.entries.get(variable)
    }

    pub fn apply(&self, ty: &Type) -> Type {
        match ty {
            Type::Var(variable) => self
                .get(variable)
                .cloned()
                .unwrap_or_else(|| Type::Var(*variable)),
            Type::Builtin(_) => ty.clone(),
            Type::Arrow { parameters, result } => {
                let parameters = parameters
                    .iter()
                    .map(|param| self.apply(param))
                    .collect::<Vec<_>>();
                let result = self.apply(result);
                Type::arrow(parameters, result)
            }
            Type::App {
                constructor,
                arguments,
            } => {
                let arguments = arguments.iter().map(|arg| self.apply(arg)).collect();
                Type::app(constructor.clone(), arguments)
            }
            Type::Slice { element } => Type::slice(self.apply(element)),
            Type::Ref { target, mutable } => Type::reference(self.apply(target), *mutable),
        }
    }

    pub fn apply_unwrap(&self, ty: Type) -> Type {
        self.apply(&ty)
    }

    pub fn merge(&mut self, other: Substitution) {
        for (variable, ty) in other.entries {
            self.entries.insert(variable, ty);
        }
    }
}

impl From<IndexMap<TypeVariable, Type>> for Substitution {
    fn from(entries: IndexMap<TypeVariable, Type>) -> Self {
        Self { entries }
    }
}

/// 制約ソルバ。
#[derive(Debug, Clone)]
pub struct ConstraintSolver {
    substitution: Substitution,
}

impl ConstraintSolver {
    pub fn new() -> Self {
        Self {
            substitution: Substitution::default(),
        }
    }

    pub fn substitution(&self) -> &Substitution {
        &self.substitution
    }

    pub fn unify(&mut self, left: Type, right: Type) -> Result<(), ConstraintSolverError> {
        let left = self.substitution.apply(&left);
        let right = self.substitution.apply(&right);
        match (left, right) {
            (Type::Var(variable), ty) => self.bind_variable(variable, ty),
            (ty, Type::Var(variable)) => self.bind_variable(variable, ty),
            (Type::Builtin(left_builtin), Type::Builtin(right_builtin))
                if left_builtin == right_builtin =>
            {
                Ok(())
            }
            (
                Type::Arrow {
                    parameters: left_params,
                    result: left_result,
                },
                Type::Arrow {
                    parameters: right_params,
                    result: right_result,
                },
            ) => {
                if left_params.len() != right_params.len() {
                    return Err(ConstraintSolverError::Mismatch(
                        Type::Arrow {
                            parameters: left_params,
                            result: left_result,
                        },
                        Type::Arrow {
                            parameters: right_params,
                            result: right_result,
                        },
                    ));
                }
                for (left_param, right_param) in
                    left_params.into_iter().zip(right_params.into_iter())
                {
                    self.unify(left_param, right_param)?;
                }
                self.unify(*left_result, *right_result)
            }
            (
                Type::App {
                    constructor: left_ctor,
                    arguments: left_arguments,
                },
                Type::App {
                    constructor: right_ctor,
                    arguments: right_arguments,
                },
            ) => {
                if left_ctor != right_ctor || left_arguments.len() != right_arguments.len() {
                    return Err(ConstraintSolverError::Mismatch(
                        Type::App {
                            constructor: left_ctor,
                            arguments: left_arguments,
                        },
                        Type::App {
                            constructor: right_ctor,
                            arguments: right_arguments,
                        },
                    ));
                }
                for (left_arg, right_arg) in
                    left_arguments.into_iter().zip(right_arguments.into_iter())
                {
                    self.unify(left_arg, right_arg)?;
                }
                Ok(())
            }
            (Type::Slice { element: left }, Type::Slice { element: right }) => {
                self.unify(*left, *right)
            }
            (
                Type::Ref {
                    target: left_target,
                    mutable: left_mutable,
                },
                Type::Ref {
                    target: right_target,
                    mutable: right_mutable,
                },
            ) => {
                if left_mutable != right_mutable {
                    return Err(ConstraintSolverError::Mismatch(
                        Type::Ref {
                            target: left_target,
                            mutable: left_mutable,
                        },
                        Type::Ref {
                            target: right_target,
                            mutable: right_mutable,
                        },
                    ));
                }
                self.unify(*left_target, *right_target)
            }
            (left, right) => Err(ConstraintSolverError::Mismatch(left, right)),
        }
    }

    fn bind_variable(
        &mut self,
        variable: TypeVariable,
        ty: Type,
    ) -> Result<(), ConstraintSolverError> {
        let placeholder = Type::Var(variable);
        if ty == placeholder {
            return Ok(());
        }
        if ty.contains_variable(&variable) {
            return Err(ConstraintSolverError::Occurs(variable, ty));
        }
        self.substitution.insert(variable, ty);
        Ok(())
    }

    pub fn solve(
        &self,
        _constraints: &[Constraint],
    ) -> Result<Substitution, ConstraintSolverError> {
        Ok(self.substitution.clone())
    }
}

impl Default for ConstraintSolver {
    fn default() -> Self {
        Self::new()
    }
}

/// 制約ソルバの実行エラー。
#[derive(Debug, Error)]
pub enum ConstraintSolverError {
    #[error("constraint solver の実装が完了していません: {0}")]
    NotImplemented(String),
    #[error("{0} と {1} は一致しません")]
    Mismatch(Type, Type),
    #[error("型変数 {0} が {1} に出現するため unify できません")]
    Occurs(TypeVariable, Type),
}
