//! Core.IO の土台実装。
//! Reader/Writer の薄いラッパと、Text ストリーミング API を公開する。

use std::{fs::File as StdFile, path::Path};

mod adapters;
mod bridge;
mod buffer;
mod buffered;
mod context;
mod effects;
mod env;
mod error;
mod file;
mod metadata;
mod options;
mod permissions;
mod reader;
mod scope;
mod text_stream;
mod watcher;
mod watcher_audit;
mod writer;

use buffer::IoCopyBuffer;

pub use adapters::{FsAdapter, WatcherAdapter};
pub(crate) use bridge::{attach_bridge_stage_metadata, record_bridge_stage_probe};
pub use buffered::{buffered, read_line, BufferedReader};
pub use context::{BufferStats, IoContext, WatchStats};
pub(crate) use effects::record_io_operation;
pub use effects::take_io_effects_snapshot;
pub use env::{time_env_snapshot, TimeEnvSnapshot};
pub use error::{IoError, IoErrorKind, IoResult};
pub use file::File;
pub use metadata::FileMetadata;
pub use options::FileOptions;
pub use permissions::FilePermissions;
pub use reader::Reader;
pub use scope::{
    leak_tracker_snapshot, reset_leak_tracker, with_file, with_temp_dir, ScopeGuard,
    ScopedFileMode, TempDirGuard,
};
pub use text_stream::{
    decode_stream, encode_stream, BomHandling, InvalidSequenceStrategy, TextDecodeOptions,
    TextEncodeOptions,
};
pub use watcher::{close_watcher, watch, watch_with_limits, WatchEvent, WatchLimits, Watcher};
pub use watcher_audit::{WatcherAuditEvent, WatcherAuditSnapshot};
pub use writer::Writer;

const IO_COPY_BUFFER_SIZE: usize = 64 * 1024;

/// Reader から Writer へストリームをコピーする。
pub fn copy<R, W>(reader: &mut R, writer: &mut W) -> IoResult<u64>
where
    R: Reader + ?Sized,
    W: Writer + ?Sized,
{
    let mut total: u64 = 0;
    effects::record_buffer_allocation(IO_COPY_BUFFER_SIZE);
    let mut buffer = IoCopyBuffer::lease(IO_COPY_BUFFER_SIZE);
    loop {
        let read = reader
            .read(&mut buffer[..])
            .map_err(|err| err.map_context(|ctx| ctx.with_bytes_processed(total)))?;
        if read == 0 {
            break;
        }
        effects::record_buffer_usage(read);
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
    let context = with_reader_context(path);
    FsAdapter::global()
        .ensure_read_capability()
        .map_err(|err| err.with_context(context.clone()))?;
    effects::record_io_operation(0);
    let mut file = match StdFile::open(path) {
        Ok(file) => {
            effects::take_io_effects_snapshot();
            file
        }
        Err(err) => {
            let effects = effects::take_io_effects_snapshot();
            return Err(IoError::from_std(err, context.with_effects(effects)));
        }
    };
    let reader: &mut dyn Reader = &mut file;
    f(reader)
}

fn with_reader_context(path: &Path) -> IoContext {
    IoContext::new("with_reader")
        .with_path(path)
        .with_capability(adapters::CAP_IO_FS_READ)
        .with_effects(effects::blocking_io_effect_labels())
}

#[cfg(all(test, feature = "core-io"))]
mod tests {
    use super::*;
    use std::io::{self, Read};
    use tempfile::tempdir;

    struct FailingReader;

    impl Read for FailingReader {
        fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::new(
                io::ErrorKind::Other,
                "forced read failure for tests",
            ))
        }
    }

    #[test]
    fn reader_failure_carries_effects_and_bytes_processed() {
        let mut reader = FailingReader;
        let mut buffer = [0_u8; 32];
        let error = <FailingReader as Reader>::read(&mut reader, &mut buffer)
            .expect_err("read should surface IoError");
        let context = error
            .context()
            .expect("IoContext should be attached to read failure");
        let effects = context.effects();
        assert!(effects.io, "io effect flag should be set");
        assert!(effects.io_blocking, "io_blocking effect flag should be set");
        assert_eq!(
            context.bytes_processed(),
            Some(buffer.len() as u64),
            "bytes_processed should match requested buffer length"
        );
        assert_eq!(
            context.capability(),
            Some(adapters::CAP_IO_FS_READ),
            "read capability metadata should be recorded"
        );
    }

    #[test]
    fn with_reader_error_reports_path_and_capability() {
        let tmp = tempdir().expect("temp dir");
        let missing = tmp.path().join("missing.txt");
        let error = with_reader(&missing, |_reader| Ok(()))
            .expect_err("opening a missing file should fail");
        let context = error
            .context()
            .expect("IoContext should be attached to with_reader failure");
        assert_eq!(
            context.capability(),
            Some(adapters::CAP_IO_FS_READ),
            "with_reader must capture the read capability id"
        );
        assert_eq!(
            context.path().map(|path| path.as_path()),
            Some(missing.as_path()),
            "missing path should be propagated"
        );
        let effects = context.effects();
        assert!(effects.io, "io effect flag must be set");
        assert!(effects.io_blocking, "blocking flag must remain set");
    }
}
