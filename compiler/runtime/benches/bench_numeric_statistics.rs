#![cfg_attr(not(feature = "core_numeric"), allow(dead_code))]

#[cfg(not(feature = "core_numeric"))]
compile_error!(
    "bench_numeric_statistics requires `--features core-numeric` when invoking `cargo bench`"
);

#[cfg(feature = "core_numeric")]
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
#[cfg(feature = "core_numeric")]
use reml_runtime::numeric::{mean, percentile, variance};
#[cfg(feature = "core_numeric")]
use reml_runtime::prelude::iter::Iter;

#[cfg(feature = "core_numeric")]
const LARGE_SAMPLE_SIZES: &[usize] = &[1_000, 50_000];

#[cfg(feature = "core_numeric")]
fn iter_from_vec(values: Vec<f64>) -> Iter<f64> {
    Iter::from_list(values)
}

#[cfg(feature = "core_numeric")]
fn generate_large_drift_samples(len: usize) -> Vec<f64> {
    let mut values = Vec::with_capacity(len);
    for i in 0..len {
        let oscillation = ((i as f64).sin() * 1e-3) + ((i % 7) as f64) * 1e-6;
        if i % 17 == 0 {
            values.push(1e12 + oscillation);
        } else {
            values.push(oscillation);
        }
    }
    values
}

#[cfg(feature = "core_numeric")]
fn generate_random_walk(len: usize) -> Vec<f64> {
    let mut acc = 0.0;
    let mut values = Vec::with_capacity(len);
    for i in 0..len {
        let delta = (((i * 13) % 23) as f64 - 11.0) * 1e-2;
        acc += delta;
        values.push(acc);
    }
    values
}

#[cfg(feature = "core_numeric")]
fn generate_heavy_tail(len: usize) -> Vec<f64> {
    let mut values = Vec::with_capacity(len);
    for i in 0..len {
        let base = ((i * 31 + 7) % 997 + 1) as f64;
        values.push((1.0 / base.powf(1.3)) * 1e4);
    }
    values
}

#[cfg(feature = "core_numeric")]
fn bench_mean(c: &mut Criterion) {
    let mut group = c.benchmark_group("numeric_mean");
    for &len in LARGE_SAMPLE_SIZES {
        let data = generate_large_drift_samples(len);
        group.bench_with_input(
            BenchmarkId::new("mean_large_drift", len),
            &data,
            |b, samples| {
                b.iter(|| {
                    let iter = iter_from_vec(samples.clone());
                    let value = mean(iter);
                    black_box(value.expect("non-empty dataset"));
                });
            },
        );
    }
    group.finish();
}

#[cfg(feature = "core_numeric")]
fn bench_variance(c: &mut Criterion) {
    let mut group = c.benchmark_group("numeric_variance");
    for &len in LARGE_SAMPLE_SIZES {
        let data = generate_random_walk(len);
        group.bench_with_input(
            BenchmarkId::new("variance_random_walk", len),
            &data,
            |b, samples| {
                b.iter(|| {
                    let iter = iter_from_vec(samples.clone());
                    let value = variance(iter);
                    black_box(value.expect("random walk must produce variance"));
                });
            },
        );
    }
    group.finish();
}

#[cfg(feature = "core_numeric")]
fn bench_percentile(c: &mut Criterion) {
    let mut group = c.benchmark_group("numeric_percentile");
    for &len in LARGE_SAMPLE_SIZES {
        let data = generate_heavy_tail(len);
        group.bench_with_input(
            BenchmarkId::new("percentile_heavy_tail", len),
            &data,
            |b, samples| {
                b.iter(|| {
                    let median =
                        percentile(iter_from_vec(samples.clone()), 0.5).expect("median exists");
                    let p95 = percentile(iter_from_vec(samples.clone()), 0.95).expect("p95 exists");
                    black_box((median, p95));
                });
            },
        );
    }
    group.finish();
}

#[cfg(feature = "core_numeric")]
criterion_group!(numeric_stats, bench_mean, bench_variance, bench_percentile);
#[cfg(feature = "core_numeric")]
criterion_main!(numeric_stats);
