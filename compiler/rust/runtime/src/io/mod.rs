//! Core.IO の土台実装。
//! Reader/Writer の薄いラッパと、Text ストリーミング API を公開する。

use std::{fs::File as StdFile, path::Path};

mod adapters;
mod buffered;
mod context;
mod effects;
mod env;
mod error;
mod file;
mod metadata;
mod options;
mod permissions;
mod scope;
mod reader;
mod text_stream;
mod watcher;
mod writer;

pub use adapters::{FsAdapter, WatcherAdapter};
pub use buffered::{buffered, read_line, BufferedReader};
pub use context::{BufferStats, IoContext, WatchStats};
pub use effects::take_io_effects_snapshot;
pub use env::{time_env_snapshot, TimeEnvSnapshot};
pub use error::{IoError, IoErrorKind, IoResult};
pub use file::File;
pub use metadata::FileMetadata;
pub use options::FileOptions;
pub use permissions::FilePermissions;
pub use scope::{
    leak_tracker_snapshot, reset_leak_tracker, ScopedFileMode, ScopeGuard, TempDirGuard,
    with_file, with_temp_dir,
};
pub use reader::Reader;
pub use watcher::{
    close_watcher, watch, watch_with_limits, WatchEvent, WatchLimits, Watcher,
};
pub use text_stream::{
    decode_stream, encode_stream, BomHandling, InvalidSequenceStrategy, TextDecodeOptions,
    TextEncodeOptions,
};
pub use writer::Writer;

const IO_COPY_BUFFER_SIZE: usize = 64 * 1024;

/// Reader から Writer へストリームをコピーする。
pub fn copy<R, W>(reader: &mut R, writer: &mut W) -> IoResult<u64>
where
    R: Reader + ?Sized,
    W: Writer + ?Sized,
{
    let mut total: u64 = 0;
    let mut buffer = [0_u8; IO_COPY_BUFFER_SIZE];
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|err| err.map_context(|ctx| ctx.with_bytes_processed(total)))?;
        if read == 0 {
            break;
        }
        writer
            .write_all(&buffer[..read])
            .map_err(|err| err.map_context(|ctx| ctx.with_bytes_processed(total)))?;
        total = total.saturating_add(read as u64);
    }
    Ok(total)
}

/// `std::fs::File` を開いて `Reader` クロージャへ委譲する。
pub fn with_reader<P, F, T>(path: P, f: F) -> IoResult<T>
where
    P: AsRef<Path>,
    F: FnOnce(&mut dyn Reader) -> IoResult<T>,
{
    let path = path.as_ref();
    FsAdapter::global()
        .ensure_read_capability()
        .map_err(|err| err.with_context(with_reader_context(path)))?;
    effects::record_io_operation(1);
    let mut file =
        StdFile::open(path).map_err(|err| IoError::from_std(err, with_reader_context(path)))?;
    let reader: &mut dyn Reader = &mut file;
    f(reader)
}

fn with_reader_context(path: &Path) -> IoContext {
    IoContext::new("with_reader")
        .with_path(path)
        .with_capability(adapters::CAP_IO_FS_READ)
        .with_effects(effects::blocking_io_effect_labels())
}
