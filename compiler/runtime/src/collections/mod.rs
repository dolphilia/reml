//! Runtime 内で共有するコレクション基盤モジュール群。
//! 現時点では永続構造用モジュールと監査ブリッジを提供する。

pub mod audit_bridge;
pub mod mutable;
pub mod persistent;
