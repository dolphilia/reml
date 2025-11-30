use std::cmp;

use crate::text::Str;

use super::{
    adapters,
    buffer::IoCopyBuffer,
    effects::{record_buffer_allocation, record_buffer_usage},
    take_io_effects_snapshot, FsAdapter, IoContext, IoError, IoErrorKind, IoResult, Reader,
};

const MIN_BUFFER_CAPACITY: usize = 4 * 1024;
const MAX_BUFFER_CAPACITY: usize = 1024 * 1024;

/// Reader をリングバッファで包み、`read_line` などのユーティリティを提供する。
#[derive(Debug)]
pub struct BufferedReader<R> {
    inner: R,
    buffer: IoCopyBuffer,
    start: usize,
    end: usize,
    context: IoContext,
}

/// `BufferedReader` を生成するショートカット。
pub fn buffered<R>(reader: R, capacity: usize) -> IoResult<BufferedReader<R>>
where
    R: Reader,
{
    BufferedReader::new(reader, capacity)
}

/// `BufferedReader` に蓄積したデータから 1 行ずつ読み出す。
pub fn read_line<R>(reader: &mut BufferedReader<R>) -> IoResult<Option<Str<'static>>>
where
    R: Reader,
{
    reader.read_line()
}

impl<R> BufferedReader<R>
where
    R: Reader,
{
    pub fn new(reader: R, capacity: usize) -> IoResult<Self> {
        let normalized = normalize_capacity(capacity)?;
        let mut context = buffered_reader_context();
        FsAdapter::global()
            .ensure_buffered_io_capability()
            .map_err(|err| err.with_context(context.clone()))?;
        record_buffer_allocation(normalized);
        context.set_effects(take_io_effects_snapshot());
        context.update_buffer_usage(normalized, 0);
        Ok(Self {
            inner: reader,
            buffer: IoCopyBuffer::lease(normalized),
            start: 0,
            end: 0,
            context,
        })
    }

    pub fn capacity(&self) -> usize {
        self.buffer.len()
    }

    pub fn context(&self) -> &IoContext {
        &self.context
    }

    fn available(&self) -> usize {
        self.end.saturating_sub(self.start)
    }

    fn consume(&mut self, amount: usize) {
        let available = self.available();
        let consumed = cmp::min(amount, available);
        self.start = cmp::min(self.start + consumed, self.buffer.len());
        if self.start == self.end {
            self.start = 0;
            self.end = 0;
        }
        self.context
            .update_buffer_usage(self.buffer.len(), self.available());
    }

    fn refill(&mut self) -> IoResult<usize> {
        self.start = 0;
        self.end = 0;
        let bytes = self.inner.read(&mut self.buffer[..])?;
        self.end = bytes;
        self.context
            .update_buffer_usage(self.buffer.len(), self.available());
        self.context.set_effects(take_io_effects_snapshot());
        Ok(bytes)
    }

    pub fn read_line(&mut self) -> IoResult<Option<Str<'static>>> {
        let mut line_buffer: Vec<u8> = Vec::new();

        loop {
            if self.available() == 0 {
                let read = self.refill()?;
                if read == 0 {
                    if line_buffer.is_empty() {
                        return Ok(None);
                    }
                    break;
                }
            }

            if let Some(pos) = self.buffer[self.start..self.end]
                .iter()
                .position(|&byte| byte == b'\n')
            {
                let absolute = self.start + pos;
                line_buffer.extend_from_slice(&self.buffer[self.start..absolute]);
                self.consume(pos + 1);
                if let Some(b'\r') = line_buffer.last() {
                    line_buffer.pop();
                }
                break;
            } else {
                let chunk = &self.buffer[self.start..self.end];
                line_buffer.extend_from_slice(chunk);
                let consumed = chunk.len();
                self.consume(consumed);
            }
        }

        record_buffer_usage(line_buffer.len());
        self.context.set_effects(take_io_effects_snapshot());
        let text = std::string::String::from_utf8(line_buffer).map_err(|err| {
            IoError::new(IoErrorKind::InvalidInput, err.to_string())
                .with_context(self.context.clone())
        })?;
        Ok(Some(text.into()))
    }
}

impl<R> Reader for BufferedReader<R>
where
    R: Reader,
{
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
        if buf.is_empty() {
            return Ok(0);
        }

        if self.available() == 0 {
            self.refill()?;
        }

        let available = self.available();
        if available == 0 {
            return Ok(0);
        }

        let to_copy = cmp::min(buf.len(), available);
        buf[..to_copy].copy_from_slice(&self.buffer[self.start..self.start + to_copy]);
        self.consume(to_copy);
        Ok(to_copy)
    }
}

fn normalize_capacity(requested: usize) -> IoResult<usize> {
    if requested > MAX_BUFFER_CAPACITY {
        return Err(IoError::new(
            IoErrorKind::InvalidInput,
            "buffer capacity exceeds supported maximum (1 MiB)",
        )
        .with_context(buffered_reader_context()));
    }
    let normalized = cmp::max(requested, MIN_BUFFER_CAPACITY);
    Ok(normalized)
}

fn buffered_reader_context() -> IoContext {
    IoContext::new("buffered_reader").with_capability(adapters::CAP_MEMORY_BUFFERED_IO)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn clamps_small_capacity_and_records_mem_effect() {
        let cursor = Cursor::new(b"".to_vec());
        let reader = buffered(cursor, 1024).expect("buffered ok");
        let effects_snapshot = reader.context().effects();
        assert!(effects_snapshot.mem);
        assert!(effects_snapshot.mem_bytes >= MIN_BUFFER_CAPACITY);
        assert_eq!(reader.capacity(), MIN_BUFFER_CAPACITY);
        assert_eq!(
            reader.context().capability(),
            Some(adapters::CAP_MEMORY_BUFFERED_IO)
        );
        let buffer_stats = reader
            .context()
            .buffer()
            .expect("buffer stats should be recorded");
        assert_eq!(buffer_stats.capacity() as usize, MIN_BUFFER_CAPACITY);
    }

    #[test]
    fn rejects_excessive_capacity() {
        let cursor = Cursor::new(b"".to_vec());
        let err = buffered(cursor, MAX_BUFFER_CAPACITY + 1).unwrap_err();
        assert_eq!(err.kind(), IoErrorKind::InvalidInput);
    }

    #[test]
    fn read_line_supports_crlf_and_last_line() {
        let cursor = Cursor::new(b"line1\nline2\r\nlast".to_vec());
        let mut reader = buffered(cursor, 8 * 1024).expect("buffered ok");

        let line1 = read_line(&mut reader).expect("line1").expect("value");
        assert_eq!(line1.as_str(), "line1");

        let line2 = read_line(&mut reader).expect("line2").expect("value");
        assert_eq!(line2.as_str(), "line2");

        let last = read_line(&mut reader).expect("last").expect("value");
        assert_eq!(last.as_str(), "last");

        assert!(read_line(&mut reader).expect("eof").is_none());
    }

    #[test]
    fn read_line_returns_empty_string_for_blank_lines() {
        let cursor = Cursor::new(b"\n\n".to_vec());
        let mut reader = buffered(cursor, MIN_BUFFER_CAPACITY).expect("buffered ok");
        let first = read_line(&mut reader).expect("blank").expect("value");
        assert_eq!(first.as_str(), "");
        let second = read_line(&mut reader).expect("blank").expect("value");
        assert_eq!(second.as_str(), "");
        assert!(read_line(&mut reader).expect("eof").is_none());
    }
}
