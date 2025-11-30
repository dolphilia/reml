use std::io::{Read, Write};

use tempfile::tempdir;

use reml_runtime::io::{File, FileOptions, IoErrorKind};

#[cfg(target_family = "unix")]
const METADATA_GOLDEN: &str = include_str!("golden/core_io/file_ops/metadata_basic_unix.json");
#[cfg(target_family = "windows")]
const METADATA_GOLDEN: &str = include_str!("golden/core_io/file_ops/metadata_basic_windows.json");

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

    let permissions = metadata.permissions();
    let actual = serde_json::json!({
        "size": metadata.size(),
        "is_dir": metadata.is_dir(),
        "readonly": metadata.is_readonly(),
        "permissions": {
            "has_unix_mode": permissions.unix_mode_value().is_some(),
            "has_windows_attributes": permissions.windows_attributes_value().is_some(),
        },
        "timestamps": {
            "modified_at_present": metadata.modified_at().is_some(),
        }
    });
    let expected: serde_json::Value =
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

#[cfg(target_family = "unix")]
#[test]
fn file_options_permissions_can_be_captured_unix() {
    use reml_runtime::io::FilePermissions;

    let options = FileOptions::new().permissions(FilePermissions::unix_mode(0o640));
    let snapshot = options
        .permissions_snapshot()
        .expect("permissions should be set");
    assert_eq!(snapshot.unix_mode_value(), Some(0o640));
    assert!(snapshot.windows_attributes_value().is_none());
}

#[cfg(target_family = "windows")]
#[test]
fn file_options_permissions_can_be_captured_windows() {
    use reml_runtime::io::FilePermissions;

    let options = FileOptions::new().permissions(FilePermissions::windows_attributes(0x20));
    let snapshot = options
        .permissions_snapshot()
        .expect("permissions should be set");
    assert_eq!(snapshot.windows_attributes_value(), Some(0x20));
    assert!(snapshot.unix_mode_value().is_none());
}
