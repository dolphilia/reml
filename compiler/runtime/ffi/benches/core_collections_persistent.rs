#[cfg(not(feature = "core_prelude"))]
fn main() {
    eprintln!("core_collections_persistent bench requires --features core_prelude");
}

#[cfg(feature = "core_prelude")]
fn main() {
    use reml_runtime_ffi::core_collections_metrics::collect_persistent_metrics;
    use std::time::Instant;

    let started = Instant::now();
    let metrics = collect_persistent_metrics();
    let elapsed = started.elapsed();

    println!("collected {} scenarios in {:.2?}", metrics.len(), elapsed);
}
