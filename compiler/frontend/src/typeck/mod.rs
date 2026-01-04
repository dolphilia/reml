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
pub mod telemetry;
pub mod types;

pub use capability::RuntimeCapability;
pub use constraint::iterator::{
    solve_iterator, IteratorDictInfo, IteratorKind, IteratorStageProfile, IteratorStageSnapshot,
};
pub use constraint::{Constraint, ConstraintSolver, ConstraintSolverError, Substitution};
pub use driver::{
    IteratorStageViolationInfo, TypecheckDriver, TypecheckRecoverHint, TypecheckReport,
    TypecheckViolation, TypecheckViolationKind, TypedFunctionSummary,
};
pub use env::{
    config, install_config, Binding, DualWriteGuards, InstallConfigError, RecoverConfig,
    StageContext, StageId, StageRequirement, StageTraceStep, TypeEnv, TypeRowMode, TypecheckConfig,
    TypecheckConfigBuilder,
};
pub use metrics::TypecheckMetrics;
pub use scheme::Scheme;
pub use types::{BuiltinType, CapabilityContext, Type, TypeKind, TypeVarGen, TypeVariable};
