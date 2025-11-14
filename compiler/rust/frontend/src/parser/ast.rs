use serde::Serialize;

use crate::span::Span;

#[derive(Debug, Clone, Serialize)]
pub struct Module {
    pub effects: Vec<EffectDecl>,
    pub functions: Vec<Function>,
    pub decls: Vec<Decl>,
}

impl Module {
    pub fn render(&self) -> String {
        let mut rendered = Vec::new();
        for effect in &self.effects {
            rendered.push(format!("effect {}", effect.name.name));
        }
        for decl in &self.decls {
            rendered.push(decl.render());
        }
        rendered.extend(
            self.functions
                .iter()
                .map(Function::render)
                .collect::<Vec<_>>(),
        );
        rendered.join("\n")
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct EffectDecl {
    pub name: Ident,
    pub span: Span,
    pub tag: Option<Ident>,
    pub operations: Vec<OperationDecl>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OperationDecl {
    pub name: Ident,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
pub struct Decl {
    pub attrs: Vec<Attribute>,
    pub visibility: Visibility,
    pub kind: DeclKind,
    pub span: Span,
}

impl Decl {
    pub fn render(&self) -> String {
        match &self.kind {
            DeclKind::Let { pattern, value, .. } => {
                format!("let {} = {}", pattern.render(), value.render())
            }
            DeclKind::Var { pattern, value, .. } => {
                format!("var {} = {}", pattern.render(), value.render())
            }
            DeclKind::Fn { name, .. } => format!("fn {} ...", name.name),
            DeclKind::Effect(effect) => format!("effect {}", effect.name.name),
            DeclKind::Type { name, .. } => format!("type {}", name.name),
            DeclKind::Trait { name, .. } => format!("trait {}", name.name),
            DeclKind::Impl { name, .. } => format!("impl {}", name.name),
            DeclKind::Extern { name, .. } => format!("extern {}", name.name),
            DeclKind::Handler { name, .. } => format!("handler {}", name.name),
            DeclKind::Conductor { name, .. } => format!("conductor {}", name.name),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DeclKind {
    Let {
        pattern: Pattern,
        value: Expr,
        type_annotation: Option<TypeAnnot>,
    },
    Var {
        pattern: Pattern,
        value: Expr,
        type_annotation: Option<TypeAnnot>,
    },
    Fn {
        name: Ident,
        span: Span,
    },
    Type {
        name: Ident,
        span: Span,
    },
    Trait {
        name: Ident,
        span: Span,
    },
    Impl {
        name: Ident,
        span: Span,
    },
    Extern {
        name: Ident,
        span: Span,
    },
    Effect(EffectDecl),
    Handler {
        name: Ident,
        span: Span,
    },
    Conductor {
        name: Ident,
        span: Span,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct Attribute {
    pub name: Ident,
    pub args: Vec<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    Public,
    Private,
}

#[derive(Debug, Clone, Serialize)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ExprKind {
    Literal(Literal),
    Identifier(Ident),
    ModulePath(ModulePath),
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
    PerformCall {
        call: EffectCall,
    },
    Lambda {
        params: Vec<Param>,
        ret_type: Option<TypeAnnot>,
        body: Box<Expr>,
    },
    Pipe {
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Binary {
        operator: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Unary {
        operator: UnaryOp,
        expr: Box<Expr>,
    },
    FieldAccess {
        target: Box<Expr>,
        field: Ident,
    },
    TupleAccess {
        target: Box<Expr>,
        index: u32,
    },
    Index {
        target: Box<Expr>,
        index: Box<Expr>,
    },
    Propagate {
        expr: Box<Expr>,
    },
    IfElse {
        condition: Box<Expr>,
        then_branch: Box<Expr>,
        else_branch: Option<Box<Expr>>,
    },
    Match {
        target: Box<Expr>,
        arms: Vec<MatchArm>,
    },
    While {
        condition: Box<Expr>,
        body: Box<Expr>,
    },
    For {
        pattern: Pattern,
        start: Box<Expr>,
        end: Box<Expr>,
    },
    Loop {
        body: Box<Expr>,
    },
    Handle {
        handle: HandleExpr,
    },
    Continue,
    Block {
        statements: Vec<Stmt>,
    },
    Unsafe {
        body: Box<Expr>,
    },
    Return {
        value: Option<Box<Expr>>,
    },
    Defer {
        body: Box<Expr>,
    },
    Assign {
        target: Box<Expr>,
        value: Box<Expr>,
    },
}

#[derive(Debug, Clone, Serialize)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    PipeOp,
    Custom(String),
}

impl BinaryOp {
    pub fn from_symbol(symbol: String) -> Self {
        match symbol.as_str() {
            "+" => BinaryOp::Add,
            "-" => BinaryOp::Sub,
            "*" => BinaryOp::Mul,
            "/" => BinaryOp::Div,
            "%" => BinaryOp::Mod,
            "**" => BinaryOp::Pow,
            "==" => BinaryOp::Eq,
            "!=" => BinaryOp::Ne,
            "<" => BinaryOp::Lt,
            "<=" => BinaryOp::Le,
            ">" => BinaryOp::Gt,
            ">=" => BinaryOp::Ge,
            "&&" => BinaryOp::And,
            "||" => BinaryOp::Or,
            "|>" => BinaryOp::PipeOp,
            other => BinaryOp::Custom(other.to_string()),
        }
    }

    pub fn symbol(&self) -> &str {
        match self {
            BinaryOp::Add => "+",
            BinaryOp::Sub => "-",
            BinaryOp::Mul => "*",
            BinaryOp::Div => "/",
            BinaryOp::Mod => "%",
            BinaryOp::Pow => "**",
            BinaryOp::Eq => "==",
            BinaryOp::Ne => "!=",
            BinaryOp::Lt => "<",
            BinaryOp::Le => "<=",
            BinaryOp::Gt => ">",
            BinaryOp::Ge => ">=",
            BinaryOp::And => "&&",
            BinaryOp::Or => "||",
            BinaryOp::PipeOp => "|>",
            BinaryOp::Custom(value) => value,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub enum UnaryOp {
    Not,
    Neg,
    Custom(String),
}

impl UnaryOp {
    pub fn from_symbol(symbol: &str) -> Self {
        match symbol {
            "!" => UnaryOp::Not,
            "-" => UnaryOp::Neg,
            other => UnaryOp::Custom(other.to_string()),
        }
    }

    pub fn symbol(&self) -> &str {
        match self {
            UnaryOp::Not => "!",
            UnaryOp::Neg => "-",
            UnaryOp::Custom(value) => value,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub guard: Option<Expr>,
    pub body: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
pub struct HandleExpr {
    pub target: Box<Expr>,
    pub handler: Ident,
}

#[derive(Debug, Clone, Serialize)]
pub struct EffectCall {
    pub effect: Ident,
    pub argument: Box<Expr>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Literal {
    pub value: LiteralKind,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LiteralKind {
    Int {
        value: i64,
        raw: String,
        base: IntBase,
    },
    Float {
        raw: String,
    },
    Char {
        value: String,
    },
    String {
        value: String,
        string_kind: StringKind,
    },
    Bool {
        value: bool,
    },
    Unit,
    Tuple {
        elements: Vec<Expr>,
    },
    Array {
        elements: Vec<Expr>,
    },
    Record {
        fields: Vec<RecordField>,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct RecordField {
    pub key: Ident,
    pub value: Expr,
}

impl Literal {
    fn render(&self) -> String {
        match &self.value {
            LiteralKind::Int { value, base, .. } => format!("int({value}:{})", base.label()),
            LiteralKind::Bool { value } => format!("bool({value})"),
            LiteralKind::String { value, .. } => format!("str(\"{}\")", value),
            LiteralKind::Char { value } => format!("char('{}')", value),
            LiteralKind::Float { raw } => format!("float({raw})"),
            LiteralKind::Unit => "unit".to_string(),
            LiteralKind::Tuple { elements } => format!(
                "tuple({})",
                elements
                    .iter()
                    .map(Expr::render)
                    .collect::<Vec<_>>()
                    .join(", "),
            ),
            LiteralKind::Array { elements } => format!(
                "array([{}])",
                elements
                    .iter()
                    .map(Expr::render)
                    .collect::<Vec<_>>()
                    .join(", "),
            ),
            LiteralKind::Record { fields } => {
                let entries = fields
                    .iter()
                    .map(|field| format!("{}: {}", field.key.name, field.value.render()))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("record({})", entries)
            }
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IntBase {
    Base2,
    Base8,
    Base10,
    Base16,
}

impl IntBase {
    fn label(&self) -> &'static str {
        match self {
            IntBase::Base2 => "base2",
            IntBase::Base8 => "base8",
            IntBase::Base10 => "base10",
            IntBase::Base16 => "base16",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StringKind {
    Normal,
    Raw,
    Multiline,
}

#[derive(Debug, Clone, Serialize)]
pub struct Pattern {
    pub kind: PatternKind,
    pub span: Span,
}

impl Pattern {
    fn render(&self) -> String {
        match &self.kind {
            PatternKind::Literal(literal) => literal.render(),
            PatternKind::Var(ident) => ident.name.clone(),
            PatternKind::Wildcard => "_".to_string(),
            PatternKind::Tuple { elements } => format!(
                "({})",
                elements
                    .iter()
                    .map(Pattern::render)
                    .collect::<Vec<_>>()
                    .join(", "),
            ),
            PatternKind::Record { fields, has_rest } => {
                let mut parts = fields
                    .iter()
                    .map(|field| match &field.value {
                        Some(pat) => format!("{}: {}", field.key.name, pat.render()),
                        None => field.key.name.clone(),
                    })
                    .collect::<Vec<_>>();
                if *has_rest {
                    parts.push("..".to_string());
                }
                format!("{{{}}}", parts.join(", "))
            }
            PatternKind::Constructor { name, args } => format!(
                "{}({})",
                name.name,
                args.iter()
                    .map(Pattern::render)
                    .collect::<Vec<_>>()
                    .join(", "),
            ),
            PatternKind::Guard { pattern, guard } => {
                format!("{} if {}", pattern.render(), guard.render())
            }
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PatternKind {
    Literal(Literal),
    Var(Ident),
    Wildcard,
    Tuple {
        elements: Vec<Pattern>,
    },
    Record {
        fields: Vec<PatternRecordField>,
        has_rest: bool,
    },
    Constructor {
        name: Ident,
        args: Vec<Pattern>,
    },
    Guard {
        pattern: Box<Pattern>,
        guard: Box<Expr>,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct PatternRecordField {
    pub key: Ident,
    pub value: Option<Box<Pattern>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Ident {
    pub name: String,
    pub span: Span,
}

impl Ident {
    pub fn as_str(&self) -> &str {
        &self.name
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ModulePath {
    Root {
        segments: Vec<Ident>,
    },
    Relative {
        head: RelativeHead,
        segments: Vec<Ident>,
    },
}

#[derive(Debug, Clone, Serialize)]
pub enum RelativeHead {
    #[serde(rename = "self")]
    Self_,
    #[serde(rename = "super")]
    Super(u32),
    #[serde(rename = "plain_ident")]
    PlainIdent(Ident),
}

#[derive(Debug, Clone, Serialize)]
pub struct Function {
    pub name: Ident,
    pub params: Vec<Param>,
    pub body: Expr,
    pub ret_type: Option<TypeAnnot>,
    pub span: Span,
}

impl Function {
    pub fn render(&self) -> String {
        let params = self
            .params
            .iter()
            .map(Param::render)
            .collect::<Vec<_>>()
            .join(", ");
        let ret = self
            .ret_type
            .as_ref()
            .map(|ty| format!(" -> {}", ty.render()))
            .unwrap_or_default();
        format!(
            "fn {}({}){} = {}",
            self.name.name,
            params,
            ret,
            self.body.render()
        )
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Param {
    pub name: Ident,
    pub type_annotation: Option<TypeAnnot>,
    pub default: Option<Expr>,
    pub span: Span,
}

impl Param {
    fn render(&self) -> String {
        let mut text = self.name.name.clone();
        if let Some(ty) = &self.type_annotation {
            text.push_str(&format!(": {}", ty.render()));
        }
        if let Some(default) = &self.default {
            text.push_str(&format!(" = {}", default.render()));
        }
        text
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TypeAnnot {
    pub kind: TypeKind,
    pub span: Span,
}

impl TypeAnnot {
    fn render(&self) -> String {
        self.kind.render()
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TypeKind {
    Ident {
        name: Ident,
    },
    App {
        callee: Ident,
        args: Vec<TypeAnnot>,
    },
    Tuple {
        elements: Vec<TypeAnnot>,
    },
    Record {
        fields: Vec<(Ident, TypeAnnot)>,
    },
    Fn {
        params: Vec<TypeAnnot>,
        ret: Box<TypeAnnot>,
    },
}

impl TypeKind {
    fn render(&self) -> String {
        match self {
            TypeKind::Ident { name } => name.name.clone(),
            TypeKind::App { callee, args } => format!(
                "{}<{}>",
                callee.name,
                args.iter()
                    .map(|ty| ty.render())
                    .collect::<Vec<_>>()
                    .join(", "),
            ),
            TypeKind::Tuple { elements } => format!(
                "({})",
                elements
                    .iter()
                    .map(|ty| ty.render())
                    .collect::<Vec<_>>()
                    .join(", "),
            ),
            TypeKind::Record { fields } => {
                let entries = fields
                    .iter()
                    .map(|(key, ty)| format!("{}: {}", key.name, ty.render()))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("record({})", entries)
            }
            TypeKind::Fn { params, ret } => format!(
                "fn({}) -> {}",
                params
                    .iter()
                    .map(|ty| ty.render())
                    .collect::<Vec<_>>()
                    .join(", "),
                ret.render(),
            ),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StmtKind {
    Decl { decl: Decl },
    Expr { expr: Box<Expr> },
    Assign { target: Box<Expr>, value: Box<Expr> },
    Defer { expr: Box<Expr> },
}

impl Expr {
    pub fn literal(literal: Literal, span: Span) -> Self {
        Self {
            span,
            kind: ExprKind::Literal(literal),
        }
    }

    pub fn int(value: i64, raw: String, span: Span) -> Self {
        Self::literal(
            Literal {
                value: LiteralKind::Int {
                    value,
                    raw,
                    base: IntBase::Base10,
                },
            },
            span,
        )
    }

    pub fn bool(value: bool, span: Span) -> Self {
        Self::literal(
            Literal {
                value: LiteralKind::Bool { value },
            },
            span,
        )
    }

    pub fn string(value: impl Into<String>, span: Span) -> Self {
        Self::literal(
            Literal {
                value: LiteralKind::String {
                    value: value.into(),
                    string_kind: StringKind::Normal,
                },
            },
            span,
        )
    }

    pub fn identifier(ident: Ident) -> Self {
        let span = ident.span;
        Self {
            span,
            kind: ExprKind::Identifier(ident),
        }
    }

    pub fn call(callee: Expr, args: Vec<Expr>, span: Span) -> Self {
        Self {
            span,
            kind: ExprKind::Call {
                callee: Box::new(callee),
                args,
            },
        }
    }

    pub fn binary(operator: impl Into<String>, left: Expr, right: Expr, span: Span) -> Self {
        Self {
            span,
            kind: ExprKind::Binary {
                operator: BinaryOp::from_symbol(operator.into()),
                left: Box::new(left),
                right: Box::new(right),
            },
        }
    }

    pub fn if_else(condition: Expr, then_branch: Expr, else_branch: Expr, span: Span) -> Self {
        Self {
            span,
            kind: ExprKind::IfElse {
                condition: Box::new(condition),
                then_branch: Box::new(then_branch),
                else_branch: Some(Box::new(else_branch)),
            },
        }
    }

    pub fn perform(effect: Ident, argument: Expr, span: Span) -> Self {
        Self {
            span,
            kind: ExprKind::PerformCall {
                call: EffectCall {
                    effect,
                    argument: Box::new(argument),
                },
            },
        }
    }

    pub fn span(&self) -> Span {
        self.span
    }

    pub fn render(&self) -> String {
        match &self.kind {
            ExprKind::Literal(literal) => literal.render(),
            ExprKind::Identifier(ident) => format!("var({})", ident.name),
            ExprKind::ModulePath(path) => format!("module_path({:?})", path),
            ExprKind::Call { callee, args } => {
                let rendered_args = args.iter().map(Expr::render).collect::<Vec<_>>();
                format!("call({})[{}]", callee.render(), rendered_args.join(", "))
            }
            ExprKind::Binary {
                operator,
                left,
                right,
            } => format!(
                "binary({} {} {})",
                left.render(),
                operator.symbol(),
                right.render()
            ),
            ExprKind::IfElse {
                condition,
                then_branch,
                else_branch,
            } => format!(
                "if {} then {} else {}",
                condition.render(),
                then_branch.render(),
                else_branch
                    .as_ref()
                    .map(|expr| expr.render())
                    .unwrap_or_else(|| "unit".to_string())
            ),
            ExprKind::PerformCall { call } => {
                format!("perform {} {}", call.effect.name, call.argument.render())
            }
            ExprKind::Return { value } => format!(
                "return {}",
                value
                    .as_ref()
                    .map(|expr| expr.render())
                    .unwrap_or_else(|| "unit".to_string())
            ),
            ExprKind::Block { statements } => format!(
                "block({})",
                statements
                    .iter()
                    .map(|stmt| stmt.to_string())
                    .collect::<Vec<_>>()
                    .join(", "),
            ),
            ExprKind::Assign { target, value } => {
                format!("assign({} := {})", target.render(), value.render())
            }
            ExprKind::Continue => "continue".to_string(),
            other => format!("expr({:?})", other),
        }
    }
}

impl ToString for Stmt {
    fn to_string(&self) -> String {
        match &self.kind {
            StmtKind::Decl { decl } => decl.render(),
            StmtKind::Expr { expr } => expr.render(),
            StmtKind::Assign { target, value } => {
                format!("assign({} := {})", target.render(), value.render())
            }
            StmtKind::Defer { expr } => format!("defer({})", expr.render()),
        }
    }
}
