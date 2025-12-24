//! Rust 側から Reml ランタイムへの FFI アクセスを提供する最小層。
//! 手動定義の `extern` 宣言と安全なラッパーをまとめる。
use crate::ffi_contract::{
    check_contract, emit_contract_violation, maybe_log_stage_mismatch, ContractViolation,
};
use serde_json::{json, Value};
use std::{
    cell::RefCell,
    ffi::CStr,
    fmt::{self, Display},
    mem,
    os::raw::{c_char, c_void},
    ptr::NonNull,
};

mod audit;
mod capability_handle;
mod capability_metadata;
mod capability;
#[cfg(feature = "core_prelude")]
#[path = "../../src/collections/mod.rs"]
pub mod collections;
#[cfg(feature = "core_prelude")]
#[path = "../../src/config/mod.rs"]
pub mod config;
#[cfg(feature = "core_prelude")]
#[path = "../../src/data/mod.rs"]
pub mod data;
#[cfg(feature = "core_prelude")]
pub mod core_collections_metrics;
#[cfg(feature = "core_prelude")]
pub mod core_prelude;
#[cfg(feature = "core_prelude")]
pub use core_prelude as prelude;
#[cfg(feature = "runtime_support")]
#[path = "../../src/io/mod.rs"]
pub mod io;
#[cfg(feature = "runtime_support")]
#[path = "../../src/test/mod.rs"]
pub mod test;
pub mod stage {
    pub use crate::capability_metadata::{StageId, StageParseError, StageRequirement};
}
#[cfg(all(feature = "core_prelude", feature = "runtime_support"))]
#[path = "../../src/text/mod.rs"]
pub mod text;
#[cfg(all(feature = "core_prelude", not(feature = "runtime_support")))]
pub mod text {
    use serde_json::{Map, Value};

    pub fn take_text_audit_metadata() -> Option<Map<String, Value>> {
        None
    }
}
mod env;
mod ffi_contract;
#[cfg(feature = "core_prelude")]
mod handles;
mod manifest_contract;
mod registry;
#[path = "../../src/runtime/mod.rs"]
#[cfg(feature = "runtime_support")]
pub mod runtime;
mod security;

pub use audit::{AuditContext, AuditEntry, AuditError, AuditSink};
pub use capability_handle::{
    AsyncRuntimeCapability, AuditCapability, CapabilityHandle, GcCapability, IoCapability,
    MetricsCapability, PluginCapability, SecurityCapability,
};
pub use capability_metadata::{
    CapabilityDescriptor, CapabilityId, CapabilityProvider, StageId, StageRequirement,
};
pub use env::{
    get_env, remove_env, set_env, EnvAdapterError, EnvContext as RuntimeEnvContext, EnvError,
    PlatformSnapshot,
};
#[cfg(feature = "core_prelude")]
pub use handles::{register_ref_capability, register_table_csv_capability, RefHandle};
pub use manifest_contract::{
    CapabilityContractSpan, ConductorCapabilityContract, ConductorCapabilityRequirement,
};
pub use registry::{
    BridgeIntent, BridgeStageTraceStep, CapabilityError, CapabilityRegistry, RuntimeBridgeRegistry,
    RuntimeBridgeStreamSignal,
};
pub use security::{CallOptions, SecurityError, SecurityPolicy};

/// Reml ランタイムの文字列表現（`{ ptr, i64 }`）。
#[repr(C)]
#[derive(Copy, Clone)]
pub struct ReMlString {
    /// UTF-8 データへのポインタ。
    pub data: *const c_char,
    /// バイト数。
    pub length: i64,
}

impl ReMlString {
    /// 生のバイト列を取得する（NULL チェック付き）。
    pub unsafe fn as_bytes(&self) -> Option<&[u8]> {
        if self.data.is_null() {
            return None;
        }
        if self.length < 0 {
            return None;
        }
        Some(std::slice::from_raw_parts(
            self.data as *const u8,
            self.length as usize,
        ))
    }

    /// UTF-8 文字列として解釈する。
    pub unsafe fn as_str(&self) -> Option<&str> {
        self.as_bytes()
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
    }
}

/// 機能ブリッジの状態を監査ログに送るためのステータス。
#[repr(i32)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BridgeStatus {
    /// 正常終了。
    Ok = 0,
    /// 借用経路。
    Borrowed = 1,
    /// 移譲経路。
    Transferred = 2,
    /// その他の異常。
    Failure = 100,
}

impl BridgeStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            BridgeStatus::Ok => "ok",
            BridgeStatus::Borrowed => "borrowed",
            BridgeStatus::Transferred => "transferred",
            BridgeStatus::Failure => "failure",
        }
    }
}

/// 所有権付きポインタ。`inc_ref`/`dec_ref` を自動化する。
pub struct ForeignPtr {
    ptr: NonNull<c_void>,
}

/// ソースコードの位置範囲を保持する Span。
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

impl Span {
    /// `end < start` のときは `start` に丸める。
    pub const fn new(start: u32, end: u32) -> Self {
        if end < start {
            Self { start, end: start }
        } else {
            Self { start, end }
        }
    }

    /// 長さ（バイト数）。
    pub const fn len(&self) -> u32 {
        self.end.saturating_sub(self.start)
    }

    /// 空範囲か。
    pub const fn is_empty(&self) -> bool {
        self.start >= self.end
    }
}

/// 所有権移動の分類。Audit の `bridge.ownership` に対応する。
#[repr(i32)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Ownership {
    Borrowed = 1,
    Transferred = 2,
    Pinned = 3,
}

impl Ownership {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Ownership::Borrowed => "borrowed",
            Ownership::Transferred => "transferred",
            Ownership::Pinned => "pinned",
        }
    }
}

const RETURN_WRAP_STATUS: &str = "wrap";
const RETURN_WRAP_AND_RELEASE_STATUS: &str = "wrap_and_release";
const RETURN_FAILURE_STATUS: &str = "failure";
const RETURN_WRAP_FN: &str = "wrap_foreign_ptr";
const RETURN_RELEASE_HANDLER_NONE: &str = "none";
const RETURN_RELEASE_HANDLER_DEC_REF: &str = "dec_ref";
const RETURN_RC_NONE: &str = "none";
const RETURN_RC_DEC_REF: &str = "dec_ref";

/// Audit 用に渡すデータを一括管理する構造体。
#[derive(Copy, Clone)]
pub struct BridgeReturnAuditMetadata<'a> {
    pub ownership: Ownership,
    pub status: &'a str,
    pub wrap: &'a str,
    pub release_handler: &'a str,
    pub rc_adjustment: &'a str,
}

impl<'a> BridgeReturnAuditMetadata<'a> {
    fn as_entries(&self) -> BridgeReturnAuditEntries<'a> {
        BridgeReturnAuditEntries {
            ownership: self.ownership.as_str(),
            status: self.status,
            wrap: self.wrap,
            release_handler: self.release_handler,
            rc_adjustment: self.rc_adjustment,
        }
    }

    pub const fn borrowed_wrap() -> Self {
        Self {
            ownership: Ownership::Borrowed,
            status: RETURN_WRAP_STATUS,
            wrap: RETURN_WRAP_FN,
            release_handler: RETURN_RELEASE_HANDLER_NONE,
            rc_adjustment: RETURN_RC_NONE,
        }
    }

    pub const fn transferred_wrap_and_release() -> Self {
        Self {
            ownership: Ownership::Transferred,
            status: RETURN_WRAP_AND_RELEASE_STATUS,
            wrap: RETURN_WRAP_FN,
            release_handler: RETURN_RELEASE_HANDLER_DEC_REF,
            rc_adjustment: RETURN_RC_DEC_REF,
        }
    }

    pub const fn failure(ownership: Ownership) -> Self {
        Self {
            ownership,
            status: RETURN_FAILURE_STATUS,
            wrap: RETURN_RELEASE_HANDLER_NONE,
            release_handler: RETURN_RELEASE_HANDLER_NONE,
            rc_adjustment: RETURN_RC_NONE,
        }
    }

    pub fn pending(ownership: Ownership) -> Self {
        Self {
            ownership,
            status: "pending",
            wrap: "wrap_foreign_ptr",
            release_handler: "dec_ref",
            rc_adjustment: "none",
        }
    }

    pub fn with_status(self, status: &'a str) -> Self {
        Self { status, ..self }
    }
}

/// `AuditEnvelope.metadata.bridge` に渡すメタデータ。
#[derive(Copy, Clone)]
pub struct BridgeAuditMetadata<'a> {
    pub status: BridgeStatus,
    pub ownership: Ownership,
    pub span: Span,
    pub target: &'a str,
    pub arch: &'a str,
    pub platform: &'a str,
    pub abi: &'a str,
    pub expected_abi: &'a str,
    pub symbol: &'a str,
    pub extern_symbol: &'a str,
    pub extern_name: &'a str,
    pub link_name: &'a str,
    pub return_info: BridgeReturnAuditMetadata<'a>,
}

impl<'a> BridgeAuditMetadata<'a> {
    /// `AuditEnvelope.metadata.bridge` に対応する文字列化されたキー一覧。
    pub fn as_entries(&self) -> BridgeAuditEntries<'a> {
        BridgeAuditEntries {
            status: self.status.as_str(),
            ownership: self.ownership.as_str(),
            span: self.span,
            target: self.target,
            arch: self.arch,
            platform: self.platform,
            abi: self.abi,
            expected_abi: self.expected_abi,
            symbol: self.symbol,
            extern_symbol: self.extern_symbol,
            extern_name: self.extern_name,
            link_name: self.link_name,
            return_info: self.return_info.as_entries(),
        }
    }

    pub fn with_return_info(&self, return_info: BridgeReturnAuditMetadata<'a>) -> Self {
        Self {
            return_info,
            ..*self
        }
    }
}

pub struct BridgeReturnAuditEntries<'a> {
    pub ownership: &'static str,
    pub status: &'a str,
    pub wrap: &'a str,
    pub release_handler: &'a str,
    pub rc_adjustment: &'a str,
}

pub struct BridgeAuditEntries<'a> {
    pub status: &'static str,
    pub ownership: &'static str,
    pub span: Span,
    pub target: &'a str,
    pub arch: &'a str,
    pub platform: &'a str,
    pub abi: &'a str,
    pub expected_abi: &'a str,
    pub symbol: &'a str,
    pub extern_symbol: &'a str,
    pub extern_name: &'a str,
    pub link_name: &'a str,
    pub return_info: BridgeReturnAuditEntries<'a>,
}

/// `RuntimeString::to_bridge_metadata` に渡す設定。
#[derive(Copy, Clone)]
pub struct BridgeAuditMetadataArgs<'a> {
    pub status: BridgeStatus,
    pub target: &'a str,
    pub arch: &'a str,
    pub platform: &'a str,
    pub abi: &'a str,
    pub expected_abi: &'a str,
    pub symbol: &'a str,
    pub extern_symbol: &'a str,
    pub extern_name: &'a str,
    pub link_name: &'a str,
    pub return_info: BridgeReturnAuditMetadata<'a>,
}

/// ランタイム文字列と Span を組み合わせたラッパ。
pub struct RuntimeString {
    inner: ReMlString,
    span: Span,
    ownership: Ownership,
}

impl RuntimeString {
    /// ポインタから構築するユーティリティ。
    pub unsafe fn from_parts(
        ptr: *const c_char,
        len: i64,
        span: Span,
        ownership: Ownership,
    ) -> Option<Self> {
        if ptr.is_null() {
            return None;
        }
        Some(Self {
            inner: ReMlString {
                data: ptr,
                length: len,
            },
            span,
            ownership,
        })
    }

    /// 内部文字列を `&str` として解釈する。
    pub fn as_str(&self) -> Option<&str> {
        unsafe { self.inner.as_str() }
    }

    /// Span を返す。
    pub fn span(&self) -> Span {
        self.span
    }

    /// 所有権カテゴリを返す。
    pub fn ownership(&self) -> Ownership {
        self.ownership
    }

    /// Audit に渡すメタデータ。
    pub fn to_bridge_metadata<'a>(
        &'a self,
        args: BridgeAuditMetadataArgs<'a>,
    ) -> BridgeAuditMetadata<'a> {
        BridgeAuditMetadata {
            status: args.status,
            ownership: self.ownership,
            span: self.span,
            target: args.target,
            arch: args.arch,
            platform: args.platform,
            abi: args.abi,
            expected_abi: args.expected_abi,
            symbol: args.symbol,
            extern_symbol: args.extern_symbol,
            extern_name: args.extern_name,
            link_name: args.link_name,
            return_info: args.return_info,
        }
    }
}

thread_local! {
    static RECORDED_RETURN_INFO: RefCell<Option<BridgeReturnAuditMetadata<'static>>> =
        RefCell::new(None);
}

fn record_bridge_return_metadata(metadata: BridgeReturnAuditMetadata<'static>) {
    RECORDED_RETURN_INFO.with(|slot| {
        *slot.borrow_mut() = Some(metadata);
    });
}

fn take_bridge_return_metadata() -> Option<BridgeReturnAuditMetadata<'static>> {
    RECORDED_RETURN_INFO.with(|slot| slot.borrow_mut().take())
}

fn note_borrowed_return() {
    record_bridge_return_metadata(BridgeReturnAuditMetadata::borrowed_wrap());
}

fn note_transferred_return() {
    record_bridge_return_metadata(BridgeReturnAuditMetadata::transferred_wrap_and_release());
}

impl ForeignPtr {
    /// 生ポインタから安全なハンドルを構築する。
    pub unsafe fn from_raw(ptr: *mut c_void) -> Option<Self> {
        NonNull::new(ptr).map(|ptr| ForeignPtr { ptr })
    }

    /// 新しいオブジェクトを割り当てる。
    pub fn allocate_payload(size: usize) -> Self {
        let raw = unsafe { mem_alloc(size) };
        let ptr = NonNull::new(raw)
            .unwrap_or_else(|| panic!("mem_alloc が NULL を返しました（size = {}）", size));
        ForeignPtr { ptr }
    }

    /// 保持中のポインタを取り出す。
    pub fn as_ptr(&self) -> *mut c_void {
        self.ptr.as_ptr()
    }
}

impl Clone for ForeignPtr {
    fn clone(&self) -> Self {
        unsafe {
            inc_ref(self.ptr.as_ptr());
        }
        ForeignPtr { ptr: self.ptr }
    }
}

impl Drop for ForeignPtr {
    fn drop(&mut self) {
        unsafe {
            dec_ref(self.ptr.as_ptr());
        }
    }
}

// `reml_runtime` ライブラリへのリンク。
#[link(name = "reml_runtime")]
extern "C" {
    fn mem_alloc(size: usize) -> *mut c_void;
    #[allow(dead_code)]
    fn mem_free(ptr: *mut c_void);
    fn inc_ref(ptr: *mut c_void);
    fn dec_ref(ptr: *mut c_void);
    fn panic(msg: *const c_char, len: i64);
    fn print_i64(value: i64);
    fn string_eq(a: *const ReMlString, b: *const ReMlString) -> i32;
    fn string_compare(a: *const ReMlString, b: *const ReMlString) -> i32;
    fn reml_ffi_bridge_record_status(status: i32);
    fn reml_ffi_acquire_borrowed_result(ptr: *mut c_void) -> *mut c_void;
    fn reml_ffi_acquire_transferred_result(ptr: *mut c_void) -> *mut c_void;
}

/// パニックメッセージをランタイムへ伝える。
pub fn runtime_panic(message: &CStr) -> ! {
    let len = message.to_bytes().len() as i64;
    unsafe {
        panic(message.as_ptr(), len);
    }
    unreachable!("panic は戻りません");
}

/// デバッグ用の整数出力。
pub fn print_i64_debug(value: i64) {
    unsafe {
        print_i64(value);
    }
}

/// `ReMlString` による等価比較。
pub fn string_eq_bridge(a: &ReMlString, b: &ReMlString) -> bool {
    unsafe { string_eq(a as *const _, b as *const _) != 0 }
}

/// `ReMlString` の順序比較。
pub fn string_compare_bridge(a: &ReMlString, b: &ReMlString) -> i32 {
    unsafe { string_compare(a as *const _, b as *const _) }
}

/// 監査ログへステータスを送信。
pub fn record_bridge_status(status: BridgeStatus) {
    unsafe {
        reml_ffi_bridge_record_status(status as i32);
    }
}

/// 手動で借用経路のハンドルを取得。
pub fn acquire_borrowed_result(source: &ForeignPtr) -> ForeignPtr {
    let raw = unsafe { reml_ffi_acquire_borrowed_result(source.as_ptr()) };
    note_borrowed_return();
    unsafe {
        ForeignPtr::from_raw(raw)
            .unwrap_or_else(|| panic!("reml_ffi_acquire_borrowed_result が NULL を返しました"))
    }
}

/// 所有権が移譲された結果を取得し、元のハンドルの `Drop` を防ぐ。
pub fn acquire_transferred_result(source: ForeignPtr) -> ForeignPtr {
    let raw = unsafe { reml_ffi_acquire_transferred_result(source.as_ptr()) };
    note_transferred_return();
    mem::forget(source);
    unsafe {
        ForeignPtr::from_raw(raw)
            .unwrap_or_else(|| panic!("reml_ffi_acquire_transferred_result が NULL を返しました"))
    }
}

/// Audit で使用する `bridge.status` を `extern` 呼び出しで記録する補助。
pub fn record_bridge_with_metadata(
    ctx: &AuditContext,
    event: impl Into<String>,
    meta: &BridgeAuditMetadata<'_>,
    payload: Value,
) -> Result<(), AuditError> {
    record_bridge_status(meta.status);
    ctx.log_bridge_metadata(event, meta, payload)
}

/// FFI 呼び出しの共通処理: capability と audit を検証し、ライフサイクル監査を記録する。
pub fn audited_bridge_call<F, R>(
    options: &CallOptions,
    capability_id: &str,
    symbol: &str,
    metadata: &BridgeAuditMetadata<'_>,
    body: F,
) -> Result<R, BridgeError>
where
    F: FnOnce(&AuditContext, &CapabilityDescriptor) -> Result<R, BridgeError>,
{
    let registry = CapabilityRegistry::registry();
    let ctx = options.new_context(symbol).map_err(BridgeError::Audit)?;
    let handle = match registry.verify_capability_handle(
        capability_id,
        options.stage_requirement,
        &options.security_policy.required_effects,
    ) {
        Ok(handle) => handle,
        Err(err) => {
            maybe_log_stage_mismatch(
                &ctx,
                metadata,
                &registry,
                capability_id,
                options.stage_requirement,
                &options.security_policy.required_effects,
                &err,
            )
            .map_err(BridgeError::Audit)?;
            return Err(BridgeError::Capability(err));
        }
    };

    options
        .security_policy
        .verify(handle.descriptor())
        .map_err(BridgeError::Security)?;

    if let Some(violation) = check_contract(metadata) {
        emit_contract_violation(&ctx, metadata, violation.clone()).map_err(BridgeError::Audit)?;
        return Err(BridgeError::Contract(violation));
    }

    let log_payload = json!({
        "capability": handle.descriptor().id,
        "stage": handle.descriptor().stage.to_string(),
        "symbol": symbol,
    });
    record_bridge_with_metadata(&ctx, "ffi.call.start", metadata, log_payload)
        .map_err(BridgeError::Audit)?;

    take_bridge_return_metadata();
    let result = body(&ctx, handle.descriptor());
    let status = match &result {
        Ok(_) => BridgeStatus::Ok,
        Err(_) => BridgeStatus::Failure,
    };
    let end_payload = json!({
        "capability": handle.descriptor().id,
        "stage": handle.descriptor().stage.to_string(),
        "symbol": symbol,
        "status": status.as_str(),
    });
    record_bridge_with_metadata(&ctx, "ffi.call.end", metadata, end_payload)
        .map_err(BridgeError::Audit)?;

    let recorded_return_info = take_bridge_return_metadata();
    let base_return_info = recorded_return_info.unwrap_or(metadata.return_info);
    let failure_return_info = recorded_return_info
        .map(|info| info.with_status(RETURN_FAILURE_STATUS))
        .unwrap_or_else(|| BridgeReturnAuditMetadata::failure(metadata.return_info.ownership));
    let result_return_info = if status == BridgeStatus::Ok {
        base_return_info
    } else {
        failure_return_info
    };
    let result_metadata = metadata.with_return_info(result_return_info);
    let result_payload = json!({
        "capability": handle.descriptor().id,
        "stage": handle.descriptor().stage.to_string(),
        "symbol": symbol,
        "status": status.as_str(),
        "return_status": result_return_info.status,
    });
    record_bridge_with_metadata(&ctx, "ffi.call.result", &result_metadata, result_payload)
        .map_err(BridgeError::Audit)?;

    result
}

/// Bridge 連携で発生するエラー。
#[derive(Debug)]
pub enum BridgeError {
    Capability(CapabilityError),
    Security(SecurityError),
    Audit(AuditError),
    Contract(ContractViolation),
}

impl From<CapabilityError> for BridgeError {
    fn from(err: CapabilityError) -> Self {
        BridgeError::Capability(err)
    }
}

impl From<SecurityError> for BridgeError {
    fn from(err: SecurityError) -> Self {
        BridgeError::Security(err)
    }
}

impl From<AuditError> for BridgeError {
    fn from(err: AuditError) -> Self {
        BridgeError::Audit(err)
    }
}

impl From<ContractViolation> for BridgeError {
    fn from(err: ContractViolation) -> Self {
        BridgeError::Contract(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::c_char;

    #[test]
    fn span_length_and_empty() {
        let span = Span::new(3, 7);
        assert_eq!(span.len(), 4);
        assert!(!span.is_empty());

        let reversed = Span::new(10, 5);
        assert_eq!(reversed.start, 10);
        assert_eq!(reversed.end, 10);
        assert!(reversed.is_empty());
    }

    #[test]
    fn runtime_string_metadata_roundtrip() {
        let data = b"bridged\x00";
        let ptr = data.as_ptr() as *const c_char;
        let span = Span::new(0, 7);
        let rt_string =
            unsafe { RuntimeString::from_parts(ptr, 7, span, Ownership::Borrowed).unwrap() };
        let metadata = rt_string.to_bridge_metadata(BridgeAuditMetadataArgs {
            status: BridgeStatus::Borrowed,
            target: "local",
            arch: "x86_64",
            platform: "linux-x64",
            abi: "sysv",
            expected_abi: "sysv",
            symbol: "foo",
            extern_symbol: "foo",
            extern_name: "foo",
            link_name: "foo",
            return_info: BridgeReturnAuditMetadata::pending(Ownership::Borrowed),
        });
        let entries = metadata.as_entries();
        assert_eq!(entries.status, "borrowed");
        assert_eq!(entries.ownership, "borrowed");
        assert_eq!(entries.span.len(), 7);
        assert_eq!(entries.target, "local");
        assert_eq!(entries.arch, "x86_64");
        assert_eq!(entries.return_info.status, "pending");
        assert_eq!(entries.return_info.wrap, "wrap_foreign_ptr");
    }

    #[test]
    fn borrowed_return_metadata_is_tracked() {
        let _ = take_bridge_return_metadata();
        note_borrowed_return();
        let entry =
            take_bridge_return_metadata().expect("borrowed return metadata が記録されているはず");
        assert_eq!(entry.status, RETURN_WRAP_STATUS);
        assert_eq!(entry.wrap, RETURN_WRAP_FN);
        assert_eq!(entry.release_handler, RETURN_RELEASE_HANDLER_NONE);
        assert_eq!(entry.rc_adjustment, RETURN_RC_NONE);
        assert_eq!(entry.ownership, Ownership::Borrowed);
    }

    #[test]
    fn transferred_return_metadata_is_tracked() {
        let _ = take_bridge_return_metadata();
        note_transferred_return();
        let entry = take_bridge_return_metadata()
            .expect("transferred return metadata が記録されているはず");
        assert_eq!(entry.status, RETURN_WRAP_AND_RELEASE_STATUS);
        assert_eq!(entry.wrap, RETURN_WRAP_FN);
        assert_eq!(entry.release_handler, RETURN_RELEASE_HANDLER_DEC_REF);
        assert_eq!(entry.rc_adjustment, RETURN_RC_DEC_REF);
        assert_eq!(entry.ownership, Ownership::Transferred);
    }
}

impl Display for ForeignPtr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<ForeignPtr ptr={:?}>", self.ptr)
    }
}
