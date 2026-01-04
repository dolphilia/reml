use std::{io::Write, path::Path};

use reml_runtime::io::{
    leak_tracker_snapshot, reset_leak_tracker, with_file, with_temp_dir, FileOptions, IoContext,
    IoError, ScopedFileMode,
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct LeakDetectionCase {
    expected: LeakExpectations,
}

#[derive(Debug, Deserialize)]
struct LeakExpectations {
    open_files: usize,
    temp_dirs: usize,
}

#[test]
fn scoped_resources_cleanup_matches_expected_snapshot() {
    reset_leak_tracker();
    let case: LeakDetectionCase = serde_json::from_str(include_str!(
        "data/core_io/leak_detection/scoped_cleanup.json"
    ))
    .expect("leak detection expectations json");

    let temp_dir_path = with_temp_dir("leak-check", |guard| {
        let file_path = guard.path().join("scoped.txt");
        let options = FileOptions::new().write(true).truncate(true).create(true);
        with_file(&file_path, ScopedFileMode::create(options), |file| {
            file.write_all(b"scoped cleanup")
                .map_err(|err| map_std_io_error(&file_path, err))?;
            file.sync_all()
        })?;
        Ok(file_path)
    })
    .expect("scoped temp dir should succeed");

    assert!(
        !temp_dir_path.exists(),
        "temporary directory should be removed after scope ends"
    );

    let snapshot = leak_tracker_snapshot();
    assert_eq!(snapshot.open_files, case.expected.open_files);
    assert_eq!(snapshot.temp_dirs, case.expected.temp_dirs);
}

fn map_std_io_error(path: &Path, err: std::io::Error) -> IoError {
    IoError::from_std(
        err,
        IoContext::new("tests.io.leak.write")
            .with_path(path.to_path_buf())
            .with_capability("io.fs.write"),
    )
}
