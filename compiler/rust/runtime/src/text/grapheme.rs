use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use super::Str;

/// Grapheme 単位の情報を保持する簡易クラスタ。
#[derive(Debug, Clone)]
pub struct GraphemeCluster {
  pub cluster: String,
  pub display_width: usize,
  pub byte_len: usize,
  pub is_emoji: bool,
}

/// `GraphemeCluster` 列とオフセットキャッシュを保持する PoC 用シーケンス。
#[derive(Debug, Clone)]
pub struct GraphemeSeq {
  clusters: Vec<GraphemeCluster>,
  byte_offsets: Vec<usize>,
  total_bytes: usize,
}

impl GraphemeSeq {
  pub fn clusters(&self) -> &[GraphemeCluster] {
    &self.clusters
  }

  pub fn byte_offsets(&self) -> &[usize] {
    &self.byte_offsets
  }

  pub fn total_display_width(&self) -> usize {
    self.clusters.iter().map(|g| g.display_width).sum()
  }

  pub fn stats(&self) -> GraphemeStats {
    let total_display_width = self.total_display_width();
    let emoji_clusters = self.clusters.iter().filter(|g| g.is_emoji).count();
    GraphemeStats {
      grapheme_count: self.clusters.len(),
      total_bytes: self.total_bytes,
      total_display_width,
      avg_width: if self.clusters.is_empty() {
        0.0
      } else {
        total_display_width as f64 / self.clusters.len() as f64
      },
      emoji_ratio: if self.clusters.is_empty() {
        0.0
      } else {
        emoji_clusters as f64 / self.clusters.len() as f64
      },
      cache_hits: self.clusters.len(),
      cache_miss: 0,
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
}

/// unicode-segmentation + unicode-width を利用した PoC 実装。
pub fn segment_graphemes(str_ref: &Str<'_>) -> GraphemeSeq {
  let source = str_ref.as_str();
  let mut clusters = Vec::new();
  let mut offsets = Vec::new();

  for (offset, cluster) in source.grapheme_indices(true) {
    offsets.push(offset);
    let display_width = UnicodeWidthStr::width(cluster).max(1);
    let is_emoji = contains_emoji(cluster);
    clusters.push(GraphemeCluster {
      cluster: cluster.to_string(),
      display_width,
      byte_len: cluster.len(),
      is_emoji,
    });
  }

  GraphemeSeq {
    clusters,
    byte_offsets: offsets,
    total_bytes: source.len(),
  }
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
pub fn grapheme_stats(str_ref: &Str<'_>) -> GraphemeStats {
  segment_graphemes(str_ref).stats()
}
