//! Core.Diagnostics のメトリクス連携部。
//!
//! `docs/spec/3-4-core-numeric-time.md` §4 で定義されている
//! `MetricPoint` と `emit_metric` の最小実装を提供する。

mod metric_point;

pub use metric_point::*;
