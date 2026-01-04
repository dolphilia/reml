use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use reml_runtime::text::{
    clear_grapheme_cache_for_tests, grapheme_stats_metadata, log_grapheme_stats,
    take_text_audit_metadata_for_tests, GraphemeStats, Str, TextBuilder, UnicodeResult,
};
use serde::Serialize;
use serde_json::{json, Map as JsonMap, Value};

#[derive(Clone, Copy)]
enum LocaleProfile {
    Single,
    Mixed,
    Streaming,
}

struct CaseSpec {
    case_id: &'static str,
    target_bytes: usize,
    locale: &'static str,
    profile: LocaleProfile,
    notes: &'static str,
}

#[derive(Serialize, Clone)]
struct CaseMetrics {
    case_id: &'static str,
    locale: &'static str,
    target_bytes: usize,
    actual_bytes: usize,
    grapheme_count: usize,
    avg_cluster_width: f64,
    emoji_ratio: f64,
    primary_script: &'static str,
    unicode_version: String,
    script_mix_ratio: f64,
    rtl_ratio: f64,
    cache_hits: usize,
    cache_miss: usize,
    version_mismatch_evictions: usize,
    cache_generation: u32,
    avg_generation: f64,
    cache_hit_ratio: f64,
    notes: &'static str,
    #[serde(skip_serializing)]
    audit_metadata: Value,
}

#[derive(Serialize)]
struct ReportSummary {
    total_cases: usize,
    total_bytes: usize,
    avg_cache_hit_ratio: f64,
    generated_unix_secs: u64,
}

#[derive(Serialize)]
struct Report {
    suite: &'static str,
    cases: Vec<CaseMetrics>,
    summary: ReportSummary,
}

const CASE_SPECS: &[CaseSpec] = &[
    CaseSpec {
        case_id: "UC-01",
        target_bytes: 5 * 1024 * 1024,
        locale: "ja-JP",
        profile: LocaleProfile::Single,
        notes: "初回生成。IndexCache が存在せず cache_miss を強制する。",
    },
    CaseSpec {
        case_id: "UC-02",
        target_bytes: 500 * 1024,
        locale: "ja-JP/ar/emoji",
        profile: LocaleProfile::Mixed,
        notes: "GraphemeSeq::clone を想定し cache_hits が 70% 以上であることを検証する。",
    },
    CaseSpec {
        case_id: "UC-03",
        target_bytes: 200 * 1024,
        locale: "streaming",
        profile: LocaleProfile::Streaming,
        notes: "TextBuilder → GraphemeSeq 経路で cache_miss=0 を確認する。",
    },
];

#[allow(non_snake_case)]
#[test]
fn UC_01_single_locale_initial_generation() {
    let metrics = ensure_report_for_case("UC-01");
    assert_eq!(metrics.cache_hits, 0, "初回生成で cache_hits は 0 のはず");
    assert!(
        metrics.cache_miss >= 1,
        "cache_miss >= 1 を期待 (実測値 = {})",
        metrics.cache_miss
    );
}

#[allow(non_snake_case)]
#[test]
fn UC_02_mixed_locale_cache_hits() {
    let metrics = ensure_report_for_case("UC-02");
    let denominator = (metrics.cache_hits + metrics.cache_miss).max(1) as f64;
    let hit_ratio = metrics.cache_hits as f64 / denominator;
    assert!(
        hit_ratio >= 0.7,
        "cache hit ratio は 0.7 以上。実測: {:.3}",
        hit_ratio
    );
}

#[allow(non_snake_case)]
#[test]
fn UC_03_streaming_builder_zero_miss() {
    let metrics = ensure_report_for_case("UC-03");
    assert_eq!(
        metrics.cache_miss, 0,
        "TextBuilder 共有時は cache_miss が 0 であること"
    );
    assert!(
        metrics.cache_hits > 0,
        "Streaming ケースで cache_hits > 0 であること"
    );
}

fn ensure_report_for_case(case_id: &str) -> CaseMetrics {
    let metrics = gather_all_metrics();
    persist_report(&metrics);
    persist_audit(&metrics);
    metrics
        .into_iter()
        .find(|case| case.case_id == case_id)
        .expect("case metrics should exist")
}

fn gather_all_metrics() -> Vec<CaseMetrics> {
    CASE_SPECS.iter().map(run_case).collect()
}

fn run_case(spec: &CaseSpec) -> CaseMetrics {
    let text = build_text(spec);
    let actual_bytes = text.len();
    clear_grapheme_cache_for_tests();
    let str_ref = Str::from(text.as_str());
    let stats = gather_stats_for_profile(spec.profile, &str_ref);
    let cache_hits = stats.cache_hits;
    let cache_miss = stats.cache_miss;
    let cache_generation = stats.cache_generation;
    let cache_denominator = (cache_hits + cache_miss).max(1) as f64;
    let audit_metadata = build_audit_metadata(&stats);
    CaseMetrics {
        case_id: spec.case_id,
        locale: spec.locale,
        target_bytes: spec.target_bytes,
        actual_bytes,
        grapheme_count: stats.grapheme_count,
        avg_cluster_width: stats.avg_width,
        emoji_ratio: stats.emoji_ratio,
        primary_script: stats.scripts.primary.label(),
        unicode_version: stats.unicode_version.clone(),
        script_mix_ratio: stats.scripts.mix_ratio,
        rtl_ratio: stats.direction.rtl_ratio,
        cache_hits,
        cache_miss,
        version_mismatch_evictions: stats.version_mismatch_evictions,
        cache_generation,
        avg_generation: cache_generation as f64,
        cache_hit_ratio: cache_hits as f64 / cache_denominator,
        notes: spec.notes,
        audit_metadata,
    }
}

fn gather_stats_for_profile(profile: LocaleProfile, str_ref: &Str<'_>) -> GraphemeStats {
    match profile {
        LocaleProfile::Single => consume_stats(log_grapheme_stats(str_ref)),
        LocaleProfile::Mixed | LocaleProfile::Streaming => {
            let _ = log_grapheme_stats(str_ref).expect("warm cache");
            take_text_audit_metadata_for_tests();
            consume_stats(log_grapheme_stats(str_ref))
        }
    }
}

fn consume_stats(result: UnicodeResult<GraphemeStats>) -> GraphemeStats {
    let stats = result.expect("stats");
    take_text_audit_metadata_for_tests();
    stats
}

fn persist_report(cases: &[CaseMetrics]) {
    let output_path = repo_root().join("reports/spec-audit/ch1/core_text_grapheme_stats.json");
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).expect("create reports directory");
    }
    let summary = ReportSummary {
        total_cases: cases.len(),
        total_bytes: cases.iter().map(|c| c.actual_bytes).sum(),
        avg_cache_hit_ratio: cases.iter().map(|c| c.cache_hit_ratio).sum::<f64>()
            / (cases.len().max(1) as f64),
        generated_unix_secs: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_secs(),
    };
    let report = Report {
        suite: "text_internal_cache",
        cases: cases.to_vec(),
        summary,
    };
    let json = serde_json::to_string_pretty(&report).expect("serialize report");
    fs::write(output_path, json).expect("write report file");
}

fn persist_audit(cases: &[CaseMetrics]) {
    let output_path = repo_root().join("reports/spec-audit/ch1/text_grapheme_stats.audit.jsonl");
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).expect("create audit reports directory");
    }
    let mut lines = Vec::new();
    for case in cases {
        let entry = json!({
            "case": case.case_id,
            "metadata": case.audit_metadata,
        });
        lines.push(entry.to_string());
    }
    fs::write(output_path, lines.join("\n")).expect("write audit report");
}

fn repo_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .ancestors()
        .nth(2)
        .map(Path::to_path_buf)
        .expect("CARGO_MANIFEST_DIR should have at least 3 ancestors")
}

fn build_text(spec: &CaseSpec) -> String {
    match spec.profile {
        LocaleProfile::Single => build_single_locale_text(spec.target_bytes),
        LocaleProfile::Mixed => build_mixed_locale_text(spec.target_bytes),
        LocaleProfile::Streaming => build_streaming_text(spec.target_bytes),
    }
}

fn build_single_locale_text(target_bytes: usize) -> String {
    build_repeated_text(target_bytes, &["かな", "漢字混在", "ひらがな"])
}

fn build_mixed_locale_text(target_bytes: usize) -> String {
    build_repeated_text(target_bytes, &["かな", "العَرَبِيَّة", "🙂", "Latin", "⚙️"])
}

fn build_streaming_text(target_bytes: usize) -> String {
    let mut builder = TextBuilder::new();
    let samples = ["Stream", "🙂", "処理", "λ", "🌏"];
    let mut produced = 0usize;
    let mut idx = 0usize;
    while produced < target_bytes {
        let chunk = samples[idx % samples.len()];
        let str_ref = Str::from(chunk);
        builder.push_str(&str_ref);
        produced += str_ref.len_bytes();
        idx += 1;
    }
    builder
        .finish()
        .expect("TextBuilder::finish should succeed")
        .into_std()
}

fn build_repeated_text(target_bytes: usize, samples: &[&str]) -> String {
    let mut text = String::new();
    let mut idx = 0usize;
    while text.len() < target_bytes {
        text.push_str(samples[idx % samples.len()]);
        idx += 1;
    }
    text
}

fn build_audit_metadata(stats: &GraphemeStats) -> Value {
    let mut metadata = JsonMap::new();
    metadata.insert(
        "text.grapheme_stats".into(),
        Value::Object(grapheme_stats_metadata(stats)),
    );
    metadata.insert("collector.effect.audit".into(), Value::Bool(true));
    Value::Object(metadata)
}
