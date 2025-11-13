//! 型推論スタックのルートモジュール。
//!
//! W3 時点では設定・メトリクス・簡易型推論の骨組みのみを提供し、
//! 今後 `types.rs` や `constraint.rs` を拡張していく。

mod constraint;
mod capability;
mod driver;
pub mod env;
mod metrics;
mod scheme;
mod types;

pub use constraint::{Constraint, ConstraintSolver, ConstraintSolverError, Substitution};
pub use driver::{
    TypecheckDriver, TypecheckReport, TypecheckViolation, TypecheckViolationKind,
    TypedFunctionSummary,
};
pub use env::{
    config, install_config, Binding, DualWriteGuards, InstallConfigError, RecoverConfig,
    StageContext, StageId, StageRequirement, TypeEnv, TypeRowMode, TypecheckConfig,
    TypecheckConfigBuilder,
};
pub use metrics::TypecheckMetrics;
pub use scheme::Scheme;
pub use types::{BuiltinType, CapabilityContext, Type, TypeKind, TypeVarGen, TypeVariable};
