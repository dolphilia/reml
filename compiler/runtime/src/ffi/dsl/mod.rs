//! Core.Ffi.Dsl の最小ランタイム実装。
//! 仕様: `docs/spec/3-9-core-async-ffi-unsafe.md` §2.4.1.

use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use serde_json::{Map as JsonMap, Value};
use std::{fmt, sync::Arc};

use crate::{
    audit::AuditEnvelope,
    prelude::ensure::{DiagnosticSeverity, GuardDiagnostic},
};

const FFI_DIAGNOSTIC_DOMAIN: &str = "ffi";
const FFI_WRAP_INVALID_ARGUMENT_CODE: &str = "ffi.wrap.invalid_argument";
const FFI_WRAP_NULL_RETURN_CODE: &str = "ffi.wrap.null_return";
const FFI_WRAP_OWNERSHIP_VIOLATION_CODE: &str = "ffi.wrap.ownership_violation";
const FFI_CALL_EXECUTOR_MISSING_CODE: &str = "ffi.call.executor_missing";
const FFI_CALL_EXECUTOR_ALREADY_SET_CODE: &str = "ffi.call.executor_already_set";
const FFI_SIGNATURE_INVALID_CODE: &str = "ffi.signature.invalid";

static FFI_CALL_EXECUTOR: OnceCell<Arc<dyn FfiCallExecutor>> = OnceCell::new();

/// FFI 型表現。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FfiType {
    Void,
    Bool,
    I8,
    U8,
    I16,
    U16,
    I32,
    U32,
    I64,
    U64,
    F32,
    F64,
    Ptr(Box<FfiType>),
    ConstPtr(Box<FfiType>),
    Struct(FfiStruct),
    Enum(FfiEnum),
    Fn(Box<FfiFnSig>),
}

/// 代表的なプリミティブ型定数。
pub const INT: FfiType = FfiType::I32;
pub const DOUBLE: FfiType = FfiType::F64;

/// ポインタ型を生成する。
pub fn ptr(inner: FfiType) -> FfiType {
    FfiType::Ptr(Box::new(inner))
}

/// const ポインタ型を生成する。
pub fn const_ptr(inner: FfiType) -> FfiType {
    FfiType::ConstPtr(Box::new(inner))
}

/// FFI 関数シグネチャ。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FfiFnSig {
    pub params: Vec<FfiType>,
    pub returns: Box<FfiType>,
    pub variadic: bool,
}

/// FFI シグネチャを構築するヘルパ。
pub fn fn_sig(params: Vec<FfiType>, returns: FfiType, variadic: bool) -> FfiFnSig {
    FfiFnSig {
        params,
        returns: Box::new(returns),
        variadic,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FfiCallSpec {
    pub name: String,
    pub calling_conv: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(alias = "return")]
    pub ret: Option<String>,
    #[serde(default)]
    pub variadic: bool,
}

impl FfiCallSpec {
    pub fn to_signature(&self) -> Result<FfiFnSig, FfiError> {
        let mut params = Vec::with_capacity(self.args.len());
        for label in &self.args {
            params.push(parse_mir_ffi_type(label)?);
        }
        let returns = match &self.ret {
            Some(label) => parse_mir_ffi_type(label)?,
            None => FfiType::Void,
        };
        Ok(FfiFnSig {
            params,
            returns: Box::new(returns),
            variadic: self.variadic,
        })
    }
}

fn parse_mir_ffi_type(label: &str) -> Result<FfiType, FfiError> {
    let trimmed = label.trim();
    if trimmed.is_empty() {
        return Err(FfiError::new(
            FfiErrorKind::InvalidSignature,
            "FFI 型が空のため解析できません",
        )
        .with_code(FFI_SIGNATURE_INVALID_CODE));
    }
    if trimmed.eq_ignore_ascii_case("&str") {
        return Ok(FfiType::ConstPtr(Box::new(FfiType::U8)));
    }
    if let Some(rest) = trimmed.strip_prefix('&') {
        let rest = rest.trim_start();
        let (mutable, inner) = match rest.strip_prefix("mut") {
            Some(after_mut) => (true, after_mut.trim_start()),
            None => (false, rest),
        };
        let inner = if inner.is_empty() {
            FfiType::Void
        } else {
            parse_mir_ffi_type(inner)?
        };
        return Ok(if mutable {
            FfiType::Ptr(Box::new(inner))
        } else {
            FfiType::ConstPtr(Box::new(inner))
        });
    }
    if trimmed.starts_with('[') && trimmed.ends_with(']') && trimmed.len() >= 2 {
        let inner = trimmed[1..trimmed.len() - 1].trim();
        let inner = if inner.is_empty() {
            FfiType::Void
        } else {
            parse_mir_ffi_type(inner)?
        };
        // スライスは FFI 型に直接対応がないため、要素型ポインタとして扱う。
        return Ok(FfiType::Ptr(Box::new(inner)));
    }
    let normalized = trimmed.to_ascii_lowercase();
    match normalized.as_str() {
        "unit" | "void" | "()" => Ok(FfiType::Void),
        "bool" => Ok(FfiType::Bool),
        "i8" | "int8" => Ok(FfiType::I8),
        "u8" | "uint8" => Ok(FfiType::U8),
        "i16" | "int16" => Ok(FfiType::I16),
        "u16" | "uint16" => Ok(FfiType::U16),
        "i32" | "int32" => Ok(FfiType::I32),
        "u32" | "uint32" => Ok(FfiType::U32),
        "i64" | "int64" | "int" => Ok(FfiType::I64),
        "u64" | "uint64" => Ok(FfiType::U64),
        "f32" | "float" => Ok(FfiType::F32),
        "f64" | "double" => Ok(FfiType::F64),
        "pointer" | "ptr" | "i8*" => Ok(FfiType::Ptr(Box::new(FfiType::Void))),
        "string" | "str" => Ok(FfiType::ConstPtr(Box::new(FfiType::U8))),
        _ => Err(FfiError::new(
            FfiErrorKind::InvalidSignature,
            format!("未対応の FFI 型です: {}", label),
        )
        .with_code(FFI_SIGNATURE_INVALID_CODE)),
    }
}

/// 構造体表現。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FfiStruct {
    pub name: String,
    pub fields: Vec<FfiField>,
    pub repr: FfiRepr,
}

/// フィールド定義。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FfiField {
    pub name: String,
    pub ty: FfiType,
}

/// repr 指定。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FfiRepr {
    C,
    Transparent,
    Packed,
}

/// 列挙型表現。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FfiEnum {
    pub name: String,
    pub repr: FfiIntRepr,
    pub variants: Vec<FfiVariant>,
}

/// 列挙バリアント。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FfiVariant {
    pub name: String,
    pub value: Option<i64>,
}

/// 整数 repr。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FfiIntRepr {
    I8,
    U8,
    I16,
    U16,
    I32,
    U32,
    I64,
    U64,
}

/// ライブラリ識別子。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FfiLibraryHandle {
    identifier: String,
}

impl FfiLibraryHandle {
    fn new(identifier: impl Into<String>) -> Self {
        Self {
            identifier: identifier.into(),
        }
    }

    /// 監査ログに利用するライブラリ識別子。
    pub fn audit_label(&self) -> &str {
        &self.identifier
    }
}

/// ライブラリ情報。
#[derive(Debug, Clone)]
pub struct FfiLibrary {
    name: String,
    handle: FfiLibraryHandle,
}

impl FfiLibrary {
    /// ライブラリ名を返す。
    pub fn name(&self) -> &str {
        &self.name
    }

    /// ライブラリハンドルを返す。
    pub fn handle(&self) -> &FfiLibraryHandle {
        &self.handle
    }

    /// シンボルと署名を紐付ける。
    pub fn bind_fn(&self, name: impl AsRef<str>, sig: FfiFnSig) -> Result<FfiRawFn, FfiError> {
        let symbol = name.as_ref().trim();
        if symbol.is_empty() {
            return Err(FfiError::new(
                FfiErrorKind::SymbolNotFound,
                "空のシンボル名はバインドできません",
            ));
        }
        Ok(FfiRawFn {
            symbol: symbol.to_string(),
            signature: sig,
            library: self.handle.clone(),
            call_handler: None,
        })
    }

    pub fn bind_fn_from_mir_spec(&self, spec: &FfiCallSpec) -> Result<FfiRawFn, FfiError> {
        let signature = spec.to_signature()?;
        self.bind_fn(&spec.name, signature)
    }
}

/// ライブラリを解決する。
pub fn bind_library(name: impl AsRef<str>) -> Result<FfiLibrary, FfiError> {
    let label = name.as_ref().trim();
    if label.is_empty() {
        return Err(FfiError::new(
            FfiErrorKind::LibraryNotFound,
            "ライブラリ名が空のため解決できません",
        ));
    }
    Ok(FfiLibrary {
        name: label.to_string(),
        handle: FfiLibraryHandle::new(label),
    })
}

/// 低レベル FFI 関数。
#[derive(Clone)]
pub struct FfiRawFn {
    symbol: String,
    signature: FfiFnSig,
    library: FfiLibraryHandle,
    call_handler: Option<Arc<dyn Fn(&[FfiValue]) -> Result<FfiValue, FfiError> + Send + Sync>>,
}

impl fmt::Debug for FfiRawFn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FfiRawFn")
            .field("symbol", &self.symbol)
            .field("signature", &self.signature)
            .field("library", &self.library.audit_label())
            .finish()
    }
}

impl FfiRawFn {
    /// シンボル名を返す。
    pub fn symbol(&self) -> &str {
        &self.symbol
    }

    /// ライブラリ識別子を返す。
    pub fn library_label(&self) -> &str {
        self.library.audit_label()
    }

    /// シグネチャを返す。
    pub fn signature(&self) -> &FfiFnSig {
        &self.signature
    }

    /// 低レベル呼び出しを行う。
    pub fn call(&self, args: &[FfiValue]) -> Result<FfiValue, FfiError> {
        if let Some(handler) = &self.call_handler {
            return handler(args);
        }
        if let Some(executor) = FFI_CALL_EXECUTOR.get() {
            return executor.call(self, args);
        }
        Err(
            FfiError::new(FfiErrorKind::CallFailed, "FFI 呼び出しエンジンが未登録です")
                .with_code(FFI_CALL_EXECUTOR_MISSING_CODE),
        )
    }

    /// 監査ログを付与して呼び出す。
    pub fn call_with_audit(
        &self,
        args: Vec<FfiValue>,
        envelope: &mut AuditEnvelope,
        effect_flags: &[&str],
    ) -> Result<FfiValue, FfiError> {
        let result = self.call(&args);
        let status = if result.is_ok() { "success" } else { "failed" };
        insert_call_audit_metadata(
            envelope,
            &FfiCallAuditInfo {
                library: self.library.audit_label(),
                symbol: &self.symbol,
                effect_flags,
                status,
                wrapper: None,
                call_site: None,
                capability: None,
                capability_stage: None,
                latency_ns: None,
            },
        );
        result
    }

    /// 呼び出しハンドラを差し替える。
    pub fn with_call_handler(
        mut self,
        handler: Arc<dyn Fn(&[FfiValue]) -> Result<FfiValue, FfiError> + Send + Sync>,
    ) -> Self {
        self.call_handler = Some(handler);
        self
    }
}

/// ラッパー仕様。
#[derive(Debug, Clone)]
pub struct FfiWrapSpec {
    pub name: String,
    pub null_check: bool,
    pub ownership: Option<Ownership>,
    pub error_map: Option<String>,
}

/// 所有権区分。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ownership {
    Borrowed,
    Owned,
    Transferred,
}

impl Ownership {
    pub fn as_str(&self) -> &'static str {
        match self {
            Ownership::Borrowed => "borrowed",
            Ownership::Owned => "owned",
            Ownership::Transferred => "transferred",
        }
    }
}

/// ラッパーの呼び出しモード。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FfiWrapCallMode {
    Wrapped,
    Raw,
}

impl FfiWrapCallMode {
    fn as_str(&self) -> &'static str {
        match self {
            FfiWrapCallMode::Wrapped => "wrapped",
            FfiWrapCallMode::Raw => "raw",
        }
    }
}

/// 安全ラッパー。
#[derive(Debug, Clone)]
pub struct FfiWrappedFn {
    raw: FfiRawFn,
    spec: FfiWrapSpec,
}

impl FfiWrappedFn {
    /// ラッパーの監査メタデータを `AuditEnvelope` に挿入する。
    pub fn apply_audit_metadata(&self, envelope: &mut AuditEnvelope) {
        insert_wrapper_audit_metadata(envelope, &self.spec, FfiWrapCallMode::Wrapped);
        mark_call_wrapper(envelope);
    }

    /// ラッパー呼び出し。
    pub fn call(&self, args: Vec<FfiValue>) -> Result<FfiValue, FfiError> {
        self.validate_arguments(&args)?;
        let result = self.raw.call(&args)?;
        self.validate_return_value(&result)?;
        Ok(result)
    }

    /// 監査ログを付与して呼び出す。
    pub fn call_with_audit(
        &self,
        args: Vec<FfiValue>,
        envelope: &mut AuditEnvelope,
        effect_flags: &[&str],
    ) -> Result<FfiValue, FfiError> {
        self.apply_audit_metadata(envelope);
        let result = self.call(args);
        let status = if result.is_ok() { "success" } else { "failed" };
        insert_call_audit_metadata(
            envelope,
            &FfiCallAuditInfo {
                library: self.raw.library.audit_label(),
                symbol: &self.raw.symbol,
                effect_flags,
                status,
                wrapper: Some("ffi.wrap"),
                call_site: None,
                capability: None,
                capability_stage: None,
                latency_ns: None,
            },
        );
        result
    }

    fn validate_arguments(&self, args: &[FfiValue]) -> Result<(), FfiError> {
        let sig = &self.raw.signature;
        if !sig.variadic && args.len() != sig.params.len() {
            return Err(self.invalid_argument_error());
        }
        if sig.params.len() > args.len() {
            return Err(self.invalid_argument_error());
        }
        for (value, expected) in args.iter().zip(sig.params.iter()) {
            if !value.matches_type(expected) {
                return Err(self.invalid_argument_error());
            }
        }
        Ok(())
    }

    fn validate_return_value(&self, value: &FfiValue) -> Result<(), FfiError> {
        if !value.matches_type(&self.raw.signature.returns) {
            return Err(self.invalid_argument_error());
        }
        if self.spec.null_check && value.is_null_ptr() {
            return Err(self.null_return_error());
        }
        if let Some(ownership) = self.spec.ownership {
            if !self.raw.signature.returns.is_pointer() {
                return Err(self.ownership_violation_error(ownership));
            }
            if value.is_null_ptr() && matches!(ownership, Ownership::Owned | Ownership::Transferred)
            {
                return Err(self.ownership_violation_error(ownership));
            }
        }
        Ok(())
    }

    fn invalid_argument_error(&self) -> FfiError {
        let mut extensions = JsonMap::new();
        let mut wrap_info = JsonMap::new();
        let expected_signature = serde_json::to_value(&self.raw.signature).unwrap_or(Value::Null);
        wrap_info.insert("expected_signature".into(), expected_signature);
        extensions.insert("ffi.wrap".into(), Value::Object(wrap_info));
        FfiError::new(
            FfiErrorKind::InvalidArgument,
            "FFI ラッパーの引数が期待値と一致しません",
        )
        .with_code(FFI_WRAP_INVALID_ARGUMENT_CODE)
        .with_extensions(extensions)
    }

    fn null_return_error(&self) -> FfiError {
        let mut extensions = JsonMap::new();
        let mut wrap_info = JsonMap::new();
        wrap_info.insert("symbol".into(), Value::String(self.raw.symbol.clone()));
        if let Some(ownership) = self.spec.ownership {
            wrap_info.insert("ownership".into(), Value::String(ownership.as_str().into()));
        }
        extensions.insert("ffi.wrap".into(), Value::Object(wrap_info));
        FfiError::new(FfiErrorKind::NullReturn, "FFI 呼び出しが NULL を返しました")
            .with_code(FFI_WRAP_NULL_RETURN_CODE)
            .with_extensions(extensions)
    }

    fn ownership_violation_error(&self, ownership: Ownership) -> FfiError {
        let mut audit_metadata = JsonMap::new();
        audit_metadata.insert(
            "ffi.wrapper.ownership".into(),
            Value::String(ownership.as_str().into()),
        );
        audit_metadata.insert(
            "ffi.wrapper.call_mode".into(),
            Value::String(FfiWrapCallMode::Wrapped.as_str().into()),
        );
        FfiError::new(
            FfiErrorKind::OwnershipViolation,
            "FFI 所有権前提が満たされていません",
        )
        .with_code(FFI_WRAP_OWNERSHIP_VIOLATION_CODE)
        .with_audit_metadata(audit_metadata)
    }
}

/// FFI ラッパを生成する。
pub fn wrap(raw: FfiRawFn, spec: FfiWrapSpec) -> Result<FfiWrappedFn, FfiError> {
    if spec.name.trim().is_empty() {
        return Err(FfiError::new(
            FfiErrorKind::InvalidArgument,
            "ラッパー名が空のため生成できません",
        )
        .with_code(FFI_WRAP_INVALID_ARGUMENT_CODE));
    }
    Ok(FfiWrappedFn { raw, spec })
}

/// FFI への入力値。
#[derive(Debug, Clone)]
pub enum FfiValue {
    Void,
    Bool(bool),
    I8(i8),
    U8(u8),
    I16(i16),
    U16(u16),
    I32(i32),
    U32(u32),
    I64(i64),
    U64(u64),
    F32(f32),
    F64(f64),
    Ptr(Option<usize>),
    ConstPtr(Option<usize>),
    Struct { name: String },
    Enum { name: String, value: i64 },
    FnPtr(usize),
}

impl FfiValue {
    fn matches_type(&self, ty: &FfiType) -> bool {
        match (self, ty) {
            (FfiValue::Void, FfiType::Void) => true,
            (FfiValue::Bool(_), FfiType::Bool) => true,
            (FfiValue::I8(_), FfiType::I8) => true,
            (FfiValue::U8(_), FfiType::U8) => true,
            (FfiValue::I16(_), FfiType::I16) => true,
            (FfiValue::U16(_), FfiType::U16) => true,
            (FfiValue::I32(_), FfiType::I32) => true,
            (FfiValue::U32(_), FfiType::U32) => true,
            (FfiValue::I64(_), FfiType::I64) => true,
            (FfiValue::U64(_), FfiType::U64) => true,
            (FfiValue::F32(_), FfiType::F32) => true,
            (FfiValue::F64(_), FfiType::F64) => true,
            // ポインタの内側型は最小実装では検証しない。
            (FfiValue::Ptr(_), FfiType::Ptr(_)) => true,
            (FfiValue::ConstPtr(_), FfiType::ConstPtr(_)) => true,
            (FfiValue::Struct { name }, FfiType::Struct(def)) => {
                name.is_empty() || name == &def.name
            }
            (FfiValue::Enum { name, .. }, FfiType::Enum(def)) => {
                name.is_empty() || name == &def.name
            }
            (FfiValue::FnPtr(_), FfiType::Fn(_)) => true,
            _ => false,
        }
    }

    fn is_null_ptr(&self) -> bool {
        match self {
            FfiValue::Ptr(Some(addr)) | FfiValue::ConstPtr(Some(addr)) => *addr == 0,
            FfiValue::Ptr(None) | FfiValue::ConstPtr(None) => true,
            _ => false,
        }
    }
}

impl FfiType {
    fn is_pointer(&self) -> bool {
        matches!(self, FfiType::Ptr(_) | FfiType::ConstPtr(_))
    }
}

/// FFI 呼び出しの実行エンジン。
pub trait FfiCallExecutor: Send + Sync {
    fn call(&self, raw: &FfiRawFn, args: &[FfiValue]) -> Result<FfiValue, FfiError>;
}

/// FFI 呼び出しエンジンを登録する。
pub fn set_ffi_call_executor(executor: Arc<dyn FfiCallExecutor>) -> Result<(), FfiError> {
    FFI_CALL_EXECUTOR.set(executor).map_err(|_| {
        FfiError::new(
            FfiErrorKind::CallFailed,
            "FFI 呼び出しエンジンは既に登録されています",
        )
        .with_code(FFI_CALL_EXECUTOR_ALREADY_SET_CODE)
    })
}

/// FFI エラーの種別。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FfiErrorKind {
    LibraryNotFound,
    SymbolNotFound,
    InvalidSignature,
    InvalidArgument,
    NullReturn,
    OwnershipViolation,
    CallFailed,
}

/// FFI エラー。
#[derive(Debug, Clone)]
pub struct FfiError {
    pub kind: FfiErrorKind,
    pub message: String,
    pub diagnostic_code: Option<&'static str>,
    pub severity: DiagnosticSeverity,
    pub extensions: JsonMap<String, Value>,
    pub audit_metadata: JsonMap<String, Value>,
}

impl FfiError {
    pub fn new(kind: FfiErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            diagnostic_code: None,
            severity: DiagnosticSeverity::Error,
            extensions: JsonMap::new(),
            audit_metadata: JsonMap::new(),
        }
    }

    pub fn with_code(mut self, code: &'static str) -> Self {
        self.diagnostic_code = Some(code);
        self
    }

    pub fn with_extensions(mut self, extensions: JsonMap<String, Value>) -> Self {
        self.extensions = extensions;
        self
    }

    pub fn with_audit_metadata(mut self, audit_metadata: JsonMap<String, Value>) -> Self {
        self.audit_metadata = audit_metadata;
        self
    }

    pub fn diagnostic_code(&self) -> Option<&'static str> {
        self.diagnostic_code
    }

    /// Guard 診断へ変換する。
    pub fn into_guard_diagnostic(self) -> GuardDiagnostic {
        GuardDiagnostic {
            code: self.diagnostic_code.unwrap_or("ffi.error"),
            domain: FFI_DIAGNOSTIC_DOMAIN,
            severity: self.severity,
            message: self.message,
            notes: Vec::new(),
            extensions: self.extensions,
            audit_metadata: self.audit_metadata,
        }
    }
}

impl fmt::Display for FfiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for FfiError {}

/// `ffi.wrapper` 監査メタデータを挿入する。
pub fn insert_wrapper_audit_metadata(
    envelope: &mut AuditEnvelope,
    spec: &FfiWrapSpec,
    call_mode: FfiWrapCallMode,
) {
    let mut wrapper = JsonMap::new();
    wrapper.insert("name".into(), Value::String(spec.name.clone()));
    wrapper.insert("null_check".into(), Value::Bool(spec.null_check));
    if let Some(ownership) = spec.ownership {
        wrapper.insert("ownership".into(), Value::String(ownership.as_str().into()));
    }
    if let Some(error_map) = &spec.error_map {
        wrapper.insert("error_map".into(), Value::String(error_map.clone()));
    }
    wrapper.insert("call_mode".into(), Value::String(call_mode.as_str().into()));
    envelope
        .metadata
        .insert("ffi.wrapper".into(), Value::Object(wrapper));
}

/// `ffi.call` の `wrapper` 属性を付与する。
pub fn mark_call_wrapper(envelope: &mut AuditEnvelope) {
    let wrapper = Value::String("ffi.wrap".into());
    match envelope.metadata.get_mut("ffi") {
        Some(Value::Object(obj)) => {
            obj.insert("wrapper".into(), wrapper);
        }
        _ => {
            let mut obj = JsonMap::new();
            obj.insert("wrapper".into(), wrapper);
            envelope.metadata.insert("ffi".into(), Value::Object(obj));
        }
    }
}

/// `ffi.call` 用の監査メタデータ。
pub struct FfiCallAuditInfo<'a> {
    pub library: &'a str,
    pub symbol: &'a str,
    pub effect_flags: &'a [&'a str],
    pub status: &'a str,
    pub wrapper: Option<&'a str>,
    pub call_site: Option<&'a str>,
    pub capability: Option<&'a str>,
    pub capability_stage: Option<&'a str>,
    pub latency_ns: Option<u64>,
}

/// `ffi.call` 監査メタデータを挿入する。
pub fn insert_call_audit_metadata(envelope: &mut AuditEnvelope, info: &FfiCallAuditInfo<'_>) {
    let mut effect_flags = info
        .effect_flags
        .iter()
        .map(|flag| Value::String((*flag).to_string()))
        .collect::<Vec<_>>();
    effect_flags.sort_by(|a, b| a.as_str().cmp(&b.as_str()));
    let mut obj = JsonMap::new();
    obj.insert("event".into(), Value::String("ffi.call".into()));
    obj.insert("library".into(), Value::String(info.library.to_string()));
    obj.insert("symbol".into(), Value::String(info.symbol.to_string()));
    if let Some(call_site) = info.call_site {
        obj.insert("call_site".into(), Value::String(call_site.to_string()));
    }
    obj.insert("effect_flags".into(), Value::Array(effect_flags));
    if let Some(latency) = info.latency_ns {
        obj.insert(
            "latency_ns".into(),
            Value::Number(serde_json::Number::from(latency)),
        );
    }
    obj.insert("status".into(), Value::String(info.status.to_string()));
    if let Some(capability) = info.capability {
        obj.insert("capability".into(), Value::String(capability.to_string()));
    }
    if let Some(stage) = info.capability_stage {
        obj.insert("capability_stage".into(), Value::String(stage.to_string()));
    }
    if let Some(wrapper) = info.wrapper {
        obj.insert("wrapper".into(), Value::String(wrapper.to_string()));
    }
    envelope.metadata.insert("ffi".into(), Value::Object(obj));
}

#[cfg(test)]
mod tests {
    use super::{FfiCallSpec, FfiType};
    use serde_json::Value;

    #[test]
    fn ffi_call_spec_from_mir_json_variadic_and_types() {
        let payload = r#"
{
  "functions": [
    {
      "name": "@main",
      "ffi_calls": [
        {
          "name": "printf",
          "calling_conv": "ccc",
          "args": ["i32", "&mut i64", "[u8]", "string", "()"],
          "return": "i32",
          "variadic": true
        }
      ]
    }
  ]
}
"#;
        let json: Value = serde_json::from_str(payload).expect("MIR JSON を parse できること");
        let call = json["functions"][0]["ffi_calls"][0].clone();
        let spec: FfiCallSpec = serde_json::from_value(call).expect("FfiCallSpec に変換できること");
        let sig = spec.to_signature().expect("FfiFnSig へ変換できること");
        assert!(sig.variadic, "variadic が維持されること");
        assert_eq!(
            sig.params,
            vec![
                FfiType::I32,
                FfiType::Ptr(Box::new(FfiType::I64)),
                FfiType::Ptr(Box::new(FfiType::U8)),
                FfiType::ConstPtr(Box::new(FfiType::U8)),
                FfiType::Void,
            ]
        );
        assert_eq!(*sig.returns, FfiType::I32);
    }
}
