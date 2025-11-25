use super::{Str, UnicodeResult};
use unicode_width::UnicodeWidthStr;

/// 書記素幅変換のモード。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WidthMode {
  Narrow,
  Wide,
  EmojiCompat,
}

/// width_map の観測値。
#[derive(Debug, Clone, Copy, Default)]
pub struct WidthMapStats {
  pub grapheme_count: usize,
  pub original_width: usize,
  pub corrected_width: usize,
  pub corrections_applied: usize,
}

struct WidthCorrection {
  sequence: &'static str,
  corrected_width: usize,
}

const EMOJI_CORRECTIONS: &[WidthCorrection] = &[
  WidthCorrection {
    sequence: "👨‍👩‍👧‍👦",
    corrected_width: 4,
  },
  WidthCorrection {
    sequence: "🇯🇵",
    corrected_width: 4,
  },
];

/// `width_map` の基本実装。現状はテキストを変更せず、幅補正のみを適用する。
pub fn width_map(str_ref: &Str<'_>, mode: WidthMode) -> UnicodeResult<super::String> {
  Ok(width_map_with_stats(str_ref, mode).0)
}

/// 幅補正の統計値を得ながら `width_map` を適用する。
pub fn width_map_with_stats(
  str_ref: &Str<'_>,
  mode: WidthMode,
) -> (super::String, WidthMapStats) {
  let mut stats = WidthMapStats::default();
  accumulate_widths(str_ref, mode, &mut stats);
  (super::String::from_str(str_ref.as_str()), stats)
}

fn accumulate_widths(str_ref: &Str<'_>, mode: WidthMode, stats: &mut WidthMapStats) {
  for grapheme in str_ref.iter_graphemes() {
    stats.grapheme_count += 1;
    let base_width = base_width(grapheme, mode);
    stats.original_width += base_width;
    let corrected = corrected_width(grapheme, mode, base_width, stats);
    stats.corrected_width += corrected;
  }
}

fn base_width(grapheme: &str, mode: WidthMode) -> usize {
  let mut width = UnicodeWidthStr::width(grapheme).max(1);
  if matches!(mode, WidthMode::Wide) {
    if grapheme.chars().all(|ch| ch.is_ascii_graphic()) {
      width = width.max(2);
    }
  }
  width
}

fn corrected_width(
  grapheme: &str,
  mode: WidthMode,
  base_width: usize,
  stats: &mut WidthMapStats,
) -> usize {
  if !matches!(mode, WidthMode::EmojiCompat) {
    return base_width;
  }
  if let Some(entry) = EMOJI_CORRECTIONS.iter().find(|c| c.sequence == grapheme) {
    stats.corrections_applied += 1;
    entry.corrected_width
  } else {
    base_width
  }
}
