//! `core_prelude` / 標準仕様が想定する `crate::capability::*` 名前空間を FFI 層で再現する。
//! 実装本体は各モジュール（`registry.rs` や `manifest_contract.rs`）にあるため、
//! ここでは参照パスの橋渡しのみを担う。

pub mod contract {
    pub use crate::manifest_contract::{
        CapabilityContractSpan, ConductorCapabilityContract, ConductorCapabilityRequirement,
    };
}

pub mod registry {
    pub use crate::registry::{
        BridgeIntent, BridgeStageTraceStep, CapabilityError, CapabilityRegistry,
        RuntimeBridgeRegistry, RuntimeBridgeStreamSignal,
    };
}

pub use crate::capability_metadata::PluginCapabilityMetadata;
pub use crate::registry::{CapabilityError, CapabilityRegistry};
