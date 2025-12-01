#![cfg_attr(not(all(feature = "core_io", feature = "core_path")), allow(dead_code))]

#[cfg(not(all(feature = "core_io", feature = "core_path")))]
compile_error!(
    "bench_core_io requires `--features \"core-io core-path\"` when invoking `cargo bench`"
);

#[cfg(all(feature = "core_io", feature = "core_path"))]
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
#[cfg(all(feature = "core_io", feature = "core_path"))]
use reml_runtime::io::{buffered, copy, read_line, WatchEvent};
#[cfg(all(feature = "core_io", feature = "core_path"))]
use reml_runtime::path::{normalize_path_str, PathStyle};
#[cfg(all(feature = "core_io", feature = "core_path"))]
use reml_runtime::text::Str;
#[cfg(all(feature = "core_io", feature = "core_path"))]
use std::io::Cursor;
#[cfg(all(feature = "core_io", feature = "core_path"))]
use std::path::PathBuf;

#[cfg(all(feature = "core_io", feature = "core_path"))]
const COPY_SIZES: &[usize] = &[64 * 1024, 2 * 1024 * 1024];
#[cfg(all(feature = "core_io", feature = "core_path"))]
const LINE_DATASETS: &[(usize, usize)] = &[(256, 64), (2048, 80)];
#[cfg(all(feature = "core_io", feature = "core_path"))]
const PATH_SAMPLES: &[&str] = &[
    "./logs/../logs/api/./../app.log",
    "/var//tmp///../tmp/cache/./segments",
    r"C:\\Projects\\kestrel\\..\\kestrel\\logs\\..\\config\\settings.toml",
    r"..\\temp\\..\\temp\\artifacts\\release",
    r"\\networkshare\\bucket\\..\\bucket\\reports",
    "../../sandbox/./../sandbox/data/input",
];
#[cfg(all(feature = "core_io", feature = "core_path"))]
const WATCH_EVENT_BATCH: usize = 512;

#[cfg(all(feature = "core_io", feature = "core_path"))]
fn core_io_benchmarks(c: &mut Criterion) {
    bench_reader_copy(c);
    bench_buffered_read_line(c);
    bench_path_normalize(c);
    bench_watcher_throughput(c);
}

#[cfg(all(feature = "core_io", feature = "core_path"))]
fn bench_reader_copy(c: &mut Criterion) {
    let mut group = c.benchmark_group("core_io_reader_copy");
    for &size in COPY_SIZES {
        let payload = vec![0x55_u8; size];
        group.bench_with_input(BenchmarkId::new("copy_bytes", size), &payload, |b, data| {
            b.iter(|| {
                let mut reader = Cursor::new(data.clone());
                let mut writer = Cursor::new(vec![0_u8; data.len()]);
                let transferred = copy(&mut reader, &mut writer).expect("copy succeeds");
                black_box(transferred)
            });
        });
    }
    group.finish();
}

#[cfg(all(feature = "core_io", feature = "core_path"))]
fn bench_buffered_read_line(c: &mut Criterion) {
    let mut group = c.benchmark_group("core_io_buffered_read_line");
    for &(lines, width) in LINE_DATASETS {
        let dataset = generate_line_dataset(lines, width);
        group.bench_with_input(BenchmarkId::new("lines", lines), &dataset, |b, data| {
            b.iter(|| {
                let cursor = Cursor::new(data.clone());
                let mut reader = buffered(cursor, 32 * 1024).expect("buffered reader");
                let mut count = 0;
                while let Some(line) = read_line(&mut reader).expect("read_line ok") {
                    black_box(line.len_bytes());
                    count += 1;
                }
                black_box(count)
            });
        });
    }
    group.finish();
}

#[cfg(all(feature = "core_io", feature = "core_path"))]
fn bench_path_normalize(c: &mut Criterion) {
    let samples: Vec<String> = PATH_SAMPLES
        .iter()
        .map(|sample| sample.to_string())
        .collect();
    let mut group = c.benchmark_group("core_path_normalize");
    group.bench_function("normalize_native", |b| {
        b.iter(|| normalize_paths(PathStyle::Native, &samples));
    });
    group.bench_function("normalize_posix", |b| {
        b.iter(|| normalize_paths(PathStyle::Posix, &samples));
    });
    group.bench_function("normalize_windows", |b| {
        b.iter(|| normalize_paths(PathStyle::Windows, &samples));
    });
    group.finish();
}

#[cfg(all(feature = "core_io", feature = "core_path"))]
fn bench_watcher_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("core_io_watcher_throughput");
    let events = build_watch_events(WATCH_EVENT_BATCH);
    group.bench_function("watch_event_batch", |b| {
        b.iter(|| {
            let mut delivered = 0;
            for event in events.iter() {
                match event {
                    WatchEvent::Created(path)
                    | WatchEvent::Modified(path)
                    | WatchEvent::Deleted(path) => {
                        black_box(path);
                        delivered += 1;
                    }
                }
            }
            black_box(delivered)
        });
    });
    group.finish();
}

#[cfg(all(feature = "core_io", feature = "core_path"))]
fn generate_line_dataset(lines: usize, width: usize) -> Vec<u8> {
    let mut buffer = Vec::with_capacity(lines * (width + 1));
    for idx in 0..lines {
        let mut line = format!("entry-{idx:04}-");
        while line.len() < width {
            let ch = b'a' + (idx as u8 % 26);
            line.push(ch as char);
        }
        line.truncate(width);
        buffer.extend_from_slice(line.as_bytes());
        buffer.push(b'\n');
    }
    buffer
}

#[cfg(all(feature = "core_io", feature = "core_path"))]
fn normalize_paths(style: PathStyle, samples: &[String]) {
    for sample in samples {
        let normalized =
            normalize_path_str(Str::from(sample.as_str()), style).expect("normalize path");
        black_box(normalized.len_bytes());
    }
}

#[cfg(all(feature = "core_io", feature = "core_path"))]
fn build_watch_events(count: usize) -> Vec<WatchEvent> {
    let mut events = Vec::with_capacity(count);
    for idx in 0..count {
        let path = PathBuf::from(format!("watch/bench/event-{idx}.log"));
        let event = match idx % 3 {
            0 => WatchEvent::Created(path),
            1 => WatchEvent::Modified(path),
            _ => WatchEvent::Deleted(path),
        };
        events.push(event);
    }
    events
}

#[cfg(all(feature = "core_io", feature = "core_path"))]
criterion_group!(core_io, core_io_benchmarks);
#[cfg(all(feature = "core_io", feature = "core_path"))]
criterion_main!(core_io);
