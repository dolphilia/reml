//! Core.Text/Unicode 用のプレースホルダーモジュール。
//! まずは Bytes/Str/String/UnicodeError のラッパのみを提供し、
//! フェーズ 3 の実装タスク着手に必要な足場を整える。

mod builder;
mod bytes;
mod case;
mod diagnostics;
mod effects;
mod error;
mod grapheme;
mod identifier;
mod locale;
mod normalize;
mod pretty;
mod span_highlight;
mod str_ref;
mod text_string;
mod width;

use crate::prelude::iter::{EffectLabels, EffectSet};
use serde_json::{Map as JsonMap, Value};

pub use crate::io::{
    decode_stream, encode_stream, BomHandling, InvalidSequenceStrategy, TextDecodeOptions,
    TextEncodeOptions,
};
pub use builder::{builder, TextBuilder};
pub use bytes::Bytes;
pub use case::{to_lower, to_upper};
pub use diagnostics::{grapheme_stats_metadata, insert_grapheme_stats_metadata};
pub use error::{UnicodeEffectInfo, UnicodeError, UnicodeErrorKind, UnicodeResult};
pub use grapheme::{
    clear_grapheme_cache_for_tests, grapheme_stats, log_grapheme_stats, segment_graphemes,
    DirectionStats, Grapheme, GraphemeIter, GraphemeSeq, GraphemeStats, ScriptCategory,
    ScriptStats, TextDirection,
};
pub use identifier::{prepare_identifier, prepare_identifier_with_locale};
pub use locale::LocaleId;
pub use normalize::{is_normalized, normalize, NormalizationForm};
pub use pretty::{
    concat, cst_doc, cst_printer, group, line, nest, render, softline, text, CstPrinter, Doc,
};
pub use span_highlight::{span_highlight, SpanHighlight};
pub use str_ref::Str;
pub use text_string::String;
pub use width::{width_map, width_map_with_stats, WidthMapStats, WidthMode};

/// Text API が記録した効果を取得し、観測用にリセットする。
pub fn take_text_effects_snapshot() -> EffectLabels {
    effects::take_recorded_effects().to_labels()
}

pub(crate) fn record_text_mem_copy(bytes: usize) {
    effects::record_mem_copy(bytes);
}

pub(crate) fn record_text_unicode_event(bytes: usize) {
    effects::record_unicode_event(bytes);
}

pub(crate) fn merge_text_effects(effects: EffectSet) {
    effects::merge_effects(effects);
}

pub(crate) fn take_text_audit_metadata() -> Option<JsonMap<std::string::String, Value>> {
    effects::take_audit_metadata_payload()
}

#[doc(hidden)]
pub fn take_text_audit_metadata_for_tests() {
    effects::drain_audit_metadata_for_tests();
}
