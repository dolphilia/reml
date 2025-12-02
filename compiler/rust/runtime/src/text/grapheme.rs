use std::borrow::Cow;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, Once};

use unicode_segmentation::{Graphemes, UnicodeSegmentation, UNICODE_VERSION};
use unicode_width::UnicodeWidthStr;

use super::{effects, Str, UnicodeResult};

const CACHE_VERSION: u32 = 1;
const UNICODE_VERSION_TUPLE: (u8, u8, u8) = (
    UNICODE_VERSION.0 as u8,
    UNICODE_VERSION.1 as u8,
    UNICODE_VERSION.2 as u8,
);
const SCRIPT_BUCKETS: usize = 6;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct IndexCacheVersion {
    cache_version: u32,
    unicode_version: (u8, u8, u8),
}

impl IndexCacheVersion {
    fn current() -> Self {
        Self {
            cache_version: CACHE_VERSION,
            unicode_version: UNICODE_VERSION_TUPLE,
        }
    }

    fn matches(&self) -> bool {
        self.cache_version == CACHE_VERSION && self.unicode_version == UNICODE_VERSION_TUPLE
    }
}

#[derive(Debug, Clone, Copy)]
struct IndexCacheGeneration {
    id: u32,
    version: IndexCacheVersion,
}

impl IndexCacheGeneration {
    fn new(id: u32) -> Self {
        Self {
            id,
            version: IndexCacheVersion::current(),
        }
    }
}

enum CacheLookup {
    Hit {
        offsets: Arc<Vec<usize>>,
        generation: IndexCacheGeneration,
    },
    VersionMismatch,
    Miss,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScriptCategory {
    Latin = 0,
    Han = 1,
    Kana = 2,
    Arabic = 3,
    Emoji = 4,
    Other = 5,
}

impl ScriptCategory {
    pub const ALL: [ScriptCategory; SCRIPT_BUCKETS] = [
        ScriptCategory::Latin,
        ScriptCategory::Han,
        ScriptCategory::Kana,
        ScriptCategory::Arabic,
        ScriptCategory::Emoji,
        ScriptCategory::Other,
    ];

    fn as_index(self) -> usize {
        self as usize
    }

    fn from_index(index: usize) -> ScriptCategory {
        ScriptCategory::ALL
            .get(index)
            .copied()
            .unwrap_or(ScriptCategory::Other)
    }

    fn direction(self) -> TextDirection {
        match self {
            ScriptCategory::Arabic => TextDirection::RightToLeft,
            ScriptCategory::Emoji => TextDirection::Neutral,
            _ => TextDirection::LeftToRight,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ScriptCategory::Latin => "latin",
            ScriptCategory::Han => "han",
            ScriptCategory::Kana => "kana",
            ScriptCategory::Arabic => "arabic",
            ScriptCategory::Emoji => "emoji",
            ScriptCategory::Other => "other",
        }
    }
}

impl Default for ScriptCategory {
    fn default() -> Self {
        ScriptCategory::Other
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextDirection {
    LeftToRight,
    RightToLeft,
    Neutral,
}

#[derive(Debug, Clone, Copy)]
pub struct ScriptStats {
    pub primary: ScriptCategory,
    pub primary_ratio: f64,
    pub mix_ratio: f64,
}

impl Default for ScriptStats {
    fn default() -> Self {
        ScriptStats {
            primary: ScriptCategory::Other,
            primary_ratio: 0.0,
            mix_ratio: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DirectionStats {
    pub rtl_ratio: f64,
}

impl Default for DirectionStats {
    fn default() -> Self {
        DirectionStats { rtl_ratio: 0.0 }
    }
}

#[derive(Default)]
struct ScriptHistogram {
    counts: [usize; SCRIPT_BUCKETS],
}

impl ScriptHistogram {
    fn record(&mut self, script: ScriptCategory) {
        self.counts[script.as_index()] += 1;
    }

    fn primary(&self) -> (ScriptCategory, usize) {
        let mut best_script = ScriptCategory::Other;
        let mut best_count = 0usize;
        for (idx, &count) in self.counts.iter().enumerate() {
            if count > best_count {
                best_count = count;
                best_script = ScriptCategory::from_index(idx);
            }
        }
        (best_script, best_count)
    }
}

#[derive(Default)]
struct IndexCacheStore {
    entries: HashMap<u64, CacheEntry>,
}

struct CacheEntry {
    len: usize,
    hash: u64,
    offsets: Arc<Vec<usize>>,
    generation: IndexCacheGeneration,
}

static CACHE_INIT: Once = Once::new();
static mut INDEX_CACHE: Option<Mutex<IndexCacheStore>> = None;
static CACHE_GENERATION: AtomicU32 = AtomicU32::new(0);

fn cache_handle() -> &'static Mutex<IndexCacheStore> {
    unsafe {
        CACHE_INIT.call_once(|| {
            INDEX_CACHE = Some(Mutex::new(IndexCacheStore::default()));
        });
        INDEX_CACHE
            .as_ref()
            .expect("Index cache should be initialized")
    }
}

/// Grapheme 単位の情報を保持する簡易クラスタ。
#[derive(Debug, Clone)]
pub struct Grapheme<'a> {
    cluster: Cow<'a, str>,
    display_width: usize,
    byte_len: usize,
    is_emoji: bool,
    script: ScriptCategory,
    direction: TextDirection,
}

impl<'a> Grapheme<'a> {
    pub fn as_str(&self) -> &str {
        &self.cluster
    }

    pub fn display_width(&self) -> usize {
        self.display_width
    }

    pub fn is_emoji(&self) -> bool {
        self.is_emoji
    }

    pub fn byte_len(&self) -> usize {
        self.byte_len
    }

    pub fn script(&self) -> ScriptCategory {
        self.script
    }

    pub fn direction(&self) -> TextDirection {
        self.direction
    }
}

/// `GraphemeCluster` 列とオフセットキャッシュを保持するシーケンス。
#[derive(Debug, Clone)]
pub struct GraphemeSeq<'a> {
    clusters: Vec<Grapheme<'a>>,
    byte_offsets: Arc<Vec<usize>>,
    total_bytes: usize,
    cache_hits: usize,
    cache_miss: usize,
    cache_generation: u32,
    cache_version: u32,
    unicode_version: (u8, u8, u8),
    version_mismatch_evictions: usize,
}

impl<'a> GraphemeSeq<'a> {
    pub fn clusters(&self) -> &[Grapheme<'a>] {
        &self.clusters
    }

    pub fn byte_offsets(&self) -> &[usize] {
        self.byte_offsets.as_ref().as_slice()
    }

    pub fn len(&self) -> usize {
        self.clusters.len()
    }

    pub fn is_empty(&self) -> bool {
        self.clusters.is_empty()
    }

    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &Grapheme<'a>> + ExactSizeIterator + '_ {
        self.clusters.iter()
    }

    pub fn get(&self, index: usize) -> Option<&Grapheme<'a>> {
        self.clusters.get(index)
    }

    pub fn total_bytes(&self) -> usize {
        self.total_bytes
    }

    pub fn byte_offset_at(&self, index: usize) -> Option<usize> {
        self.byte_offsets.get(index).copied()
    }

    pub fn grapheme_index_for_byte(&self, byte_offset: usize) -> Option<usize> {
        self.byte_offsets.binary_search(&byte_offset).ok()
    }

    pub fn grapheme_at_byte_offset(&self, byte_offset: usize) -> Option<&Grapheme<'a>> {
        if byte_offset >= self.total_bytes {
            return None;
        }
        match self.byte_offsets.binary_search(&byte_offset) {
            Ok(index) => self.clusters.get(index),
            Err(pos) if pos == 0 => None,
            Err(pos) => self.clusters.get(pos - 1),
        }
    }

    pub fn total_display_width(&self) -> usize {
        self.clusters.iter().map(|g| g.display_width()).sum()
    }

    pub fn stats(&self) -> GraphemeStats {
        let total_display_width = self.total_display_width();
        let emoji_clusters = self.clusters.iter().filter(|g| g.is_emoji()).count();
        let grapheme_count = self.clusters.len();
        let mut script_hist = ScriptHistogram::default();
        let mut rtl_clusters = 0usize;
        for cluster in &self.clusters {
            script_hist.record(cluster.script());
            if cluster.direction() == TextDirection::RightToLeft {
                rtl_clusters += 1;
            }
        }
        let (primary_script, primary_count) = script_hist.primary();
        let script_stats = if grapheme_count == 0 {
            ScriptStats::default()
        } else {
            let primary_ratio = primary_count as f64 / grapheme_count as f64;
            ScriptStats {
                primary: primary_script,
                primary_ratio,
                mix_ratio: (1.0 - primary_ratio).max(0.0),
            }
        };
        let direction = if grapheme_count == 0 {
            DirectionStats::default()
        } else {
            DirectionStats {
                rtl_ratio: rtl_clusters as f64 / grapheme_count as f64,
            }
        };
        GraphemeStats {
            grapheme_count,
            total_bytes: self.total_bytes,
            total_display_width,
            avg_width: if grapheme_count == 0 {
                0.0
            } else {
                total_display_width as f64 / grapheme_count as f64
            },
            emoji_ratio: if grapheme_count == 0 {
                0.0
            } else {
                emoji_clusters as f64 / grapheme_count as f64
            },
            scripts: script_stats,
            direction,
            cache_hits: self.cache_hits,
            cache_miss: self.cache_miss,
            cache_generation: self.cache_generation,
            cache_version: self.cache_version,
            unicode_version: format_unicode_version(self.unicode_version),
            version_mismatch_evictions: self.version_mismatch_evictions,
        }
    }
}

impl<'a> IntoIterator for GraphemeSeq<'a> {
    type Item = Grapheme<'a>;
    type IntoIter = std::vec::IntoIter<Grapheme<'a>>;

    fn into_iter(self) -> Self::IntoIter {
        self.clusters.into_iter()
    }
}

impl<'a, 'b> IntoIterator for &'b GraphemeSeq<'a> {
    type Item = &'b Grapheme<'a>;
    type IntoIter = std::slice::Iter<'b, Grapheme<'a>>;

    fn into_iter(self) -> Self::IntoIter {
        self.clusters.iter()
    }
}

/// `log_grapheme_stats` の将来要件を見据えた簡易統計。
#[derive(Debug, Clone)]
pub struct GraphemeStats {
    pub grapheme_count: usize,
    pub total_bytes: usize,
    pub total_display_width: usize,
    pub avg_width: f64,
    pub emoji_ratio: f64,
    pub scripts: ScriptStats,
    pub direction: DirectionStats,
    pub cache_hits: usize,
    pub cache_miss: usize,
    pub cache_generation: u32,
    pub cache_version: u32,
    pub unicode_version: String,
    pub version_mismatch_evictions: usize,
}

/// unicode-segmentation + unicode-width を利用した実装。
pub fn segment_graphemes<'a>(str_ref: &'a Str<'a>) -> UnicodeResult<GraphemeSeq<'a>> {
    let source = str_ref.as_str();
    let bytes = source.as_bytes();

    let mut version_mismatch_evictions = 0usize;
    let (clusters, offsets, cache_hits, cache_miss, cache_generation) =
        match fetch_cached_offsets(bytes) {
            CacheLookup::Hit {
                offsets,
                generation,
            } => {
                let clusters = build_clusters_from_offsets(source, offsets.as_ref());
                let hit_count = offsets.len();
                (clusters, offsets, hit_count, 0, generation)
            }
            CacheLookup::VersionMismatch => {
                version_mismatch_evictions = 1;
                let (clusters, offsets, generation) = build_and_store_offsets(source, bytes);
                let cache_miss = clusters.len();
                (clusters, offsets, 0, cache_miss, generation)
            }
            CacheLookup::Miss => {
                let (clusters, offsets, generation) = build_and_store_offsets(source, bytes);
                let cache_miss = clusters.len();
                (clusters, offsets, 0, cache_miss, generation)
            }
        };

    Ok(GraphemeSeq {
        clusters,
        byte_offsets: offsets,
        total_bytes: source.len(),
        cache_hits,
        cache_miss,
        cache_generation: cache_generation.id,
        cache_version: cache_generation.version.cache_version,
        unicode_version: cache_generation.version.unicode_version,
        version_mismatch_evictions,
    })
}

fn contains_emoji(cluster: &str) -> bool {
    cluster.chars().any(|ch| {
        let code = ch as u32;
        (0x1F1E6..=0x1F1FF).contains(&code) // Regional indicators
      || (0x1F300..=0x1FAFF).contains(&code)
      || ch == '\u{200D}' // ZWJ joins emoji family sequences
    })
}

fn detect_script(cluster: &str, is_emoji: bool) -> ScriptCategory {
    if is_emoji {
        return ScriptCategory::Emoji;
    }
    for ch in cluster.chars() {
        if let Some(script) = classify_scalar(ch) {
            return script;
        }
    }
    ScriptCategory::Other
}

fn classify_scalar(ch: char) -> Option<ScriptCategory> {
    let code = ch as u32;
    if is_latin_scalar(code) {
        Some(ScriptCategory::Latin)
    } else if is_kana_scalar(code) {
        Some(ScriptCategory::Kana)
    } else if is_han_scalar(code) {
        Some(ScriptCategory::Han)
    } else if is_arabic_scalar(code) {
        Some(ScriptCategory::Arabic)
    } else {
        None
    }
}

fn is_latin_scalar(code: u32) -> bool {
    (0x0041..=0x02AF).contains(&code)
        || (0x0030..=0x0039).contains(&code)
        || (0x1E00..=0x1EFF).contains(&code)
        || (0x2C60..=0x2C7F).contains(&code)
        || (0xA720..=0xA7FF).contains(&code)
}

fn is_kana_scalar(code: u32) -> bool {
    (0x3040..=0x30FF).contains(&code)
        || (0x31F0..=0x31FF).contains(&code)
        || (0xFF66..=0xFF9F).contains(&code)
}

fn is_han_scalar(code: u32) -> bool {
    (0x3400..=0x9FFF).contains(&code)
        || (0xF900..=0xFAFF).contains(&code)
        || (0x20000..=0x2CEAF).contains(&code)
}

fn is_arabic_scalar(code: u32) -> bool {
    (0x0600..=0x077F).contains(&code)
        || (0x08A0..=0x08FF).contains(&code)
        || (0xFB50..=0xFDFF).contains(&code)
        || (0xFE70..=0xFEFF).contains(&code)
}

/// `Str` から直接 `GraphemeSeq` を構築し統計を取得するヘルパ。
pub fn grapheme_stats(str_ref: &Str<'_>) -> UnicodeResult<GraphemeStats> {
    segment_graphemes(str_ref).map(|seq| seq.stats())
}

/// 監査ログへの配線を見据えた計測 API。現状は `GraphemeStats` を返すのみ。
pub fn log_grapheme_stats(str_ref: &Str<'_>) -> UnicodeResult<GraphemeStats> {
    let stats = grapheme_stats(str_ref)?;
    effects::record_audit_event_with_metadata(&stats);
    Ok(stats)
}

/// `Str` から書記素列を反復するためのラッパ。
pub struct GraphemeIter<'a> {
    inner: Graphemes<'a>,
}

impl<'a> GraphemeIter<'a> {
    pub(crate) fn new(source: &'a str) -> Self {
        Self {
            inner: source.graphemes(true),
        }
    }
}

impl<'a> Iterator for GraphemeIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

fn build_clusters_with_offsets<'a>(source: &'a str) -> (Vec<Grapheme<'a>>, Vec<usize>) {
    let mut clusters = Vec::new();
    let mut offsets = Vec::new();
    for (offset, cluster) in source.grapheme_indices(true) {
        offsets.push(offset);
        clusters.push(make_grapheme(cluster));
    }
    (clusters, offsets)
}

fn build_clusters_from_offsets<'a>(source: &'a str, offsets: &[usize]) -> Vec<Grapheme<'a>> {
    let mut clusters = Vec::with_capacity(offsets.len());
    for (index, start) in offsets.iter().enumerate() {
        let end = offsets
            .get(index + 1)
            .copied()
            .unwrap_or_else(|| source.len());
        if *start > end || end > source.len() {
            continue;
        }
        let cluster = &source[*start..end];
        clusters.push(make_grapheme(cluster));
    }
    clusters
}

fn build_and_store_offsets<'a>(
    source: &'a str,
    bytes: &[u8],
) -> (Vec<Grapheme<'a>>, Arc<Vec<usize>>, IndexCacheGeneration) {
    let (clusters, offsets_vec) = build_clusters_with_offsets(source);
    let offsets_arc = Arc::new(offsets_vec);
    let generation = store_cached_offsets(bytes, offsets_arc.clone());
    (clusters, offsets_arc, generation)
}

fn make_grapheme(cluster: &str) -> Grapheme<'_> {
    let display_width = UnicodeWidthStr::width(cluster).max(1);
    let is_emoji = contains_emoji(cluster);
    let script = detect_script(cluster, is_emoji);
    Grapheme {
        cluster: Cow::Borrowed(cluster),
        display_width,
        byte_len: cluster.len(),
        is_emoji,
        script,
        direction: script.direction(),
    }
}

fn format_unicode_version(version: (u8, u8, u8)) -> String {
    format!("{}.{}.{}", version.0, version.1, version.2)
}

fn fetch_cached_offsets(bytes: &[u8]) -> CacheLookup {
    let hash = hash_bytes(bytes);
    let cache = cache_handle();
    if let Ok(store) = cache.lock() {
        if let Some(entry) = store.entries.get(&hash) {
            if entry.len == bytes.len() && entry.hash == hash {
                if entry.generation.version.matches() {
                    return CacheLookup::Hit {
                        offsets: entry.offsets.clone(),
                        generation: entry.generation,
                    };
                } else {
                    return CacheLookup::VersionMismatch;
                }
            }
        }
    }
    CacheLookup::Miss
}

fn store_cached_offsets(bytes: &[u8], offsets: Arc<Vec<usize>>) -> IndexCacheGeneration {
    let hash = hash_bytes(bytes);
    let next_id = CACHE_GENERATION.fetch_add(1, Ordering::Relaxed) + 1;
    let generation = IndexCacheGeneration::new(next_id);
    let entry = CacheEntry {
        len: bytes.len(),
        hash,
        offsets,
        generation,
    };
    let cache = cache_handle();
    if let Ok(mut guard) = cache.lock() {
        guard.entries.insert(hash, entry);
    }
    generation
}

fn hash_bytes(bytes: &[u8]) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    let mut hasher = DefaultHasher::new();
    hasher.write_u64(bytes.len() as u64);
    bytes.hash(&mut hasher);
    hasher.finish()
}

/// テスト専用ヘルパ。キャッシュを初期化して結果が独立するようにする。
pub fn clear_grapheme_cache_for_tests() {
    if let Ok(mut guard) = cache_handle().lock() {
        guard.entries.clear();
    }
    CACHE_GENERATION.store(0, Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::{effects, Str};
    use serde_json::json;

    #[test]
    fn reports_script_mix_and_direction() {
        clear_grapheme_cache_for_tests();
        let text = Str::from("かなA🙂ا");
        let seq = segment_graphemes(&text).expect("segment");
        let stats = seq.stats();
        // "かなA🙂ا" consists of 5 Unicode grapheme clusters (two Hiragana, A, emoji, Arabic).
        assert_eq!(stats.grapheme_count, 5);
        assert_eq!(stats.scripts.primary, ScriptCategory::Kana);
        assert!(stats.scripts.mix_ratio > 0.0);
        assert!(stats.direction.rtl_ratio > 0.0);
    }

    #[test]
    fn iterators_support_double_ended_iteration() {
        let text = Str::from("ab");
        let seq = segment_graphemes(&text).expect("segment");
        let mut iter = seq.iter();
        assert_eq!(iter.next().map(|g| g.as_str()), Some("a"));
        assert_eq!(iter.next_back().map(|g| g.as_str()), Some("b"));
    }

    #[test]
    fn log_grapheme_stats_marks_audit_effect() {
        effects::take_recorded_effects();
        let text = Str::from("Audit");
        let _ = log_grapheme_stats(&text).expect("stats ok");
        let labels = effects::take_recorded_effects();
        assert!(
            labels.contains_audit(),
            "log_grapheme_stats should mark audit effect"
        );
        let metadata = effects::take_audit_metadata_payload()
            .expect("metadata should exist after log_grapheme_stats");
        assert!(
            metadata.contains_key("text.grapheme_stats"),
            "metadata should contain text.grapheme_stats"
        );
        let range = metadata
            .get("text.utf8.range")
            .and_then(|value| value.as_object())
            .expect("text.utf8.range metadata should exist");
        assert_eq!(range.get("start"), Some(&json!(0)));
        assert_eq!(range.get("end"), Some(&json!(5)));
        assert_eq!(range.get("length"), Some(&json!(5)));
    }
}
