#[cfg(not(feature = "core_prelude"))]
fn main() {
    eprintln!("core_prelude feature is required: cargo run --features core_prelude --example core_collections_metrics");
}

#[cfg(feature = "core_prelude")]
fn main() -> std::io::Result<()> {
    use reml_runtime_ffi::core_collections_metrics::{
        collect_persistent_metrics, render_metrics_csv, write_metrics_csv,
    };
    use std::env;

    let metrics = collect_persistent_metrics();
    if let Some(path) = env::args().nth(1) {
        write_metrics_csv(path, &metrics)?;
    } else {
        println!("{}", render_metrics_csv(&metrics));
    }
    Ok(())
}
