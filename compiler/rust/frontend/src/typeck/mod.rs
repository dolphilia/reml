//! 型推論スタックのルートモジュール。
//!
//! W3 時点では設定・メトリクス・簡易型推論の骨組みのみを提供し、
//! 今後 `types.rs` や `constraint.rs` を拡張していく。

mod driver;
pub mod env;
mod metrics;

pub use driver::{
    TypecheckDriver, TypecheckReport, TypecheckViolation, TypecheckViolationKind,
    TypedFunctionSummary,
};
pub use env::{
    config, install_config, DualWriteGuards, InstallConfigError, RecoverConfig, StageContext,
    StageId, StageRequirement, TypeRowMode, TypecheckConfig, TypecheckConfigBuilder,
};
pub use metrics::TypecheckMetrics;
