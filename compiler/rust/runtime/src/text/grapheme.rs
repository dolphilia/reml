use std::borrow::Cow;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, Once};

use unicode_segmentation::{Graphemes, UnicodeSegmentation};
use unicode_width::UnicodeWidthStr;

use super::{Str, UnicodeResult};

const CACHE_VERSION: u32 = 1;

#[derive(Default)]
struct IndexCacheStore {
  entries: HashMap<u64, CacheEntry>,
}

struct CacheEntry {
  len: usize,
  hash: u64,
  offsets: Arc<Vec<usize>>,
  generation: u32,
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
pub struct GraphemeCluster<'a> {
  cluster: Cow<'a, str>,
  display_width: usize,
  byte_len: usize,
  is_emoji: bool,
}

impl<'a> GraphemeCluster<'a> {
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
}

/// `GraphemeCluster` 列とオフセットキャッシュを保持するシーケンス。
#[derive(Debug, Clone)]
pub struct GraphemeSeq<'a> {
  clusters: Vec<GraphemeCluster<'a>>,
  byte_offsets: Arc<Vec<usize>>,
  total_bytes: usize,
  cache_hits: usize,
  cache_miss: usize,
  cache_generation: u32,
}

impl<'a> GraphemeSeq<'a> {
  pub fn clusters(&self) -> &[GraphemeCluster<'a>] {
    &self.clusters
  }

  pub fn byte_offsets(&self) -> &[usize] {
    self.byte_offsets.as_ref().as_slice()
  }

  pub fn total_display_width(&self) -> usize {
    self.clusters.iter().map(|g| g.display_width()).sum()
  }

  pub fn stats(&self) -> GraphemeStats {
    let total_display_width = self.total_display_width();
    let emoji_clusters = self.clusters.iter().filter(|g| g.is_emoji()).count();
    let grapheme_count = self.clusters.len();
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
      cache_hits: self.cache_hits,
      cache_miss: self.cache_miss,
      cache_generation: self.cache_generation,
      cache_version: CACHE_VERSION,
    }
  }
}

/// `log_grapheme_stats` の将来要件を見据えた簡易統計。
#[derive(Debug, Clone, Copy)]
pub struct GraphemeStats {
  pub grapheme_count: usize,
  pub total_bytes: usize,
  pub total_display_width: usize,
  pub avg_width: f64,
  pub emoji_ratio: f64,
  pub cache_hits: usize,
  pub cache_miss: usize,
  pub cache_generation: u32,
  pub cache_version: u32,
}

/// unicode-segmentation + unicode-width を利用した実装。
pub fn segment_graphemes<'a>(str_ref: &'a Str<'a>) -> UnicodeResult<GraphemeSeq<'a>> {
  let source = str_ref.as_str();
  let bytes = source.as_bytes();

  let (clusters, offsets, cache_hits, cache_miss, cache_generation) =
    if let Some((offsets, generation)) = fetch_cached_offsets(bytes) {
      let clusters = build_clusters_from_offsets(source, offsets.as_ref());
      let hit_count = offsets.len();
      (clusters, offsets, hit_count, 0, generation)
    } else {
      let (clusters, offsets_vec) = build_clusters_with_offsets(source);
      let offsets_arc = Arc::new(offsets_vec);
      let generation = store_cached_offsets(bytes, offsets_arc.clone());
      let cache_miss = clusters.len();
      (clusters, offsets_arc, 0, cache_miss, generation)
    };

  Ok(GraphemeSeq {
    clusters,
    byte_offsets: offsets,
    total_bytes: source.len(),
    cache_hits,
    cache_miss,
    cache_generation,
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

/// `Str` から直接 `GraphemeSeq` を構築し統計を取得するヘルパ。
pub fn grapheme_stats(str_ref: &Str<'_>) -> UnicodeResult<GraphemeStats> {
  segment_graphemes(str_ref).map(|seq| seq.stats())
}

/// 監査ログへの配線を見据えた計測 API。現状は `GraphemeStats` を返すのみ。
pub fn log_grapheme_stats(str_ref: &Str<'_>) -> UnicodeResult<GraphemeStats> {
  grapheme_stats(str_ref)
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

fn build_clusters_with_offsets<'a>(
  source: &'a str,
) -> (Vec<GraphemeCluster<'a>>, Vec<usize>) {
  let mut clusters = Vec::new();
  let mut offsets = Vec::new();
  for (offset, cluster) in source.grapheme_indices(true) {
    offsets.push(offset);
    clusters.push(make_cluster(cluster));
  }
  (clusters, offsets)
}

fn build_clusters_from_offsets<'a>(
  source: &'a str,
  offsets: &[usize],
) -> Vec<GraphemeCluster<'a>> {
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
    clusters.push(make_cluster(cluster));
  }
  clusters
}

fn make_cluster(cluster: &str) -> GraphemeCluster<'_> {
  let display_width = UnicodeWidthStr::width(cluster).max(1);
  let is_emoji = contains_emoji(cluster);
  GraphemeCluster {
    cluster: Cow::Borrowed(cluster),
    display_width,
    byte_len: cluster.len(),
    is_emoji,
  }
}

fn fetch_cached_offsets(bytes: &[u8]) -> Option<(Arc<Vec<usize>>, u32)> {
  let hash = hash_bytes(bytes);
  let cache = cache_handle();
  cache
    .lock()
    .ok()?
    .entries
    .get(&hash)
    .filter(|entry| entry.len == bytes.len() && entry.hash == hash)
    .map(|entry| (entry.offsets.clone(), entry.generation))
}

fn store_cached_offsets(bytes: &[u8], offsets: Arc<Vec<usize>>) -> u32 {
  let hash = hash_bytes(bytes);
  let generation = CACHE_GENERATION.fetch_add(1, Ordering::Relaxed) + 1;
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
