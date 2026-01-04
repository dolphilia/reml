use super::constraint::Substitution;
use super::types::{Type, TypeVarGen, TypeVariable};
use indexmap::IndexMap;
use serde::Serialize;
use smol_str::SmolStr;

pub type ConstraintName = SmolStr;

/// 型スキーム。量化変数・制約付き型を保持する。
#[derive(Debug, Clone, Serialize)]
pub struct Scheme {
    pub quantifiers: Vec<TypeVariable>,
    pub constraints: IndexMap<ConstraintName, Type>,
    pub ty: Type,
}

impl Scheme {
    pub fn simple(ty: Type) -> Self {
        Self {
            quantifiers: Vec::new(),
            constraints: IndexMap::new(),
            ty,
        }
    }

    pub fn generalize(ty: Type) -> Self {
        Self {
            quantifiers: Vec::new(),
            constraints: IndexMap::new(),
            ty,
        }
    }

    pub fn with_constraint(mut self, name: impl Into<ConstraintName>, ty: Type) -> Self {
        self.constraints.insert(name.into(), ty);
        self
    }

    pub fn instantiate(&self, generator: &mut TypeVarGen) -> Type {
        if self.quantifiers.is_empty() {
            return self.ty.clone();
        }
        let mut substitution = Substitution::default();
        for quantifier in &self.quantifiers {
            substitution.insert(*quantifier, Type::var(generator.next()));
        }
        substitution.apply_unwrap(self.ty.clone())
    }
}
