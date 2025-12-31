//! Core.Native の最小 API。
//! `effect {native}` を伴う intrinsic 呼び出しと監査キーの記録を担う。

use serde_json::Value;

use crate::audit::AuditEnvelope;

pub const INTRINSIC_SQRT_F64: &str = "llvm.sqrt.f64";
pub const INTRINSIC_CTPOP_I64: &str = "llvm.ctpop.i64";
pub const INTRINSIC_CTPOP_I32: &str = "llvm.ctpop.i32";
pub const INTRINSIC_MEMCPY_P0_P0_I64: &str = "llvm.memcpy.p0.p0.i64";

pub const SIGNATURE_SQRT_F64: &str = "(f64) -> f64";
pub const SIGNATURE_CTPOP_I64: &str = "(i64) -> i64";
pub const SIGNATURE_CTPOP_I32: &str = "(i32) -> i32";
pub const SIGNATURE_MEMCPY_P0_P0_I64: &str = "(ptr, ptr, i64) -> void";

/// `native.inline_asm.*` の監査メタデータを `AuditEnvelope` に挿入する。
pub fn insert_inline_asm_audit_metadata<S: AsRef<str>>(
    envelope: &mut AuditEnvelope,
    template_hash: &str,
    constraints: &[S],
) {
    record_inline_asm_audit_metadata(envelope, template_hash, constraints);
}

/// `native.llvm_ir.*` の監査メタデータを `AuditEnvelope` に挿入する。
pub fn insert_llvm_ir_audit_metadata<S: AsRef<str>>(
    envelope: &mut AuditEnvelope,
    template_hash: &str,
    inputs: &[S],
) {
    record_llvm_ir_audit_metadata(envelope, template_hash, inputs);
}

/// `native.intrinsic.*` の監査メタデータを `AuditEnvelope` に挿入する。
pub fn insert_intrinsic_audit_metadata(envelope: &mut AuditEnvelope, name: &str, signature: &str) {
    record_intrinsic_audit_metadata(envelope, name, signature);
}

/// `native.embed.*` の監査メタデータを `AuditEnvelope` に挿入する。
pub fn insert_embed_entrypoint_audit_metadata(
    envelope: &mut AuditEnvelope,
    entrypoint: &str,
    abi_version: &str,
) {
    record_embed_entrypoint_metadata(envelope, entrypoint, abi_version);
}

pub(crate) fn record_intrinsic_audit_metadata(
    envelope: &mut AuditEnvelope,
    name: &str,
    signature: &str,
) {
    envelope
        .metadata
        .insert("native.intrinsic.used".into(), Value::Bool(true));
    envelope
        .metadata
        .insert("intrinsic.name".into(), Value::String(name.to_string()));
    envelope.metadata.insert(
        "intrinsic.signature".into(),
        Value::String(signature.to_string()),
    );
}

pub(crate) fn record_inline_asm_audit_metadata<S: AsRef<str>>(
    envelope: &mut AuditEnvelope,
    template_hash: &str,
    constraints: &[S],
) {
    envelope
        .metadata
        .insert("native.inline_asm.used".into(), Value::Bool(true));
    envelope.metadata.insert(
        "asm.template_hash".into(),
        Value::String(template_hash.to_string()),
    );
    envelope.metadata.insert(
        "asm.constraints".into(),
        Value::Array(
            constraints
                .iter()
                .map(|value| Value::String(value.as_ref().to_string()))
                .collect(),
        ),
    );
}

pub(crate) fn record_llvm_ir_audit_metadata<S: AsRef<str>>(
    envelope: &mut AuditEnvelope,
    template_hash: &str,
    inputs: &[S],
) {
    envelope
        .metadata
        .insert("native.llvm_ir.used".into(), Value::Bool(true));
    envelope.metadata.insert(
        "llvm_ir.template_hash".into(),
        Value::String(template_hash.to_string()),
    );
    envelope.metadata.insert(
        "llvm_ir.inputs".into(),
        Value::Array(
            inputs
                .iter()
                .map(|value| Value::String(value.as_ref().to_string()))
                .collect(),
        ),
    );
}

pub(crate) fn record_embed_entrypoint_metadata(
    envelope: &mut AuditEnvelope,
    entrypoint: &str,
    abi_version: &str,
) {
    envelope.metadata.insert(
        "native.embed.entrypoint".into(),
        Value::String(entrypoint.to_string()),
    );
    envelope.metadata.insert(
        "embed.abi.version".into(),
        Value::String(abi_version.to_string()),
    );
}

/// `effect {native}` を要求する平方根 intrinsic。
pub fn sqrt_f64(value: f64, audit: Option<&mut AuditEnvelope>) -> f64 {
    if let Some(envelope) = audit {
        record_intrinsic_audit_metadata(envelope, INTRINSIC_SQRT_F64, SIGNATURE_SQRT_F64);
    }
    value.sqrt()
}

/// `effect {native}` を要求する popcount intrinsic (i64)。
pub fn ctpop_i64(value: i64, audit: Option<&mut AuditEnvelope>) -> i64 {
    if let Some(envelope) = audit {
        record_intrinsic_audit_metadata(envelope, INTRINSIC_CTPOP_I64, SIGNATURE_CTPOP_I64);
    }
    value.count_ones() as i64
}

/// `effect {native}` を要求する popcount intrinsic (i32)。
pub fn ctpop_i32(value: i32, audit: Option<&mut AuditEnvelope>) -> i32 {
    if let Some(envelope) = audit {
        record_intrinsic_audit_metadata(envelope, INTRINSIC_CTPOP_I32, SIGNATURE_CTPOP_I32);
    }
    value.count_ones() as i32
}

/// `effect {native}` を要求する memcpy intrinsic。
///
/// `count` は要素数で、`T: Copy` のみを許可する。
pub unsafe fn memcpy<T: Copy>(
    dst: *mut T,
    src: *const T,
    count: usize,
    audit: Option<&mut AuditEnvelope>,
) {
    if count == 0 {
        return;
    }
    if let Some(envelope) = audit {
        record_intrinsic_audit_metadata(
            envelope,
            INTRINSIC_MEMCPY_P0_P0_I64,
            SIGNATURE_MEMCPY_P0_P0_I64,
        );
    }
    std::ptr::copy_nonoverlapping(src, dst, count);
}
