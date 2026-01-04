use serde::Serialize;
use smol_str::SmolStr;
use std::collections::HashSet;
use std::fmt;

/// 委譲される型変数を表す識別子。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub struct TypeVariable {
    id: u32,
}

impl TypeVariable {
    pub fn new(id: u32) -> Self {
        Self { id }
    }
    pub fn id(&self) -> u32 {
        self.id
    }
}

impl fmt::Display for TypeVariable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "'t{}", self.id)
    }
}

/// 型のタグを示す列挙型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TypeKind {
    Variable,
    Builtin,
    Arrow,
    Application,
    Slice,
    Ref,
}

/// Reml 型システムの基礎を構成する列挙型。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Type {
    Var(TypeVariable),
    Builtin(BuiltinType),
    Arrow {
        parameters: Vec<Type>,
        result: Box<Type>,
    },
    App {
        constructor: SmolStr,
        arguments: Vec<Type>,
    },
    Slice {
        element: Box<Type>,
    },
    Ref {
        target: Box<Type>,
        mutable: bool,
    },
}

impl Type {
    pub fn var(variable: TypeVariable) -> Self {
        Self::Var(variable)
    }

    pub fn builtin(builtin: BuiltinType) -> Self {
        Self::Builtin(builtin)
    }

    pub fn arrow(parameters: Vec<Type>, result: Type) -> Self {
        Self::Arrow {
            parameters,
            result: Box::new(result),
        }
    }

    pub fn app(constructor: impl Into<SmolStr>, arguments: Vec<Type>) -> Self {
        Self::App {
            constructor: constructor.into(),
            arguments,
        }
    }

    pub fn slice(element: Type) -> Self {
        Self::Slice {
            element: Box::new(element),
        }
    }

    pub fn reference(target: Type, mutable: bool) -> Self {
        Self::Ref {
            target: Box::new(target),
            mutable,
        }
    }

    pub fn kind(&self) -> TypeKind {
        match self {
            Type::Var(_) => TypeKind::Variable,
            Type::Builtin(_) => TypeKind::Builtin,
            Type::Arrow { .. } => TypeKind::Arrow,
            Type::App { .. } => TypeKind::Application,
            Type::Slice { .. } => TypeKind::Slice,
            Type::Ref { .. } => TypeKind::Ref,
        }
    }

    pub fn contains_variable(&self, target: &TypeVariable) -> bool {
        match self {
            Type::Var(variable) => variable == target,
            Type::Arrow { parameters, result } => {
                parameters
                    .iter()
                    .any(|parameter| parameter.contains_variable(target))
                    || result.contains_variable(target)
            }
            Type::App { arguments, .. } => arguments
                .iter()
                .any(|argument| argument.contains_variable(target)),
            Type::Slice { element } => element.contains_variable(target),
            Type::Ref { target: inner, .. } => inner.contains_variable(target),
            _ => false,
        }
    }

    pub fn free_type_variables(&self) -> HashSet<TypeVariable> {
        let mut vars = HashSet::new();
        self.collect_free_type_variables(&mut vars);
        vars
    }

    fn collect_free_type_variables(&self, vars: &mut HashSet<TypeVariable>) {
        match self {
            Type::Var(variable) => {
                vars.insert(*variable);
            }
            Type::Arrow { parameters, result } => {
                for parameter in parameters {
                    parameter.collect_free_type_variables(vars);
                }
                result.collect_free_type_variables(vars);
            }
            Type::App { arguments, .. } => {
                for argument in arguments {
                    argument.collect_free_type_variables(vars);
                }
            }
            Type::Slice { element } => {
                element.collect_free_type_variables(vars);
            }
            Type::Ref { target, .. } => {
                target.collect_free_type_variables(vars);
            }
            _ => {}
        }
    }

    pub fn label(&self) -> String {
        self.to_string()
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Var(var) => write!(f, "{}", var),
            Type::Builtin(builtin) => write!(f, "{}", builtin),
            Type::Arrow { parameters, result } => {
                write!(f, "(")?;
                for (idx, param) in parameters.iter().enumerate() {
                    if idx > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", param)?;
                }
                write!(f, ") -> {}", result)
            }
            Type::App {
                constructor,
                arguments,
            } => {
                write!(f, "{}", constructor)?;
                if arguments.is_empty() {
                    return Ok(());
                }
                write!(f, "<")?;
                for (idx, arg) in arguments.iter().enumerate() {
                    if idx > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", arg)?;
                }
                write!(f, ">")
            }
            Type::Slice { element } => write!(f, "[{}]", element),
            Type::Ref { target, mutable } => {
                if *mutable {
                    write!(f, "&mut {}", target)
                } else {
                    write!(f, "&{}", target)
                }
            }
        }
    }
}

/// 組み込み型の種別。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BuiltinType {
    Int,
    UInt,
    Float,
    Bool,
    Char,
    Str,
    Bytes,
    Unit,
    Unknown,
}

impl BuiltinType {
    pub fn as_str(&self) -> &'static str {
        match self {
            BuiltinType::Int => "Int",
            BuiltinType::UInt => "UInt",
            BuiltinType::Float => "Float",
            BuiltinType::Bool => "Bool",
            BuiltinType::Char => "Char",
            BuiltinType::Str => "Str",
            BuiltinType::Bytes => "Bytes",
            BuiltinType::Unit => "Unit",
            BuiltinType::Unknown => "Unknown",
        }
    }
}

impl fmt::Display for BuiltinType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Capability のコンテキスト情報を表す簡易構造体。
#[derive(Debug, Clone, Serialize)]
pub struct CapabilityContext {
    pub stage: SmolStr,
    pub requirements: Vec<SmolStr>,
}

impl Default for CapabilityContext {
    fn default() -> Self {
        Self {
            stage: SmolStr::new("stable"),
            requirements: Vec::new(),
        }
    }
}

impl CapabilityContext {
    pub fn with_stage(stage: impl Into<SmolStr>) -> Self {
        Self {
            stage: stage.into(),
            requirements: Vec::new(),
        }
    }

    pub fn add_requirement(mut self, requirement: impl Into<SmolStr>) -> Self {
        self.requirements.push(requirement.into());
        self
    }
}

/// 新しい型変数を順に生成するユーティリティ。
#[derive(Debug, Clone, Default)]
pub struct TypeVarGen {
    counter: u32,
}

impl TypeVarGen {
    pub fn next(&mut self) -> TypeVariable {
        let current = self.counter;
        self.counter = self.counter.wrapping_add(1);
        TypeVariable::new(current)
    }

    pub fn fresh_type(&mut self) -> Type {
        Type::var(self.next())
    }
}
