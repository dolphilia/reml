//! Reml runtime crate (WIP)。
//! `collections` や `prelude` モジュールを公開しておき、
//! 将来のフロントエンド／ランタイム統合で利用する。

pub mod collections;
pub mod config;
pub mod io;
#[cfg(feature = "core_time")]
pub mod diagnostics;
#[cfg(feature = "core_numeric")]
pub mod numeric;
pub mod prelude;
pub mod registry;
pub mod stage;
pub mod text;
#[cfg(feature = "core_time")]
pub mod time;

pub use stage::{StageId, StageRequirement};
