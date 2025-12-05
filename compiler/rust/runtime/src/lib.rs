//! Reml runtime crate (WIP)。
//! `collections` や `prelude` モジュールを公開しておき、
//! 将来のフロントエンド／ランタイム統合で利用する。

mod anyhow;
pub mod audit;
pub mod capability;
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
pub mod run_config;
pub mod stage;
pub mod text;
#[cfg(any(feature = "core_time", feature = "metrics"))]
pub mod time;

pub use capability::{
    contract::{
        CapabilityContractSpan, ConductorCapabilityContract, ConductorCapabilityRequirement,
    },
    descriptor::{
        CapabilityDescriptor, CapabilityId, CapabilityProvider, CapabilityTimestamp, EffectTag,
    },
    registry::{CapabilityError, CapabilityRegistry},
};
pub use stage::{StageId, StageParseError, StageRequirement};
