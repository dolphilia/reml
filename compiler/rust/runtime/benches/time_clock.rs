#![cfg_attr(not(feature = "core-time"), allow(dead_code))]

#[cfg(not(feature = "core-time"))]
compile_error!("time_clock benchmark requires `--features core-time` when invoking cargo bench");

#[cfg(feature = "core-time")]
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
#[cfg(feature = "core-time")]
use reml_runtime::time::{duration_between, monotonic_now, now, Duration, Timestamp};

#[cfg(feature = "core-time")]
fn bench_now(c: &mut Criterion) {
    c.bench_function("time_now_latency", |b| {
        b.iter(|| {
            let ts = now().expect("now() should succeed");
            black_box(ts);
        })
    });
}

#[cfg(feature = "core-time")]
fn bench_monotonic_now(c: &mut Criterion) {
    c.bench_function("time_monotonic_now_latency", |b| {
        b.iter(|| {
            let ts = monotonic_now().expect("monotonic_now() should succeed");
            black_box(ts);
        })
    });
}

#[cfg(feature = "core-time")]
fn bench_duration_between(c: &mut Criterion) {
    let mut group = c.benchmark_group("time_duration_between");
    let reference = Timestamp::try_from_parts(1_700_000_000, 0).expect("valid reference ts");
    let offsets = [
        Duration::from_millis(1),
        Duration::from_millis(5),
        Duration::from_millis(10),
    ];
    for offset in offsets {
        group.bench_with_input(
            BenchmarkId::new("duration_between", offset.total_nanoseconds()),
            &offset,
            |b, delta| {
                b.iter(|| {
                    let end = reference.add_duration(*delta);
                    let duration = duration_between(reference, end);
                    black_box(duration);
                })
            },
        );
    }
    group.finish();
}

#[cfg(feature = "core-time")]
criterion_group!(
    time_clock,
    bench_now,
    bench_monotonic_now,
    bench_duration_between
);
#[cfg(feature = "core-time")]
criterion_main!(time_clock);
