//! Rust フロントエンド PoC 用の最小 AST 定義。

use serde::Serialize;

use crate::span::Span;

#[derive(Debug, Clone, Serialize)]
pub struct Module {
    pub functions: Vec<Function>,
}

impl Module {
    pub fn render(&self) -> String {
        self.functions
            .iter()
            .map(Function::render)
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Function {
    pub name: String,
    pub params: Vec<Param>,
    pub body: Expr,
    pub span: Span,
}

impl Function {
    pub fn render(&self) -> String {
        let params = self
            .params
            .iter()
            .map(|param| param.name.clone())
            .collect::<Vec<_>>()
            .join(", ");
        format!("fn {}({}) = {}", self.name, params, self.body.render())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Param {
    pub name: String,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum Expr {
    Int {
        value: i64,
        span: Span,
    },
    Identifier {
        name: String,
        span: Span,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
        span: Span,
    },
    Binary {
        operator: String,
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },
}

impl Expr {
    pub fn int(value: i64, span: Span) -> Self {
        Self::Int { value, span }
    }

    pub fn identifier(name: impl Into<String>, span: Span) -> Self {
        Self::Identifier {
            name: name.into(),
            span,
        }
    }

    pub fn call(callee: Expr, args: Vec<Expr>, span: Span) -> Self {
        Self::Call {
            callee: Box::new(callee),
            args,
            span,
        }
    }

    pub fn binary(operator: impl Into<String>, left: Expr, right: Expr, span: Span) -> Self {
        Self::Binary {
            operator: operator.into(),
            left: Box::new(left),
            right: Box::new(right),
            span,
        }
    }

    pub fn span(&self) -> Span {
        match self {
            Expr::Int { span, .. }
            | Expr::Identifier { span, .. }
            | Expr::Call { span, .. }
            | Expr::Binary { span, .. } => *span,
        }
    }

    pub fn render(&self) -> String {
        match self {
            Expr::Int { value, .. } => format!("int({value}:base10)"),
            Expr::Identifier { name, .. } => format!("var({name})"),
            Expr::Call { callee, args, .. } => {
                let rendered_args = args.iter().map(Expr::render).collect::<Vec<_>>();
                format!("call({})[{}]", callee.render(), rendered_args.join(", "))
            }
            Expr::Binary {
                operator,
                left,
                right,
                ..
            } => format!("binary({} {} {})", left.render(), operator, right.render()),
        }
    }
}
