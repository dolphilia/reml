//! 型推論スタックのルートモジュール。
//!
//! W3 時点では設定・メトリクス・簡易型推論の骨組みのみを提供し、
//! 今後 `types.rs` や `constraint.rs` を拡張していく。

mod capability;
pub mod constraint;
mod driver;
pub mod env;
mod metrics;
mod scheme;
pub mod types;

pub use capability::RuntimeCapability;
pub use constraint::{Constraint, ConstraintSolver, ConstraintSolverError, Substitution};
pub use driver::{
    TypecheckDriver, TypecheckReport, TypecheckViolation, TypecheckViolationKind,
    TypedFunctionSummary,
};
pub use env::{
    config, install_config, Binding, DualWriteGuards, InstallConfigError, RecoverConfig,
    StageContext, StageTraceStep, StageId, StageRequirement, TypeEnv, TypeRowMode, TypecheckConfig,
    TypecheckConfigBuilder,
};
pub use metrics::TypecheckMetrics;
pub use scheme::Scheme;
pub use types::{BuiltinType, CapabilityContext, Type, TypeKind, TypeVarGen, TypeVariable};
