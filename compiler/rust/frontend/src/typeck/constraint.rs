use indexmap::IndexMap;
use serde::Serialize;
use smol_str::SmolStr;
use thiserror::Error;

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
#[derive(Debug, Default)]
pub struct ConstraintSolver;

impl ConstraintSolver {
    pub fn new() -> Self {
        Self
    }

    pub fn solve(
        &self,
        _constraints: &[Constraint],
    ) -> Result<Substitution, ConstraintSolverError> {
        Ok(Substitution::default())
    }
}

/// 制約ソルバの実行エラー。
#[derive(Debug, Error)]
pub enum ConstraintSolverError {
    #[error("constraint solver の実装が完了していません: {0}")]
    NotImplemented(String),
}
