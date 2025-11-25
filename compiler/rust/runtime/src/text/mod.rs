//! Core.Text/Unicode 用のプレースホルダーモジュール。
//! まずは Bytes/Str/String/UnicodeError のラッパのみを提供し、
//! フェーズ 3 の実装タスク着手に必要な足場を整える。

mod bytes;
mod error;
mod grapheme;
mod str_ref;
mod text_string;

pub use bytes::Bytes;
pub use error::{UnicodeError, UnicodeErrorKind, UnicodeResult};
pub use grapheme::{
  grapheme_stats,
  segment_graphemes,
  GraphemeCluster,
  GraphemeSeq,
  GraphemeStats,
};
pub use str_ref::Str;
pub use text_string::String;
