//! Core.IO の土台実装。
//! Reader/Writer の薄いラッパと、Text ストリーミング API を公開する。

mod effects;
mod env;
mod error;
mod reader;
mod text_stream;
mod writer;

pub use env::{time_env_snapshot, TimeEnvSnapshot};
pub use error::{IoContext, IoError, IoErrorKind, IoResult};
pub use reader::Reader;
pub use text_stream::{
    decode_stream, encode_stream, BomHandling, InvalidSequenceStrategy, TextDecodeOptions,
    TextEncodeOptions,
};
pub use writer::Writer;
