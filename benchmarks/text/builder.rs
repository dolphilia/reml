use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use reml_runtime::text::{Bytes, Str, TextBuilder};

const SAMPLE_TEXT: &str = include_str!("data/multilingual.txt");

fn split_words() -> Vec<&'static str> {
    SAMPLE_TEXT.split_whitespace().collect()
}

fn split_graphemes() -> Vec<String> {
    let reference = Str::from(SAMPLE_TEXT);
    reference
        .iter_graphemes()
        .map(|grapheme| grapheme.to_string())
        .collect()
}

fn prebuilt_bytes(words: &[&'static str]) -> Vec<Bytes> {
    words
        .iter()
        .map(|segment| Bytes::from_slice(segment.as_bytes()))
        .collect()
}

fn bench_builder(c: &mut Criterion) {
    let words = split_words();
    let graphemes = split_graphemes();
    let bytes = prebuilt_bytes(&words);
    let total_bytes = SAMPLE_TEXT.as_bytes().len();
    let mut group = c.benchmark_group("text::builder");
    group.throughput(Throughput::Bytes(total_bytes as u64));

    group.bench_function("push_str_finish", |b| {
        b.iter(|| {
            let _ = reml_runtime::text::take_text_effects_snapshot();
            let mut builder = TextBuilder::with_capacity(total_bytes);
            for word in &words {
                let str_ref = Str::from(*word);
                builder.push_str(&str_ref);
                builder.push_grapheme(" ");
            }
            let text = builder.finish().expect("finish");
            let effects = reml_runtime::text::take_text_effects_snapshot();
            criterion::black_box((text, effects));
        });
    });

    group.bench_function("push_bytes_finish", |b| {
        b.iter(|| {
            let _ = reml_runtime::text::take_text_effects_snapshot();
            let mut builder = TextBuilder::with_capacity(total_bytes);
            for chunk in &bytes {
                builder.push_bytes(chunk);
            }
            builder.push_grapheme("\n");
            let text = builder.finish().expect("finish");
            let effects = reml_runtime::text::take_text_effects_snapshot();
            criterion::black_box((text, effects));
        });
    });

    group.bench_function("push_grapheme_finish", |b| {
        b.iter(|| {
            let _ = reml_runtime::text::take_text_effects_snapshot();
            let mut builder = TextBuilder::with_capacity(total_bytes);
            for cluster in &graphemes {
                builder.push_grapheme(cluster);
            }
            let text = builder.finish().expect("finish");
            let effects = reml_runtime::text::take_text_effects_snapshot();
            criterion::black_box((text, effects));
        });
    });

    group.finish();
}

criterion_group!(benches, bench_builder);
criterion_main!(benches);
