use criterion::{criterion_group, criterion_main, BatchSize, Criterion, Throughput};
use reml_runtime::text::{normalize, NormalizationForm, String as TextString};

const SAMPLE_TEXT: &str = include_str!("data/multilingual.txt");

fn sample_input() -> TextString {
    TextString::from_str(SAMPLE_TEXT)
}

fn bench_normalization(c: &mut Criterion) {
    let base = sample_input();
    let total_bytes = base.as_str().as_bytes().len();
    let mut group = c.benchmark_group("text::normalization");
    group.throughput(Throughput::Bytes(total_bytes as u64));

    for (label, form) in [
        ("nfc", NormalizationForm::Nfc),
        ("nfd", NormalizationForm::Nfd),
        ("nfkc", NormalizationForm::Nfkc),
        ("nfkd", NormalizationForm::Nfkd),
    ] {
        let template = base.clone();
        group.bench_function(label, move |b| {
            b.iter_batched(
                || template.clone(),
                |input| {
                    let normalized = normalize(input, form).expect("normalize");
                    criterion::black_box(normalized);
                },
                BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

criterion_group!(benches, bench_normalization);
criterion_main!(benches);
