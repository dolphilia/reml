use reml_runtime::text::{
    decode_stream, grapheme_stats, take_text_effects_snapshot, BomHandling,
    InvalidSequenceStrategy, Str, TextDecodeOptions,
};
use serde::Serialize;
use std::env;
use std::error::Error;
use std::fs::{self, File};
use std::io::BufReader;
use std::path::PathBuf;
use unicode_segmentation::UnicodeSegmentation;

fn main() -> Result<(), Box<dyn Error>> {
    let mut input: Option<PathBuf> = None;
    let mut bom = BomHandling::Auto;
    let mut invalid = InvalidSequenceStrategy::Error;
    let mut output: Option<PathBuf> = None;
    let mut chunk_size: Option<usize> = None;

    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--input" => {
                let value = args.next().ok_or("--input requires a path")?;
                input = Some(PathBuf::from(value));
            }
            "--bom" => {
                let value = args.next().ok_or("--bom requires a value")?;
                bom = parse_bom(&value)?;
            }
            "--invalid" => {
                let value = args.next().ok_or("--invalid requires a value")?;
                invalid = parse_invalid(&value)?;
            }
            "--replace" => {
                invalid = InvalidSequenceStrategy::Replace;
            }
            "--output" => {
                let value = args.next().ok_or("--output requires a path")?;
                output = Some(PathBuf::from(value));
            }
            "--chunk-size" => {
                let value = args.next().ok_or("--chunk-size requires a value")?;
                let size = value
                    .parse::<usize>()
                    .map_err(|_| format!("chunk-size must be an integer: {value}"))?;
                chunk_size = Some(size);
            }
            "--help" => {
                print_help();
                return Ok(());
            }
            other => {
                return Err(format!("unknown argument: {other}").into());
            }
        }
    }

    let input_path = input.ok_or("--input is required")?;
    let file = File::open(&input_path)?;
    let file_size = file.metadata().ok().map(|meta| meta.len());
    let mut reader = BufReader::new(file);

    let mut options = TextDecodeOptions::default()
        .with_bom_handling(bom)
        .with_invalid_sequence(invalid);
    if let Some(size) = chunk_size {
        options = options.with_buffer_size(size);
    }
    take_text_effects_snapshot();
    let text = decode_stream(&mut reader, options)
        .map_err(|err| format!("decode_stream failed: {}", err.message()))?;
    let str_ref = Str::from(text.as_str());
    let stats = grapheme_stats(&str_ref)
        .map_err(|err| format!("grapheme_stats failed: {}", err.message()))?;
    let effects = EffectSnapshot::from(take_text_effects_snapshot());

    let report = DecodeReport {
        input: input_path.display().to_string(),
        bytes: file_size,
        chars: text.as_str().chars().count(),
        graphemes: stats.grapheme_count,
        avg_width: stats.avg_width,
        bom_policy: bom_label(bom),
        invalid_policy: invalid_label(invalid),
        preview: preview(text.as_str()),
        effects,
    };

    let payload = serde_json::to_string_pretty(&report)?;
    if let Some(path) = output {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, format!("{payload}\n"))?;
    }
    println!("{payload}");
    Ok(())
}

fn parse_bom(value: &str) -> Result<BomHandling, Box<dyn Error>> {
    match value {
        "auto" => Ok(BomHandling::Auto),
        "require" => Ok(BomHandling::Require),
        "ignore" => Ok(BomHandling::Ignore),
        other => Err(format!("unknown BOM policy: {other}").into()),
    }
}

fn parse_invalid(value: &str) -> Result<InvalidSequenceStrategy, Box<dyn Error>> {
    match value {
        "error" => Ok(InvalidSequenceStrategy::Error),
        "replace" => Ok(InvalidSequenceStrategy::Replace),
        other => Err(format!("unknown invalid strategy: {other}").into()),
    }
}

fn bom_label(value: BomHandling) -> &'static str {
    match value {
        BomHandling::Auto => "auto",
        BomHandling::Require => "require",
        BomHandling::Ignore => "ignore",
    }
}

fn invalid_label(value: InvalidSequenceStrategy) -> &'static str {
    match value {
        InvalidSequenceStrategy::Error => "error",
        InvalidSequenceStrategy::Replace => "replace",
    }
}

fn preview(source: &str) -> String {
    const MAX_GRAPHEMES: usize = 32;
    let mut preview = String::new();
    for (index, grapheme) in source.graphemes(true).enumerate() {
        if index >= MAX_GRAPHEMES {
            preview.push('…');
            break;
        }
        preview.push_str(grapheme);
    }
    preview
}

fn print_help() {
    eprintln!(
        "Usage: cargo run --bin text_stream_decode -- --input <file> [--bom auto|require|ignore] [--invalid error|replace|--replace] [--chunk-size <bytes>] [--output <file>]"
    );
    eprintln!("Reads a UTF-8 stream via Core.Text decode_stream and prints grapheme/effect metrics as JSON.");
}

#[derive(Serialize)]
struct DecodeReport {
    input: String,
    bytes: Option<u64>,
    chars: usize,
    graphemes: usize,
    avg_width: f64,
    bom_policy: &'static str,
    invalid_policy: &'static str,
    preview: String,
    effects: EffectSnapshot,
}

#[derive(Serialize)]
struct EffectSnapshot {
    mem: bool,
    mutating: bool,
    debug: bool,
    async_pending: bool,
    audit: bool,
    cell: bool,
    rc: bool,
    unicode: bool,
    io: bool,
    transfer: bool,
    mem_bytes: usize,
    predicate_calls: usize,
    rc_ops: usize,
}

impl From<reml_runtime::prelude::iter::EffectLabels> for EffectSnapshot {
    fn from(labels: reml_runtime::prelude::iter::EffectLabels) -> Self {
        Self {
            mem: labels.mem,
            mutating: labels.mutating,
            debug: labels.debug,
            async_pending: labels.async_pending,
            audit: labels.audit,
            cell: labels.cell,
            rc: labels.rc,
            unicode: labels.unicode,
            io: labels.io,
            transfer: labels.transfer,
            mem_bytes: labels.mem_bytes,
            predicate_calls: labels.predicate_calls,
            rc_ops: labels.rc_ops,
        }
    }
}
