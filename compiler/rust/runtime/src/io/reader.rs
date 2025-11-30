use std::io::Read;

use crate::text::{Bytes, UnicodeError};

use super::{
    adapters::CAP_IO_FS_READ,
    effects::{blocking_io_effect_labels, record_io_operation, take_io_effects_snapshot},
    FsAdapter, IoContext, IoError, IoErrorKind, IoResult, Writer,
};

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

    /// 指定したサイズのバッファを確保して読み出し、`Bytes` として返す。
    fn read_exact_bytes(&mut self, size: usize) -> IoResult<Bytes> {
        let mut buffer = vec![0_u8; size];
        self.read_exact(&mut buffer)?;
        Bytes::from_vec(buffer).map_err(|error| unicode_error_to_io(error, "read_exact_bytes"))
    }

    /// EOF まで読み出し、`Bytes` として返す。メモリ効果を伴う。
    fn read_to_end(&mut self) -> IoResult<Bytes> {
        let mut buffer = Vec::with_capacity(8 * 1024);
        let mut chunk = [0_u8; 8 * 1024];
        loop {
            let read = self.read(&mut chunk)?;
            if read == 0 {
                break;
            }
            buffer.extend_from_slice(&chunk[..read]);
        }
        Bytes::from_vec(buffer).map_err(|error| unicode_error_to_io(error, "read_to_end"))
    }

    /// `Writer` へコピーするショートカット。
    fn copy_to<W: Writer>(&mut self, writer: &mut W) -> IoResult<u64>
    where
        Self: Sized,
    {
        super::copy(self, writer)
    }
}

impl<T> Reader for T
where
    T: Read,
{
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
        FsAdapter::global()
            .ensure_read_capability()
            .map_err(|err| err.with_context(read_context("read")))?;
        record_io_operation(buf.len());
        match Read::read(self, buf) {
            Ok(bytes) => {
                take_io_effects_snapshot();
                Ok(bytes)
            }
            Err(err) => {
                let effects = take_io_effects_snapshot();
                Err(IoError::from_std(
                    err,
                    read_context("read")
                        .with_bytes_processed(buf.len() as u64)
                        .with_effects(effects),
                ))
            }
        }
    }
}

fn read_context(operation: &'static str) -> IoContext {
    IoContext::new(operation)
        .with_capability(CAP_IO_FS_READ)
        .with_effects(blocking_io_effect_labels())
}

fn unicode_error_to_io(error: UnicodeError, operation: &'static str) -> IoError {
    IoError::new(IoErrorKind::InvalidInput, error.message().to_owned())
        .with_context(IoContext::new(operation))
}
