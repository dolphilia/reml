//! Core.Text/Unicode 用のプレースホルダーモジュール。
//! まずは Bytes/Str/String/UnicodeError のラッパのみを提供し、
//! フェーズ 3 の実装タスク着手に必要な足場を整える。

mod builder;
mod bytes;
mod case;
mod effects;
mod error;
mod grapheme;
mod identifier;
mod locale;
mod normalize;
mod str_ref;
mod text_string;
mod width;

pub use builder::{builder, TextBuilder};
pub use bytes::Bytes;
pub use case::{to_lower, to_upper};
pub use error::{UnicodeEffectInfo, UnicodeError, UnicodeErrorKind, UnicodeResult};
pub use grapheme::{
    clear_grapheme_cache_for_tests, grapheme_stats, log_grapheme_stats, segment_graphemes,
    DirectionStats, Grapheme, GraphemeIter, GraphemeSeq, GraphemeStats, ScriptCategory,
    ScriptStats, TextDirection,
};
pub use identifier::{prepare_identifier, prepare_identifier_with_locale};
pub use locale::LocaleId;
pub use normalize::{is_normalized, normalize, NormalizationForm};
pub use str_ref::Str;
pub use text_string::String;
pub use width::{width_map, width_map_with_stats, WidthMapStats, WidthMode};
