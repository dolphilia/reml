use std::io::{Cursor, Read, Write};

use reml_runtime::text::{
    decode_stream, encode_stream, take_text_effects_snapshot, BomHandling, InvalidSequenceStrategy,
    Str, TextDecodeOptions, TextEncodeOptions, UnicodeErrorKind,
};

#[test]
fn decode_stream_strips_bom_by_default() {
    let data = b"\xEF\xBB\xBFHello Reml!".to_vec();
    let mut reader = Cursor::new(data);
    let text = decode_stream(&mut reader, TextDecodeOptions::default()).expect("decode");
    assert_eq!(text.as_str(), "Hello Reml!");
}

#[test]
fn decode_stream_requires_bom_when_requested() {
    let mut reader = Cursor::new(b"abc".to_vec());
    let err = decode_stream(
        &mut reader,
        TextDecodeOptions::default().with_bom_handling(BomHandling::Require),
    )
    .expect_err("missing BOM should fail");
    assert_eq!(err.kind(), UnicodeErrorKind::DecodeFailure);
}

#[test]
fn decode_stream_allows_replacement_strategy() {
    let bytes = vec![0x66, 0x6f, 0xff, 0x6f];
    let mut reader = Cursor::new(bytes);
    let options = TextDecodeOptions::default()
        .with_invalid_sequence(InvalidSequenceStrategy::Replace)
        .with_bom_handling(BomHandling::Ignore);
    let text = decode_stream(&mut reader, options).expect("decode with replacement");
    assert_eq!(text.as_str(), "fo\u{fffd}o");
}

#[test]
fn decode_stream_propagates_io_errors() {
    struct FailingReader;
    impl Read for FailingReader {
        fn read(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
            Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "boom",
            ))
        }
    }
    let mut reader = FailingReader;
    let err = decode_stream(&mut reader, TextDecodeOptions::default()).expect_err("should fail");
    assert_eq!(err.kind(), UnicodeErrorKind::DecodeFailure);
    assert_eq!(err.phase(), "io.decode.eof");
}

#[test]
fn encode_stream_writes_data_including_bom() {
    let mut writer = Cursor::new(Vec::new());
    let text = Str::from("Reml");
    encode_stream(
        &mut writer,
        text,
        TextEncodeOptions::default()
            .with_bom(true)
            .with_buffer_size(2),
    )
    .expect("encode");
    let buffer = writer.into_inner();
    assert_eq!(&buffer[..3], b"\xEF\xBB\xBF");
    assert_eq!(&buffer[3..], b"Reml");
}

#[test]
fn encode_stream_reports_write_failures() {
    struct FailingWriter;
    impl Write for FailingWriter {
        fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::new(std::io::ErrorKind::WriteZero, "boom"))
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut writer = FailingWriter;
    let err = encode_stream(&mut writer, Str::from("data"), TextEncodeOptions::default())
        .expect_err("write failure");
    assert_eq!(err.kind(), UnicodeErrorKind::EncodeFailure);
}

#[test]
fn decode_stream_handles_chunk_boundaries() {
    let data = "あいうえお".as_bytes().to_vec();
    let mut reader = Cursor::new(data);
    let options = TextDecodeOptions::default().with_buffer_size(2);
    let text = decode_stream(&mut reader, options).expect("decode");
    assert_eq!(text.as_str(), "あいうえお");
}

#[test]
fn decode_stream_records_effects() {
    take_text_effects_snapshot();
    let mut reader = Cursor::new(b"chunked".to_vec());
    let options = TextDecodeOptions::default().with_buffer_size(3);
    let text = decode_stream(&mut reader, options).expect("decode");
    assert_eq!(text.as_str(), "chunked");
    let effects = take_text_effects_snapshot();
    assert!(effects.io, "io effect should be recorded");
    assert!(effects.mem, "mem effect should be recorded");
    assert!(!effects.unicode, "unicode effect should be absent");
    assert_eq!(effects.mem_bytes, 7);
}

#[test]
fn decode_stream_replacement_marks_unicode_effect() {
    take_text_effects_snapshot();
    let bytes = vec![0xe3, 0x81, 0x82, 0xff, 0xe3, 0x81, 0x84];
    let mut reader = Cursor::new(bytes);
    let options = TextDecodeOptions::default()
        .with_invalid_sequence(InvalidSequenceStrategy::Replace)
        .with_buffer_size(4);
    let text = decode_stream(&mut reader, options).expect("decode with replace");
    assert_eq!(text.as_str(), "あ�い");
    let effects = take_text_effects_snapshot();
    assert!(effects.unicode, "unicode effect should be recorded");
}
