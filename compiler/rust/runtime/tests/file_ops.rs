use std::fs;
use std::io::{Read, Write};

use serde_json::Value;
use tempfile::tempdir;

use reml_runtime::io::{File, FileOptions, IoErrorKind};

const METADATA_GOLDEN: &str =
    include_str!("golden/core_io/file_ops/metadata_basic.json");

#[test]
fn file_create_write_metadata_remove() {
    let dir = tempdir().expect("temp dir");
    let path = dir.path().join("sample.txt");

    let options = FileOptions::new()
        .write(true)
        .append(false)
        .truncate(true)
        .create(true);
    let mut file = File::create(&path, options).expect("create file");
    file.write_all(b"hello world").expect("write data");
    file.sync_all().expect("sync all");

    let metadata = file.metadata().expect("metadata");
    assert_eq!(metadata.size(), 11);
    assert!(!metadata.is_dir());

    let actual = serde_json::json!({
        "size": metadata.size(),
        "is_dir": metadata.is_dir(),
        "readonly": metadata.is_readonly(),
    });
    let expected: Value =
        serde_json::from_str(METADATA_GOLDEN).expect("metadata golden JSON");
    assert_eq!(actual, expected);

    let mut reopen = File::open(&path).expect("open after create");
    let mut buffer = Vec::new();
    reopen
        .read_to_end(&mut buffer)
        .expect("read via std::io::Read");
    assert_eq!(buffer, b"hello world");

    File::remove(&path).expect("remove file");
    assert!(!path.exists());
}

#[test]
fn file_open_missing_returns_error_with_path() {
    let dir = tempdir().expect("temp dir");
    let path = dir.path().join("missing.txt");

    let error = File::open(&path).expect_err("open should fail");
    assert_eq!(error.kind(), IoErrorKind::NotFound);
    let recorded_path = error.path().expect("path metadata");
    assert_eq!(recorded_path, &path);
}
