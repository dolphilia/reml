use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use reml_runtime::io::{
    close_watcher, watch, watch_with_limits, IoErrorKind, WatchEvent, WatchLimits,
    WatcherAuditSnapshot,
};
use serde_json::json;
use tempfile::tempdir;

#[test]
fn watch_reports_create_and_delete_events() {
    let prev_backend = std::env::var("REML_WATCHER_BACKEND").ok();
    std::env::set_var("REML_WATCHER_BACKEND", "poll");
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
    let handle = watcher.handle();
    // Give the backend watcher time to start before we trigger filesystem events.
    thread::sleep(Duration::from_millis(200));

    fs::copy(simple_case_fixture("initial.txt"), &file_path).expect("copy fixture file");
    thread::sleep(Duration::from_millis(100));
    fs::write(&file_path, "updated text").expect("update sample file");
    thread::sleep(Duration::from_millis(50));
    fs::remove_file(&file_path).expect("remove sample file");

    let mut collected: Vec<WatchEvent> = Vec::new();
    let mut seen_created = false;
    let mut seen_deleted = false;
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(3) && (!seen_created || !seen_deleted) {
        if let Ok(event) = rx.recv_timeout(Duration::from_millis(200)) {
            if matches!(
                event,
                WatchEvent::Created(ref path)
                    if path.file_name().map_or(false, |name| name == "sample.txt")
            ) {
                seen_created = true;
            }
            if matches!(
                event,
                WatchEvent::Deleted(ref path)
                    if path.file_name().map_or(false, |name| name == "sample.txt")
            ) {
                seen_deleted = true;
            }
            collected.push(event);
        }
    }

    assert!(seen_created, "expected created event, got {collected:?}");
    assert!(seen_deleted, "expected deleted event, got {collected:?}");

    close_watcher(watcher).expect("watcher should close");
    let snapshot = handle.audit_snapshot();
    assert!(
        snapshot.total_events >= 2,
        "audit snapshot should contain at least 2 events"
    );
    persist_watcher_audit_report("simple_case", &snapshot);
    if let Some(prev_backend) = prev_backend {
        std::env::set_var("REML_WATCHER_BACKEND", prev_backend);
    } else {
        std::env::remove_var("REML_WATCHER_BACKEND");
    }
}

#[test]
fn watch_with_limits_rejects_invalid_path() {
    let invalid_path = PathBuf::from("/non-existent/watch/path");
    let limits = WatchLimits::default();

    let error = watch_with_limits(vec![invalid_path.clone()], limits, |_| {})
        .expect_err("watch should fail");
    assert_eq!(error.kind(), IoErrorKind::InvalidInput);
}

fn simple_case_fixture(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/watcher/simple_case")
        .join(relative)
}

fn persist_watcher_audit_report(case: &str, snapshot: &WatcherAuditSnapshot) {
    if snapshot.is_empty() {
        return;
    }
    let metadata = snapshot.clone().into_metadata();
    let entry = json!({
        "case": case,
        "metadata": metadata,
    });
    let output_path = repo_root().join("reports/spec-audit/ch3/io_watcher-simple_case.jsonl");
    if let Some(parent) = output_path.parent() {
        if let Err(err) = fs::create_dir_all(parent) {
            eprintln!("failed to create audit directory: {err}");
            return;
        }
    }
    if let Err(err) = fs::write(&output_path, format!("{}\n", entry)) {
        eprintln!("failed to persist watcher audit report at {output_path:?}: {err}");
    }
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .map(Path::to_path_buf)
        .expect("CARGO_MANIFEST_DIR should have at least 3 ancestors")
}
