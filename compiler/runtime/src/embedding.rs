//! 埋め込み API (C ABI) の最小実装。
//! `native.embed.*` の監査メタデータを記録し、Phase 4 の最小フローを提供する。

use crate::audit::{AuditEnvelope, AuditEvent};
use crate::native::insert_embed_entrypoint_audit_metadata;
use once_cell::sync::Lazy;
use serde_json::{Map as JsonMap, Value};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;
use std::sync::Mutex;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

const EMBED_ABI_VERSION: &str = "0.1.0";
const EMBED_CAPABILITY: &str = "native.embed";

static EMBED_AUDIT_EVENTS: Lazy<Mutex<Vec<AuditEvent>>> = Lazy::new(|| Mutex::new(Vec::new()));

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemlEmbedStatus {
    Ok = 0,
    Error = 1,
    AbiMismatch = 2,
    UnsupportedTarget = 3,
    InvalidArgument = 4,
}

#[repr(C)]
pub struct RemlEmbedContext {
    abi_version: String,
    loaded_source: Option<String>,
    last_error: Option<CString>,
}

impl RemlEmbedContext {
    fn new(abi_version: String) -> Self {
        Self {
            abi_version,
            loaded_source: None,
            last_error: None,
        }
    }

    fn set_error(&mut self, message: impl AsRef<str>) {
        self.last_error = CString::new(message.as_ref())
            .ok()
            .or_else(|| Some(CString::new("unknown error").unwrap()));
    }
}

#[no_mangle]
pub extern "C" fn reml_create_context(
    abi_version: *const c_char,
    out_context: *mut *mut RemlEmbedContext,
) -> RemlEmbedStatus {
    if out_context.is_null() {
        record_embed_audit("reml_create_context", "unknown", None);
        return RemlEmbedStatus::InvalidArgument;
    }

    let abi_version = match read_c_string(abi_version) {
        Some(value) => value,
        None => {
            record_embed_audit("reml_create_context", "unknown", None);
            return RemlEmbedStatus::InvalidArgument;
        }
    };

    if abi_version != EMBED_ABI_VERSION {
        record_embed_audit(
            "reml_create_context",
            &abi_version,
            Some(("native.embed.abi_mismatch", true)),
        );
        return RemlEmbedStatus::AbiMismatch;
    }

    if !is_supported_target() {
        record_embed_audit(
            "reml_create_context",
            &abi_version,
            Some(("native.embed.unsupported_target", true)),
        );
        return RemlEmbedStatus::UnsupportedTarget;
    }

    let context = Box::new(RemlEmbedContext::new(abi_version.clone()));
    unsafe {
        *out_context = Box::into_raw(context);
    }
    record_embed_audit("reml_create_context", &abi_version, None);
    RemlEmbedStatus::Ok
}

#[no_mangle]
pub extern "C" fn reml_load_module(
    context: *mut RemlEmbedContext,
    source: *const u8,
    length: usize,
) -> RemlEmbedStatus {
    let context = unsafe { context.as_mut() };
    let Some(context) = context else {
        record_embed_audit("reml_load_module", "unknown", None);
        return RemlEmbedStatus::InvalidArgument;
    };

    if source.is_null() || length == 0 {
        context.set_error("source が空です");
        record_embed_audit("reml_load_module", &context.abi_version, None);
        return RemlEmbedStatus::InvalidArgument;
    }

    let bytes = unsafe { std::slice::from_raw_parts(source, length) };
    let source = match std::str::from_utf8(bytes) {
        Ok(value) => value.to_string(),
        Err(_) => {
            context.set_error("source が UTF-8 ではありません");
            record_embed_audit("reml_load_module", &context.abi_version, None);
            return RemlEmbedStatus::Error;
        }
    };

    context.loaded_source = Some(source);
    context.last_error = None;
    record_embed_audit("reml_load_module", &context.abi_version, None);
    RemlEmbedStatus::Ok
}

#[no_mangle]
pub extern "C" fn reml_run(
    context: *mut RemlEmbedContext,
    entrypoint: *const c_char,
) -> RemlEmbedStatus {
    let context = unsafe { context.as_mut() };
    let Some(context) = context else {
        record_embed_audit("reml_run", "unknown", None);
        return RemlEmbedStatus::InvalidArgument;
    };

    let entrypoint = read_c_string(entrypoint).unwrap_or_else(|| "main".to_string());
    if context.loaded_source.is_none() {
        context.set_error("module がロードされていません");
        record_embed_audit("reml_run", &context.abi_version, None);
        return RemlEmbedStatus::Error;
    }

    let _ = entrypoint;
    context.last_error = None;
    record_embed_audit("reml_run", &context.abi_version, None);
    RemlEmbedStatus::Ok
}

#[no_mangle]
pub extern "C" fn reml_dispose_context(context: *mut RemlEmbedContext) -> RemlEmbedStatus {
    if context.is_null() {
        record_embed_audit("reml_dispose_context", "unknown", None);
        return RemlEmbedStatus::InvalidArgument;
    }

    let abi_version = unsafe { &(*context).abi_version }.clone();
    unsafe {
        drop(Box::from_raw(context));
    }
    record_embed_audit("reml_dispose_context", &abi_version, None);
    RemlEmbedStatus::Ok
}

#[no_mangle]
pub extern "C" fn reml_last_error(context: *const RemlEmbedContext) -> *const c_char {
    let context = unsafe { context.as_ref() };
    let Some(context) = context else {
        return ptr::null();
    };
    context
        .last_error
        .as_ref()
        .map(|value| value.as_ptr())
        .unwrap_or(ptr::null())
}

/// 記録済みの埋め込み監査イベントを取得してクリアする。
pub fn take_embed_audit_events() -> Vec<AuditEvent> {
    let mut events = EMBED_AUDIT_EVENTS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let drained = events.clone();
    events.clear();
    drained
}

fn read_c_string(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    unsafe {
        CStr::from_ptr(ptr)
            .to_str()
            .ok()
            .map(|value| value.to_string())
    }
}

fn is_supported_target() -> bool {
    if let Ok(value) = std::env::var("REML_EMBED_FORCE_UNSUPPORTED") {
        if value == "1" || value.eq_ignore_ascii_case("true") {
            return false;
        }
    }
    matches!(std::env::consts::OS, "macos" | "linux" | "windows")
}

fn record_embed_audit(entrypoint: &str, abi_version: &str, marker: Option<(&str, bool)>) {
    let timestamp = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".into());
    let mut envelope =
        AuditEnvelope::from_parts(JsonMap::new(), None, None, Some(EMBED_CAPABILITY.into()));
    insert_embed_entrypoint_audit_metadata(&mut envelope, entrypoint, abi_version);
    if let Some((key, value)) = marker {
        envelope
            .metadata
            .insert(key.to_string(), Value::Bool(value));
    }
    let event = AuditEvent::new(timestamp, envelope);
    let mut events = EMBED_AUDIT_EVENTS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    events.push(event);
}
