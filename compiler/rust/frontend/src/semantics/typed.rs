use serde::Serialize;

use crate::parser::ast::{Ident, Literal};
use crate::span::Span;

/// TypecheckDriver が生成する型付き AST。
/// `docs/plans/rust-migration/appendix/typed_ast_schema_draft.md` に記されたフィールドに
/// 追従することを目指しているが、現時点では関数本体と式の型ラベルに絞っている。
#[derive(Debug, Clone, Serialize)]
pub struct TypedModule {
    pub functions: Vec<TypedFunction>,
}

impl Default for TypedModule {
    fn default() -> Self {
        Self {
            functions: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TypedFunction {
    pub name: String,
    pub span: Span,
    pub params: Vec<TypedParam>,
    pub return_type: String,
    pub body: TypedExpr,
}

#[derive(Debug, Clone, Serialize)]
pub struct TypedParam {
    pub name: String,
    pub span: Span,
    pub ty: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TypedExpr {
    pub span: Span,
    pub kind: TypedExprKind,
    pub ty: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TypedExprKind {
    Literal(Literal),
    Identifier {
        ident: Ident,
    },
    Call {
        callee: Box<TypedExpr>,
        args: Vec<TypedExpr>,
    },
    Binary {
        operator: String,
        left: Box<TypedExpr>,
        right: Box<TypedExpr>,
    },
    IfElse {
        condition: Box<TypedExpr>,
        then_branch: Box<TypedExpr>,
        else_branch: Box<TypedExpr>,
    },
    PerformCall {
        call: TypedEffectCall,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct TypedEffectCall {
    pub effect: Ident,
    pub argument: Box<TypedExpr>,
}
