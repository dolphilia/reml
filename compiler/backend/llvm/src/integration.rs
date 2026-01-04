use crate::codegen::{
    summarize_pattern, ActivePatternKind, CodegenContext, GeneratedFunction, MatchArmLowering,
    MatchLoweringPlan, MirActivePatternCall, MirExpr, MirExprKind, MirFunction,
    MirInlineAsmInput, MirInlineAsmOutput, MirJumpTarget, MirLambdaCapture, MirLambdaParam,
    MirMatchArm, MirPattern, MirPatternKind, MirPatternRecordField, MirSlicePattern, MirSliceRest,
    MirStmt, MirStmtKind, PatternLowering,
};
use crate::ffi_lowering::FfiCallSignature;
use crate::target_machine::{
    CodeModel, DataLayoutSpec, OptimizationLevel, RelocModel, TargetMachine, TargetMachineBuilder,
    Triple, WindowsToolchainConfig,
};
use crate::type_mapping::RemlType;
use crate::verify::Verifier;
use serde::Deserialize;
use serde_json::Value;
use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::{fmt, fs::File, io, path::Path};

/// 生成関数の差分ログ用レコード。
#[derive(Clone, Debug)]
pub struct BackendFunctionRecord {
    pub name: String,
    pub return_layout: String,
    pub calling_conv: String,
    pub attributes: Vec<String>,
    pub lowered_calls: Vec<String>,
    pub branch_plans: Vec<String>,
    pub basic_blocks: Vec<String>,
    pub llvm_blocks: Vec<String>,
    pub llvm_ir: String,
}

impl BackendFunctionRecord {
    fn from_generated(func: &GeneratedFunction) -> Self {
        Self {
            name: func.name.clone(),
            return_layout: func.layout.description.clone(),
            calling_conv: func.calling_conv.clone(),
            attributes: func.attributes.clone(),
            lowered_calls: func
                .lowered_calls
                .iter()
                .map(|call| call.describe())
                .collect(),
            branch_plans: func.branch_plans.clone(),
            basic_blocks: func
                .basic_blocks
                .iter()
                .map(|block| block.describe_llvm())
                .collect(),
            llvm_blocks: func
                .llvm_blocks
                .iter()
                .map(|block| block.describe())
                .collect(),
            llvm_ir: func.llvm_ir.clone(),
        }
    }
}

/// W3 デモ用の差分スナップショット。
#[derive(Clone, Debug)]
pub struct BackendDiffSnapshot {
    pub module_name: String,
    pub target_triple: String,
    pub backend_abi: String,
    pub data_layout: String,
    pub windows_toolchain: Option<String>,
    pub functions: Vec<BackendFunctionRecord>,
    pub diagnostics: Vec<String>,
    pub audit_entries: Vec<String>,
    pub bridge_metadata: Vec<String>,
    pub passed: bool,
}

impl BackendDiffSnapshot {
    fn quote(value: &str) -> String {
        value.replace('\\', "\\\\").replace('"', "\\\"")
    }

    fn array_of_strings(values: &[String], indent: &str) -> String {
        let mut buf = String::new();
        buf.push('[');
        if !values.is_empty() {
            buf.push('\n');
            for (idx, value) in values.iter().enumerate() {
                buf.push_str(indent);
                buf.push_str("  \"");
                buf.push_str(&Self::quote(value));
                buf.push('"');
                if idx + 1 != values.len() {
                    buf.push(',');
                }
                buf.push('\n');
            }
            buf.push_str(indent);
        }
        buf.push(']');
        buf
    }

    fn function_record_json(&self, record: &BackendFunctionRecord, indent: &str) -> String {
        let mut buf = String::new();
        buf.push_str("{\n");
        buf.push_str(indent);
        buf.push_str("  \"name\": \"");
        buf.push_str(&Self::quote(&record.name));
        buf.push_str("\",\n");
        buf.push_str(indent);
        buf.push_str("  \"return_layout\": \"");
        buf.push_str(&Self::quote(&record.return_layout));
        buf.push_str("\",\n");
        buf.push_str(indent);
        buf.push_str("  \"calling_conv\": \"");
        buf.push_str(&Self::quote(&record.calling_conv));
        buf.push_str("\",\n");
        buf.push_str(indent);
        buf.push_str("  \"attributes\": ");
        buf.push_str(&Self::array_of_strings(
            &record.attributes,
            &(indent.to_string() + "  "),
        ));
        buf.push_str(",\n");
        buf.push_str(indent);
        buf.push_str("  \"ffi_calls\": ");
        buf.push_str(&Self::array_of_strings(
            &record.lowered_calls,
            &(indent.to_string() + "  "),
        ));
        buf.push_str(",\n");
        buf.push_str(indent);
        buf.push_str("  \"match_branches\": ");
        buf.push_str(&Self::array_of_strings(
            &record.branch_plans,
            &(indent.to_string() + "  "),
        ));
        buf.push_str(",\n");
        buf.push_str(indent);
        buf.push_str("  \"basic_blocks\": ");
        buf.push_str(&Self::array_of_strings(
            &record.basic_blocks,
            &(indent.to_string() + "  "),
        ));
        buf.push_str(",\n");
        buf.push_str(indent);
        buf.push_str("  \"llvm_blocks\": ");
        buf.push_str(&Self::array_of_strings(
            &record.llvm_blocks,
            &(indent.to_string() + "  "),
        ));
        buf.push_str(",\n");
        buf.push_str(indent);
        buf.push_str("  \"llvm_ir\": \"");
        buf.push_str(&Self::quote(&record.llvm_ir));
        buf.push_str("\"\n");
        buf.push('\n');
        buf.push_str(indent);
        buf.push('}');
        buf
    }

    /// JSON 形式のログを返す。
    pub fn to_pretty_json(&self) -> String {
        let mut buf = String::new();
        buf.push_str("{\n");
        buf.push_str("  \"module\": \"");
        buf.push_str(&Self::quote(&self.module_name));
        buf.push_str("\",\n");
        buf.push_str("  \"target_triple\": \"");
        buf.push_str(&Self::quote(&self.target_triple));
        buf.push_str("\",\n");
        buf.push_str("  \"backend_abi\": \"");
        buf.push_str(&Self::quote(&self.backend_abi));
        buf.push_str("\",\n");
        buf.push_str("  \"data_layout\": \"");
        buf.push_str(&Self::quote(&self.data_layout));
        buf.push_str("\",\n");
        if let Some(toolchain) = &self.windows_toolchain {
            buf.push_str("  \"windows_toolchain\": \"");
            buf.push_str(&Self::quote(toolchain));
            buf.push_str("\",\n");
        }
        buf.push_str("  \"functions\": [\n");
        for (index, function) in self.functions.iter().enumerate() {
            buf.push_str("    ");
            buf.push_str(&self.function_record_json(function, "    "));
            if index + 1 != self.functions.len() {
                buf.push(',');
            }
            buf.push('\n');
        }
        buf.push_str("  ],\n");
        buf.push_str("  \"diagnostics\": ");
        buf.push_str(&Self::array_of_strings(&self.diagnostics, "  "));
        buf.push_str(",\n");
        buf.push_str("  \"audit_entries\": ");
        buf.push_str(&Self::array_of_strings(&self.audit_entries, "  "));
        buf.push_str(",\n");
        buf.push_str("  \"bridge_metadata\": ");
        buf.push_str(&Self::array_of_strings(&self.bridge_metadata, "  "));
        buf.push_str(",\n");
        buf.push_str("  \"passed\": ");
        buf.push_str(if self.passed { "true" } else { "false" });
        buf.push('\n');
        buf.push('}');
        buf
    }
}

/// モジュール全体と MIR 関数の構造を JSON から読み込む。
#[derive(Debug, Deserialize)]
struct MirModuleSpec {
    #[serde(default)]
    schema_version: Option<String>,
    module: Option<String>,
    #[serde(default)]
    metadata: Vec<String>,
    #[serde(default)]
    runtime_symbols: Vec<String>,
    functions: Vec<MirFunctionJson>,
    #[serde(default)]
    externs: Vec<MirExternJson>,
    #[serde(default)]
    active_patterns: Vec<Value>,
    #[serde(default)]
    dict_refs: Vec<DictRefJson>,
    #[serde(default)]
    impls: BTreeMap<String, MirImplSpecJson>,
    #[serde(default)]
    qualified_calls: BTreeMap<String, MirQualifiedCallJson>,
    #[serde(default)]
    impl_registry_duplicates: Vec<String>,
    #[serde(default)]
    impl_registry_unresolved: Vec<String>,
}

impl MirModuleSpec {
    fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, MirSnapshotError> {
        let file = File::open(path)?;
        let spec = serde_json::from_reader(file)?;
        Ok(spec)
    }

    fn collect_todo_diagnostics(&self) -> Vec<String> {
        let mut diagnostics = Vec::new();
        let mut call_keys = BTreeSet::new();
        for function in &self.functions {
            if function.is_async {
                diagnostics.push(format!(
                    "Backend.backend.todo.async_function: function={}",
                    function.name
                ));
            }
            for expr in &function.exprs {
                if matches!(expr.kind, MirExprKindJson::Call { .. }) {
                    call_keys.insert(format!("{}#{}", function.name, expr.id));
                }
                match &expr.kind {
                    MirExprKindJson::EffectBlock { .. } => diagnostics.push(format!(
                        "Backend.backend.todo.effect_block: function={} expr_id={}",
                        function.name, expr.id
                    )),
                    MirExprKindJson::Unsafe { .. } => diagnostics.push(format!(
                        "Backend.backend.todo.unsafe_block: function={} expr_id={}",
                        function.name, expr.id
                    )),
                    MirExprKindJson::Async { .. } => diagnostics.push(format!(
                        "Backend.backend.todo.async: function={} expr_id={}",
                        function.name, expr.id
                    )),
                    MirExprKindJson::Await { .. } => diagnostics.push(format!(
                        "Backend.backend.todo.await: function={} expr_id={}",
                        function.name, expr.id
                    )),
                    _ => {}
                }
            }
        }
        for extern_decl in &self.externs {
            let name = extern_decl.name.as_deref().unwrap_or("<none>");
            let abi = extern_decl.abi.as_deref().unwrap_or("<none>");
            let symbol = extern_decl.symbol.as_deref().unwrap_or("<none>");
            diagnostics.push(format!(
                "Backend.backend.todo.extern_decl: name={name} abi={abi} symbol={symbol}"
            ));
        }
        for key in &call_keys {
            if !self.qualified_calls.contains_key(key) {
                diagnostics.push(format!(
                    "Backend.backend.todo.qualified_call_missing: key={key}"
                ));
            }
        }
        for (key, call) in &self.qualified_calls {
            let owner_value = call.owner.as_deref().unwrap_or("");
            let owner = if owner_value.is_empty() {
                "<none>"
            } else {
                owner_value
            };
            let name_value = call.name.as_deref().unwrap_or("");
            let name = if name_value.is_empty() {
                "<none>"
            } else {
                name_value
            };
            let kind = call.kind.as_str();
            let impl_id = call.impl_id.as_deref().unwrap_or("<none>");
            if call.kind == MirQualifiedCallKindJson::Unknown || owner_value.is_empty() {
                diagnostics.push(format!(
                    "Backend.backend.todo.qualified_call_unresolved: key={key} owner={owner} name={name} kind={kind} impl_id={impl_id}"
                ));
                continue;
            }
            if call.kind == MirQualifiedCallKindJson::TraitMethod && call.impl_id.is_none() {
                diagnostics.push(format!(
                    "Backend.backend.todo.trait_impl_unresolved: key={key} owner={owner} name={name} kind={kind}"
                ));
            }
        }
        for (impl_id, impl_spec) in &self.impls {
            if impl_spec.trait_name.is_some() && impl_spec.associated_types.is_empty() {
                diagnostics.push(format!(
                    "Backend.backend.todo.impl.associated_types_missing: impl_id={impl_id}"
                ));
            }
        }
        for duplicate in &self.impl_registry_duplicates {
            diagnostics.push(format!(
                "Backend.backend.todo.impl_registry.duplicate: impl_id={duplicate}"
            ));
        }
        for unresolved in &self.impl_registry_unresolved {
            diagnostics.push(format!(
                "Backend.backend.todo.impl_registry.unresolved: impl_id={unresolved}"
            ));
        }
        diagnostics.extend(self.collect_type_diagnostics());
        diagnostics
    }

    fn collect_type_diagnostics(&self) -> Vec<String> {
        let mut diagnostics = Vec::new();
        for function in &self.functions {
            for param in &function.params {
                let token = param.type_token();
                parse_reml_type_with_diagnostics(&token, Some(&mut diagnostics));
            }
            if let Some(ret) = function.return_type.as_deref() {
                parse_reml_type_with_diagnostics(ret, Some(&mut diagnostics));
            }
            for ffi in &function.ffi_calls {
                for arg in &ffi.args {
                    parse_reml_type_with_diagnostics(arg, Some(&mut diagnostics));
                }
                if let Some(ret) = ffi.ret.as_deref() {
                    parse_reml_type_with_diagnostics(ret, Some(&mut diagnostics));
                }
            }
        }
        diagnostics
    }

    fn into_functions(self) -> Vec<MirFunction> {
        self.functions
            .into_iter()
            .map(MirFunctionJson::into_mir)
            .collect()
    }
}

fn default_calling_conv() -> String {
    "ccc".into()
}

#[derive(Debug, Deserialize)]
struct DictRefJson {
    #[serde(default)]
    id: Option<usize>,
    #[serde(default)]
    impl_id: Option<String>,
    #[serde(default)]
    requirements: Vec<String>,
    #[serde(default)]
    ty: Option<String>,
    #[serde(default)]
    span: Option<MirSpanJson>,
}

#[derive(Debug, Deserialize)]
struct MirImplSpecJson {
    #[serde(rename = "trait", default)]
    trait_name: Option<String>,
    #[serde(default)]
    target: Option<String>,
    #[serde(default)]
    associated_types: Vec<MirAssociatedTypeJson>,
    #[serde(default)]
    methods: Vec<String>,
    #[serde(default)]
    span: Option<MirSpanJson>,
}

#[derive(Debug, Deserialize)]
struct MirAssociatedTypeJson {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    ty: Option<String>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum MirQualifiedCallKindJson {
    TypeMethod,
    TypeAssoc,
    TraitMethod,
    #[serde(other)]
    Unknown,
}

impl MirQualifiedCallKindJson {
    fn as_str(&self) -> &'static str {
        match self {
            MirQualifiedCallKindJson::TypeMethod => "type_method",
            MirQualifiedCallKindJson::TypeAssoc => "type_assoc",
            MirQualifiedCallKindJson::TraitMethod => "trait_method",
            MirQualifiedCallKindJson::Unknown => "unknown",
        }
    }
}

impl Default for MirQualifiedCallKindJson {
    fn default() -> Self {
        MirQualifiedCallKindJson::Unknown
    }
}

#[derive(Debug, Deserialize)]
struct MirQualifiedCallJson {
    #[serde(default)]
    kind: MirQualifiedCallKindJson,
    #[serde(default)]
    owner: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    impl_id: Option<String>,
    #[serde(default)]
    receiver_ty: Option<String>,
    #[serde(default)]
    impl_candidates: Vec<String>,
    #[serde(default)]
    span: Option<MirSpanJson>,
}

#[derive(Debug, Deserialize)]
struct MirSpanJson {
    start: u32,
    end: u32,
}

/// 単体 MIR 関数の JSON 表現。
#[derive(Debug, Deserialize)]
struct MirFunctionJson {
    name: String,
    #[serde(default = "default_calling_conv")]
    calling_conv: String,
    #[serde(default)]
    params: Vec<MirParamJson>,
    #[serde(alias = "return")]
    return_type: Option<String>,
    #[serde(default)]
    attributes: Vec<String>,
    #[serde(default)]
    is_async: bool,
    #[serde(default)]
    is_unsafe: bool,
    #[serde(default)]
    varargs: bool,
    #[serde(default)]
    ffi_calls: Vec<FfiCallJson>,
    #[serde(default)]
    exprs: Vec<MirExprJson>,
    #[serde(default)]
    body: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum MirParamJson {
    Bare(String),
    Detailed {
        #[serde(default)]
        ty: Option<String>,
    },
}

impl MirParamJson {
    fn into_type_token(self) -> String {
        match self {
            MirParamJson::Bare(token) => token,
            MirParamJson::Detailed { ty } => ty.unwrap_or_else(|| "pointer".into()),
        }
    }

    fn type_token(&self) -> Cow<'_, str> {
        match self {
            MirParamJson::Bare(token) => Cow::Borrowed(token),
            MirParamJson::Detailed { ty } => {
                Cow::Borrowed(ty.as_deref().unwrap_or("pointer"))
            }
        }
    }
}

impl MirFunctionJson {
    fn into_mir(self) -> MirFunction {
        let mut builder = MirFunction::new(self.name, self.calling_conv);
        for param in self.params {
            builder = builder.with_param(parse_reml_type(&param.into_type_token()));
        }
        if let Some(ret) = self.return_type {
            builder = builder.with_return(parse_reml_type(&ret));
        }

        for attr in self.attributes {
            builder = builder.with_attribute(attr);
        }
        for ffi in self.ffi_calls {
            builder = builder.with_ffi_call(ffi.into_signature());
        }
        let exprs = convert_exprs(self.exprs);
        builder.match_plans = extract_match_plans(&exprs);
        builder = builder.with_exprs(self.body, exprs);
        builder
    }
}

/// FFI 呼び出しの JSON 抽象。
#[derive(Debug, Deserialize)]
struct FfiCallJson {
    name: String,
    calling_conv: String,
    #[serde(default)]
    args: Vec<String>,
    #[serde(alias = "return")]
    ret: Option<String>,
    #[serde(default)]
    variadic: bool,
}

impl FfiCallJson {
    fn into_signature(self) -> FfiCallSignature {
        FfiCallSignature {
            name: self.name,
            calling_conv: self.calling_conv,
            args: self
                .args
                .into_iter()
                .map(|arg| parse_reml_type(&arg))
                .collect(),
            ret: self.ret.map(|ret| parse_reml_type(&ret)),
            variadic: self.variadic,
        }
    }
}

/// フロントエンド MIR から Match/Pattern 情報を抽出する簡易モデル。
#[derive(Debug, Deserialize)]
struct MirExprJson {
    id: usize,
    #[serde(default)]
    ty: String,
    kind: MirExprKindJson,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum MirExprKindJson {
    Block {
        #[serde(default)]
        statements: Vec<MirStmtJson>,
        #[serde(default)]
        tail: Option<usize>,
        #[serde(default)]
        defers: Vec<usize>,
        #[serde(default)]
        defer_lifo: Vec<usize>,
    },
    Return {
        #[serde(default)]
        value: Option<usize>,
    },
    Propagate {
        expr: usize,
    },
    Panic {
        #[serde(default)]
        argument: Option<usize>,
    },
    Lambda {
        #[serde(default)]
        params: Vec<MirLambdaParamJson>,
        body: usize,
        #[serde(default)]
        captures: Vec<MirLambdaCaptureJson>,
    },
    Rec {
        target: usize,
        #[serde(default)]
        ident: Option<Value>,
    },
    Match {
        target: usize,
        #[serde(default)]
        arms: Vec<MirMatchArmJson>,
        #[serde(default)]
        lowering: Option<MatchLoweringPlanJson>,
    },
    Call {
        callee: usize,
        #[serde(default)]
        args: Vec<usize>,
    },
    Binary {
        #[serde(default)]
        operator: String,
        left: usize,
        right: usize,
    },
    IfElse {
        condition: usize,
        then_branch: usize,
        else_branch: usize,
    },
    PerformCall {
        call: MirEffectCallJson,
    },
    EffectBlock {
        body: usize,
    },
    Async {
        body: usize,
        #[serde(default)]
        is_move: bool,
    },
    Await {
        expr: usize,
    },
    Unsafe {
        body: usize,
    },
    InlineAsm {
        template: String,
        #[serde(default)]
        outputs: Vec<MirInlineAsmOutputJson>,
        #[serde(default)]
        inputs: Vec<MirInlineAsmInputJson>,
        #[serde(default)]
        clobbers: Vec<String>,
        #[serde(default)]
        options: Vec<String>,
    },
    LlvmIr {
        #[serde(default)]
        result_type: String,
        template: String,
        #[serde(default)]
        inputs: Vec<usize>,
    },
    FieldAccess {
        target: usize,
        field: String,
    },
    Index {
        target: usize,
        index: usize,
    },
    Identifier {
        ident: Value,
    },
    Literal {
        value: Value,
    },
    Unknown,
}

#[derive(Debug, Deserialize)]
struct MirExternJson {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    abi: Option<String>,
    #[serde(default)]
    symbol: Option<String>,
    #[serde(default)]
    span: Option<MirSpanJson>,
}

#[derive(Debug, Deserialize)]
struct MirLambdaParamJson {
    #[serde(default)]
    name: String,
    #[serde(default)]
    ty: String,
}

#[derive(Debug, Deserialize)]
struct MirLambdaCaptureJson {
    #[serde(default)]
    name: String,
    #[serde(default)]
    mutable: bool,
}

#[derive(Debug, Deserialize)]
struct MirStmtJson {
    kind: MirStmtKindJson,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum MirStmtKindJson {
    Let {
        pattern: MirPatternJson,
        value: usize,
        #[serde(default)]
        mutable: bool,
    },
    Expr {
        expr: usize,
    },
    Assign {
        target: usize,
        value: usize,
    },
    Defer {
        expr: usize,
    },
}

#[derive(Debug, Deserialize)]
struct MirEffectCallJson {
    effect: Value,
    argument: usize,
}

#[derive(Debug, Deserialize)]
struct MirInlineAsmOutputJson {
    constraint: String,
    target: usize,
}

#[derive(Debug, Deserialize)]
struct MirInlineAsmInputJson {
    constraint: String,
    expr: usize,
}

#[derive(Debug, Deserialize)]
struct MirMatchArmJson {
    pattern: MirPatternJson,
    #[serde(default)]
    guard: Option<usize>,
    #[serde(default)]
    alias: Option<String>,
    body: usize,
}

#[derive(Debug, Deserialize)]
struct MirPatternJson {
    kind: MirPatternKindSpec,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum MirPatternKindSpec {
    Tagged(MirPatternKindJson),
    Active(MirActivePatternCallJson),
    Other(Value),
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum MirPatternKindJson {
    Wildcard,
    Var {
        name: String,
    },
    Literal(Value),
    Tuple {
        elements: Vec<MirPatternJson>,
    },
    Record {
        fields: Vec<MirPatternRecordFieldJson>,
        #[serde(default)]
        has_rest: bool,
    },
    Constructor {
        name: String,
        args: Vec<MirPatternJson>,
    },
    Binding {
        name: String,
        pattern: Box<MirPatternJson>,
        #[serde(default)]
        via_at: bool,
    },
    Or {
        variants: Vec<MirPatternJson>,
    },
    Slice(MirSlicePatternJson),
    Range {
        #[serde(default)]
        start: Option<Box<MirPatternJson>>,
        #[serde(default)]
        end: Option<Box<MirPatternJson>>,
        inclusive: bool,
    },
    Regex {
        pattern: String,
    },
}

#[derive(Debug, Deserialize)]
struct MirPatternRecordFieldJson {
    key: String,
    #[serde(default)]
    value: Option<Box<MirPatternJson>>,
}

#[derive(Debug, Deserialize)]
struct MirSlicePatternJson {
    #[serde(default)]
    head: Vec<MirPatternJson>,
    #[serde(default)]
    rest: Option<MirSliceRestJson>,
    #[serde(default)]
    tail: Vec<MirPatternJson>,
}

#[derive(Debug, Deserialize)]
struct MirSliceRestJson {
    #[serde(default)]
    binding: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MirActivePatternCallJson {
    #[serde(default)]
    kind: MirActivePatternKindJson,
    name: String,
    #[serde(default)]
    argument: Option<Box<MirPatternJson>>,
    #[serde(default)]
    input_binding: Option<String>,
    #[serde(default)]
    miss_target: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(untagged)]
enum MirActivePatternKindJson {
    Tagged {
        kind: String,
    },
    Direct(String),
    #[default]
    Unknown,
}

impl MirActivePatternKindJson {
    fn as_str(&self) -> Option<&str> {
        match self {
            MirActivePatternKindJson::Tagged { kind } => Some(kind.as_str()),
            MirActivePatternKindJson::Direct(kind) => Some(kind.as_str()),
            MirActivePatternKindJson::Unknown => None,
        }
    }
}

#[derive(Debug, Deserialize)]
struct MatchLoweringPlanJson {
    #[serde(default)]
    owner: Option<String>,
    #[serde(default)]
    target_type: Option<String>,
    #[serde(default)]
    arm_count: Option<usize>,
    #[serde(default)]
    arms: Vec<MatchArmLoweringJson>,
}

#[derive(Debug, Deserialize)]
struct MatchArmLoweringJson {
    pattern: PatternLoweringJson,
    #[serde(default)]
    has_guard: bool,
    #[serde(default)]
    alias: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PatternLoweringJson {
    label: String,
    #[serde(default)]
    miss_on_none: bool,
    #[serde(default)]
    always_matches: bool,
    #[serde(default)]
    children: Vec<PatternLoweringJson>,
}

fn convert_exprs(exprs: Vec<MirExprJson>) -> Vec<MirExpr> {
    exprs
        .into_iter()
        .map(|expr| MirExpr {
            id: expr.id,
            ty: expr.ty,
            kind: convert_expr_kind(expr.kind),
        })
        .collect()
}

fn convert_expr_kind(kind: MirExprKindJson) -> MirExprKind {
    match kind {
        MirExprKindJson::Block {
            statements,
            tail,
            defers,
            defer_lifo,
        } => MirExprKind::Block {
            statements: statements.into_iter().map(convert_stmt).collect(),
            tail,
            defers,
            defer_lifo,
        },
        MirExprKindJson::Return { value } => MirExprKind::Return { value },
        MirExprKindJson::Propagate { expr } => MirExprKind::Propagate { expr },
        MirExprKindJson::Panic { argument } => MirExprKind::Panic { argument },
        MirExprKindJson::Lambda {
            params,
            body,
            captures,
        } => MirExprKind::Lambda {
            params: params
                .into_iter()
                .map(|param| MirLambdaParam {
                    name: param.name,
                    ty: param.ty,
                })
                .collect(),
            body,
            captures: captures
                .into_iter()
                .map(|capture| MirLambdaCapture {
                    name: capture.name,
                    mutable: capture.mutable,
                })
                .collect(),
        },
        MirExprKindJson::Rec { target, ident } => MirExprKind::Rec {
            target,
            ident: ident.map(value_summary),
        },
        MirExprKindJson::Match {
            target,
            arms,
            lowering,
        } => MirExprKind::Match {
            target,
            arms: arms.into_iter().map(convert_match_arm).collect(),
            lowering: convert_match_lowering(lowering),
        },
        MirExprKindJson::Call { callee, args } => MirExprKind::Call { callee, args },
        MirExprKindJson::Binary {
            operator,
            left,
            right,
        } => MirExprKind::Binary {
            operator,
            left,
            right,
        },
        MirExprKindJson::IfElse {
            condition,
            then_branch,
            else_branch,
        } => MirExprKind::IfElse {
            condition,
            then_branch,
            else_branch,
        },
        MirExprKindJson::PerformCall { call } => MirExprKind::PerformCall {
            effect: value_summary(call.effect),
            argument: call.argument,
        },
        MirExprKindJson::EffectBlock { body } => MirExprKind::EffectBlock { body },
        MirExprKindJson::Async { .. } => MirExprKind::Unknown,
        MirExprKindJson::Await { .. } => MirExprKind::Unknown,
        MirExprKindJson::Unsafe { body } => MirExprKind::Unsafe { body },
        MirExprKindJson::InlineAsm {
            template,
            outputs,
            inputs,
            clobbers,
            options,
        } => MirExprKind::InlineAsm {
            template,
            outputs: outputs
                .into_iter()
                .map(|output| MirInlineAsmOutput {
                    constraint: output.constraint,
                    target: output.target,
                })
                .collect(),
            inputs: inputs
                .into_iter()
                .map(|input| MirInlineAsmInput {
                    constraint: input.constraint,
                    expr: input.expr,
                })
                .collect(),
            clobbers,
            options,
        },
        MirExprKindJson::LlvmIr {
            result_type,
            template,
            inputs,
        } => MirExprKind::LlvmIr {
            result_type,
            template,
            inputs,
        },
        MirExprKindJson::FieldAccess { target, field } => {
            MirExprKind::FieldAccess { target, field }
        }
        MirExprKindJson::Index { target, index } => MirExprKind::Index { target, index },
        MirExprKindJson::Identifier { ident } => MirExprKind::Identifier {
            summary: value_summary(ident),
        },
        MirExprKindJson::Literal { value } => MirExprKind::Literal {
            summary: value_summary(value),
        },
        MirExprKindJson::Unknown => MirExprKind::Unknown,
    }
}

fn convert_stmt(stmt: MirStmtJson) -> MirStmt {
    let kind = match stmt.kind {
        MirStmtKindJson::Let {
            pattern,
            value,
            mutable,
        } => MirStmtKind::Let {
            pattern: convert_pattern(pattern),
            value,
            mutable,
        },
        MirStmtKindJson::Expr { expr } => MirStmtKind::Expr { expr },
        MirStmtKindJson::Assign { target, value } => MirStmtKind::Assign { target, value },
        MirStmtKindJson::Defer { expr } => MirStmtKind::Defer { expr },
    };
    MirStmt { kind }
}

fn convert_match_lowering(plan: Option<MatchLoweringPlanJson>) -> Option<MatchLoweringPlan> {
    plan.map(|plan| MatchLoweringPlan {
        owner: plan.owner,
        target_type: plan.target_type,
        arm_count: plan.arm_count,
        arms: plan
            .arms
            .into_iter()
            .map(|arm| MatchArmLowering {
                pattern: convert_pattern_lowering(arm.pattern),
                has_guard: arm.has_guard,
                alias: arm.alias,
            })
            .collect(),
    })
}

fn convert_pattern_lowering(pattern: PatternLoweringJson) -> PatternLowering {
    PatternLowering {
        label: pattern.label,
        miss_on_none: pattern.miss_on_none,
        always_matches: pattern.always_matches,
        children: pattern
            .children
            .into_iter()
            .map(convert_pattern_lowering)
            .collect(),
    }
}

fn convert_match_arm(arm: MirMatchArmJson) -> MirMatchArm {
    MirMatchArm {
        pattern: convert_pattern(arm.pattern),
        guard: arm.guard,
        alias: arm.alias,
        body: arm.body,
    }
}

fn convert_pattern(pattern: MirPatternJson) -> MirPattern {
    let kind = match pattern.kind {
        MirPatternKindSpec::Tagged(tagged) => convert_tagged_pattern(tagged),
        MirPatternKindSpec::Active(call) => MirPatternKind::Active(convert_active_pattern(call)),
        MirPatternKindSpec::Other(value) => convert_pattern_fallback(value),
    };
    MirPattern { kind }
}

fn convert_tagged_pattern(pattern: MirPatternKindJson) -> MirPatternKind {
    match pattern {
        MirPatternKindJson::Wildcard => MirPatternKind::Wildcard,
        MirPatternKindJson::Var { name } => MirPatternKind::Var { name },
        MirPatternKindJson::Literal(value) => MirPatternKind::Literal {
            summary: value_summary(value),
        },
        MirPatternKindJson::Tuple { elements } => MirPatternKind::Tuple {
            elements: elements.into_iter().map(convert_pattern).collect(),
        },
        MirPatternKindJson::Record { fields, has_rest } => MirPatternKind::Record {
            fields: fields
                .into_iter()
                .map(|field| MirPatternRecordField {
                    key: field.key,
                    value: field.value.map(|value| Box::new(convert_pattern(*value))),
                })
                .collect(),
            has_rest,
        },
        MirPatternKindJson::Constructor { name, args } => MirPatternKind::Constructor {
            name,
            args: args.into_iter().map(convert_pattern).collect(),
        },
        MirPatternKindJson::Binding {
            name,
            pattern,
            via_at,
        } => MirPatternKind::Binding {
            name,
            pattern: Box::new(convert_pattern(*pattern)),
            via_at,
        },
        MirPatternKindJson::Or { variants } => MirPatternKind::Or {
            variants: variants.into_iter().map(convert_pattern).collect(),
        },
        MirPatternKindJson::Slice(spec) => MirPatternKind::Slice(convert_slice_pattern(spec)),
        MirPatternKindJson::Range {
            start,
            end,
            inclusive,
        } => MirPatternKind::Range {
            start: start.map(|value| Box::new(convert_pattern(*value))),
            end: end.map(|value| Box::new(convert_pattern(*value))),
            inclusive,
        },
        MirPatternKindJson::Regex { pattern } => MirPatternKind::Regex { pattern },
    }
}

fn convert_active_pattern(call: MirActivePatternCallJson) -> MirActivePatternCall {
    MirActivePatternCall {
        name: call.name,
        kind: convert_active_kind(call.kind),
        argument: call.argument.map(|value| Box::new(convert_pattern(*value))),
        input_binding: call.input_binding,
        miss_target: convert_jump_target(call.miss_target),
    }
}

fn convert_pattern_fallback(value: Value) -> MirPatternKind {
    if let Ok(tagged) = serde_json::from_value::<MirPatternKindJson>(value.clone()) {
        return convert_tagged_pattern(tagged);
    }
    if let Ok(active) = serde_json::from_value::<MirActivePatternCallJson>(value) {
        return MirPatternKind::Active(convert_active_pattern(active));
    }
    MirPatternKind::Wildcard
}

fn convert_slice_pattern(pattern: MirSlicePatternJson) -> MirSlicePattern {
    MirSlicePattern {
        head: pattern.head.into_iter().map(convert_pattern).collect(),
        rest: pattern.rest.map(|rest| MirSliceRest {
            binding: rest.binding,
        }),
        tail: pattern.tail.into_iter().map(convert_pattern).collect(),
    }
}

fn convert_active_kind(kind: MirActivePatternKindJson) -> ActivePatternKind {
    match kind.as_str() {
        Some("partial") => ActivePatternKind::Partial,
        Some("total") => ActivePatternKind::Total,
        _ => ActivePatternKind::Unknown,
    }
}

fn convert_jump_target(label: Option<String>) -> Option<MirJumpTarget> {
    match label.as_deref() {
        Some("next_arm") => Some(MirJumpTarget::NextArm),
        _ => None,
    }
}

fn extract_match_plans(exprs: &[MirExpr]) -> Vec<String> {
    let mut plans = Vec::new();
    for expr in exprs {
        if let MirExprKind::Match { arms, lowering, .. } = &expr.kind {
            let pattern_labels: Vec<String> = arms
                .iter()
                .map(|arm| summarize_pattern(&arm.pattern))
                .collect();
            let lowering_label = lowering
                .as_ref()
                .and_then(|l| l.target_type.clone())
                .unwrap_or_else(|| "unknown".into());
            let arm_with_guard = arms.iter().filter(|arm| arm.guard.is_some()).count();
            let plan = format!(
                "match#{} ty={} arms={} guard_arms={} patterns=[{}]",
                expr.id,
                lowering_label,
                arms.len(),
                arm_with_guard,
                pattern_labels.join("|")
            );
            plans.push(plan);
        }
    }
    plans
}

fn value_summary(value: Value) -> String {
    match value {
        Value::String(text) => text,
        Value::Number(num) => num.to_string(),
        Value::Bool(flag) => flag.to_string(),
        other => other.to_string(),
    }
}

/// MIR JSON ロード/差分生成で発生するエラー。
#[derive(Debug)]
pub enum MirSnapshotError {
    Io(io::Error),
    Json(serde_json::Error),
}

impl fmt::Display for MirSnapshotError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MirSnapshotError::Io(err) => write!(f, "I/O エラー: {}", err),
            MirSnapshotError::Json(err) => write!(f, "JSON パースエラー: {}", err),
        }
    }
}

impl Error for MirSnapshotError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            MirSnapshotError::Io(err) => Some(err),
            MirSnapshotError::Json(err) => Some(err),
        }
    }
}

impl From<io::Error> for MirSnapshotError {
    fn from(err: io::Error) -> Self {
        MirSnapshotError::Io(err)
    }
}

impl From<serde_json::Error> for MirSnapshotError {
    fn from(err: serde_json::Error) -> Self {
        MirSnapshotError::Json(err)
    }
}

/// 生成した MIR 関数リストから差分スナップショットを生成する。
pub fn generate_snapshot(
    module_name: impl Into<String>,
    target_machine: TargetMachine,
    runtime_symbols: Vec<String>,
    metadata: Vec<String>,
    functions: Vec<MirFunction>,
) -> BackendDiffSnapshot {
    let module_name = module_name.into();
    let mut codegen = CodegenContext::new(target_machine.clone(), runtime_symbols);
    metadata
        .into_iter()
        .for_each(|entry| codegen.with_metadata(entry));
    for function in &functions {
        codegen.emit_function(function);
    }
    let module = codegen.finish_module(module_name.clone());
    let verification = Verifier::new().verify_module(&module);
    BackendDiffSnapshot {
        module_name,
        target_triple: module.target.triple.to_string(),
        backend_abi: module.target.backend_abi().to_string(),
        data_layout: module.target.data_layout.description.clone(),
        windows_toolchain: module
            .windows_toolchain
            .as_ref()
            .map(|cfg| cfg.toolchain_name.clone()),
        functions: module
            .functions
            .iter()
            .map(BackendFunctionRecord::from_generated)
            .collect(),
        diagnostics: verification
            .diagnostics
            .into_iter()
            .map(|diag| format!("{}.{}: {}", diag.domain, diag.code, diag.message))
            .collect(),
        audit_entries: verification
            .audit_log
            .entries
            .into_iter()
            .map(|entry| format!("{}={}", entry.key, entry.value))
            .collect(),
        bridge_metadata: module.bridge_metadata.snapshot_entries(),
        passed: verification.passed,
    }
}

/// MIR JSON から差分スナップショットを生成する補助。
pub fn generate_snapshot_from_mir_json<P: AsRef<Path>>(
    path: P,
    target_machine: TargetMachine,
    runtime_symbols: Vec<String>,
    metadata: Vec<String>,
    default_module_name: impl Into<String>,
) -> Result<BackendDiffSnapshot, MirSnapshotError> {
    let module_default = default_module_name.into();
    let spec = MirModuleSpec::from_file(path)?;
    let todo_diagnostics = spec.collect_todo_diagnostics();
    let module_name = spec
        .module
        .clone()
        .unwrap_or_else(|| module_default.clone());
    let mut runtime_symbols = runtime_symbols;
    runtime_symbols.extend(spec.runtime_symbols.iter().cloned());
    let mut metadata = metadata;
    metadata.extend(spec.metadata.iter().cloned());
    let functions = spec.into_functions();
    let mut snapshot = generate_snapshot(
        module_name,
        target_machine,
        runtime_symbols,
        metadata,
        functions,
    );
    snapshot.diagnostics.extend(todo_diagnostics);
    Ok(snapshot)
}

/// JSON ファイルから MIR 関数リストをロードする。
pub fn load_mir_functions_from_json<P: AsRef<Path>>(
    path: P,
) -> Result<Vec<MirFunction>, MirSnapshotError> {
    let spec = MirModuleSpec::from_file(path)?;
    Ok(spec.into_functions())
}

/// W3 相当の差分スナップショットを生成する。
pub fn generate_w3_snapshot() -> BackendDiffSnapshot {
    let windows_toolchain = WindowsToolchainConfig {
        toolchain_name: "msvc-llvm-19.1.1".into(),
        llc_path: "C:\\llvm-19.1.1\\bin\\llc.exe".into(),
        opt_path: "C:\\llvm-19.1.1\\bin\\opt.exe".into(),
    };
    let target_machine = TargetMachineBuilder::new()
        .with_triple(Triple::WindowsMSVC)
        .with_cpu("x86-64")
        .with_features("+sse4.2,+popcnt")
        .with_relocation_model(RelocModel::Static)
        .with_code_model(CodeModel::Large)
        .with_optimization_level(OptimizationLevel::O2)
        .with_data_layout(DataLayoutSpec::new(
            "e-m:w-p:64:64-f64:64:64-v128:128:128-a:0:64",
        ))
        .with_windows_toolchain(windows_toolchain.clone())
        .build();

    let mut codegen = CodegenContext::new(
        target_machine.clone(),
        vec![
            "mem_alloc".into(),
            "inc_ref".into(),
            "dec_ref".into(),
            "panic".into(),
        ],
    );
    codegen.with_metadata("phase=W3");
    codegen.with_metadata("runtime=llvm");

    let entry = MirFunction::new("@k__main", "ccc")
        .with_param(RemlType::Pointer)
        .with_param(RemlType::I64)
        .with_return(RemlType::I32)
        .with_attribute("nounwind")
        .with_attribute("uwtable")
        .with_ffi_call(FfiCallSignature {
            name: "mem_alloc".into(),
            calling_conv: "ccc".into(),
            args: vec![RemlType::I64],
            ret: Some(RemlType::Pointer),
            variadic: false,
        })
        .with_ffi_call(FfiCallSignature {
            name: "panic".into(),
            calling_conv: "ccc".into(),
            args: vec![RemlType::String],
            ret: None,
            variadic: false,
        });

    let _ = codegen.emit_function(&entry);
    let module = codegen.finish_module("reml_backend_module");
    let verification = Verifier::new().verify_module(&module);

    BackendDiffSnapshot {
        module_name: module.name.clone(),
        target_triple: module.target.triple.to_string(),
        backend_abi: module.target.backend_abi().to_string(),
        data_layout: module.target.data_layout.description.clone(),
        windows_toolchain: module
            .windows_toolchain
            .as_ref()
            .map(|cfg| cfg.toolchain_name.clone()),
        functions: module
            .functions
            .iter()
            .map(BackendFunctionRecord::from_generated)
            .collect(),
        diagnostics: verification
            .diagnostics
            .into_iter()
            .map(|diag| format!("{}.{}: {}", diag.domain, diag.code, diag.message))
            .collect(),
        audit_entries: verification
            .audit_log
            .entries
            .into_iter()
            .map(|entry| format!("{}={}", entry.key, entry.value))
            .collect(),
        bridge_metadata: module.bridge_metadata.snapshot_entries(),
        passed: verification.passed,
    }
}

fn parse_reml_type(token: &str) -> RemlType {
    parse_reml_type_with_diagnostics(token, None)
}

fn parse_reml_type_with_diagnostics(
    token: &str,
    mut diagnostics: Option<&mut Vec<String>>,
) -> RemlType {
    let trimmed = token.trim();
    if trimmed.eq_ignore_ascii_case("&str") {
        return RemlType::String;
    }
    if let Some(rest) = trimmed.strip_prefix('&') {
        let rest = rest.trim_start();
        let (mutable, inner) = match rest.strip_prefix("mut") {
            Some(after_mut) => (true, after_mut.trim_start()),
            None => (false, rest),
        };
        if inner.is_empty() {
            return RemlType::Pointer;
        }
        return RemlType::Ref {
            mutable,
            to: Box::new(parse_reml_type(inner)),
        };
    }
    if trimmed.starts_with('[') && trimmed.ends_with(']') && trimmed.len() >= 2 {
        let inner = trimmed[1..trimmed.len() - 1].trim();
        if inner.is_empty() {
            return RemlType::Pointer;
        }
        if let Some((element, length)) = inner.split_once(';') {
            let element = element.trim();
            let length = length.trim();
            if element.is_empty() {
                push_fixed_array_diagnostic(
                    &mut diagnostics,
                    trimmed,
                    "element_empty",
                );
                return RemlType::Pointer;
            }
            if length.is_empty() {
                push_fixed_array_diagnostic(
                    &mut diagnostics,
                    trimmed,
                    "length_empty",
                );
                return RemlType::Pointer;
            }
            if length.starts_with('-') {
                push_fixed_array_diagnostic(
                    &mut diagnostics,
                    trimmed,
                    "length_negative",
                );
                return RemlType::Pointer;
            }
            if !length.chars().all(|ch| ch.is_ascii_digit()) {
                push_fixed_array_diagnostic(
                    &mut diagnostics,
                    trimmed,
                    "length_not_decimal",
                );
                return RemlType::Pointer;
            }
            let length = match length.parse::<u64>() {
                Ok(value) => value,
                Err(_) => {
                    push_fixed_array_diagnostic(
                        &mut diagnostics,
                        trimmed,
                        "length_overflow",
                    );
                    return RemlType::Pointer;
                }
            };
            let element = parse_reml_type_with_diagnostics(element, diagnostics.as_deref_mut());
            return RemlType::Array {
                element: Box::new(element),
                length,
            };
        }
        return RemlType::Slice(Box::new(parse_reml_type_with_diagnostics(
            inner,
            diagnostics,
        )));
    }
    if let Some((name, inner)) = split_generic_type(trimmed) {
        if name.eq_ignore_ascii_case("set") {
            let inner = if inner.is_empty() {
                RemlType::Pointer
            } else {
                parse_reml_type(inner)
            };
            return RemlType::Set(Box::new(inner));
        }
    }
    let normalized = trimmed.to_ascii_lowercase();
    match normalized.as_str() {
        "unit" | "void" => RemlType::Unit,
        "bool" => RemlType::Bool,
        "i32" | "int32" => RemlType::I32,
        "i64" | "int64" => RemlType::I64,
        "f64" | "double" => RemlType::F64,
        "pointer" | "ptr" | "i8*" => RemlType::Pointer,
        "string" | "str" => RemlType::String,
        _ => RemlType::Pointer,
    }
}

fn push_fixed_array_diagnostic(
    diagnostics: &mut Option<&mut Vec<String>>,
    token: &str,
    reason: &str,
) {
    if let Some(diagnostics) = diagnostics.as_deref_mut() {
        diagnostics.push(format!(
            "Backend.backend.todo.fixed_array_type: token={token} reason={reason}"
        ));
    }
}

fn split_generic_type(token: &str) -> Option<(&str, &str)> {
    let trimmed = token.trim();
    let open = trimmed.find('<')?;
    if !trimmed.ends_with('>') {
        return None;
    }
    let name = trimmed[..open].trim();
    let inner = trimmed[open + 1..trimmed.len() - 1].trim();
    if name.is_empty() {
        return None;
    }
    Some((name, inner))
}

#[cfg(test)]
mod tests {
    use super::{
        generate_snapshot_from_mir_json, load_mir_functions_from_json, parse_reml_type,
        parse_reml_type_with_diagnostics, MirModuleSpec, MirSnapshotError,
    };
    use crate::target_machine::{
        CodeModel, DataLayoutSpec, OptimizationLevel, RelocModel, TargetMachineBuilder, Triple,
        WindowsToolchainConfig,
    };
    use crate::type_mapping::RemlType;
    use std::{env, fs};

    fn test_target_machine() -> crate::target_machine::TargetMachine {
        TargetMachineBuilder::new()
            .with_triple(Triple::LinuxGNU)
            .with_relocation_model(RelocModel::Static)
            .with_code_model(CodeModel::Small)
            .with_optimization_level(OptimizationLevel::O1)
            .with_data_layout(DataLayoutSpec::new("e-m:e-p:64:64-f64:64:64-a:0:64"))
            .build()
    }

    #[test]
    #[ignore]
    fn dump_branch_plans_from_mir_path() -> Result<(), MirSnapshotError> {
        let path =
            env::var("MIR_PATH").expect("MIR_PATH 環境変数で MIR JSON のパスを指定してください");
        let target_machine = test_target_machine();
        let snapshot = generate_snapshot_from_mir_json(
            &path,
            target_machine,
            vec![],
            vec!["phase=dump".into()],
            "mir_dump",
        )?;
        for func in snapshot.functions {
            println!("fn {}:", func.name);
            for plan in func.branch_plans {
                println!("  {}", plan);
            }
        }
        Ok(())
    }

    #[test]
    #[ignore]
    fn dump_llvm_ir_from_mir_path() -> Result<(), MirSnapshotError> {
        let path =
            env::var("MIR_PATH").expect("MIR_PATH 環境変数で MIR JSON のパスを指定してください");
        let target_machine = test_target_machine();
        let snapshot = generate_snapshot_from_mir_json(
            &path,
            target_machine,
            vec![],
            vec!["phase=dump".into()],
            "mir_dump",
        )?;
        for func in snapshot.functions {
            println!("fn {}:\n{}\n", func.name, func.llvm_ir);
        }
        Ok(())
    }

    #[test]
    fn parse_reml_type_synonyms() {
        assert_eq!(parse_reml_type("i32"), RemlType::I32);
        assert_eq!(parse_reml_type("Int64"), RemlType::I64);
        assert_eq!(parse_reml_type("ptr"), RemlType::Pointer);
        assert_eq!(parse_reml_type("unknown"), RemlType::Pointer);
        assert_eq!(
            parse_reml_type("Set<Str>"),
            RemlType::Set(Box::new(RemlType::String))
        );
        assert_eq!(
            parse_reml_type("&i64"),
            RemlType::Ref {
                mutable: false,
                to: Box::new(RemlType::I64)
            }
        );
        assert_eq!(
            parse_reml_type("&mut [i32]"),
            RemlType::Ref {
                mutable: true,
                to: Box::new(RemlType::Slice(Box::new(RemlType::I32)))
            }
        );
    }

    fn assert_fixed_array_diagnostic(diagnostics: &[String], reason: &str) {
        let reason_token = format!("reason={reason}");
        assert!(
            diagnostics.iter().any(|entry| {
                entry.contains("Backend.backend.todo.fixed_array_type")
                    && entry.contains(&reason_token)
            }),
            "diagnostic に {reason_token} を含むこと (got: {diagnostics:?})"
        );
    }

    #[test]
    fn parse_reml_type_fixed_array() {
        let mut diagnostics = Vec::new();
        let ty = parse_reml_type_with_diagnostics("[i64; 6]", Some(&mut diagnostics));
        assert_eq!(
            ty,
            RemlType::Array {
                element: Box::new(RemlType::I64),
                length: 6
            }
        );
        assert!(diagnostics.is_empty(), "diagnostic が空であること");
    }

    #[test]
    fn parse_reml_type_fixed_array_invalid_length() {
        let mut diagnostics = Vec::new();
        let ty = parse_reml_type_with_diagnostics("[i64; X]", Some(&mut diagnostics));
        assert_eq!(ty, RemlType::Pointer);
        assert_fixed_array_diagnostic(&diagnostics, "length_not_decimal");

        diagnostics.clear();
        let ty = parse_reml_type_with_diagnostics("[i64; -1]", Some(&mut diagnostics));
        assert_eq!(ty, RemlType::Pointer);
        assert_fixed_array_diagnostic(&diagnostics, "length_negative");

        diagnostics.clear();
        let ty = parse_reml_type_with_diagnostics(
            "[i64; 18446744073709551616]",
            Some(&mut diagnostics),
        );
        assert_eq!(ty, RemlType::Pointer);
        assert_fixed_array_diagnostic(&diagnostics, "length_overflow");
    }

    #[test]
    fn snapshot_from_json_file() -> Result<(), MirSnapshotError> {
        let spec = r#"
    {
      "module": "json_module",
      "metadata": ["phase=json"],
      "functions": [
        {
          "name": "@json_main",
          "calling_conv": "ccc",
          "params": ["pointer", "i64"],
          "return": "i32",
          "attributes": ["nounwind"],
          "ffi_calls": [
            {"name": "panic", "calling_conv": "ccc", "args": ["string"], "return": null}
          ]
        }
      ]
    }
    "#;
        let tmp = env::temp_dir().join("reml_mir_test.json");
        fs::write(&tmp, spec)?;
        let windows_toolchain = WindowsToolchainConfig {
            toolchain_name: "test-llvm".into(),
            llc_path: "llc".into(),
            opt_path: "opt".into(),
        };
        let target_machine = TargetMachineBuilder::new()
            .with_triple(Triple::LinuxGNU)
            .with_relocation_model(RelocModel::Static)
            .with_code_model(CodeModel::Small)
            .with_optimization_level(OptimizationLevel::O1)
            .with_data_layout(DataLayoutSpec::new("e-m:e-p:64:64-f64:64:64-a:0:64"))
            .with_windows_toolchain(windows_toolchain.clone())
            .build();
        let snapshot = generate_snapshot_from_mir_json(
            &tmp,
            target_machine,
            vec!["mem_alloc".into()],
            vec!["runtime=json".into()],
            "json_module",
        )?;
        assert_eq!(snapshot.module_name, "json_module");
        assert!(
            snapshot
                .diagnostics
                .iter()
                .any(|entry| entry.contains("target.profile.missing")),
            "target profile missing diagnostic を含むこと"
        );
        fs::remove_file(tmp)?;
        Ok(())
    }

    #[test]
    fn load_functions_from_json_file() -> Result<(), MirSnapshotError> {
        let spec = r#"
    {
      "functions": [
        {"name": "@json_main", "calling_conv": "ccc"}
      ]
    }
    "#;
        let tmp = env::temp_dir().join("reml_mir_list.json");
        fs::write(&tmp, spec)?;
        let functions = load_mir_functions_from_json(&tmp)?;
        assert_eq!(functions.len(), 1);
        assert_eq!(functions[0].name, "@json_main");
        fs::remove_file(tmp)?;
        Ok(())
    }

    #[test]
    fn load_async_effect_block_exprs_from_json() -> Result<(), MirSnapshotError> {
        let spec = r#"
    {
      "module": "json_module",
      "externs": [
        {
          "name": "c_func",
          "abi": "C",
          "symbol": "c_func",
          "span": {"start": 0, "end": 1}
        }
      ],
      "functions": [
        {
          "name": "@json_main",
          "calling_conv": "ccc",
          "is_async": true,
          "is_unsafe": true,
          "varargs": true,
          "exprs": [
            {"id": 0, "ty": "unit", "kind": {"kind": "effect_block", "body": 1}},
            {"id": 1, "ty": "unit", "kind": {"kind": "async", "body": 2, "is_move": true}},
            {"id": 2, "ty": "unit", "kind": {"kind": "await", "expr": 3}},
            {"id": 3, "ty": "unit", "kind": {"kind": "unsafe", "body": 4}},
            {"id": 4, "ty": "unit", "kind": {"kind": "block"}}
          ],
          "body": 0
        }
      ]
    }
    "#;
        let tmp = env::temp_dir().join("reml_mir_async_kinds.json");
        fs::write(&tmp, spec)?;
        let functions = load_mir_functions_from_json(&tmp)?;
        let func = functions.get(0).expect("関数が読み込まれること");
        assert_eq!(func.name, "@json_main");
        assert_eq!(func.body, Some(0));
        assert_eq!(func.exprs.len(), 5);
        fs::remove_file(tmp)?;
        Ok(())
    }

    #[test]
    fn collect_todo_diagnostics_from_async_effect_unsafe_externs() -> Result<(), MirSnapshotError> {
        let spec = r#"
    {
      "module": "json_module",
      "externs": [
        {
          "name": "c_func",
          "abi": "C",
          "symbol": "c_func",
          "span": {"start": 0, "end": 1}
        }
      ],
      "functions": [
        {
          "name": "@json_main",
          "calling_conv": "ccc",
          "is_async": true,
          "exprs": [
            {"id": 0, "ty": "unit", "kind": {"kind": "effect_block", "body": 1}},
            {"id": 1, "ty": "unit", "kind": {"kind": "async", "body": 2, "is_move": true}},
            {"id": 2, "ty": "unit", "kind": {"kind": "await", "expr": 3}},
            {"id": 3, "ty": "unit", "kind": {"kind": "unsafe", "body": 4}},
            {"id": 4, "ty": "unit", "kind": {"kind": "block"}}
          ],
          "body": 0
        }
      ]
    }
    "#;
        let tmp = env::temp_dir().join("reml_mir_todo_diagnostics.json");
        fs::write(&tmp, spec)?;
        let module = MirModuleSpec::from_file(&tmp)?;
        let diagnostics = module.collect_todo_diagnostics();
        let expected = [
            "Backend.backend.todo.async_function: function=@json_main",
            "Backend.backend.todo.effect_block: function=@json_main expr_id=0",
            "Backend.backend.todo.async: function=@json_main expr_id=1",
            "Backend.backend.todo.await: function=@json_main expr_id=2",
            "Backend.backend.todo.unsafe_block: function=@json_main expr_id=3",
            "Backend.backend.todo.extern_decl: name=c_func abi=C symbol=c_func",
        ];
        for entry in expected {
            assert!(
                diagnostics.iter().any(|diagnostic| diagnostic == entry),
                "{entry} が含まれること"
            );
        }
        fs::remove_file(tmp)?;
        Ok(())
    }

    #[test]
    fn ffi_call_variadic_is_loaded_from_json() -> Result<(), MirSnapshotError> {
        let spec = r#"
    {
      "functions": [
        {
          "name": "@json_main",
          "calling_conv": "ccc",
          "ffi_calls": [
            {
              "name": "printf",
              "calling_conv": "ccc",
              "args": ["i32"],
              "return": "i32",
              "variadic": true
            }
          ]
        }
      ]
    }
    "#;
        let tmp = env::temp_dir().join("reml_mir_variadic_test.json");
        fs::write(&tmp, spec)?;
        let functions = load_mir_functions_from_json(&tmp)?;
        let sig = functions
            .get(0)
            .and_then(|func| func.ffi_calls.get(0))
            .expect("ffi_calls が1件以上あること");
        assert!(sig.variadic, "variadic が true であること");
        fs::remove_file(tmp)?;
        Ok(())
    }

    #[test]
    fn llvm_ir_option_canonical_has_ctor_payload_and_expr_lowering() -> Result<(), MirSnapshotError>
    {
        let repo_root = env!("CARGO_MANIFEST_DIR");
        let path = std::path::Path::new(repo_root)
            .join("../../../tmp/mir-bnf-matchexpr-option-canonical.json");
        let snapshot = generate_snapshot_from_mir_json(
            &path,
            test_target_machine(),
            vec![],
            vec!["phase=test".into()],
            "mir_test",
        )?;
        let describe = snapshot
            .functions
            .iter()
            .find(|func| func.name == "describe")
            .expect("describe 関数が存在すること");
        let llvm_ir = &describe.llvm_ir;
        assert!(
            llvm_ir.contains("@reml_ctor_payload_Some"),
            "Some(payload) の payload 抽出が IR に含まれること"
        );
        assert!(
            llvm_ir.contains("@reml_field_access") && llvm_ir.contains("@reml_call"),
            "FieldAccess/Call が IR に含まれること"
        );
        assert!(
            llvm_ir.contains("@reml_str_concat"),
            "文字列結合が IR に含まれること"
        );
        assert!(
            !llvm_ir.contains("match_result <- #"),
            "`match_result <- #...` のフォールバックが残らないこと"
        );
        Ok(())
    }

    #[test]
    fn llvm_ir_result_guard_else_has_ctor_payload_and_guard_eval() -> Result<(), MirSnapshotError> {
        let repo_root = env!("CARGO_MANIFEST_DIR");
        let path = std::path::Path::new(repo_root)
            .join("../../../tmp/mir-bnf-matchexpr-result-guard-else-ok.json");
        let snapshot = generate_snapshot_from_mir_json(
            &path,
            test_target_machine(),
            vec![],
            vec!["phase=test".into()],
            "mir_test",
        )?;
        let classify = snapshot
            .functions
            .iter()
            .find(|func| func.name == "classify")
            .expect("classify 関数が存在すること");
        let llvm_ir = &classify.llvm_ir;
        assert!(
            llvm_ir.contains("@reml_ctor_payload_Ok") && llvm_ir.contains("@reml_ctor_payload_Err"),
            "Ok/Err の payload 抽出が IR に含まれること"
        );
        assert!(
            llvm_ir.contains("srem i64"),
            "ガード式（%）が IR に落ちること"
        );
        assert!(
            !llvm_ir.contains("match_result <- #"),
            "`match_result <- #...` のフォールバックが残らないこと"
        );
        Ok(())
    }

    #[test]
    fn llvm_ir_sanitizes_emoji_identifiers() -> Result<(), MirSnapshotError> {
        let spec = r#"
    {
      "functions": [
        {
          "name": "@main\uD83D\uDE80",
          "calling_conv": "ccc",
          "params": [],
          "return": "i32"
        }
      ]
    }
    "#;
        let tmp = env::temp_dir().join("reml_mir_emoji_ident.json");
        fs::write(&tmp, spec)?;
        let snapshot = generate_snapshot_from_mir_json(
            &tmp,
            test_target_machine(),
            vec![],
            vec!["phase=test".into()],
            "mir_test",
        )?;
        let func = snapshot
            .functions
            .iter()
            .find(|func| func.name == "@main\u{1F680}")
            .expect("emoji 識別子の関数が存在すること");
        assert!(
            func.llvm_ir.contains("@main_u01F680"),
            "LLVM IR では emoji 識別子がサニタイズされること"
        );
        assert!(
            !func.llvm_ir.contains("\u{1F680}"),
            "LLVM IR に生の emoji が残らないこと"
        );
        fs::remove_file(tmp)?;
        Ok(())
    }

    #[test]
    fn llvm_ir_literal_snapshots_cover_complex_literals() -> Result<(), MirSnapshotError> {
        let repo_root = env!("CARGO_MANIFEST_DIR");
        let path = std::path::Path::new(repo_root)
            .join("../../../tmp/mir-literals-backend.json");
        let snapshot = generate_snapshot_from_mir_json(
            &path,
            test_target_machine(),
            vec![],
            vec!["phase=test".into()],
            "mir_test",
        )?;

        let float_fn = snapshot
            .functions
            .iter()
            .find(|func| func.name == "literal_float")
            .expect("literal_float 関数が存在すること");
        assert!(
            float_fn.llvm_ir.contains("@reml_box_float"),
            "Float literal が reml_box_float に変換されること"
        );

        let char_fn = snapshot
            .functions
            .iter()
            .find(|func| func.name == "literal_char")
            .expect("literal_char 関数が存在すること");
        assert!(
            char_fn.llvm_ir.contains("@reml_box_char"),
            "Char literal が reml_box_char に変換されること"
        );

        let tuple_fn = snapshot
            .functions
            .iter()
            .find(|func| func.name == "literal_tuple")
            .expect("literal_tuple 関数が存在すること");
        assert!(
            tuple_fn
                .llvm_ir
                .contains("diag backend.literal.unsupported.tuple: len=2"),
            "Tuple literal の JSON 形状が len=2 として認識されること"
        );

        let array_fn = snapshot
            .functions
            .iter()
            .find(|func| func.name == "literal_array")
            .expect("literal_array 関数が存在すること");
        assert!(
            array_fn.llvm_ir.contains("@reml_array_from"),
            "Array literal が reml_array_from に変換されること"
        );

        let record_fn = snapshot
            .functions
            .iter()
            .find(|func| func.name == "literal_record")
            .expect("literal_record 関数が存在すること");
        assert!(
            record_fn.llvm_ir.contains("@reml_record_from"),
            "Record literal が reml_record_from に変換されること"
        );

        Ok(())
    }

    #[test]
    fn llvm_ir_array_literal_snapshots_cover_array_shapes() -> Result<(), MirSnapshotError> {
        let repo_root = env!("CARGO_MANIFEST_DIR");
        let path = std::path::Path::new(repo_root).join("../../../tmp/mir-array-literals.json");
        let snapshot = generate_snapshot_from_mir_json(
            &path,
            test_target_machine(),
            vec![],
            vec!["phase=test".into()],
            "mir_test",
        )?;

        let empty = snapshot
            .functions
            .iter()
            .find(|func| func.name == "array_empty_dynamic")
            .expect("array_empty_dynamic 関数が存在すること");
        assert!(
            empty
                .llvm_ir
                .contains("; array literal dynamic len=0")
                && empty.llvm_ir.contains("@reml_array_from(i64 0"),
            "空配列は len=0 の reml_array_from で生成されること"
        );

        let single = snapshot
            .functions
            .iter()
            .find(|func| func.name == "array_single_dynamic")
            .expect("array_single_dynamic 関数が存在すること");
        assert!(
            single
                .llvm_ir
                .contains("; array literal dynamic len=1")
                && single.llvm_ir.contains("array element 0"),
            "単一要素配列は len=1 かつ element 0 のコメントが含まれること"
        );

        let multi = snapshot
            .functions
            .iter()
            .find(|func| func.name == "array_multi_dynamic")
            .expect("array_multi_dynamic 関数が存在すること");
        assert!(
            multi
                .llvm_ir
                .contains("; array literal dynamic len=3")
                && multi.llvm_ir.contains("array element 2"),
            "複数要素配列は len=3 かつ element 2 のコメントが含まれること"
        );

        let fixed_match = snapshot
            .functions
            .iter()
            .find(|func| func.name == "array_fixed_matched")
            .expect("array_fixed_matched 関数が存在すること");
        assert!(
            fixed_match
                .llvm_ir
                .contains("array literal fixed-length matched: expected=2, actual=2"),
            "固定長配列は length matched コメントが含まれること"
        );

        let fixed_mismatch = snapshot
            .functions
            .iter()
            .find(|func| func.name == "array_fixed_mismatch")
            .expect("array_fixed_mismatch 関数が存在すること");
        assert!(
            fixed_mismatch
                .llvm_ir
                .contains("array literal fixed-length mismatch: expected=3, actual=2"),
            "固定長配列の不一致は mismatch コメントが含まれること"
        );

        let nested = snapshot
            .functions
            .iter()
            .find(|func| func.name == "array_nested")
            .expect("array_nested 関数が存在すること");
        let array_from_calls = nested
            .llvm_ir
            .matches("@reml_array_from")
            .count();
        assert!(
            array_from_calls >= 3,
            "ネスト配列は複数回の reml_array_from 呼び出しが含まれること"
        );

        Ok(())
    }

    fn assert_ordered_occurrences(llvm_ir: &str, labels: &[&str], note: &str) {
        let mut last = None;
        for label in labels {
            let pos = llvm_ir
                .find(label)
                .unwrap_or_else(|| panic!("{note}: {label} が見つかりません"));
            if let Some(prev) = last {
                assert!(
                    prev < pos,
                    "{note}: {label} が想定より前に出現しています"
                );
            }
            last = Some(pos);
        }
    }

    #[test]
    fn llvm_ir_record_literal_layout_is_sorted_by_field_name() -> Result<(), MirSnapshotError> {
        let repo_root = env!("CARGO_MANIFEST_DIR");
        let path = std::path::Path::new(repo_root).join("../../../tmp/mir-record-layout.json");
        let snapshot = generate_snapshot_from_mir_json(
            &path,
            test_target_machine(),
            vec![],
            vec!["phase=test".into()],
            "mir_test",
        )?;

        let point_yxz = snapshot
            .functions
            .iter()
            .find(|func| func.name == "record_layout_point_yxz")
            .expect("record_layout_point_yxz 関数が存在すること");
        let llvm_ir = &point_yxz.llvm_ir;
        assert!(
            llvm_ir.contains("record literal field_count=3 type_name=Point"),
            "type_name 付き record のコメントが含まれること"
        );
        assert_ordered_occurrences(
            llvm_ir,
            &[
                "record field 0 -> y",
                "record field 1 -> x",
                "record field 2 -> z",
            ],
            "record field の評価順序",
        );
        assert_ordered_occurrences(
            llvm_ir,
            &["record slot 0 = x", "record slot 1 = y", "record slot 2 = z"],
            "record slot の格納順序",
        );

        let point_xzy = snapshot
            .functions
            .iter()
            .find(|func| func.name == "record_layout_point_xzy")
            .expect("record_layout_point_xzy 関数が存在すること");
        let llvm_ir = &point_xzy.llvm_ir;
        assert_ordered_occurrences(
            llvm_ir,
            &[
                "record field 0 -> x",
                "record field 1 -> z",
                "record field 2 -> y",
            ],
            "record field の評価順序 (別順序)",
        );
        assert_ordered_occurrences(
            llvm_ir,
            &["record slot 0 = x", "record slot 1 = y", "record slot 2 = z"],
            "record slot の格納順序 (別順序)",
        );

        let dup_fields = snapshot
            .functions
            .iter()
            .find(|func| func.name == "record_layout_duplicate_fields")
            .expect("record_layout_duplicate_fields 関数が存在すること");
        let llvm_ir = &dup_fields.llvm_ir;
        assert_ordered_occurrences(
            llvm_ir,
            &[
                "record field 0 -> a",
                "record field 1 -> a",
                "record field 2 -> b",
            ],
            "record field の評価順序 (duplicate)",
        );
        assert_ordered_occurrences(
            llvm_ir,
            &["record slot 0 = a", "record slot 1 = a", "record slot 2 = b"],
            "record slot の格納順序 (duplicate)",
        );

        Ok(())
    }
}
