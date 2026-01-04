#![allow(dead_code)]

//! Reml コアプレリュード API（Option/Result/Never/Try 群）の雛形実装。
//!
//! WBS 2.1a では API の型構造とモジュール区切りのみを確立し、
//! 具体的なメソッド実装や効果タグ検証は WBS 2.1b 以降で実装する。

#[path = "../../../src/prelude/collectors/mod.rs"]
pub mod collectors;
#[path = "../../../src/prelude/ensure.rs"]
pub mod ensure;
#[path = "../../../src/prelude/iter/mod.rs"]
pub mod iter;
pub mod never;
pub mod option;
pub mod result;
pub mod try_support;

pub use collectors::{
    CollectError, CollectErrorKind, CollectOutcome, Collector, CollectorAuditTrail, CollectorKind,
    CollectorStageProfile, CollectorStageSnapshot, List, ListCollector, Map, MapCollector, Set,
    SetCollector, StringCollector, Table, TableCollector, VecCollector, EFFECT_MARKER_FINISH,
    EFFECT_MARKER_RESERVE, EFFECT_MARKER_WITH_CAPACITY,
};
pub use ensure::{
    ensure, ensure_not_null, DiagnosticSeverity, EnsureError, EnsureErrorBuilder, GuardDiagnostic,
    IntoDiagnostic, PreludeGuardKind, PreludeGuardMetadata,
};
pub use never::Never;
pub use option::Option;
pub use result::Result;
pub use try_support::{ControlFlow, Try, TryContext};
