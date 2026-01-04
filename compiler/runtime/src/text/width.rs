use std::str::FromStr;
use std::{borrow::Cow, cmp::Ordering};

use super::{effects, Str, UnicodeResult};
use once_cell::sync::Lazy;
use unicode_width::UnicodeWidthStr;

/// 書記素幅変換のモード。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WidthMode {
    Narrow,
    Wide,
    EmojiCompat,
}

impl FromStr for WidthMode {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "Narrow" => Ok(WidthMode::Narrow),
            "Wide" => Ok(WidthMode::Wide),
            "EmojiCompat" => Ok(WidthMode::EmojiCompat),
            _ => Err(()),
        }
    }
}

/// width_map の観測値。
#[derive(Debug, Clone, Copy, Default)]
pub struct WidthMapStats {
    pub grapheme_count: usize,
    pub original_width: usize,
    pub corrected_width: usize,
    pub corrections_applied: usize,
}

#[derive(Clone)]
struct WidthCorrection {
    sequence: String,
    mode: WidthMode,
    corrected_width: usize,
}

static WIDTH_CORRECTIONS: Lazy<Vec<WidthCorrection>> = Lazy::new(parse_width_corrections);
const WIDTH_CORRECTIONS_CSV: &str = include_str!("data/width_corrections.csv");
const EAST_ASIAN_WIDTH_DATA: &str =
    include_str!("../../../../tests/data/unicode/UCD/EastAsianWidth-15.1.0.txt");
#[derive(Debug)]
struct EastAsianWidthOverride {
    start: u32,
    end: u32,
    width: usize,
}
static EAST_ASIAN_WIDTH_OVERRIDES: Lazy<Vec<EastAsianWidthOverride>> =
    Lazy::new(parse_east_asian_overrides);

/// `width_map` の基本実装。
pub fn width_map(str_ref: &Str<'_>, mode: WidthMode) -> UnicodeResult<super::String> {
    Ok(width_map_with_stats(str_ref, mode).0)
}

/// 幅補正の統計値を得ながら `width_map` を適用する。
pub fn width_map_with_stats(str_ref: &Str<'_>, mode: WidthMode) -> (super::String, WidthMapStats) {
    let mut stats = WidthMapStats::default();
    let mut buffer = String::with_capacity(str_ref.len_bytes());
    let mut changed = false;
    for grapheme in str_ref.iter_graphemes() {
        stats.grapheme_count += 1;
        let original_width = UnicodeWidthStr::width(grapheme).max(1);
        stats.original_width += original_width;
        let mapped = map_grapheme(grapheme, mode);
        let mut _mapped_storage: Option<String> = None;
        let (mapped_str, grapheme_changed) = match mapped {
            Cow::Owned(owned) => {
                _mapped_storage = Some(owned);
                (_mapped_storage.as_ref().unwrap().as_str(), true)
            }
            Cow::Borrowed(borrowed) => (borrowed, false),
        };
        if grapheme_changed {
            changed = true;
            stats.corrections_applied += 1;
        }
        buffer.push_str(mapped_str);
        let corrected = corrected_width(mapped_str, mode, &mut stats);
        stats.corrected_width += corrected;
    }
    if !changed {
        return (super::String::from_str(str_ref.as_str()), stats);
    }
    effects::record_mem_copy(buffer.len());
    (super::String::from_std(buffer), stats)
}

fn map_grapheme<'a>(grapheme: &'a str, mode: WidthMode) -> Cow<'a, str> {
    match mode {
        WidthMode::Narrow => narrow_grapheme(grapheme),
        WidthMode::Wide | WidthMode::EmojiCompat => wide_grapheme(grapheme),
    }
}

fn wide_grapheme(grapheme: &str) -> Cow<'_, str> {
    let mut buffer = String::with_capacity(grapheme.len());
    let mut changed = false;
    let mut last_entry: Option<&KanaMapping> = None;
    for ch in grapheme.chars() {
        if let Some(mapped) = ascii_to_full(ch) {
            buffer.push(mapped);
            changed |= mapped != ch;
            last_entry = None;
            continue;
        }
        if let Some(punct) = halfwidth_punct_to_full(ch) {
            buffer.push_str(punct);
            changed = true;
            last_entry = None;
            continue;
        }
        if ch == 'ﾞ' || ch == 'ﾟ' {
            if let Some(entry) = last_entry {
                if let Some(replacement) = entry.apply_mark(ch) {
                    if !buffer.is_empty() {
                        buffer.pop();
                    }
                    buffer.push(replacement);
                    changed = true;
                    continue;
                }
            }
            buffer.push(if ch == 'ﾞ' { '゛' } else { '゜' });
            changed = true;
            last_entry = None;
            continue;
        }
        if let Some(entry) = find_kana_by_half(ch) {
            buffer.push(entry.full);
            changed |= entry.full != ch;
            last_entry = Some(entry);
            continue;
        }
        buffer.push(ch);
        last_entry = None;
    }
    if changed {
        Cow::Owned(buffer)
    } else {
        Cow::Borrowed(grapheme)
    }
}

fn narrow_grapheme(grapheme: &str) -> Cow<'_, str> {
    let mut buffer = String::with_capacity(grapheme.len());
    let mut changed = false;
    for ch in grapheme.chars() {
        if let Some(mapped) = ascii_to_half(ch) {
            buffer.push(mapped);
            changed |= mapped != ch;
            continue;
        }
        if let Some(punct) = fullwidth_punct_to_half(ch) {
            buffer.push_str(punct);
            changed = true;
            continue;
        }
        if let Some((entry, mark)) = find_kana_by_full(ch) {
            buffer.push(entry.half);
            if let Some(mark) = mark {
                buffer.push(mark);
            }
            changed = true;
            continue;
        }
        if ch == '゛' || ch == '゜' {
            buffer.push(if ch == '゛' { 'ﾞ' } else { 'ﾟ' });
            changed = true;
            continue;
        }
        buffer.push(ch);
    }
    if changed {
        Cow::Owned(buffer)
    } else {
        Cow::Borrowed(grapheme)
    }
}

fn corrected_width(grapheme: &str, mode: WidthMode, stats: &mut WidthMapStats) -> usize {
    let mut width = match mode {
        WidthMode::Narrow => UnicodeWidthStr::width(grapheme).max(1),
        WidthMode::Wide | WidthMode::EmojiCompat => UnicodeWidthStr::width_cjk(grapheme).max(1),
    };
    if matches!(mode, WidthMode::Wide | WidthMode::EmojiCompat) {
        let mut chars = grapheme.chars();
        if let Some(ch) = chars.next() {
            if chars.next().is_none() {
                if let Some(override_width) = east_asian_override(ch) {
                    if width != override_width {
                        stats.corrections_applied += 1;
                        width = override_width;
                    }
                }
            }
        }
    }
    if let Some(value) = find_width_correction(grapheme, mode) {
        stats.corrections_applied += 1;
        width = value;
    }
    width
}

fn ascii_to_full(ch: char) -> Option<char> {
    match ch {
        ' '..='~' if ch != ' ' => {
            let code = ch as u32 + 0xFEE0;
            char::from_u32(code)
        }
        ' ' => Some('\u{3000}'),
        _ => None,
    }
}

fn ascii_to_half(ch: char) -> Option<char> {
    match ch {
        '\u{3000}' => Some(' '),
        '\u{FF01}'..='\u{FF5E}' => {
            let code = ch as u32 - 0xFEE0;
            char::from_u32(code)
        }
        _ => None,
    }
}

fn halfwidth_punct_to_full(ch: char) -> Option<&'static str> {
    match ch {
        '｡' => Some("。"),
        '｢' => Some("「"),
        '｣' => Some("」"),
        '､' => Some("、"),
        '･' => Some("・"),
        'ｰ' => Some("ー"),
        _ => None,
    }
}

fn fullwidth_punct_to_half(ch: char) -> Option<&'static str> {
    match ch {
        '。' => Some("｡"),
        '「' => Some("｢"),
        '」' => Some("｣"),
        '、' => Some("､"),
        '・' => Some("･"),
        'ー' => Some("ｰ"),
        _ => None,
    }
}

fn parse_east_asian_overrides() -> Vec<EastAsianWidthOverride> {
    let mut ranges = Vec::new();
    for line in EAST_ASIAN_WIDTH_DATA.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some((range, class)) = trimmed.split_once(';') else {
            continue;
        };
        let class = class.trim();
        let width = match class {
            "W" | "F" | "A" => 2,
            _ => continue,
        };
        let (start, end) = parse_codepoint_range(range.trim());
        ranges.push(EastAsianWidthOverride { start, end, width });
    }
    ranges.sort_by_key(|entry| entry.start);
    ranges
}

fn parse_codepoint_range(range: &str) -> (u32, u32) {
    if let Some((start, end)) = range.split_once("..") {
        (
            u32::from_str_radix(start, 16).expect("invalid start"),
            u32::from_str_radix(end, 16).expect("invalid end"),
        )
    } else {
        let value = u32::from_str_radix(range, 16).expect("invalid value");
        (value, value)
    }
}

fn east_asian_override(ch: char) -> Option<usize> {
    let code = ch as u32;
    let result = EAST_ASIAN_WIDTH_OVERRIDES.binary_search_by(|entry| {
        if code < entry.start {
            Ordering::Greater
        } else if code > entry.end {
            Ordering::Less
        } else {
            Ordering::Equal
        }
    });
    result
        .ok()
        .map(|index| EAST_ASIAN_WIDTH_OVERRIDES[index].width)
}

struct KanaMapping {
    half: char,
    full: char,
    voiced: Option<char>,
    semi_voiced: Option<char>,
}

impl KanaMapping {
    fn apply_mark(&self, mark: char) -> Option<char> {
        match mark {
            'ﾞ' => self.voiced,
            'ﾟ' => self.semi_voiced,
            _ => None,
        }
    }
}

const KANA_TABLE: &[KanaMapping] = &[
    KanaMapping {
        half: 'ｦ',
        full: 'ヲ',
        voiced: Some('ヺ'),
        semi_voiced: None,
    },
    KanaMapping {
        half: 'ｧ',
        full: 'ァ',
        voiced: None,
        semi_voiced: None,
    },
    KanaMapping {
        half: 'ｨ',
        full: 'ィ',
        voiced: None,
        semi_voiced: None,
    },
    KanaMapping {
        half: 'ｩ',
        full: 'ゥ',
        voiced: None,
        semi_voiced: None,
    },
    KanaMapping {
        half: 'ｪ',
        full: 'ェ',
        voiced: None,
        semi_voiced: None,
    },
    KanaMapping {
        half: 'ｫ',
        full: 'ォ',
        voiced: None,
        semi_voiced: None,
    },
    KanaMapping {
        half: 'ｬ',
        full: 'ャ',
        voiced: None,
        semi_voiced: None,
    },
    KanaMapping {
        half: 'ｭ',
        full: 'ュ',
        voiced: None,
        semi_voiced: None,
    },
    KanaMapping {
        half: 'ｮ',
        full: 'ョ',
        voiced: None,
        semi_voiced: None,
    },
    KanaMapping {
        half: 'ｯ',
        full: 'ッ',
        voiced: None,
        semi_voiced: None,
    },
    KanaMapping {
        half: 'ｱ',
        full: 'ア',
        voiced: None,
        semi_voiced: None,
    },
    KanaMapping {
        half: 'ｲ',
        full: 'イ',
        voiced: None,
        semi_voiced: None,
    },
    KanaMapping {
        half: 'ｳ',
        full: 'ウ',
        voiced: Some('ヴ'),
        semi_voiced: None,
    },
    KanaMapping {
        half: 'ｴ',
        full: 'エ',
        voiced: None,
        semi_voiced: None,
    },
    KanaMapping {
        half: 'ｵ',
        full: 'オ',
        voiced: None,
        semi_voiced: None,
    },
    KanaMapping {
        half: 'ｶ',
        full: 'カ',
        voiced: Some('ガ'),
        semi_voiced: None,
    },
    KanaMapping {
        half: 'ｷ',
        full: 'キ',
        voiced: Some('ギ'),
        semi_voiced: None,
    },
    KanaMapping {
        half: 'ｸ',
        full: 'ク',
        voiced: Some('グ'),
        semi_voiced: None,
    },
    KanaMapping {
        half: 'ｹ',
        full: 'ケ',
        voiced: Some('ゲ'),
        semi_voiced: None,
    },
    KanaMapping {
        half: 'ｺ',
        full: 'コ',
        voiced: Some('ゴ'),
        semi_voiced: None,
    },
    KanaMapping {
        half: 'ｻ',
        full: 'サ',
        voiced: Some('ザ'),
        semi_voiced: None,
    },
    KanaMapping {
        half: 'ｼ',
        full: 'シ',
        voiced: Some('ジ'),
        semi_voiced: None,
    },
    KanaMapping {
        half: 'ｽ',
        full: 'ス',
        voiced: Some('ズ'),
        semi_voiced: None,
    },
    KanaMapping {
        half: 'ｾ',
        full: 'セ',
        voiced: Some('ゼ'),
        semi_voiced: None,
    },
    KanaMapping {
        half: 'ｿ',
        full: 'ソ',
        voiced: Some('ゾ'),
        semi_voiced: None,
    },
    KanaMapping {
        half: 'ﾀ',
        full: 'タ',
        voiced: Some('ダ'),
        semi_voiced: None,
    },
    KanaMapping {
        half: 'ﾁ',
        full: 'チ',
        voiced: Some('ヂ'),
        semi_voiced: None,
    },
    KanaMapping {
        half: 'ﾂ',
        full: 'ツ',
        voiced: Some('ヅ'),
        semi_voiced: None,
    },
    KanaMapping {
        half: 'ﾃ',
        full: 'テ',
        voiced: Some('デ'),
        semi_voiced: None,
    },
    KanaMapping {
        half: 'ﾄ',
        full: 'ト',
        voiced: Some('ド'),
        semi_voiced: None,
    },
    KanaMapping {
        half: 'ﾅ',
        full: 'ナ',
        voiced: None,
        semi_voiced: None,
    },
    KanaMapping {
        half: 'ﾆ',
        full: 'ニ',
        voiced: None,
        semi_voiced: None,
    },
    KanaMapping {
        half: 'ﾇ',
        full: 'ヌ',
        voiced: None,
        semi_voiced: None,
    },
    KanaMapping {
        half: 'ﾈ',
        full: 'ネ',
        voiced: None,
        semi_voiced: None,
    },
    KanaMapping {
        half: 'ﾉ',
        full: 'ノ',
        voiced: None,
        semi_voiced: None,
    },
    KanaMapping {
        half: 'ﾊ',
        full: 'ハ',
        voiced: Some('バ'),
        semi_voiced: Some('パ'),
    },
    KanaMapping {
        half: 'ﾋ',
        full: 'ヒ',
        voiced: Some('ビ'),
        semi_voiced: Some('ピ'),
    },
    KanaMapping {
        half: 'ﾌ',
        full: 'フ',
        voiced: Some('ブ'),
        semi_voiced: Some('プ'),
    },
    KanaMapping {
        half: 'ﾍ',
        full: 'ヘ',
        voiced: Some('ベ'),
        semi_voiced: Some('ペ'),
    },
    KanaMapping {
        half: 'ﾎ',
        full: 'ホ',
        voiced: Some('ボ'),
        semi_voiced: Some('ポ'),
    },
    KanaMapping {
        half: 'ﾏ',
        full: 'マ',
        voiced: None,
        semi_voiced: None,
    },
    KanaMapping {
        half: 'ﾐ',
        full: 'ミ',
        voiced: None,
        semi_voiced: None,
    },
    KanaMapping {
        half: 'ﾑ',
        full: 'ム',
        voiced: None,
        semi_voiced: None,
    },
    KanaMapping {
        half: 'ﾒ',
        full: 'メ',
        voiced: None,
        semi_voiced: None,
    },
    KanaMapping {
        half: 'ﾓ',
        full: 'モ',
        voiced: None,
        semi_voiced: None,
    },
    KanaMapping {
        half: 'ﾔ',
        full: 'ヤ',
        voiced: None,
        semi_voiced: None,
    },
    KanaMapping {
        half: 'ﾕ',
        full: 'ユ',
        voiced: None,
        semi_voiced: None,
    },
    KanaMapping {
        half: 'ﾖ',
        full: 'ヨ',
        voiced: None,
        semi_voiced: None,
    },
    KanaMapping {
        half: 'ﾗ',
        full: 'ラ',
        voiced: None,
        semi_voiced: None,
    },
    KanaMapping {
        half: 'ﾘ',
        full: 'リ',
        voiced: None,
        semi_voiced: None,
    },
    KanaMapping {
        half: 'ﾙ',
        full: 'ル',
        voiced: None,
        semi_voiced: None,
    },
    KanaMapping {
        half: 'ﾚ',
        full: 'レ',
        voiced: None,
        semi_voiced: None,
    },
    KanaMapping {
        half: 'ﾛ',
        full: 'ロ',
        voiced: None,
        semi_voiced: None,
    },
    KanaMapping {
        half: 'ﾜ',
        full: 'ワ',
        voiced: Some('ヷ'),
        semi_voiced: None,
    },
    KanaMapping {
        half: 'ﾝ',
        full: 'ン',
        voiced: None,
        semi_voiced: None,
    },
];

fn find_kana_by_half(ch: char) -> Option<&'static KanaMapping> {
    KANA_TABLE.iter().find(|entry| entry.half == ch)
}

fn find_kana_by_full(ch: char) -> Option<(&'static KanaMapping, Option<char>)> {
    for entry in KANA_TABLE {
        if entry.full == ch {
            return Some((entry, None));
        }
        if entry.voiced == Some(ch) {
            return Some((entry, Some('ﾞ')));
        }
        if entry.semi_voiced == Some(ch) {
            return Some((entry, Some('ﾟ')));
        }
    }
    None
}

fn find_width_correction(grapheme: &str, mode: WidthMode) -> Option<usize> {
    WIDTH_CORRECTIONS
        .iter()
        .find(|entry| entry.mode == mode && entry.sequence == grapheme)
        .map(|entry| entry.corrected_width)
}

fn parse_width_corrections() -> Vec<WidthCorrection> {
    WIDTH_CORRECTIONS_CSV
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                return None;
            }
            let mut parts = trimmed.split(',');
            let sequence = parts.next()?.trim();
            let mode_str = parts.next()?.trim();
            let width_str = parts.next()?.trim();
            let mode =
                WidthMode::from_str(mode_str).expect("invalid mode in width_corrections.csv");
            let corrected_width = width_str
                .parse::<usize>()
                .expect("invalid width in width_corrections.csv");
            Some(WidthCorrection {
                sequence: sequence.to_string(),
                mode,
                corrected_width,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::Str;

    #[test]
    fn converts_ascii_between_modes() {
        let src = Str::from("Hello World");
        let (wide, _) = width_map_with_stats(&src, WidthMode::Wide);
        assert_eq!(wide.as_str(), "Ｈｅｌｌｏ　Ｗｏｒｌｄ");
        let (narrow, _) = width_map_with_stats(&Str::from(wide.as_str()), WidthMode::Narrow);
        assert_eq!(narrow.as_str(), "Hello World");
    }

    #[test]
    fn converts_katakana_half_and_full() {
        let src = Str::from("ﾊﾝｶｸｶﾅ");
        let (wide, _) = width_map_with_stats(&src, WidthMode::Wide);
        assert_eq!(wide.as_str(), "ハンカクカナ");
        let (back, _) = width_map_with_stats(&Str::from("ガギグゲゴ"), WidthMode::Narrow);
        assert_eq!(back.as_str(), "ｶﾞｷﾞｸﾞｹﾞｺﾞ");
    }

    #[test]
    fn emoji_mode_counts_corrections() {
        let src = Str::from("👨‍👩‍👧‍👦");
        let (_, stats) = width_map_with_stats(&src, WidthMode::EmojiCompat);
        assert_eq!(stats.corrections_applied, 1);
        assert_eq!(stats.corrected_width, 4);
    }
}
