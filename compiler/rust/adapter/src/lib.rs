//! Rust 側アダプタ層の汎用ユーティリティ。
//! 各サブシステム（Env/FS/Network/Time/Random/Process/Target）に Capability・監査抽象化を提供する。

pub mod capability;
pub mod env;
pub mod fs;
pub mod network;
pub mod process;
pub mod random;
pub mod target;
pub mod time;
