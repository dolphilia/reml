use std::io::Write;

use super::{effects::record_io_operation, FsAdapter, IoError, IoErrorKind, IoResult};

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
}

impl<T> Writer for T
where
    T: Write,
{
    fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
        FsAdapter::global().ensure_write_capability()?;
        match Write::write(self, buf) {
            Ok(bytes) => {
                record_io_operation(bytes);
                Ok(bytes)
            }
            Err(err) => Err(IoError::from_std(err, "write")),
        }
    }

    fn flush(&mut self) -> IoResult<()> {
        Write::flush(self).map_err(|err| IoError::from_std(err, "flush"))
    }
}
