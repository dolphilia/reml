use std::fs::{self, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use super::{
    adapters::{FsAdapter, CAP_FS_PERMISSIONS_READ, CAP_IO_FS_READ, CAP_IO_FS_WRITE},
    effects::{
        blocking_io_effect_labels, fs_sync_effect_labels, record_fs_sync_operation,
        record_io_operation,
    },
    metadata::FileMetadata,
    options::FileOptions,
    scope::FileHandleGuard,
    IoContext, IoError, IoResult,
};

/// Core.IO 仕様に準拠したファイルハンドル。
#[derive(Debug)]
pub struct File {
    handle: std::fs::File,
    path: PathBuf,
    #[allow(dead_code)]
    handle_guard: FileHandleGuard,
}

impl File {
    /// 既存ファイルを読み込みモードで開く。
    pub fn open<P: AsRef<Path>>(path: P) -> IoResult<Self> {
        let path_ref = path.as_ref();
        let context = operation_context("file.open", path_ref, CAP_IO_FS_READ);
        FsAdapter::global()
            .ensure_read_capability()
            .map_err(|err| err.with_context(context.clone()))?;

        record_io_operation(1);
        let handle =
            std::fs::File::open(path_ref).map_err(|err| IoError::from_std(err, context.clone()))?;
        Ok(Self {
            handle,
            path: path_ref.to_path_buf(),
            handle_guard: FileHandleGuard::new(),
        })
    }

    /// 指定したオプションでファイルを作成または開く。
    pub fn create<P: AsRef<Path>>(path: P, options: FileOptions) -> IoResult<Self> {
        let path_ref = path.as_ref();
        let adapter = FsAdapter::global();

        let write_context = operation_context("file.create", path_ref, CAP_IO_FS_WRITE);
        adapter
            .ensure_write_capability()
            .map_err(|err| err.with_context(write_context.clone()))?;
        adapter
            .ensure_permissions_modify()
            .map_err(|err| err.with_context(write_context.clone()))?;

        if options.read_enabled() {
            let read_context = operation_context("file.create", path_ref, CAP_IO_FS_READ);
            adapter
                .ensure_read_capability()
                .map_err(|err| err.with_context(read_context))?;
        }

        record_io_operation(1);
        let mut open_opts = OpenOptions::new();
        options.apply_to(&mut open_opts);
        let handle = open_opts
            .open(path_ref)
            .map_err(|err| IoError::from_std(err, write_context.clone()))?;
        Ok(Self {
            handle,
            path: path_ref.to_path_buf(),
            handle_guard: FileHandleGuard::new(),
        })
    }

    /// ファイルパスを返す。
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// ファイルメタデータを取得する。
    pub fn metadata(&self) -> IoResult<FileMetadata> {
        metadata_for_path(&self.path)
    }

    /// 任意のパスのメタデータを取得する。
    pub fn metadata_at<P: AsRef<Path>>(path: P) -> IoResult<FileMetadata> {
        let path_ref = path.as_ref();
        metadata_for_path(path_ref)
    }

    /// ファイルを削除する。
    pub fn remove<P: AsRef<Path>>(path: P) -> IoResult<()> {
        let path_ref = path.as_ref();
        let adapter = FsAdapter::global();
        let context = operation_context("file.remove", path_ref, CAP_IO_FS_WRITE);
        adapter
            .ensure_write_capability()
            .map_err(|err| err.with_context(context.clone()))?;

        record_io_operation(1);
        fs::remove_file(path_ref).map_err(|err| IoError::from_std(err, context))
    }

    /// すべてのバッファを永続化する。
    pub fn sync_all(&mut self) -> IoResult<()> {
        record_fs_sync_operation();
        self.handle.sync_all().map_err(|err| {
            IoError::from_std(
                err,
                self.sync_operation_context("file.sync_all", CAP_IO_FS_WRITE),
            )
        })
    }

    /// データ領域のバッファを永続化する。
    pub fn sync_data(&mut self) -> IoResult<()> {
        record_fs_sync_operation();
        self.handle.sync_data().map_err(|err| {
            IoError::from_std(
                err,
                self.sync_operation_context("file.sync_data", CAP_IO_FS_WRITE),
            )
        })
    }

    #[allow(dead_code)]
    pub(crate) fn as_std(&self) -> &std::fs::File {
        &self.handle
    }

    #[allow(dead_code)]
    fn operation_context(&self, operation: &'static str, capability: &'static str) -> IoContext {
        IoContext::new(operation)
            .with_path(self.path.clone())
            .with_capability(capability)
            .with_effects(blocking_io_effect_labels())
    }

    fn sync_operation_context(
        &self,
        operation: &'static str,
        capability: &'static str,
    ) -> IoContext {
        IoContext::new(operation)
            .with_path(self.path.clone())
            .with_capability(capability)
            .with_effects(fs_sync_effect_labels())
    }
}

impl Drop for File {
    fn drop(&mut self) {
        record_fs_sync_operation();
    }
}

impl Read for File {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.handle.read(buf)
    }
}

impl Write for File {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.handle.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.handle.flush()
    }
}

impl Seek for File {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        self.handle.seek(pos)
    }
}

fn operation_context(operation: &'static str, path: &Path, capability: &'static str) -> IoContext {
    IoContext::new(operation)
        .with_path(path.to_path_buf())
        .with_capability(capability)
        .with_effects(blocking_io_effect_labels())
}

fn metadata_for_path(path: &Path) -> IoResult<FileMetadata> {
    let adapter = FsAdapter::global();
    let read_context = operation_context("file.metadata", path, CAP_IO_FS_READ);
    adapter
        .ensure_read_capability()
        .map_err(|err| err.with_context(read_context.clone()))?;
    let perm_context = operation_context("file.metadata", path, CAP_FS_PERMISSIONS_READ);
    adapter
        .ensure_permissions_read()
        .map_err(|err| err.with_context(perm_context.clone()))?;

    record_io_operation(1);
    fs::metadata(path)
        .map(FileMetadata::from_std)
        .map_err(|err| IoError::from_std(err, perm_context))
}
