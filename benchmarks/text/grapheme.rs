use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use reml_runtime::text::{
    clear_grapheme_cache_for_tests, log_grapheme_stats, segment_graphemes, Str,
};

const SAMPLE_TEXT: &str = include_str!("data/multilingual.txt");

fn bench_grapheme(c: &mut Criterion) {
    let total_bytes = SAMPLE_TEXT.as_bytes().len();
    let mut group = c.benchmark_group("text::grapheme");
    group.throughput(Throughput::Bytes(total_bytes as u64));

    group.bench_function("segment_cold", |b| {
        b.iter(|| {
            clear_grapheme_cache_for_tests();
            let text = Str::from(SAMPLE_TEXT);
            let seq = segment_graphemes(&text).expect("segment");
            criterion::black_box(seq.len());
        });
    });

    group.bench_function("segment_cached", |b| {
        clear_grapheme_cache_for_tests();
        {
            let text = Str::from(SAMPLE_TEXT);
            let _ = segment_graphemes(&text).expect("warm cache");
        }
        b.iter(|| {
            let text = Str::from(SAMPLE_TEXT);
            let seq = segment_graphemes(&text).expect("segment");
            criterion::black_box(seq.len());
        });
    });

    group.bench_function("log_stats", |b| {
        b.iter(|| {
            let text = Str::from(SAMPLE_TEXT);
            let stats = log_grapheme_stats(&text).expect("stats");
            criterion::black_box((stats.grapheme_count, stats.cache_hits));
        });
    });

    group.finish();
}

criterion_group!(benches, bench_grapheme);
criterion_main!(benches);
