//! Core.IO が Runtime Capability Registry と接続するためのアダプタ群。
//!
//! 仕様 `docs/plans/bootstrap-roadmap/assets/core-io-capability-map.md` で定義された
//! Capability ID を Rust Runtime から実際に検証し、Stage 情報をキャッシュする。

use once_cell::sync::OnceCell;

use crate::{
    registry::{CapabilityError, CapabilityRegistry},
    stage::{StageId, StageRequirement},
};

use super::{record_bridge_stage_probe, IoError, IoErrorKind, IoResult};

pub(crate) const CAP_IO_FS_READ: &str = "io.fs.read";
pub(crate) const CAP_IO_FS_WRITE: &str = "io.fs.write";
pub(crate) const CAP_FS_PERMISSIONS_READ: &str = "fs.permissions.read";
pub(crate) const CAP_FS_PERMISSIONS_MODIFY: &str = "fs.permissions.modify";
pub(crate) const CAP_FS_SYMLINK_QUERY: &str = "fs.symlink.query";
pub(crate) const CAP_FS_SYMLINK_MODIFY: &str = "fs.symlink.modify";
pub(crate) const CAP_FS_WATCH_NATIVE: &str = "fs.watcher.native";
pub(crate) const CAP_FS_WATCH_RECURSIVE: &str = "fs.watcher.recursive";
pub(crate) const CAP_SECURITY_FS_POLICY: &str = "security.fs.policy";
pub(crate) const CAP_MEMORY_BUFFERED_IO: &str = "memory.buffered_io";
pub(crate) const CAP_WATCH_RESOURCE_LIMITS: &str = "watcher.resource_limits";

/// ファイルシステム操作向け Capability を検証するアダプタ。
pub struct FsAdapter {
    registry: CapabilityRegistry,
}

impl FsAdapter {
    /// グローバルアダプタを取得する。
    pub fn global() -> &'static Self {
        static INSTANCE: OnceCell<FsAdapter> = OnceCell::new();
        INSTANCE.get_or_init(|| FsAdapter {
            registry: CapabilityRegistry::registry(),
        })
    }

    /// `io.fs.read` Capability を検証する。
    pub fn ensure_read_capability(&self) -> IoResult<()> {
        self.ensure_stage(
            &IO_FS_READ_STAGE,
            CAP_IO_FS_READ,
            StageRequirement::AtLeast(StageId::Beta),
        )
    }

    /// `io.fs.write` Capability を検証する。
    pub fn ensure_write_capability(&self) -> IoResult<()> {
        self.ensure_stage(
            &IO_FS_WRITE_STAGE,
            CAP_IO_FS_WRITE,
            StageRequirement::AtLeast(StageId::Beta),
        )
    }

    /// `fs.permissions.read` Capability を検証する。
    pub fn ensure_permissions_read(&self) -> IoResult<()> {
        self.ensure_stage(
            &FS_PERMISSIONS_READ_STAGE,
            CAP_FS_PERMISSIONS_READ,
            StageRequirement::Exact(StageId::Stable),
        )
    }

    /// `fs.permissions.modify` Capability を検証する。
    pub fn ensure_permissions_modify(&self) -> IoResult<()> {
        self.ensure_stage(
            &FS_PERMISSIONS_MODIFY_STAGE,
            CAP_FS_PERMISSIONS_MODIFY,
            StageRequirement::Exact(StageId::Stable),
        )
    }

    /// `fs.symlink.query` Capability を検証する。
    pub fn ensure_symlink_query(&self) -> IoResult<()> {
        self.ensure_stage(
            &FS_SYMLINK_QUERY_STAGE,
            CAP_FS_SYMLINK_QUERY,
            StageRequirement::AtLeast(StageId::Beta),
        )
    }

    /// `fs.symlink.modify` Capability を検証する。
    pub fn ensure_symlink_modify(&self) -> IoResult<()> {
        self.ensure_stage(
            &FS_SYMLINK_MODIFY_STAGE,
            CAP_FS_SYMLINK_MODIFY,
            StageRequirement::Exact(StageId::Stable),
        )
    }

    /// `security.fs.policy` Capability を検証する。
    pub fn ensure_security_policy(&self) -> IoResult<()> {
        self.ensure_stage(
            &FS_POLICY_STAGE,
            CAP_SECURITY_FS_POLICY,
            StageRequirement::Exact(StageId::Stable),
        )
    }

    /// `memory.buffered_io` Capability を検証する。
    pub fn ensure_buffered_io_capability(&self) -> IoResult<()> {
        self.ensure_stage(
            &MEMORY_BUFFERED_IO_STAGE,
            CAP_MEMORY_BUFFERED_IO,
            StageRequirement::AtLeast(StageId::Beta),
        )
    }

    fn ensure_stage(
        &self,
        cache: &OnceCell<StageId>,
        capability: &'static str,
        requirement: StageRequirement,
    ) -> IoResult<()> {
        if let Some(stage) = cache.get() {
            record_bridge_stage_probe(capability, requirement, *stage);
            return Ok(());
        }
        match self
            .registry
            .verify_stage_for_io(capability, requirement)
        {
            Ok(stage) => {
                let _ = cache.set(stage);
                 record_bridge_stage_probe(capability, requirement, stage);
                Ok(())
            }
            Err(err) => Err(capability_error_to_io(capability, err)),
        }
    }
}

/// ファイル監視 Capability を検証するアダプタ（実装が入るまで Stage 判定のみ）。
pub struct WatcherAdapter {
    registry: CapabilityRegistry,
}

impl WatcherAdapter {
    /// グローバルインスタンスを取得する。
    pub fn global() -> &'static Self {
        static INSTANCE: OnceCell<WatcherAdapter> = OnceCell::new();
        INSTANCE.get_or_init(|| WatcherAdapter {
            registry: CapabilityRegistry::registry(),
        })
    }

    /// `fs.watcher.native` Capability を検証する。
    pub fn ensure_native_capability(&self) -> IoResult<()> {
        self.ensure_stage(
            &FS_WATCH_NATIVE_STAGE,
            CAP_FS_WATCH_NATIVE,
            StageRequirement::AtLeast(StageId::Beta),
        )
    }

    /// `fs.watcher.recursive` Capability を検証する。
    pub fn ensure_recursive_capability(&self) -> IoResult<()> {
        self.ensure_stage(
            &FS_WATCH_RECURSIVE_STAGE,
            CAP_FS_WATCH_RECURSIVE,
            StageRequirement::Exact(StageId::Stable),
        )
    }

    /// `watcher.resource_limits` Capability を検証する。
    pub fn ensure_resource_limit_capability(&self) -> IoResult<()> {
        self.ensure_stage(
            &WATCH_RESOURCE_LIMITS_STAGE,
            CAP_WATCH_RESOURCE_LIMITS,
            StageRequirement::AtLeast(StageId::Beta),
        )
    }

    fn ensure_stage(
        &self,
        cache: &OnceCell<StageId>,
        capability: &'static str,
        requirement: StageRequirement,
    ) -> IoResult<()> {
        if let Some(stage) = cache.get() {
            record_bridge_stage_probe(capability, requirement, *stage);
            return Ok(());
        }
        match self
            .registry
            .verify_stage_for_io(capability, requirement)
        {
            Ok(stage) => {
                let _ = cache.set(stage);
                record_bridge_stage_probe(capability, requirement, stage);
                Ok(())
            }
            Err(err) => Err(capability_error_to_io(capability, err)),
        }
    }
}

fn capability_error_to_io(capability: &'static str, err: CapabilityError) -> IoError {
    IoError::new(
        IoErrorKind::SecurityViolation,
        format!("capability `{capability}` is not available: {}", err),
    )
}

static IO_FS_READ_STAGE: OnceCell<StageId> = OnceCell::new();
static IO_FS_WRITE_STAGE: OnceCell<StageId> = OnceCell::new();
static FS_PERMISSIONS_READ_STAGE: OnceCell<StageId> = OnceCell::new();
static FS_PERMISSIONS_MODIFY_STAGE: OnceCell<StageId> = OnceCell::new();
static FS_SYMLINK_QUERY_STAGE: OnceCell<StageId> = OnceCell::new();
static FS_SYMLINK_MODIFY_STAGE: OnceCell<StageId> = OnceCell::new();
static FS_POLICY_STAGE: OnceCell<StageId> = OnceCell::new();
static MEMORY_BUFFERED_IO_STAGE: OnceCell<StageId> = OnceCell::new();
static FS_WATCH_NATIVE_STAGE: OnceCell<StageId> = OnceCell::new();
static FS_WATCH_RECURSIVE_STAGE: OnceCell<StageId> = OnceCell::new();
static WATCH_RESOURCE_LIMITS_STAGE: OnceCell<StageId> = OnceCell::new();

#[cfg(test)]
mod tests {
    use super::{FsAdapter, WatcherAdapter};

    #[test]
    fn fs_adapter_ensures_capabilities() {
        let adapter = FsAdapter::global();
        adapter.ensure_read_capability().unwrap();
        adapter.ensure_write_capability().unwrap();
        adapter.ensure_permissions_read().unwrap();
        adapter.ensure_permissions_modify().unwrap();
        adapter.ensure_symlink_query().unwrap();
        adapter.ensure_symlink_modify().unwrap();
        adapter.ensure_security_policy().unwrap();
        adapter.ensure_buffered_io_capability().unwrap();
    }

    #[test]
    fn watcher_adapter_ensures_capabilities() {
        let adapter = WatcherAdapter::global();
        adapter.ensure_native_capability().unwrap();
        adapter.ensure_recursive_capability().unwrap();
    }
}
