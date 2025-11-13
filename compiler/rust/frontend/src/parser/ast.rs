//! Rust フロントエンドにおける AST 定義。
//! `docs/plans/rust-migration/1-1-ast-and-ir-alignment.md` に記載された OCaml
//! AST を参考にしつつ、JSON 出力の安定化と dual-write 比較に必要な構造を整理する。

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
}

#[derive(Debug, Clone, Serialize)]
pub struct Decl {
    pub kind: DeclKind,
    pub span: Span,
}

impl Decl {
    fn render(&self) -> String {
        match &self.kind {
            DeclKind::Let { pattern, value } => {
                format!("let {} = {}", pattern.render(), value.render())
            }
            DeclKind::Effect(effect) => format!("effect {}", effect.name.name),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DeclKind {
    Let { pattern: Pattern, value: Expr },
    Effect(EffectDecl),
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
                    .join(", ")
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
                    .join(", ")
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
pub enum Literal {
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

impl Literal {
    fn render(&self) -> String {
        match self {
            Literal::Int { value, base, .. } => format!("int({value}:{})", base.label()),
            Literal::Bool { value } => format!("bool({value})"),
            Literal::String { value, .. } => format!("str(\"{}\")", value),
            Literal::Char { value } => format!("char('{}')", value),
            Literal::Float { raw } => format!("float({raw})"),
            Literal::Unit => "unit".to_string(),
            Literal::Tuple { elements } => format!(
                "tuple({})",
                elements
                    .iter()
                    .map(Expr::render)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Literal::Array { elements } => format!(
                "array[{}]",
                elements
                    .iter()
                    .map(Expr::render)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Literal::Record { fields } => format!(
                "record{{{}}}",
                fields
                    .iter()
                    .map(|field| format!("{}: {}", field.name.name, field.value.render()))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct RecordField {
    pub name: Ident,
    pub value: Expr,
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
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ExprKind {
    Literal(Literal),
    Identifier(Ident),
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
    Binary {
        operator: String,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    IfElse {
        condition: Box<Expr>,
        then_branch: Box<Expr>,
        else_branch: Box<Expr>,
    },
    PerformCall {
        call: EffectCall,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct EffectCall {
    pub effect: Ident,
    pub argument: Box<Expr>,
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
            Literal::Int {
                value,
                raw,
                base: IntBase::Base10,
            },
            span,
        )
    }

    pub fn bool(value: bool, span: Span) -> Self {
        Self::literal(Literal::Bool { value }, span)
    }

    pub fn string(value: impl Into<String>, span: Span) -> Self {
        Self::literal(
            Literal::String {
                value: value.into(),
                string_kind: StringKind::Normal,
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
                operator: operator.into(),
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
                else_branch: Box::new(else_branch),
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
            ExprKind::Call { callee, args } => {
                let rendered_args = args.iter().map(Expr::render).collect::<Vec<_>>();
                format!("call({})[{}]", callee.render(), rendered_args.join(", "))
            }
            ExprKind::Binary {
                operator,
                left,
                right,
            } => format!("binary({} {} {})", left.render(), operator, right.render()),
            ExprKind::IfElse {
                condition,
                then_branch,
                else_branch,
            } => format!(
                "if {} then {} else {}",
                condition.render(),
                then_branch.render(),
                else_branch.render()
            ),
            ExprKind::PerformCall { call } => {
                format!("perform {} {}", call.effect.name, call.argument.render())
            }
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Function {
    pub name: Ident,
    pub params: Vec<Param>,
    pub body: Expr,
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
        format!("fn {}({}) = {}", self.name.name, params, self.body.render())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Param {
    pub name: Ident,
}

impl Param {
    fn render(&self) -> String {
        self.name.name.clone()
    }
}
