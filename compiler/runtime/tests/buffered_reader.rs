use std::io::Cursor;

use reml_runtime::io::{buffered, read_line};
use serde::Deserialize;

const CONTEXT_SNAPSHOT_JSON: &str =
    include_str!("data/core_io/buffered_reader/context_snapshot.json");

#[derive(Debug, Deserialize)]
struct ContextSnapshot {
    expected: ExpectedSnapshot,
}

#[derive(Debug, Deserialize)]
struct ExpectedSnapshot {
    operation: String,
    capability: String,
    buffer: BufferSnapshot,
}

#[derive(Debug, Deserialize)]
struct BufferSnapshot {
    capacity: u32,
}

#[test]
fn buffered_reader_context_snapshot_matches_expected_golden() {
    let cursor = Cursor::new(b"line one\n".to_vec());
    let reader = buffered(cursor, 1024).expect("buffered reader should succeed");
    let context = reader.context();
    let snapshot: ContextSnapshot =
        serde_json::from_str(CONTEXT_SNAPSHOT_JSON).expect("snapshot json should parse");
    assert_eq!(context.operation(), snapshot.expected.operation);
    assert_eq!(
        context.capability().expect("capability should be set"),
        snapshot.expected.capability
    );
    let buffer_stats = context
        .buffer()
        .expect("buffer stats should be recorded for buffered reader");
    assert_eq!(buffer_stats.capacity(), snapshot.expected.buffer.capacity);
}

#[test]
fn buffered_reader_updates_fill_stats_after_read_line() {
    let cursor = Cursor::new(b"first\nsecond".to_vec());
    let mut reader = buffered(cursor, 4096).expect("buffered reader should succeed");
    let first = read_line(&mut reader)
        .expect("read_line should succeed")
        .expect("line value expected");
    assert_eq!(first.as_str(), "first");
    let stats = reader
        .context()
        .buffer()
        .expect("buffer stats should exist after read_line");
    assert!(stats.fill() <= stats.capacity());
}
