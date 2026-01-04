use std::fs::Metadata as StdMetadata;
use std::time::SystemTime;

use super::permissions::FilePermissions;

#[cfg(any(feature = "core_time", feature = "metrics"))]
use crate::time::{self, Timestamp};
#[cfg(not(any(feature = "core_time", feature = "metrics")))]
use std::time::SystemTime as Timestamp;

/// ファイルシステムから取得したメタデータのスナップショット。
#[derive(Debug, Clone)]
pub struct FileMetadata {
    size: u64,
    readonly: bool,
    is_dir: bool,
    permissions: FilePermissions,
    created_at: Option<Timestamp>,
    modified_at: Option<Timestamp>,
    accessed_at: Option<Timestamp>,
}

impl FileMetadata {
    pub(crate) fn from_std(metadata: StdMetadata) -> Self {
        let readonly = metadata.permissions().readonly();
        let is_dir = metadata.is_dir();
        let size = metadata.len();
        Self {
            size,
            readonly,
            is_dir,
            permissions: FilePermissions::from_metadata(&metadata),
            created_at: convert_time(metadata.created()),
            modified_at: convert_time(metadata.modified()),
            accessed_at: convert_time(metadata.accessed()),
        }
    }

    pub fn size(&self) -> u64 {
        self.size
    }

    pub fn is_readonly(&self) -> bool {
        self.readonly
    }

    pub fn is_dir(&self) -> bool {
        self.is_dir
    }

    pub fn permissions(&self) -> FilePermissions {
        self.permissions
    }

    pub fn created_at(&self) -> Option<Timestamp> {
        self.created_at
    }

    pub fn modified_at(&self) -> Option<Timestamp> {
        self.modified_at
    }

    pub fn accessed_at(&self) -> Option<Timestamp> {
        self.accessed_at
    }
}

#[cfg(any(feature = "core_time", feature = "metrics"))]
fn convert_time(value: Result<SystemTime, std::io::Error>) -> Option<Timestamp> {
    match value {
        Ok(time) => Timestamp::from_system_time(time).ok(),
        Err(_) => None,
    }
}

#[cfg(not(any(feature = "core_time", feature = "metrics")))]
fn convert_time(value: Result<SystemTime, std::io::Error>) -> Option<Timestamp> {
    value.ok()
}
