use serde::Serialize;

use crate::span::Span;

#[derive(Debug, Clone, Serialize)]
pub struct Module {
    pub header: Option<ModuleHeader>,
    pub uses: Vec<UseDecl>,
    pub effects: Vec<EffectDecl>,
    pub functions: Vec<Function>,
    pub active_patterns: Vec<ActivePatternDecl>,
    pub decls: Vec<Decl>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exprs: Vec<Expr>,
}

impl Module {
    pub fn render(&self) -> String {
        let mut rendered = Vec::new();
        if let Some(header) = &self.header {
            rendered.push(format!("module {}", header.path.render()));
        }
        for use_decl in &self.uses {
            rendered.push(use_decl.render());
        }
        for effect in &self.effects {
            rendered.push(format!("effect {}", effect.name.name));
        }
        for active_pattern in &self.active_patterns {
            rendered.push(active_pattern.render());
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
        rendered.extend(self.exprs.iter().map(Expr::render));
        rendered.join("\n")
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ModuleBody {
    pub effects: Vec<EffectDecl>,
    pub functions: Vec<Function>,
    pub active_patterns: Vec<ActivePatternDecl>,
    pub decls: Vec<Decl>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exprs: Vec<Expr>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModuleDecl {
    pub path: ModulePath,
    pub body: ModuleBody,
    pub span: Span,
}

fn is_false(value: &bool) -> bool {
    !*value
}

fn render_generics(generics: &[Ident]) -> String {
    if generics.is_empty() {
        String::new()
    } else {
        format!(
            "<{}>",
            generics
                .iter()
                .map(|ident| ident.name.clone())
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ModuleHeader {
    pub path: ModulePath,
    pub visibility: Visibility,
    pub attrs: Vec<Attribute>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
pub struct UseDecl {
    pub is_pub: bool,
    pub tree: UseTree,
    pub span: Span,
}

impl UseDecl {
    pub fn render(&self) -> String {
        let mut text = String::new();
        if self.is_pub {
            text.push_str("pub ");
        }
        text.push_str("use ");
        text.push_str(&self.tree.render());
        text
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum UseTree {
    Path {
        path: ModulePath,
        alias: Option<Ident>,
    },
    Brace {
        path: ModulePath,
        items: Vec<UseItem>,
    },
}

impl UseTree {
    pub fn render(&self) -> String {
        match self {
            UseTree::Path { path, alias } => {
                let mut text = path.render();
                if let Some(alias_ident) = alias {
                    text.push_str(" as ");
                    text.push_str(&alias_ident.name);
                }
                text
            }
            UseTree::Brace { path, items } => {
                let rendered_items = items
                    .iter()
                    .map(UseItem::render)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{}.{{{rendered_items}}}", path.render())
            }
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct UseItem {
    pub name: Option<Ident>,
    pub alias: Option<Ident>,
    pub nested: Vec<UseItem>,
    pub glob: bool,
    pub span: Span,
}

impl UseItem {
    pub fn render(&self) -> String {
        if self.glob {
            return "*".to_string();
        }
        let mut text = self
            .name
            .as_ref()
            .map(|ident| ident.name.clone())
            .unwrap_or_default();
        if let Some(alias_ident) = &self.alias {
            text.push_str(" as ");
            text.push_str(&alias_ident.name);
        }
        if !self.nested.is_empty() {
            let nested = self
                .nested
                .iter()
                .map(UseItem::render)
                .collect::<Vec<_>>()
                .join(", ");
            text.push_str(&format!(".{{{nested}}}"));
        }
        text
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum FixityKind {
    Prefix,
    Postfix,
    InfixLeft,
    InfixRight,
    InfixNonassoc,
    Ternary,
}

impl FixityKind {
    pub fn label(&self) -> &'static str {
        match self {
            FixityKind::Prefix => "prefix",
            FixityKind::Postfix => "postfix",
            FixityKind::InfixLeft => "infix_left",
            FixityKind::InfixRight => "infix_right",
            FixityKind::InfixNonassoc => "infix_nonassoc",
            FixityKind::Ternary => "ternary",
        }
    }

    pub fn keyword(&self) -> &'static str {
        match self {
            FixityKind::Prefix => ":prefix",
            FixityKind::Postfix => ":postfix",
            FixityKind::InfixLeft => ":infix_left",
            FixityKind::InfixRight => ":infix_right",
            FixityKind::InfixNonassoc => ":infix_nonassoc",
            FixityKind::Ternary => ":ternary",
        }
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
    pub attrs: Vec<Attribute>,
    pub name: Ident,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<TypeAnnot>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
pub struct HandlerDecl {
    pub name: Ident,
    pub entries: Vec<HandlerEntry>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum HandlerEntry {
    Operation {
        attrs: Vec<Attribute>,
        name: Ident,
        params: Vec<Param>,
        body: Expr,
        span: Span,
    },
    Return {
        value_ident: Ident,
        body: Expr,
        span: Span,
    },
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
        let visibility = match self.visibility {
            Visibility::Public => "pub ",
            Visibility::Private => "",
        };
        match &self.kind {
            DeclKind::Let { pattern, value, .. } => {
                format!("{visibility}let {} = {}", pattern.render(), value.render())
            }
            DeclKind::Var { pattern, value, .. } => {
                format!("{visibility}var {} = {}", pattern.render(), value.render())
            }
            DeclKind::Const { name, value, .. } => {
                format!("{visibility}const {} = {}", name.name, value.render())
            }
            DeclKind::Fn { signature } => format!("{visibility}{}", signature.render()),
            DeclKind::Effect(effect) => format!("{visibility}effect {}", effect.name.name),
            DeclKind::Type { decl } => format!("{visibility}{}", decl.render()),
            DeclKind::Struct(decl) => format!("{visibility}struct {}", decl.name.name),
            DeclKind::Enum(decl) => format!("{visibility}enum {}", decl.name.name),
            DeclKind::Trait(trait_decl) => format!("{visibility}trait {}", trait_decl.name.name),
            DeclKind::Impl(impl_decl) => {
                let mut label = String::from("impl");
                if let Some(trait_ref) = &impl_decl.trait_ref {
                    label.push(' ');
                    label.push_str(&trait_ref.render());
                    label.push(' ');
                    label.push_str("for ");
                } else {
                    label.push(' ');
                }
                label.push_str(&impl_decl.target.render());
                format!("{visibility}{label}")
            }
            DeclKind::Extern { abi, .. } => format!("{visibility}extern \"{abi}\" ..."),
            DeclKind::Handler(handler) => format!("{visibility}handler {}", handler.name.name),
            DeclKind::Conductor(decl) => format!("{visibility}conductor {}", decl.name.name),
            DeclKind::Module(decl) => format!("{visibility}module {}", decl.path.render()),
            DeclKind::Macro(decl) => format!("{visibility}macro {}", decl.name.name),
            DeclKind::ActorSpec(decl) => format!("{visibility}actor spec {}", decl.name.name),
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
    Const {
        name: Ident,
        value: Expr,
        type_annotation: TypeAnnot,
    },
    Fn {
        signature: FunctionSignature,
    },
    Type {
        #[serde(flatten)]
        decl: TypeDecl,
    },
    Struct(StructDecl),
    Enum(EnumDecl),
    Trait(TraitDecl),
    Impl(ImplDecl),
    Extern {
        abi: String,
        functions: Vec<ExternItem>,
        span: Span,
    },
    Effect(EffectDecl),
    Handler(HandlerDecl),
    Conductor(ConductorDecl),
    Module(ModuleDecl),
    Macro(MacroDecl),
    ActorSpec(ActorSpecDecl),
}

#[derive(Debug, Clone, Serialize)]
pub struct TypeDecl {
    pub name: Ident,
    pub generics: Vec<Ident>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<TypeDeclBody>,
    pub span: Span,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_span: Option<Span>,
}

impl TypeDecl {
    pub fn render(&self) -> String {
        let generics = render_generics(&self.generics);
        match &self.body {
            Some(TypeDeclBody::Alias { ty }) => {
                format!(
                    "type alias {}{} = {}",
                    self.name.name,
                    generics,
                    ty.render()
                )
            }
            Some(TypeDeclBody::Newtype { ty }) => {
                format!("type {}{} = new {}", self.name.name, generics, ty.render())
            }
            Some(TypeDeclBody::Sum { variants }) => format!(
                "type {}{} = {}",
                self.name.name,
                generics,
                variants
                    .iter()
                    .map(TypeDeclVariant::render)
                    .collect::<Vec<_>>()
                    .join(" | ")
            ),
            None => format!("type {}{}", self.name.name, generics),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TypeDeclBody {
    Alias { ty: TypeAnnot },
    Newtype { ty: TypeAnnot },
    Sum { variants: Vec<TypeDeclVariant> },
}

#[derive(Debug, Clone, Serialize)]
pub struct TypeDeclVariant {
    pub name: Ident,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<TypeDeclVariantPayload>,
    pub span: Span,
}

impl TypeDeclVariant {
    fn render(&self) -> String {
        match &self.payload {
            Some(TypeDeclVariantPayload::Record { .. }) => {
                format!("{} {}", self.name.name, self.payload_render())
            }
            Some(TypeDeclVariantPayload::Tuple { .. }) => {
                format!("{}{}", self.name.name, self.payload_render())
            }
            None => self.name.name.clone(),
        }
    }

    fn payload_render(&self) -> String {
        self.payload
            .as_ref()
            .map(TypeDeclVariantPayload::render)
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TypeDeclVariantPayload {
    Record {
        fields: Vec<TypeRecordField>,
        #[serde(default, skip_serializing_if = "is_false")]
        has_rest: bool,
    },
    Tuple {
        elements: Vec<TypeTupleElement>,
    },
}

impl TypeDeclVariantPayload {
    fn render(&self) -> String {
        match self {
            TypeDeclVariantPayload::Record { fields, has_rest } => {
                let mut entries = fields
                    .iter()
                    .map(TypeRecordField::render)
                    .collect::<Vec<_>>();
                if *has_rest {
                    entries.push("..".to_string());
                }
                let joined = entries.join(", ");
                if joined.is_empty() {
                    "{}".to_string()
                } else {
                    format!("{{ {} }}", joined)
                }
            }
            TypeDeclVariantPayload::Tuple { elements } => {
                let entries = elements
                    .iter()
                    .map(TypeTupleElement::render)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("({})", entries)
            }
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ExternItem {
    pub attrs: Vec<Attribute>,
    pub visibility: Visibility,
    pub signature: FunctionSignature,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConductorDecl {
    pub name: Ident,
    pub dsl_defs: Vec<ConductorDslDef>,
    pub channels: Vec<ConductorChannelRoute>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution: Option<ConductorExecutionBlock>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub monitoring: Option<ConductorMonitoringBlock>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
pub struct EffectAnnotation {
    pub tags: Vec<Ident>,
    pub span: Span,
}

impl EffectAnnotation {
    pub fn render(&self) -> String {
        self.tags
            .iter()
            .map(|ident| ident.name.clone())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct FunctionSignature {
    pub name: Ident,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub qualified_name: Option<QualifiedName>,
    pub generics: Vec<Ident>,
    pub params: Vec<Param>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub varargs: bool,
    pub ret_type: Option<TypeAnnot>,
    pub where_clause: Vec<WherePredicate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effect: Option<EffectAnnotation>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub is_async: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub is_unsafe: bool,
    pub span: Span,
}

impl FunctionSignature {
    pub fn binding_key(&self) -> String {
        self.qualified_name
            .as_ref()
            .map(QualifiedName::render)
            .unwrap_or_else(|| self.name.name.clone())
    }

    pub fn render(&self) -> String {
        let params = self
            .params
            .iter()
            .map(Param::render)
            .collect::<Vec<_>>()
            .join(", ");
        let generics = if self.generics.is_empty() {
            String::new()
        } else {
            format!(
                "<{}>",
                self.generics
                    .iter()
                    .map(|ident| ident.name.clone())
                    .collect::<Vec<_>>()
                    .join(", "),
            )
        };
        let ret = self
            .ret_type
            .as_ref()
            .map(|ty| format!(" -> {}", ty.render()))
            .unwrap_or_default();
        let where_clause = if self.where_clause.is_empty() {
            String::new()
        } else {
            format!(
                " where {}",
                self.where_clause
                    .iter()
                    .map(WherePredicate::render)
                    .collect::<Vec<_>>()
                    .join(", "),
            )
        };
        let effect = self
            .effect
            .as_ref()
            .map(|annot| format!(" !{{{}}}", annot.render()))
            .unwrap_or_default();
        let async_prefix = if self.is_async { "async " } else { "" };
        let unsafe_prefix = if self.is_unsafe { "unsafe " } else { "" };
        format!(
            "{async_prefix}{unsafe_prefix}fn {}{}({}){}{}{}",
            self.name.name, generics, params, ret, where_clause, effect
        )
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TraitDecl {
    pub name: Ident,
    pub generics: Vec<Ident>,
    pub where_clause: Vec<WherePredicate>,
    pub items: Vec<TraitItem>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
pub struct StructDecl {
    pub name: Ident,
    pub generics: Vec<Ident>,
    pub fields: Vec<TypeRecordField>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
pub struct EnumDecl {
    /// enum 宣言は型宣言（合成型）として扱う想定。
    pub name: Ident,
    pub generics: Vec<Ident>,
    pub variants: Vec<EnumVariant>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum VariantPayload {
    Record { fields: Vec<TypeRecordField> },
    Tuple { elements: Vec<TypeTupleElement> },
}

impl VariantPayload {
    fn render(&self) -> String {
        match self {
            VariantPayload::Record { fields } => {
                let entries = fields
                    .iter()
                    .map(TypeRecordField::render)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{{ {} }}", entries)
            }
            VariantPayload::Tuple { elements } => {
                let entries = elements
                    .iter()
                    .map(TypeTupleElement::render)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("({})", entries)
            }
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct EnumVariant {
    pub name: Ident,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<VariantPayload>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
pub struct TraitItem {
    pub attrs: Vec<Attribute>,
    pub kind: TraitItemKind,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TraitItemKind {
    Function {
        signature: FunctionSignature,
        #[serde(skip_serializing_if = "Option::is_none")]
        default_body: Option<Expr>,
    },
    AssociatedType {
        name: Ident,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        bounds: Vec<TraitRef>,
        #[serde(skip_serializing_if = "Option::is_none")]
        default: Option<TypeAnnot>,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct ImplDecl {
    pub generics: Vec<Ident>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trait_ref: Option<TraitRef>,
    pub target: TypeAnnot,
    pub where_clause: Vec<WherePredicate>,
    pub items: Vec<ImplItem>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ImplItem {
    Function(Function),
    Decl(Decl),
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WherePredicate {
    TypeBound {
        target: TypeAnnot,
        bounds: Vec<TraitRef>,
        span: Span,
    },
    Trait {
        trait_ref: TraitRef,
    },
}

impl WherePredicate {
    pub fn render(&self) -> String {
        match self {
            WherePredicate::TypeBound { target, bounds, .. } => {
                let bounds = bounds
                    .iter()
                    .map(TraitRef::render)
                    .collect::<Vec<_>>()
                    .join(" + ");
                format!("{}: {}", target.render(), bounds)
            }
            WherePredicate::Trait { trait_ref } => trait_ref.render(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TraitRef {
    pub name: Ident,
    pub args: Vec<TypeAnnot>,
    pub span: Span,
}

impl TraitRef {
    pub fn render(&self) -> String {
        if self.args.is_empty() {
            self.name.name.clone()
        } else {
            let args = self
                .args
                .iter()
                .map(TypeAnnot::render)
                .collect::<Vec<_>>()
                .join(", ");
            format!("{}<{}>", self.name.name, args)
        }
    }

    pub fn from_type_annotation(ty: &TypeAnnot) -> Option<Self> {
        match &ty.kind {
            TypeKind::Ident { name } => Some(TraitRef {
                name: name.clone(),
                args: Vec::new(),
                span: ty.span,
            }),
            TypeKind::App { callee, args } => Some(TraitRef {
                name: callee.clone(),
                args: args.clone(),
                span: ty.span,
            }),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ConductorDslDef {
    pub alias: Ident,
    pub target: Ident,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pipeline: Option<ConductorPipelineSpec>,
    pub tails: Vec<ConductorDslTail>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConductorPipelineSpec {
    pub expr: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConductorDslTail {
    pub stage: Ident,
    pub args: Vec<ConductorArg>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConductorArg {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<Ident>,
    pub value: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConductorChannelRoute {
    pub source: ConductorEndpoint,
    pub target: ConductorEndpoint,
    pub payload: TypeAnnot,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConductorEndpoint {
    pub path: Ident,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConductorExecutionBlock {
    pub body: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConductorMonitoringBlock {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<ConductorMonitorTarget>,
    pub body: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ConductorMonitorTarget {
    Module(Ident),
    Endpoint(ConductorEndpoint),
}

#[derive(Debug, Clone, Serialize)]
pub struct Attribute {
    pub name: Ident,
    pub args: Vec<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
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
    FixityLiteral(FixityKind),
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
    Rec {
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
    Break {
        value: Option<Box<Expr>>,
    },
    Handle {
        handle: HandleExpr,
    },
    EffectBlock {
        body: Box<Expr>,
    },
    Async {
        body: Box<Expr>,
        #[serde(default, skip_serializing_if = "is_false")]
        is_move: bool,
    },
    Await {
        expr: Box<Expr>,
    },
    Continue,
    Block {
        attrs: Vec<Attribute>,
        statements: Vec<Stmt>,
        /// ブロックスコープ内の defer を出現順で保持する。
        defers: Vec<Expr>,
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
    InlineAsm(InlineAsmExpr),
    LlvmIr(LlvmIrExpr),
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
            "^" => BinaryOp::Pow,
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
            BinaryOp::Pow => "^",
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
    pub guard_used_if: bool,
    pub body: Expr,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alias: Option<Ident>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
pub struct HandleExpr {
    pub target: Box<Expr>,
    pub handler: HandlerDecl,
    pub with_keyword: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct EffectCall {
    pub effect: Ident,
    pub argument: Box<Expr>,
}

#[derive(Debug, Clone, Serialize)]
pub struct InlineAsmOutput {
    pub constraint: String,
    pub target: Expr,
}

#[derive(Debug, Clone, Serialize)]
pub struct InlineAsmInput {
    pub constraint: String,
    pub expr: Expr,
}

#[derive(Debug, Clone, Serialize)]
pub struct InlineAsmExpr {
    pub template: String,
    pub outputs: Vec<InlineAsmOutput>,
    pub inputs: Vec<InlineAsmInput>,
    pub clobbers: Vec<String>,
    pub options: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LlvmIrExpr {
    pub result_type: TypeAnnot,
    pub template: String,
    pub inputs: Vec<Expr>,
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
    Set {
        elements: Vec<Expr>,
    },
    Record {
        #[serde(skip_serializing_if = "Option::is_none")]
        type_name: Option<Ident>,
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
            LiteralKind::Set { elements } => format!(
                "set([{}])",
                elements
                    .iter()
                    .map(Expr::render)
                    .collect::<Vec<_>>()
                    .join(", "),
            ),
            LiteralKind::Record { fields, .. } => {
                let entries = fields
                    .iter()
                    .map(|field| format!("{}: {}", field.key.name, field.value.render()))
                    .collect::<Vec<_>>()
                    .join(", ");
                let prefix = match &literal_type_name(self) {
                    Some(type_name) => format!("{} ", type_name),
                    None => String::new(),
                };
                format!("record({}{{{}}})", prefix, entries)
            }
        }
    }
}

fn literal_type_name(literal: &Literal) -> Option<String> {
    match &literal.value {
        LiteralKind::Record { type_name, .. } => type_name.as_ref().map(|ident| ident.name.clone()),
        _ => None,
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
    pub fn render(&self) -> String {
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
            PatternKind::Binding {
                name,
                pattern,
                via_at,
            } => {
                if *via_at {
                    format!("{} @ {}", name.name, pattern.render())
                } else {
                    format!("{} as {}", pattern.render(), name.name)
                }
            }
            PatternKind::Or { variants } => variants
                .iter()
                .map(Pattern::render)
                .collect::<Vec<_>>()
                .join(" | "),
            PatternKind::Slice { elements } => {
                let rendered = elements
                    .iter()
                    .map(|elem| match elem {
                        SlicePatternItem::Element(pat) => pat.render(),
                        SlicePatternItem::Rest { ident: None } => "..".to_string(),
                        SlicePatternItem::Rest { ident: Some(ident) } => {
                            format!("..{}", ident.name)
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("[{}]", rendered)
            }
            PatternKind::Range {
                start,
                end,
                inclusive,
            } => {
                let start_display = start
                    .as_ref()
                    .map(|pat| pat.render())
                    .unwrap_or_else(|| "".to_string());
                let end_display = end
                    .as_ref()
                    .map(|pat| pat.render())
                    .unwrap_or_else(|| "".to_string());
                let eq = if *inclusive { "=" } else { "" };
                format!("{start_display}..{eq}{end_display}")
            }
            PatternKind::Regex {
                pattern,
                string_kind,
            } => match string_kind {
                StringKind::Raw => format!("r\"{pattern}\""),
                StringKind::Multiline => format!("\"\"\"{pattern}\"\"\""),
                StringKind::Normal => format!("\"{pattern}\""),
            },
            PatternKind::ActivePattern {
                name,
                is_partial,
                argument,
            } => {
                let head = if *is_partial {
                    format!("(|{}|_|)", name.name)
                } else {
                    format!("(|{}|)", name.name)
                };
                match argument {
                    Some(arg) => format!("{head} {}", arg.render()),
                    None => head,
                }
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
    Binding {
        name: Ident,
        pattern: Box<Pattern>,
        via_at: bool,
    },
    Or {
        variants: Vec<Pattern>,
    },
    Slice {
        elements: Vec<SlicePatternItem>,
    },
    Range {
        start: Option<Box<Pattern>>,
        end: Option<Box<Pattern>>,
        inclusive: bool,
    },
    Regex {
        pattern: String,
        string_kind: StringKind,
    },
    ActivePattern {
        name: Ident,
        is_partial: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        argument: Option<Box<Pattern>>,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct PatternRecordField {
    pub key: Ident,
    pub value: Option<Box<Pattern>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SlicePatternItem {
    Element(Pattern),
    Rest {
        #[serde(skip_serializing_if = "Option::is_none")]
        ident: Option<Ident>,
    },
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
pub struct QualifiedName {
    pub segments: Vec<Ident>,
    pub span: Span,
}

impl QualifiedName {
    pub fn render(&self) -> String {
        self.segments
            .iter()
            .map(|segment| segment.name.clone())
            .collect::<Vec<_>>()
            .join(".")
    }

    pub fn to_ident(&self) -> Ident {
        let mut iter = self.segments.iter();
        let mut name = match iter.next() {
            Some(segment) => segment.name.clone(),
            None => String::new(),
        };
        for segment in iter {
            name.push_str("::");
            name.push_str(&segment.name);
        }
        Ident {
            name,
            span: self.span,
        }
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

impl ModulePath {
    pub fn render(&self) -> String {
        match self {
            ModulePath::Root { segments } => {
                let mut parts = String::from("::");
                parts.push_str(
                    &segments
                        .iter()
                        .map(|segment| segment.name.clone())
                        .collect::<Vec<_>>()
                        .join("."),
                );
                parts
            }
            ModulePath::Relative { head, segments } => {
                let mut parts = Vec::new();
                parts.push(head.render());
                parts.extend(segments.iter().map(|segment| segment.name.clone()));
                parts.join(".")
            }
        }
    }
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

impl RelativeHead {
    fn render(&self) -> String {
        match self {
            RelativeHead::Self_ => "self".to_string(),
            RelativeHead::Super(depth) => {
                if *depth <= 1 {
                    "super".to_string()
                } else {
                    std::iter::repeat("super")
                        .take(*depth as usize)
                        .collect::<Vec<_>>()
                        .join(".")
                }
            }
            RelativeHead::PlainIdent(ident) => ident.name.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Function {
    pub name: Ident,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub qualified_name: Option<QualifiedName>,
    pub visibility: Visibility,
    pub generics: Vec<Ident>,
    pub params: Vec<Param>,
    pub body: Expr,
    pub ret_type: Option<TypeAnnot>,
    pub where_clause: Vec<WherePredicate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effect: Option<EffectAnnotation>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub is_async: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub is_unsafe: bool,
    pub span: Span,
    pub attrs: Vec<Attribute>,
}

impl Function {
    pub fn binding_key(&self) -> String {
        self.qualified_name
            .as_ref()
            .map(QualifiedName::render)
            .unwrap_or_else(|| self.name.name.clone())
    }

    pub fn render(&self) -> String {
        let params = self
            .params
            .iter()
            .map(Param::render)
            .collect::<Vec<_>>()
            .join(", ");
        let generics = if self.generics.is_empty() {
            String::new()
        } else {
            format!(
                "<{}>",
                self.generics
                    .iter()
                    .map(|ident| ident.name.clone())
                    .collect::<Vec<_>>()
                    .join(", "),
            )
        };
        let ret = self
            .ret_type
            .as_ref()
            .map(|ty| format!(" -> {}", ty.render()))
            .unwrap_or_default();
        let where_clause = if self.where_clause.is_empty() {
            String::new()
        } else {
            format!(
                " where {}",
                self.where_clause
                    .iter()
                    .map(WherePredicate::render)
                    .collect::<Vec<_>>()
                    .join(", "),
            )
        };
        let effect = self
            .effect
            .as_ref()
            .map(|annot| format!(" !{{{}}}", annot.render()))
            .unwrap_or_default();
        let visibility = match self.visibility {
            Visibility::Public => "pub ",
            Visibility::Private => "",
        };
        let async_prefix = if self.is_async { "async " } else { "" };
        let unsafe_prefix = if self.is_unsafe { "unsafe " } else { "" };
        format!(
            "{visibility}{async_prefix}{unsafe_prefix}fn {}{}({}){}{}{} = {}",
            self.name.name,
            generics,
            params,
            ret,
            where_clause,
            effect,
            self.body.render()
        )
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ActivePatternDecl {
    pub name: Ident,
    pub is_partial: bool,
    pub params: Vec<Param>,
    pub body: Expr,
    pub span: Span,
    pub attrs: Vec<Attribute>,
    pub visibility: Visibility,
}

impl ActivePatternDecl {
    pub fn render(&self) -> String {
        let head = if self.is_partial {
            format!("(|{}|_|)", self.name.name)
        } else {
            format!("(|{}|)", self.name.name)
        };
        let params = self
            .params
            .iter()
            .map(Param::render)
            .collect::<Vec<_>>()
            .join(", ");
        let mut text = String::new();
        if let Visibility::Public = self.visibility {
            text.push_str("pub ");
        }
        text.push_str("pattern ");
        text.push_str(&head);
        text.push('(');
        text.push_str(&params);
        text.push(')');
        text.push_str(" = ");
        text.push_str(&self.body.render());
        text
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct MacroDecl {
    pub name: Ident,
    pub params: Vec<Param>,
    pub body: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
pub struct ActorSpecDecl {
    pub name: Ident,
    pub params: Vec<Param>,
    pub body: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
pub struct Param {
    pub pattern: Pattern,
    pub type_annotation: Option<TypeAnnot>,
    pub default: Option<Expr>,
    pub span: Span,
}

impl Param {
    fn render(&self) -> String {
        let mut text = self.pattern.render();
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotation_kind: Option<AnnotationKind>,
    pub span: Span,
}

impl TypeAnnot {
    pub fn render(&self) -> String {
        self.kind.render()
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TypeUnionVariant {
    Type {
        ty: TypeAnnot,
    },
    Variant {
        name: Ident,
        #[serde(skip_serializing_if = "Option::is_none")]
        payload: Option<VariantPayload>,
        span: Span,
    },
}

impl TypeUnionVariant {
    fn render(&self) -> String {
        match self {
            TypeUnionVariant::Type { ty } => ty.render(),
            TypeUnionVariant::Variant {
                name,
                payload: Some(payload),
                ..
            } => match payload {
                VariantPayload::Record { .. } => format!("{} {}", name.name, payload.render()),
                VariantPayload::Tuple { .. } => format!("{}{}", name.name, payload.render()),
            },
            TypeUnionVariant::Variant { name, .. } => name.name.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AnnotationKind {
    Return,
    HandlerResume,
    Operation,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TypeKind {
    Ident {
        name: Ident,
    },
    Literal {
        value: TypeLiteral,
    },
    Array {
        element: Box<TypeAnnot>,
        length: TypeArrayLength,
    },
    App {
        callee: Ident,
        args: Vec<TypeAnnot>,
    },
    Tuple {
        elements: Vec<TypeTupleElement>,
    },
    Record {
        fields: Vec<TypeRecordField>,
    },
    Slice {
        element: Box<TypeAnnot>,
    },
    Ref {
        target: Box<TypeAnnot>,
        mutable: bool,
    },
    Fn {
        params: Vec<TypeAnnot>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        param_labels: Vec<Option<Ident>>,
        ret: Box<TypeAnnot>,
    },
    Union {
        variants: Vec<TypeUnionVariant>,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct TypeArrayLength {
    pub value: i64,
    pub raw: String,
    pub span: Span,
}

impl TypeKind {
    fn render(&self) -> String {
        match self {
            TypeKind::Ident { name } => name.name.clone(),
            TypeKind::Literal { value } => value.render(),
            TypeKind::Array { element, length } => {
                format!("[{}; {}]", element.render(), length.raw)
            }
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
                    .map(TypeTupleElement::render)
                    .collect::<Vec<_>>()
                    .join(", "),
            ),
            TypeKind::Record { fields } => {
                let entries = fields
                    .iter()
                    .map(TypeRecordField::render)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("record({})", entries)
            }
            TypeKind::Slice { element } => format!("[{}]", element.render()),
            TypeKind::Ref { target, mutable } => {
                if *mutable {
                    format!("&mut {}", target.render())
                } else {
                    format!("&{}", target.render())
                }
            }
            TypeKind::Fn {
                params,
                param_labels,
                ret,
            } => {
                let mut rendered_params = Vec::with_capacity(params.len());
                for (index, param) in params.iter().enumerate() {
                    let label = param_labels
                        .get(index)
                        .and_then(|label| label.as_ref())
                        .map(|ident| ident.name.as_str());
                    if let Some(label) = label {
                        rendered_params.push(format!("{label}: {}", param.render()));
                    } else {
                        rendered_params.push(param.render());
                    }
                }
                format!("fn({}) -> {}", rendered_params.join(", "), ret.render())
            }
            TypeKind::Union { variants } => variants
                .iter()
                .map(TypeUnionVariant::render)
                .collect::<Vec<_>>()
                .join(" | "),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TypeLiteral {
    Int { value: i64, raw: String },
    String { value: String },
}

impl TypeLiteral {
    fn render(&self) -> String {
        match self {
            TypeLiteral::Int { raw, .. } => raw.clone(),
            TypeLiteral::String { value } => format!("\"{value}\""),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TypeTupleElement {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<Ident>,
    pub ty: TypeAnnot,
}

impl TypeTupleElement {
    fn render(&self) -> String {
        match &self.label {
            Some(label) => format!("{}: {}", label.name, self.ty.render()),
            None => self.ty.render(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TypeRecordField {
    pub label: Ident,
    pub ty: TypeAnnot,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_expr: Option<Expr>,
}

impl TypeRecordField {
    fn render(&self) -> String {
        let mut rendered = format!("{}: {}", self.label.name, self.ty.render());
        if let Some(default_expr) = &self.default_expr {
            rendered.push_str(&format!(" = {}", default_expr.render()));
        }
        rendered
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

    pub fn fixity(kind: FixityKind, span: Span) -> Self {
        Self {
            span,
            kind: ExprKind::FixityLiteral(kind),
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

    pub fn float(raw: impl Into<String>, span: Span) -> Self {
        Self::literal(
            Literal {
                value: LiteralKind::Float { raw: raw.into() },
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

    pub fn pipe(left: Expr, right: Expr, span: Span) -> Self {
        Self {
            span,
            kind: ExprKind::Pipe {
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

    pub fn block(statements: Vec<Stmt>, span: Span) -> Self {
        Self::block_with_attrs(statements, Vec::new(), span)
    }

    pub fn block_with_attrs(statements: Vec<Stmt>, attrs: Vec<Attribute>, span: Span) -> Self {
        let defers = collect_block_defers(&statements);
        Self {
            span,
            kind: ExprKind::Block {
                attrs,
                statements,
                defers,
            },
        }
    }

    pub fn assign(target: Expr, value: Expr, span: Span) -> Self {
        Self {
            span,
            kind: ExprKind::Assign {
                target: Box::new(target),
                value: Box::new(value),
            },
        }
    }

    pub fn field_access(target: Expr, field: Ident, span: Span) -> Self {
        Self {
            span,
            kind: ExprKind::FieldAccess {
                target: Box::new(target),
                field,
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
            ExprKind::Rec { expr } => format!("rec {}", expr.render()),
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
            ExprKind::Block { statements, .. } => format!(
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
            ExprKind::Break { value } => format!(
                "break {}",
                value
                    .as_ref()
                    .map(|expr| expr.render())
                    .unwrap_or_else(|| "unit".to_string())
            ),
            ExprKind::Continue => "continue".to_string(),
            ExprKind::EffectBlock { body } => format!("effect {}", body.render()),
            ExprKind::Async { body, is_move } => {
                if *is_move {
                    format!("async move {}", body.render())
                } else {
                    format!("async {}", body.render())
                }
            }
            ExprKind::Await { expr } => format!("await {}", expr.render()),
            other => format!("expr({:?})", other),
        }
    }
}

fn collect_block_defers(statements: &[Stmt]) -> Vec<Expr> {
    let mut defers = Vec::new();
    for stmt in statements {
        if let StmtKind::Defer { expr } = &stmt.kind {
            defers.push((**expr).clone());
        }
    }
    defers
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
