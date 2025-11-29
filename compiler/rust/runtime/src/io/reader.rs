use std::io::Read;

use super::{effects::record_io_operation, FsAdapter, IoError, IoErrorKind, IoResult};

/// Core.IO 互換の Reader トレイト。
pub trait Reader {
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize>;

    fn read_exact(&mut self, buf: &mut [u8]) -> IoResult<()> {
        let mut filled = 0;
        while filled < buf.len() {
            match self.read(&mut buf[filled..]) {
                Ok(0) => {
                    return Err(IoError::new(
                        IoErrorKind::UnexpectedEof,
                        "reader reached EOF before filling buffer",
                    ))
                }
                Ok(read) => filled += read,
                Err(err) => return Err(err),
            }
        }
        Ok(())
    }
}

impl<T> Reader for T
where
    T: Read,
{
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
        FsAdapter::global().ensure_read_capability()?;
        match Read::read(self, buf) {
            Ok(bytes) => {
                record_io_operation(bytes);
                Ok(bytes)
            }
            Err(err) => Err(IoError::from_std(err, "read")),
        }
    }
}
