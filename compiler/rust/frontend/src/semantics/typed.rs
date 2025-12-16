use serde::Serialize;

use crate::parser::ast::{Ident, Literal};
use crate::span::Span;

pub type DictRefId = usize;

/// TypecheckDriver が生成する型付き AST。
/// `docs/plans/rust-migration/appendix/typed_ast_schema_draft.md` に記された構造に合わせ、
/// TypedModule に関数・dict_ref・scheme を含める。
#[derive(Debug, Clone, Serialize, Default)]
pub struct TypedModule {
    pub functions: Vec<TypedFunction>,
    pub active_patterns: Vec<TypedActivePattern>,
    pub dict_refs: Vec<DictRef>,
    pub schemes: Vec<SchemeInfo>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TypedFunction {
    pub name: String,
    pub span: Span,
    pub params: Vec<TypedParam>,
    pub return_type: String,
    pub body: TypedExpr,
    pub dict_ref_ids: Vec<DictRefId>,
    pub scheme_id: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TypedParam {
    pub name: String,
    pub span: Span,
    pub ty: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ActivePatternKind {
    Partial,
    Total,
}

/// Active Pattern が戻り値をどのキャリア（Option 相当か値そのものか）で返すかを表す。
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "carrier", rename_all = "snake_case")]
pub enum ActiveReturnCarrier {
    OptionLike,
    Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct TypedActivePattern {
    pub name: String,
    pub span: Span,
    pub kind: ActivePatternKind,
    pub return_carrier: ActiveReturnCarrier,
    /// 実行時に「マッチ失敗パス」（None で次アームへフォールスルー）が必要か。
    /// Total Active Pattern では常に成功するため false。
    pub has_miss_path: bool,
    pub params: Vec<TypedParam>,
    pub body: TypedExpr,
    pub dict_ref_ids: Vec<DictRefId>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TypedExpr {
    pub span: Span,
    pub kind: TypedExprKind,
    pub ty: String,
    pub dict_ref_ids: Vec<DictRefId>,
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
    #[serde(rename = "unknown")]
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
pub struct TypedEffectCall {
    pub effect: Ident,
    pub argument: Box<TypedExpr>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DictRef {
    pub id: DictRefId,
    pub impl_id: String,
    pub span: Span,
    pub requirements: Vec<String>,
    pub ty: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SchemeInfo {
    pub id: usize,
    pub quantifiers: Vec<String>,
    pub constraints: Vec<String>,
    pub ty: String,
}
