#![cfg(feature = "core_prelude")]

use crate::collections::persistent::{
    btree::{PersistentMap, PersistentMapSharingStats},
    list::{List, ListSharingStats},
};
use std::fs;
use std::path::Path;

const LIST_SAMPLE_SIZE: usize = 100_000;
const MAP_SAMPLE_SIZE: usize = 50_000;

/// ベンチマークで記録するシナリオメトリクス。
#[derive(Debug, Clone)]
pub struct ScenarioMetrics {
    pub scenario: &'static str,
    pub items: usize,
    pub input_bytes: usize,
    pub peak_effect_mem_bytes: usize,
    pub peak_mem_ratio: f64,
    pub avg_reuse_ratio: f64,
    pub shared_nodes: usize,
    pub allocation_count: usize,
}

impl ScenarioMetrics {
    fn csv_row(&self) -> String {
        format!(
            "{},{},{},{},{:.4},{:.4},{},{}",
            self.scenario,
            self.items,
            self.input_bytes,
            self.peak_effect_mem_bytes,
            self.peak_mem_ratio,
            self.avg_reuse_ratio,
            self.shared_nodes,
            self.allocation_count
        )
    }
}

/// 永続コレクションの構造共有メトリクスを収集する。
pub fn collect_persistent_metrics() -> Vec<ScenarioMetrics> {
    let list_metrics = analyze_list_scenario();
    let map_metrics = analyze_map_scenario();
    vec![list_metrics, map_metrics]
}

/// CSV 文字列を生成する。
pub fn render_metrics_csv(metrics: &[ScenarioMetrics]) -> String {
    let mut rows = Vec::with_capacity(metrics.len() + 1);
    rows.push("scenario,items,input_bytes,peak_effect_mem_bytes,peak_mem_ratio,avg_reuse_ratio,shared_nodes,allocations".to_string());
    rows.extend(metrics.iter().map(ScenarioMetrics::csv_row));
    rows.join("\n")
}

/// CSV ファイルに書き出す。
pub fn write_metrics_csv(
    path: impl AsRef<Path>,
    metrics: &[ScenarioMetrics],
) -> std::io::Result<()> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, render_metrics_csv(metrics))
}

fn analyze_list_scenario() -> ScenarioMetrics {
    let records = build_list_records(LIST_SAMPLE_SIZE);
    let versions = build_list_versions(&records);
    let stats: Vec<ListSharingStats> = versions
        .iter()
        .map(|list| list.sharing_stats_with(DslRecord::heap_bytes))
        .collect();
    let input_bytes = records.iter().map(DslRecord::heap_bytes).sum::<usize>();
    summarize_list_stats("ListPersistentPatch", LIST_SAMPLE_SIZE, input_bytes, &stats)
}

fn analyze_map_scenario() -> ScenarioMetrics {
    let entries = build_map_entries(MAP_SAMPLE_SIZE);
    let versions = build_map_versions(&entries);
    let stats: Vec<PersistentMapSharingStats> = versions
        .iter()
        .map(|map| map.sharing_stats_with(|key, value| key.len() + value.heap_bytes()))
        .collect();
    let input_bytes = entries
        .iter()
        .map(|(key, value)| key.len() + value.heap_bytes())
        .sum::<usize>();
    summarize_map_stats("MapPersistentMerge", MAP_SAMPLE_SIZE, input_bytes, &stats)
}

fn summarize_list_stats(
    scenario: &'static str,
    items: usize,
    input_bytes: usize,
    stats: &[ListSharingStats],
) -> ScenarioMetrics {
    let (peak_bytes, shared_nodes, allocations) = stats.iter().fold(
        (0usize, 0usize, 0usize),
        |(max_bytes, max_shared, max_alloc), current| {
            (
                max_bytes.max(current.estimated_heap_bytes),
                max_shared.max(current.shared_nodes),
                max_alloc.max(current.total_nodes),
            )
        },
    );
    ScenarioMetrics {
        scenario,
        items,
        input_bytes,
        peak_effect_mem_bytes: peak_bytes,
        peak_mem_ratio: if input_bytes == 0 {
            0.0
        } else {
            peak_bytes as f64 / input_bytes as f64
        },
        avg_reuse_ratio: average_ratio(stats.iter().map(ListSharingStats::reuse_ratio)),
        shared_nodes,
        allocation_count: allocations,
    }
}

fn summarize_map_stats(
    scenario: &'static str,
    items: usize,
    input_bytes: usize,
    stats: &[PersistentMapSharingStats],
) -> ScenarioMetrics {
    let (peak_bytes, shared_nodes, allocations) = stats.iter().fold(
        (0usize, 0usize, 0usize),
        |(max_bytes, max_shared, max_alloc), current| {
            (
                max_bytes.max(current.estimated_heap_bytes),
                max_shared.max(current.shared_nodes),
                max_alloc.max(current.total_nodes),
            )
        },
    );
    ScenarioMetrics {
        scenario,
        items,
        input_bytes,
        peak_effect_mem_bytes: peak_bytes,
        peak_mem_ratio: if input_bytes == 0 {
            0.0
        } else {
            peak_bytes as f64 / input_bytes as f64
        },
        avg_reuse_ratio: average_ratio(stats.iter().map(PersistentMapSharingStats::reuse_ratio)),
        shared_nodes,
        allocation_count: allocations,
    }
}

fn average_ratio<'a, I>(iter: I) -> f64
where
    I: Iterator<Item = f64>,
{
    let mut total = 0.0;
    let mut count = 0.0;
    for value in iter {
        total += value;
        count += 1.0;
    }
    if count == 0.0 {
        0.0
    } else {
        total / count
    }
}

#[derive(Clone)]
struct DslRecord {
    path: String,
    payload: String,
    ordinal: u32,
}

impl DslRecord {
    fn new(index: usize) -> Self {
        let module = match index % 4 {
            0 => "collector",
            1 => "diagnostics",
            2 => "config",
            _ => "runtime",
        };
        let bucket = index % 256;
        let path = format!("dsl://{module}/component/{bucket:04}");
        let payload = format!(
            "fn apply_patch_{bucket:04}() -> Result<Stage, Error> {{ stage({}) }} // audit:links-ch0:{module}:{:04}:{} // source:reports/spec-audit/ch0/links.md#dsl-sample:{}:{} // patch-body:{module}:{:04}:{} // replay-hash:{:016x}",
            1000 + bucket,
            bucket,
            index % 128,
            module,
            index % 512,
            bucket,
            index % 2048,
            (index as u64).wrapping_mul(0xA24BAED5)
        );
        Self {
            path,
            payload,
            ordinal: (index % 8192) as u32,
        }
    }

    fn heap_bytes(&self) -> usize {
        self.path.len() + self.payload.len() + std::mem::size_of::<u32>()
    }
}

#[derive(Clone)]
struct ConfigEntry {
    version: u32,
    checksum: String,
    enabled: bool,
    stage: &'static str,
    notes: String,
}

impl ConfigEntry {
    fn new(index: usize) -> Self {
        let stage = match index % 3 {
            0 => "beta",
            1 => "stable",
            _ => "legacy",
        };
        let checksum = format!("{:016x}", (index as u64).wrapping_mul(0x9E3779B97F4A7C15));
        let notes = format!("remap:component:{}:{}", stage, index % 97);
        Self {
            version: 100 + (index % 500) as u32,
            checksum,
            enabled: index % 5 != 0,
            stage,
            notes,
        }
    }

    fn heap_bytes(&self) -> usize {
        self.checksum.len() + self.notes.len()
    }
}

fn build_list_records(count: usize) -> Vec<DslRecord> {
    (0..count).map(DslRecord::new).collect()
}

fn build_map_entries(count: usize) -> Vec<(String, ConfigEntry)> {
    (0..count)
        .map(|index| {
            let key = format!("config://schema/{:05}/{}", index % 327, index);
            (key, ConfigEntry::new(index))
        })
        .collect()
}

fn build_list_versions(records: &[DslRecord]) -> Vec<List<DslRecord>> {
    let base = List::of_iter(records.iter().cloned());
    let mut versions = vec![base.clone()];
    let mut rolling = base;

    let chunk = (records.len() / 10).max(1);
    for segment in records.chunks(chunk).take(5) {
        for record in segment.iter().step_by(7) {
            rolling = rolling.push_front(record.clone());
        }
        versions.push(rolling.clone());
    }

    let tail = List::of_iter(records.iter().skip(records.len() / 2).cloned());
    versions.push(tail.clone());

    let merged = tail.concat(&rolling);
    versions.push(merged);
    versions
}

fn build_map_versions(
    entries: &[(String, ConfigEntry)],
) -> Vec<PersistentMap<String, ConfigEntry>> {
    let mut base = PersistentMap::new();
    for (key, value) in entries.iter().cloned() {
        base = base.insert(key, value);
    }

    let mut versions = vec![base.clone()];
    let mut working = base;

    let chunk = (entries.len() / 6).max(1);
    for (idx, segment) in entries.chunks(chunk).enumerate().take(5) {
        let mut delta = PersistentMap::new();
        for (offset, (key, value)) in segment.iter().enumerate().step_by(13) {
            let mut modified = value.clone();
            modified.enabled = (idx + offset) % 2 == 0;
            modified.version = modified.version.wrapping_add((idx * 3 + offset) as u32);
            modified.notes = format!("{}#patch{}", modified.notes, idx);
            let patch_key = format!("{key}#{}", idx);
            delta = delta.insert(patch_key, modified);
        }
        working = working.merge_with(&delta, |_, left, right| {
            if idx % 2 == 0 {
                right.clone()
            } else {
                left.clone()
            }
        });
        versions.push(working.clone());
    }
    versions
}
