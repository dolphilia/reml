//! Reml Rust LLVM バックエンドの中間層を構成するスケルトン。
//!
//! `TargetMachineBuilder`/`TypeMappingContext`/`FfiLowering` は
//! W2 の「Rust LLVM ラッパ層構築」で定義したチェックリストに沿って
//! 仕様と実装の差分を抑えるための出発点となる。

pub mod bridge_metadata;
pub mod codegen;
pub mod ffi_lowering;
pub mod integration;
pub mod intrinsics;
pub mod runtime_link;
pub mod target_diagnostics;
pub mod target_machine;
pub mod type_mapping;
pub mod unstable;
pub mod verify;

pub use codegen::{CodegenContext, GeneratedFunction, MirFunction, ModuleIr};
pub use ffi_lowering::{FfiCallSignature, FfiLowering, LoweredFfiCall};
pub use integration::{
    generate_snapshot, generate_snapshot_from_mir_json, generate_w3_snapshot,
    load_mir_functions_from_json, BackendDiffSnapshot, BackendFunctionRecord, MirSnapshotError,
};
pub use intrinsics::{IntrinsicSignature, IntrinsicStatus, IntrinsicUse};
pub use runtime_link::{
    compile_ir_with_llc, find_runtime_library, generate_link_command, link_object_with_runtime,
    link_with_runtime, LinkCommand, Platform, RuntimeLinkError,
};
pub use target_diagnostics::{PlatformInfo, RunConfigTarget, TargetDiagnosticContext};
pub use target_machine::{
    CodeModel, DataLayoutSpec, OptimizationLevel, RelocModel, TargetMachine, TargetMachineBuilder,
    Triple, WindowsToolchainConfig,
};
pub use type_mapping::{RemlType, TypeLayout, TypeMappingContext};
pub use unstable::{UnstableKind, UnstableStatus, UnstableUse};
pub use verify::{AuditEntry, AuditLog, Diagnostic, VerificationResult, Verifier};
