use std::fs;
use std::path::PathBuf;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use reml_runtime::io::{close_watcher, watch, watch_with_limits, IoErrorKind, WatchEvent, WatchLimits};
use tempfile::tempdir;

#[test]
fn watch_reports_create_and_delete_events() {
    let temp_dir = tempdir().expect("tempdir should create directory");
    let watch_path = temp_dir.path().to_path_buf();
    let file_path = watch_path.join("sample.txt");

    let (tx_raw, rx) = mpsc::channel();
    let tx = Arc::new(Mutex::new(tx_raw));
    let callback_tx = Arc::clone(&tx);
    let watcher = watch(vec![watch_path.clone()], move |event| {
        if let Ok(sender) = callback_tx.lock() {
            sender.send(event).ok();
        }
    })
    .expect("watch should initialize");

    fs::copy(simple_case_fixture("initial.txt"), &file_path).expect("copy fixture file");
    thread::sleep(Duration::from_millis(100));
    fs::write(&file_path, "updated text").expect("update sample file");
    thread::sleep(Duration::from_millis(50));
    fs::remove_file(&file_path).expect("remove sample file");

    let mut collected: Vec<WatchEvent> = Vec::new();
    let start = Instant::now();
    while collected.len() < 2 && start.elapsed() < Duration::from_secs(3) {
        if let Ok(event) = rx.recv_timeout(Duration::from_millis(200)) {
            collected.push(event);
        }
    }

    assert!(
        collected.iter().any(|event| {
            matches!(
                event,
                WatchEvent::Created(path) if path.file_name().map_or(false, |name| name == "sample.txt")
            )
        }),
        "expected created event, got {collected:?}"
    );
    assert!(
        collected.iter().any(|event| {
            matches!(
                event,
                WatchEvent::Deleted(path) if path.file_name().map_or(false, |name| name == "sample.txt")
            )
        }),
        "expected deleted event, got {collected:?}"
    );

    close_watcher(watcher).expect("watcher should close");
}

#[test]
fn watch_with_limits_rejects_invalid_path() {
    let invalid_path = PathBuf::from("/non-existent/watch/path");
    let limits = WatchLimits::default();

    let error = watch_with_limits(vec![invalid_path.clone()], limits, |_| {}).expect_err("watch should fail");
    assert_eq!(error.kind(), IoErrorKind::InvalidInput);
}

fn simple_case_fixture(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/watcher/simple_case")
        .join(relative)
}
