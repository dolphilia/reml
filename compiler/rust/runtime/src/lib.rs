//! Reml runtime crate (WIP)。
//! `collections` や `prelude` モジュールを公開しておき、
//! 将来のフロントエンド／ランタイム統合で利用する。

mod anyhow;
pub mod audit;
pub mod collections;
pub mod config;
pub mod data;
#[cfg(feature = "metrics")]
pub mod diagnostics;
pub mod io;
#[cfg(feature = "core_numeric")]
pub mod numeric;
pub mod path;
pub mod prelude;
pub mod registry;
pub mod stage;
pub mod text;
#[cfg(any(feature = "core_time", feature = "metrics"))]
pub mod time;

pub use stage::{StageId, StageRequirement};
