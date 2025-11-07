//! 型推論スタックのルートモジュール。
//!
//! 現時点では設定と dual-write 補助のみを実装し、W3 以降で
//! `types.rs` や `constraint.rs` などのサブモジュールを拡張していく。

pub mod env;

pub use env::{
    config, install_config, DualWriteGuards, InstallConfigError, RecoverConfig, StageContext,
    StageId, StageRequirement, TypeRowMode, TypecheckConfig, TypecheckConfigBuilder,
};
