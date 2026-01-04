use std::cmp;

use crate::text::{
    merge_text_effects, record_text_mem_copy, record_text_unicode_event, Str, String as TextString,
    UnicodeError, UnicodeErrorKind, UnicodeResult,
};

use super::{effects as io_effects, IoError, IoErrorKind, Reader, Writer};

const UTF8_BOM: &[u8] = b"\xEF\xBB\xBF";
const DEFAULT_BUFFER_SIZE: usize = 16 * 1024;
const MIN_BUFFER_SIZE: usize = 256;
const DECODE_PHASE: &str = "io.decode";
const ENCODE_PHASE: &str = "io.encode";
const REPLACEMENT_CHAR: char = '\u{FFFD}';

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
    reset_io_effects();
    let mut chunk = vec![0_u8; options.effective_buffer_size()];
    let mut buffer = Vec::with_capacity(options.effective_buffer_size());
    let mut text = std::string::String::new();
    let mut total_consumed = 0usize;
    let mut bom_checked = matches!(options.bom_handling, BomHandling::Ignore);
    let mut reached_eof = false;

    loop {
        if !reached_eof {
            match reader.read(&mut chunk) {
                Ok(0) => {
                    reached_eof = true;
                }
                Ok(read) => {
                    if read > 0 {
                        io_effects::record_io_operation(read);
                        merge_pending_io_effects();
                    }
                    buffer.extend_from_slice(&chunk[..read]);
                }
                Err(err) => {
                    merge_pending_io_effects();
                    return Err(io_decode_error(err));
                }
            }
        }

        if !bom_checked {
            if let Err(err) = handle_bom(
                &mut buffer,
                options.bom_handling,
                &mut bom_checked,
                reached_eof,
            ) {
                merge_pending_io_effects();
                return Err(err);
            }
            if !bom_checked {
                if reached_eof && buffer.is_empty() {
                    break;
                }
                continue;
            }
        }

        if buffer.is_empty() {
            if reached_eof {
                break;
            }
            continue;
        }

        match consume_utf8_buffer(&buffer, &mut text, options.invalid_sequence, total_consumed) {
            Ok((consumed, needs_more)) => {
                if consumed > 0 {
                    buffer.drain(..consumed);
                    total_consumed = total_consumed.saturating_add(consumed);
                }

                if needs_more {
                    if reached_eof {
                        if !buffer.is_empty() {
                            record_text_unicode_event(buffer.len());
                            match options.invalid_sequence {
                                InvalidSequenceStrategy::Error => {
                                    merge_pending_io_effects();
                                    return Err(UnicodeError::invalid_utf8(total_consumed)
                                        .with_phase(DECODE_PHASE));
                                }
                                InvalidSequenceStrategy::Replace => {
                                    text.push(REPLACEMENT_CHAR);
                                    buffer.clear();
                                }
                            }
                        }
                        break;
                    }
                    continue;
                }

                if reached_eof && buffer.is_empty() {
                    break;
                }
            }
            Err(err) => {
                merge_pending_io_effects();
                return Err(err);
            }
        }
    }

    record_text_mem_copy(text.len());
    merge_pending_io_effects();
    Ok(TextString::from_std(text))
}

pub fn encode_stream<W>(
    writer: &mut W,
    text: Str<'_>,
    options: TextEncodeOptions,
) -> UnicodeResult<()>
where
    W: Writer + ?Sized,
{
    reset_io_effects();
    if options.include_bom {
        if let Err(err) = writer.write_all(UTF8_BOM) {
            merge_pending_io_effects();
            return Err(io_encode_error(err));
        }
    }

    let bytes = text.as_str().as_bytes();
    if bytes.is_empty() {
        let result = writer.flush().map_err(io_encode_error);
        merge_pending_io_effects();
        return result;
    }

    let chunk_size = options.effective_buffer_size();
    let mut offset = 0;
    while offset < bytes.len() {
        let end = (offset + chunk_size).min(bytes.len());
        if let Err(err) = writer.write_all(&bytes[offset..end]) {
            merge_pending_io_effects();
            return Err(io_encode_error(err));
        }
        offset = end;
    }
    let result = writer.flush().map_err(io_encode_error);
    merge_pending_io_effects();
    result
}

fn handle_bom(
    buffer: &mut Vec<u8>,
    handling: BomHandling,
    bom_checked: &mut bool,
    reached_eof: bool,
) -> UnicodeResult<()> {
    if *bom_checked {
        return Ok(());
    }
    match handling {
        BomHandling::Ignore => {
            *bom_checked = true;
            return Ok(());
        }
        BomHandling::Auto | BomHandling::Require => {}
    }

    if buffer.len() >= UTF8_BOM.len() {
        if buffer.starts_with(UTF8_BOM) {
            buffer.drain(..UTF8_BOM.len());
            *bom_checked = true;
            return Ok(());
        }
        if matches!(handling, BomHandling::Require) && !buffer.is_empty() {
            return Err(missing_bom_error());
        }
        *bom_checked = true;
        return Ok(());
    }

    if reached_eof {
        if matches!(handling, BomHandling::Require) && !buffer.is_empty() {
            return Err(missing_bom_error());
        }
        *bom_checked = true;
    }
    Ok(())
}

fn consume_utf8_buffer(
    buffer: &[u8],
    output: &mut std::string::String,
    strategy: InvalidSequenceStrategy,
    total_consumed: usize,
) -> UnicodeResult<(usize, bool)> {
    let mut consumed = 0usize;
    let mut needs_more_input = false;
    while consumed < buffer.len() {
        match std::str::from_utf8(&buffer[consumed..]) {
            Ok(valid) => {
                output.push_str(valid);
                consumed = buffer.len();
                needs_more_input = false;
                break;
            }
            Err(err) => {
                let valid_up_to = err.valid_up_to();
                if valid_up_to > 0 {
                    let start = consumed;
                    let end = start + valid_up_to;
                    let prefix = std::str::from_utf8(&buffer[start..end])
                        .expect("valid UTF-8 slice per Utf8Error::valid_up_to");
                    output.push_str(prefix);
                    consumed = end;
                    continue;
                }

                if let Some(error_len) = err.error_len() {
                    record_text_unicode_event(error_len);
                    match strategy {
                        InvalidSequenceStrategy::Error => {
                            return Err(UnicodeError::invalid_utf8(total_consumed + consumed)
                                .with_phase(DECODE_PHASE));
                        }
                        InvalidSequenceStrategy::Replace => {
                            output.push(REPLACEMENT_CHAR);
                            consumed = consumed.saturating_add(error_len);
                            continue;
                        }
                    }
                } else {
                    needs_more_input = true;
                    break;
                }
            }
        }
    }
    Ok((consumed, needs_more_input))
}

fn missing_bom_error() -> UnicodeError {
    UnicodeError::new(
        UnicodeErrorKind::DecodeFailure,
        "UTF-8 BOM is required but missing",
    )
    .with_phase(DECODE_PHASE)
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
        message.push_str(&format!(" (operation: {})", context.operation()));
        if let Some(bytes) = context.bytes_processed() {
            message.push_str(&format!(", bytes_processed={bytes}"));
        }
    }
    let unexpected_eof = matches!(error.kind(), IoErrorKind::UnexpectedEof);
    let mut unicode_error = UnicodeError::new(kind, message)
        .with_phase(phase)
        .with_source(error);
    if unexpected_eof {
        unicode_error = unicode_error.with_phase("io.decode.eof");
    }
    unicode_error
}

fn merge_pending_io_effects() {
    let effects = io_effects::take_recorded_effects();
    merge_text_effects(effects);
}

fn reset_io_effects() {
    let _ = io_effects::take_recorded_effects();
}
