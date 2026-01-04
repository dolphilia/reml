use std::io::Write;

use crate::text::Bytes;

use super::{
    adapters::CAP_IO_FS_WRITE,
    effects::{blocking_io_effect_labels, record_io_operation, take_io_effects_snapshot},
    FsAdapter, IoContext, IoError, IoErrorKind, IoResult,
};

/// Core.IO 互換の Writer トレイト。
pub trait Writer {
    fn write(&mut self, buf: &[u8]) -> IoResult<usize>;
    fn flush(&mut self) -> IoResult<()>;

    fn write_all(&mut self, mut buf: &[u8]) -> IoResult<()> {
        let mut total_written: u64 = 0;
        while !buf.is_empty() {
            match self.write(buf) {
                Ok(0) => {
                    return Err(
                        IoError::new(IoErrorKind::WriteZero, "writer wrote zero bytes")
                            .with_context(
                                write_context("write_all").with_bytes_processed(total_written),
                            ),
                    )
                }
                Ok(written) => {
                    total_written = total_written.saturating_add(written as u64);
                    buf = &buf[written..];
                }
                Err(err) => {
                    return Err(err.map_context(|ctx| ctx.with_bytes_processed(total_written)))
                }
            }
        }
        Ok(())
    }

    /// `Bytes` をそのまま書き込む。
    fn write_bytes(&mut self, bytes: Bytes) -> IoResult<usize> {
        self.write(bytes.as_slice())
    }

    /// `Bytes` をすべて書き込む。
    fn write_all_bytes(&mut self, bytes: Bytes) -> IoResult<()> {
        self.write_all(bytes.as_slice())
    }
}

impl<T> Writer for T
where
    T: Write,
{
    fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
        FsAdapter::global()
            .ensure_write_capability()
            .map_err(|err| err.with_context(write_context("write")))?;
        record_io_operation(buf.len());
        match Write::write(self, buf) {
            Ok(bytes) => {
                take_io_effects_snapshot();
                Ok(bytes)
            }
            Err(err) => {
                let effects = take_io_effects_snapshot();
                Err(IoError::from_std(
                    err,
                    write_context("write")
                        .with_bytes_processed(buf.len() as u64)
                        .with_effects(effects),
                ))
            }
        }
    }

    fn flush(&mut self) -> IoResult<()> {
        record_io_operation(0);
        match Write::flush(self) {
            Ok(()) => {
                take_io_effects_snapshot();
                Ok(())
            }
            Err(err) => {
                let effects = take_io_effects_snapshot();
                Err(IoError::from_std(
                    err,
                    write_context("flush").with_effects(effects),
                ))
            }
        }
    }
}

fn write_context(operation: &'static str) -> IoContext {
    IoContext::new(operation)
        .with_capability(CAP_IO_FS_WRITE)
        .with_effects(blocking_io_effect_labels())
}
