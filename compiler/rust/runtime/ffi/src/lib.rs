//! Rust 側から Reml ランタイムへの FFI アクセスを提供する最小層。
//! 手動定義の `extern` 宣言と安全なラッパーをまとめる。
use std::{
    ffi::CStr,
    fmt::Display,
    mem,
    os::raw::{c_char, c_void},
    ptr::NonNull,
};

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
        self.as_bytes().and_then(|bytes| std::str::from_utf8(bytes).ok())
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

/// 所有権付きポインタ。`inc_ref`/`dec_ref` を自動化する。
pub struct ForeignPtr {
    ptr: NonNull<c_void>,
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

/// `reml_runtime` ライブラリへのリンク。
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

impl Display for ForeignPtr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<ForeignPtr ptr={:?}>", self.ptr)
    }
}
