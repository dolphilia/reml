use serde::Serialize;

use crate::parser::ast::{Ident, Literal};
use crate::span::Span;

pub type DictRefId = usize;

fn is_false(value: &bool) -> bool {
    !*value
}

/// TypecheckDriver が生成する型付き AST。
/// `docs/plans/rust-migration/appendix/typed_ast_schema_draft.md` に記された構造に合わせ、
/// TypedModule に関数・dict_ref・scheme を含める。
#[derive(Debug, Clone, Serialize, Default)]
pub struct TypedModule {
    pub functions: Vec<TypedFunction>,
    pub active_patterns: Vec<TypedActivePattern>,
    pub conductors: Vec<TypedConductor>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub actor_specs: Vec<TypedActorSpec>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub externs: Vec<TypedExtern>,
    pub dict_refs: Vec<DictRef>,
    pub schemes: Vec<SchemeInfo>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TypedFunction {
    pub name: String,
    pub span: Span,
    pub attributes: Vec<String>,
    pub params: Vec<TypedParam>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub varargs: bool,
    pub return_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_annotation: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub is_async: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub is_unsafe: bool,
    pub body: TypedExpr,
    pub dict_ref_ids: Vec<DictRefId>,
    pub scheme_id: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TypedParam {
    pub name: String,
    pub span: Span,
    pub ty: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotation: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TypedLambdaCapture {
    pub name: String,
    pub span: Span,
    #[serde(default, skip_serializing_if = "is_false")]
    pub mutable: bool,
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
pub struct TypedConductor {
    pub name: String,
    pub span: Span,
    pub dsl_defs: Vec<TypedConductorDslDef>,
    pub channels: Vec<TypedConductorChannel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution: Option<TypedConductorBlock>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub monitoring: Option<TypedConductorMonitoringBlock>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TypedConductorDslDef {
    pub alias: String,
    pub target: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pipeline_type: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tails: Vec<TypedConductorDslTail>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
pub struct TypedConductorDslTail {
    pub stage: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub arg_types: Vec<String>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
pub struct TypedConductorChannel {
    pub source: String,
    pub target: String,
    pub payload: String,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
pub struct TypedConductorBlock {
    pub ty: String,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
pub struct TypedConductorMonitoringBlock {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    pub ty: String,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
pub struct TypedActorSpec {
    pub name: String,
    pub span: Span,
    pub params: Vec<TypedParam>,
    pub return_type: String,
    pub body: TypedExpr,
    pub dict_ref_ids: Vec<DictRefId>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TypedExtern {
    pub name: String,
    pub span: Span,
    pub abi: String,
    pub symbol: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TypedExpr {
    pub span: Span,
    pub kind: TypedExprKind,
    pub ty: String,
    pub dict_ref_ids: Vec<DictRefId>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QualifiedCallKind {
    TypeMethod,
    TypeAssoc,
    TraitMethod,
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
pub struct QualifiedCall {
    pub kind: QualifiedCallKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub impl_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TypedStmt {
    pub span: Span,
    pub kind: TypedStmtKind,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TypedStmtKind {
    Let {
        pattern: TypedPattern,
        value: TypedExpr,
    },
    Var {
        pattern: TypedPattern,
        value: TypedExpr,
    },
    Expr {
        expr: TypedExpr,
    },
    Assign {
        target: TypedExpr,
        value: TypedExpr,
    },
    Defer {
        expr: TypedExpr,
    },
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
        #[serde(skip_serializing_if = "Option::is_none")]
        qualified: Option<QualifiedCall>,
    },
    Lambda {
        params: Vec<TypedParam>,
        #[serde(skip_serializing_if = "Option::is_none")]
        return_annotation: Option<String>,
        body: Box<TypedExpr>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        captures: Vec<TypedLambdaCapture>,
    },
    Rec {
        target: Box<TypedExpr>,
        #[serde(skip_serializing_if = "Option::is_none")]
        ident: Option<Ident>,
    },
    Block {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        statements: Vec<TypedStmt>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tail: Option<Box<TypedExpr>>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        defers: Vec<TypedExpr>,
    },
    Return {
        #[serde(skip_serializing_if = "Option::is_none")]
        value: Option<Box<TypedExpr>>,
    },
    Propagate {
        expr: Box<TypedExpr>,
    },
    Binary {
        operator: String,
        left: Box<TypedExpr>,
        right: Box<TypedExpr>,
    },
    FieldAccess {
        target: Box<TypedExpr>,
        field: Ident,
    },
    TupleAccess {
        target: Box<TypedExpr>,
        index: u32,
    },
    Index {
        target: Box<TypedExpr>,
        index: Box<TypedExpr>,
    },
    Match {
        target: Box<TypedExpr>,
        arms: Vec<TypedMatchArm>,
    },
    IfElse {
        condition: Box<TypedExpr>,
        then_branch: Box<TypedExpr>,
        else_branch: Box<TypedExpr>,
    },
    PerformCall {
        call: TypedEffectCall,
    },
    EffectBlock {
        body: Box<TypedExpr>,
    },
    Async {
        body: Box<TypedExpr>,
        #[serde(default, skip_serializing_if = "is_false")]
        is_move: bool,
    },
    Await {
        expr: Box<TypedExpr>,
    },
    Unsafe {
        body: Box<TypedExpr>,
    },
    InlineAsm {
        template: String,
        outputs: Vec<TypedInlineAsmOutput>,
        inputs: Vec<TypedInlineAsmInput>,
        clobbers: Vec<String>,
        options: Vec<String>,
    },
    LlvmIr {
        result_type: String,
        template: String,
        inputs: Vec<TypedExpr>,
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
pub struct TypedInlineAsmOutput {
    pub constraint: String,
    pub target: Box<TypedExpr>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TypedInlineAsmInput {
    pub constraint: String,
    pub expr: Box<TypedExpr>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TypedMatchArm {
    pub pattern: TypedPattern,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guard: Option<TypedExpr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
    pub body: TypedExpr,
}

#[derive(Debug, Clone, Serialize)]
pub struct TypedPattern {
    pub span: Span,
    pub kind: TypedPatternKind,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TypedPatternKind {
    Wildcard,
    Var {
        name: String,
    },
    Literal(Literal),
    Tuple {
        elements: Vec<TypedPattern>,
    },
    Record {
        fields: Vec<TypedPatternRecordField>,
        has_rest: bool,
    },
    Constructor {
        name: String,
        args: Vec<TypedPattern>,
    },
    Binding {
        name: String,
        pattern: Box<TypedPattern>,
        via_at: bool,
    },
    Or {
        variants: Vec<TypedPattern>,
    },
    Slice {
        elements: Vec<TypedSlicePatternItem>,
    },
    Range {
        start: Option<Box<TypedPattern>>,
        end: Option<Box<TypedPattern>>,
        inclusive: bool,
    },
    Regex {
        pattern: String,
    },
    ActivePattern {
        name: String,
        is_partial: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        argument: Option<Box<TypedPattern>>,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct TypedPatternRecordField {
    pub key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<Box<TypedPattern>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TypedSlicePatternItem {
    Element(TypedPattern),
    Rest {
        #[serde(skip_serializing_if = "Option::is_none")]
        ident: Option<String>,
    },
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
