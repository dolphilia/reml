//! Core.Diagnostics のメトリクス連携部。
//!
//! `docs/spec/3-4-core-numeric-time.md` §4 で定義されている
//! `MetricPoint` と `emit_metric` の最小実装を提供する。

mod audit_bridge;
mod dsl;
mod metric_point;
mod stage_guard;

pub use dsl::apply_dsl_metadata;
pub use metric_point::*;
pub(crate) use stage_guard::{
    metric_required_effects, MetricsStageGuard, METRIC_CAPABILITY_ID, METRIC_STAGE_REQUIREMENT,
};
