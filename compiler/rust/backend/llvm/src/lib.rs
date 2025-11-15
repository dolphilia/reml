//! Reml Rust LLVM バックエンドの中間層を構成するスケルトン。
//!
//! `TargetMachineBuilder`/`TypeMappingContext`/`FfiLowering` は
//! W2 の「Rust LLVM ラッパ層構築」で定義したチェックリストに沿って
//! 仕様と実装の差分を抑えるための出発点となる。

pub mod ffi_lowering;
pub mod target_machine;
pub mod type_mapping;

pub use ffi_lowering::{FfiCallSignature, FfiLowering, LoweredFfiCall};
pub use target_machine::{
  CodeModel, DataLayoutSpec, OptimizationLevel, RelocModel, TargetMachine,
  TargetMachineBuilder, Triple, WindowsToolchainConfig,
};
pub use type_mapping::{RemlType, TypeLayout, TypeMappingContext};
