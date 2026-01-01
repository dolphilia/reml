//! Reml runtime crate (WIP)。
//! `collections` や `prelude` モジュールを公開しておき、
//! 将来のフロントエンド／ランタイム統合で利用する。

mod anyhow;
pub mod audit;
pub mod capability;
pub mod cli;
pub mod collections;
pub mod config;
pub mod data;
#[cfg(feature = "metrics")]
pub mod diagnostics;
pub mod doc;
pub mod dsl;
pub mod embedding;
pub mod ffi;
pub mod io;
pub mod lsp;
pub mod native;
#[cfg(feature = "core_numeric")]
pub mod numeric;
pub mod parse;
pub mod path;
pub mod prelude;
pub mod run_config;
pub mod runtime;
pub mod system;
pub mod env;
pub mod stage;
pub mod test;
pub mod test_support;
pub mod text;
#[cfg(any(feature = "core_time", feature = "metrics"))]
pub mod time;

pub use capability::{
    contract::{
        CapabilityContractSpan, ConductorCapabilityContract, ConductorCapabilityRequirement,
    },
    descriptor::{
        CapabilityDescriptor, CapabilityId, CapabilityIsolationLevel, CapabilityMetadata,
        CapabilityPermission, CapabilityProvider, CapabilitySandboxProfile,
        CapabilitySecurityMetadata, CapabilitySecuritySignature, CapabilityTimestamp, EffectTag,
    },
    registry::{CapabilityDescriptorList, CapabilityError, CapabilityRegistry},
};
pub use stage::{StageId, StageParseError, StageRequirement};
