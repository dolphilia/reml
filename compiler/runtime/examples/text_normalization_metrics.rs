use humantime::format_rfc3339;
use reml_runtime::text::{NormalizationForm, String as TextString};
use serde::Serialize;
use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::time::{Instant, SystemTime};

const NORMALIZATION_DATASET: &str = "tests/data/unicode/UAX15/NormalizationTest-15.1.0.txt";

fn main() -> Result<(), Box<dyn Error>> {
    let mut output_path: Option<PathBuf> = None;
    let mut data_path = PathBuf::from(NORMALIZATION_DATASET);
    let mut min_mb_s = 2.0_f64;
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--output" => {
                let value = args.next().ok_or("--output requires a file path")?;
                output_path = Some(PathBuf::from(value));
            }
            "--data" => {
                let value = args.next().ok_or("--data requires a file path")?;
                data_path = PathBuf::from(value);
            }
            "--min-mb-s" => {
                let value = args.next().ok_or("--min-mb-s requires a float value")?;
                min_mb_s = value.parse::<f64>()?;
            }
            "--help" => {
                eprintln!("Usage: text_normalization_metrics [--data <file>] [--output <file>] [--min-mb-s <float>]");
                return Ok(());
            }
            other => {
                return Err(format!("unknown argument: {other}").into());
            }
        }
    }

    let contents = fs::read_to_string(&data_path)?;
    let rows = parse_rows(&contents)?;
    if rows.is_empty() {
        return Err("NormalizationTest data is empty".into());
    }
    let unicode_version =
        detect_unicode_version(&contents).unwrap_or_else(|| "unknown".to_string());
    let dataset_name = data_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(NORMALIZATION_DATASET);

    let mut cases: Vec<NormalizationMetricsCase> = Vec::new();
    let mut total_bytes: u64 = 0;
    let mut total_iterations: u64 = 0;
    let mut throughput_sum = 0.0;
    let mut throughput_count = 0_u64;

    for (label, form) in [
        ("NFC", NormalizationForm::Nfc),
        ("NFD", NormalizationForm::Nfd),
        ("NFKC", NormalizationForm::Nfkc),
        ("NFKD", NormalizationForm::Nfkd),
    ] {
        let mut bytes_processed: u64 = 0;
        let mut iterations: u64 = 0;
        let start = Instant::now();
        for row in &rows {
            for column in row.columns() {
                bytes_processed += column.len() as u64;
                iterations += 1;
                let _ = TextString::from_str(column)
                    .normalize(form)
                    .expect("normalize");
            }
        }
        let duration = start.elapsed();
        let duration_ms = duration.as_secs_f64() * 1000.0;
        let throughput = if duration_ms > 0.0 {
            (bytes_processed as f64 / 1_048_576_f64) / (duration_ms / 1000.0)
        } else {
            0.0
        };
        throughput_sum += throughput;
        throughput_count += 1;
        total_bytes += bytes_processed;
        total_iterations += iterations;

        cases.push(NormalizationMetricsCase {
            case: label.to_string(),
            form: label.to_string(),
            bytes: bytes_processed,
            duration_ms,
            throughput_mb_s: throughput,
            iterations,
            expectations: NormalizationExpectations { min_mb_s },
        });
    }

    let avg_mb_s = if throughput_count > 0 {
        throughput_sum / throughput_count as f64
    } else {
        0.0
    };

    let report = NormalizationMetricsReport {
        timestamp: format_rfc3339(SystemTime::now()).to_string(),
        unicode_version,
        dataset: dataset_name.to_string(),
        rows: rows.len(),
        cases,
        summary: NormalizationSummary {
            avg_mb_s,
            total_bytes,
            iterations: total_iterations,
            forms: throughput_count,
        },
    };

    let json = serde_json::to_string_pretty(&report)?;
    if let Some(path) = output_path {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, format!("{json}\n"))?;
    } else {
        println!("{json}");
    }
    Ok(())
}

#[derive(Clone)]
struct NormalizationRow {
    c1: String,
    c2: String,
    c3: String,
    c4: String,
    c5: String,
}

impl NormalizationRow {
    fn columns(&self) -> [&str; 5] {
        [&self.c1, &self.c2, &self.c3, &self.c4, &self.c5]
    }
}

#[derive(Serialize)]
struct NormalizationMetricsReport {
    timestamp: String,
    unicode_version: String,
    dataset: String,
    rows: usize,
    cases: Vec<NormalizationMetricsCase>,
    summary: NormalizationSummary,
}

#[derive(Serialize)]
struct NormalizationMetricsCase {
    case: String,
    form: String,
    bytes: u64,
    duration_ms: f64,
    throughput_mb_s: f64,
    iterations: u64,
    expectations: NormalizationExpectations,
}

#[derive(Serialize)]
struct NormalizationExpectations {
    min_mb_s: f64,
}

#[derive(Serialize)]
struct NormalizationSummary {
    avg_mb_s: f64,
    total_bytes: u64,
    iterations: u64,
    forms: u64,
}

fn parse_rows(contents: &str) -> Result<Vec<NormalizationRow>, Box<dyn Error>> {
    let mut rows = Vec::new();
    for (idx, line) in contents.lines().enumerate() {
        let trimmed = line.split('#').next().unwrap_or("").trim();
        if trimmed.is_empty() || trimmed.starts_with('@') {
            continue;
        }
        let row = parse_row(trimmed).map_err(|err| format!("line {}: {err}", idx + 1))?;
        rows.push(row);
    }
    Ok(rows)
}

fn parse_row(line: &str) -> Result<NormalizationRow, String> {
    let mut sequences: Vec<String> = Vec::with_capacity(5);
    for column in line.split(';') {
        let trimmed = column.trim();
        if trimmed.is_empty() {
            continue;
        }
        sequences.push(parse_sequence(trimmed)?);
    }
    if sequences.len() != 5 {
        return Err(format!("expected 5 columns, found {}", sequences.len()));
    }
    let mut iter = sequences.into_iter();
    Ok(NormalizationRow {
        c1: iter.next().unwrap(),
        c2: iter.next().unwrap(),
        c3: iter.next().unwrap(),
        c4: iter.next().unwrap(),
        c5: iter.next().unwrap(),
    })
}

fn parse_sequence(column: &str) -> Result<String, String> {
    let mut result = String::new();
    for value in column.split_whitespace() {
        if value.is_empty() {
            continue;
        }
        let scalar = u32::from_str_radix(value, 16)
            .map_err(|err| format!("invalid scalar {value:?}: {err}"))?;
        let ch = char::from_u32(scalar).ok_or_else(|| format!("invalid code point {value:?}"))?;
        result.push(ch);
    }
    Ok(result)
}

fn detect_unicode_version(contents: &str) -> Option<String> {
    for line in contents.lines() {
        let trimmed = line.trim();
        if let Some(stripped) = trimmed.strip_prefix("# NormalizationTest-") {
            let version = stripped.trim_end_matches(".txt").trim().to_string();
            if !version.is_empty() {
                return Some(version);
            }
        }
    }
    None
}
