use std::cmp;

use crate::text::{String as TextString, Str, UnicodeError, UnicodeErrorKind, UnicodeResult};

use super::{IoError, IoErrorKind, Reader, Writer};

const UTF8_BOM: &[u8] = b"\xEF\xBB\xBF";
const DEFAULT_BUFFER_SIZE: usize = 16 * 1024;
const MIN_BUFFER_SIZE: usize = 256;
const DECODE_PHASE: &str = "io.decode";
const ENCODE_PHASE: &str = "io.encode";

/// BOM の扱い方。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BomHandling {
    Auto,
    Require,
    Ignore,
}

/// 不正 UTF-8 シーケンスの扱い。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidSequenceStrategy {
    Error,
    Replace,
}

/// ストリーム decode 用オプション。
#[derive(Debug, Clone)]
pub struct TextDecodeOptions {
    pub buffer_size: usize,
    pub bom_handling: BomHandling,
    pub invalid_sequence: InvalidSequenceStrategy,
}

impl Default for TextDecodeOptions {
    fn default() -> Self {
        Self {
            buffer_size: DEFAULT_BUFFER_SIZE,
            bom_handling: BomHandling::Auto,
            invalid_sequence: InvalidSequenceStrategy::Error,
        }
    }
}

impl TextDecodeOptions {
    pub fn with_buffer_size(mut self, size: usize) -> Self {
        self.buffer_size = size;
        self
    }

    pub fn with_bom_handling(mut self, handling: BomHandling) -> Self {
        self.bom_handling = handling;
        self
    }

    pub fn with_invalid_sequence(mut self, strategy: InvalidSequenceStrategy) -> Self {
        self.invalid_sequence = strategy;
        self
    }

    fn effective_buffer_size(&self) -> usize {
        cmp::max(self.buffer_size, MIN_BUFFER_SIZE)
    }
}

/// ストリーム encode 用オプション。
#[derive(Debug, Clone)]
pub struct TextEncodeOptions {
    pub buffer_size: usize,
    pub include_bom: bool,
}

impl Default for TextEncodeOptions {
    fn default() -> Self {
        Self {
            buffer_size: DEFAULT_BUFFER_SIZE,
            include_bom: false,
        }
    }
}

impl TextEncodeOptions {
    pub fn with_buffer_size(mut self, size: usize) -> Self {
        self.buffer_size = size;
        self
    }

    pub fn with_bom(mut self, include_bom: bool) -> Self {
        self.include_bom = include_bom;
        self
    }

    fn effective_buffer_size(&self) -> usize {
        cmp::max(self.buffer_size, MIN_BUFFER_SIZE)
    }
}

pub fn decode_stream<R>(reader: &mut R, options: TextDecodeOptions) -> UnicodeResult<TextString>
where
    R: Reader + ?Sized,
{
    let mut sink = Vec::new();
    let mut chunk = vec![0_u8; options.effective_buffer_size()];
    loop {
        match reader.read(&mut chunk) {
            Ok(0) => break,
            Ok(read) => sink.extend_from_slice(&chunk[..read]),
            Err(err) => return Err(io_decode_error(err)),
        }
    }
    match options.bom_handling {
        BomHandling::Auto => {
            if sink.starts_with(UTF8_BOM) {
                sink.drain(..UTF8_BOM.len());
            }
        }
        BomHandling::Require => {
            if sink.starts_with(UTF8_BOM) {
                sink.drain(..UTF8_BOM.len());
            } else if !sink.is_empty() {
                return Err(
                    UnicodeError::new(
                        UnicodeErrorKind::DecodeFailure,
                        "UTF-8 BOM is required but missing",
                    )
                    .with_phase(DECODE_PHASE),
                );
            }
        }
        BomHandling::Ignore => {}
    }

    match options.invalid_sequence {
        InvalidSequenceStrategy::Error => {
            let text = std::string::String::from_utf8(sink)
                .map_err(|err| UnicodeError::invalid_utf8(err.utf8_error().valid_up_to()).with_phase(DECODE_PHASE))?;
            Ok(TextString::from_std(text))
        }
        InvalidSequenceStrategy::Replace => {
            let owned = std::string::String::from_utf8_lossy(&sink).into_owned();
            Ok(TextString::from_std(owned))
        }
    }
}

pub fn encode_stream<W>(writer: &mut W, text: Str<'_>, options: TextEncodeOptions) -> UnicodeResult<()>
where
    W: Writer + ?Sized,
{
    if options.include_bom {
        writer
            .write_all(UTF8_BOM)
            .map_err(io_encode_error)?;
    }

    let bytes = text.as_str().as_bytes();
    if bytes.is_empty() {
        return writer.flush().map_err(io_encode_error);
    }

    let chunk_size = options.effective_buffer_size();
    let mut offset = 0;
    while offset < bytes.len() {
        let end = (offset + chunk_size).min(bytes.len());
        writer
            .write_all(&bytes[offset..end])
            .map_err(io_encode_error)?;
        offset = end;
    }
    writer.flush().map_err(io_encode_error)
}

fn io_decode_error(error: IoError) -> UnicodeError {
    map_io_error(error, UnicodeErrorKind::DecodeFailure, DECODE_PHASE)
}

fn io_encode_error(error: IoError) -> UnicodeError {
    map_io_error(error, UnicodeErrorKind::EncodeFailure, ENCODE_PHASE)
}

fn map_io_error(error: IoError, kind: UnicodeErrorKind, phase: &'static str) -> UnicodeError {
    let mut message = format!("IO error: {}", error.message());
    if let Some(context) = error.context() {
        message.push_str(&format!(" (operation: {})", context.operation));
        if let Some(bytes) = context.bytes_processed {
            message.push_str(&format!(", bytes_processed={bytes}"));
        }
    }
    let mut unicode_error = UnicodeError::new(kind, message).with_phase(phase);
    if matches!(error.kind(), IoErrorKind::UnexpectedEof) {
        unicode_error = unicode_error.with_phase("io.decode.eof");
    }
    unicode_error
}
