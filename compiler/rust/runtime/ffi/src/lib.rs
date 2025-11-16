//! Rust 側から Reml ランタイムへの FFI アクセスを提供する最小層。
//! 手動定義の `extern` 宣言と安全なラッパーをまとめる。
use serde_json::json;
use std::{
    ffi::CStr,
    fmt::{self, Display},
    mem,
    os::raw::{c_char, c_void},
    ptr::NonNull,
};

mod audit;
mod capability_handle;
mod capability_metadata;
mod manifest_contract;
mod registry;
mod security;

pub use audit::{AuditContext, AuditEntry, AuditError, AuditSink};
pub use capability_handle::{
    AsyncRuntimeCapability, AuditCapability, CapabilityHandle, GcCapability, IoCapability,
    MetricsCapability, PluginCapability, SecurityCapability,
};
pub use capability_metadata::{
    CapabilityDescriptor, CapabilityId, CapabilityProvider, StageId, StageRequirement,
};
pub use manifest_contract::{ConductorCapabilityContract, ConductorCapabilityRequirement};
pub use registry::{CapabilityError, CapabilityRegistry};
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
#[derive(Copy, Clone, Debug)]
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

/// Audit 用に渡すデータを一括管理する構造体。
pub struct BridgeAuditMetadata<'a> {
    pub status: BridgeStatus,
    pub ownership: Ownership,
    pub span: Span,
    pub target: &'a str,
    pub platform: &'a str,
    pub abi: &'a str,
    pub symbol: &'a str,
}

impl<'a> BridgeAuditMetadata<'a> {
    /// `AuditEnvelope.metadata.bridge` に対応する文字列化されたキー一覧。
    pub fn as_entries(&self) -> BridgeAuditEntries<'a> {
        BridgeAuditEntries {
            status: self.status.as_str(),
            ownership: self.ownership.as_str(),
            span: self.span,
            target: self.target,
            platform: self.platform,
            abi: self.abi,
            symbol: self.symbol,
        }
    }
}

pub struct BridgeAuditEntries<'a> {
    pub status: &'static str,
    pub ownership: &'static str,
    pub span: Span,
    pub target: &'a str,
    pub platform: &'a str,
    pub abi: &'a str,
    pub symbol: &'a str,
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
        status: BridgeStatus,
        target: &'a str,
        platform: &'a str,
        abi: &'a str,
        symbol: &'a str,
    ) -> BridgeAuditMetadata<'a> {
        BridgeAuditMetadata {
            status,
            ownership: self.ownership,
            span: self.span,
            target,
            platform,
            abi,
            symbol,
        }
    }
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
    unsafe {
        ForeignPtr::from_raw(raw)
            .unwrap_or_else(|| panic!("reml_ffi_acquire_borrowed_result が NULL を返しました"))
    }
}

/// 所有権が移譲された結果を取得し、元のハンドルの `Drop` を防ぐ。
pub fn acquire_transferred_result(source: ForeignPtr) -> ForeignPtr {
    let raw = unsafe { reml_ffi_acquire_transferred_result(source.as_ptr()) };
    mem::forget(source);
    unsafe {
        ForeignPtr::from_raw(raw)
            .unwrap_or_else(|| panic!("reml_ffi_acquire_transferred_result が NULL を返しました"))
    }
}

/// Audit で使用する `bridge.status` を `extern` 呼び出しで記録する補助。
pub fn record_bridge_with_metadata(meta: &BridgeAuditMetadata<'_>) {
    record_bridge_status(meta.status);
    // TODO: AuditContext への記録は上位レイヤで処理する。
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
    let handle = registry
        .verify_capability_stage(
            capability_id,
            options.stage_requirement,
            &options.security_policy.required_effects,
        )
        .map_err(BridgeError::Capability)?;

    options
        .security_policy
        .verify(handle.descriptor())
        .map_err(BridgeError::Security)?;

    let ctx = options.new_context(symbol).map_err(BridgeError::Audit)?;

    let log_payload = json!({
        "capability": handle.descriptor().id,
        "stage": handle.descriptor().stage.to_string(),
        "symbol": symbol,
    });
    ctx.log_bridge_metadata("ffi.call.start", metadata, log_payload)
        .map_err(BridgeError::Audit)?;

    let result = body(&ctx, handle.descriptor());
    let status = match &result {
        Ok(_) => BridgeStatus::Ok,
        Err(_) => BridgeStatus::Failure,
    };
    let log_payload = json!({
        "capability": handle.descriptor().id,
        "stage": handle.descriptor().stage.to_string(),
        "symbol": symbol,
        "status": status.as_str(),
    });
    ctx.log_bridge_metadata("ffi.call.end", metadata, log_payload)
        .map_err(BridgeError::Audit)?;

    result
}

/// Bridge 連携で発生するエラー。
#[derive(Debug)]
pub enum BridgeError {
    Capability(CapabilityError),
    Security(SecurityError),
    Audit(AuditError),
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
        let metadata = rt_string.to_bridge_metadata(
            BridgeStatus::Borrowed,
            "local",
            "linux-x64",
            "sysv",
            "foo",
        );
        let entries = metadata.as_entries();
        assert_eq!(entries.status, "borrowed");
        assert_eq!(entries.ownership, "borrowed");
        assert_eq!(entries.span.len(), 7);
        assert_eq!(entries.target, "local");
    }
}

impl Display for ForeignPtr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<ForeignPtr ptr={:?}>", self.ptr)
    }
}
