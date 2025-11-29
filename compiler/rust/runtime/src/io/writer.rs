use std::io::Write;

use crate::text::Bytes;

use super::{
    adapters::CAP_IO_FS_WRITE,
    effects::{blocking_io_effect_labels, record_io_operation},
    FsAdapter, IoContext, IoError, IoErrorKind, IoResult,
};

/// Core.IO 互換の Writer トレイト。
pub trait Writer {
    fn write(&mut self, buf: &[u8]) -> IoResult<usize>;
    fn flush(&mut self) -> IoResult<()>;

    fn write_all(&mut self, mut buf: &[u8]) -> IoResult<()> {
        while !buf.is_empty() {
            match self.write(buf) {
                Ok(0) => {
                    return Err(IoError::new(
                        IoErrorKind::WriteZero,
                        "writer wrote zero bytes",
                    ))
                }
                Ok(written) => buf = &buf[written..],
                Err(err) => return Err(err),
            }
        }
        Ok(())
    }

    /// `Bytes` をそのまま書き込む。
    fn write_bytes(&mut self, bytes: &Bytes) -> IoResult<usize> {
        self.write(bytes.as_slice())
    }

    /// `Bytes` をすべて書き込む。
    fn write_all_bytes(&mut self, bytes: &Bytes) -> IoResult<()> {
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
        record_io_operation(1);
        match Write::write(self, buf) {
            Ok(bytes) => Ok(bytes),
            Err(err) => Err(IoError::from_std(err, write_context("write"))),
        }
    }

    fn flush(&mut self) -> IoResult<()> {
        Write::flush(self).map_err(|err| IoError::from_std(err, write_context("flush")))
    }
}

fn write_context(operation: &'static str) -> IoContext {
    IoContext::new(operation)
        .with_capability(CAP_IO_FS_WRITE)
        .with_effects(blocking_io_effect_labels())
}
