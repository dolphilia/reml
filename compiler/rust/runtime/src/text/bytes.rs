use std::ops::Range;

use super::{effects, Str, String as TextString, UnicodeError, UnicodeErrorKind, UnicodeResult};

/// UTF-8 バイト列の所有権ラッパー。
/// 仕様では IO/圧縮との境界を担うため、ここでは `Vec<u8>` を薄く包む。
#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct Bytes {
    data: Vec<u8>,
}

impl Bytes {
    /// `Vec<u8>` をそのまま受け取りラップする。
    /// 将来的には effect {mem} と UTF-8 バリデーションを組み込む。
    pub fn from_vec(vec: Vec<u8>) -> UnicodeResult<Self> {
        effects::record_transfer();
        Ok(Self { data: vec })
    }

    /// バイト列をコピーして構築するヘルパ。IO 層以外でも手軽に使える。
    pub fn from_slice(slice: &[u8]) -> Self {
        Self::from_slice_internal(slice, true)
    }

    /// 効果計測を省略した `from_slice`。内部利用専用。
    pub(crate) fn from_slice_untracked(slice: &[u8]) -> Self {
        Self::from_slice_internal(slice, false)
    }

    /// 所有権ごと `Vec<u8>` を取り出す。
    pub fn into_vec(self) -> Vec<u8> {
        self.data
    }

    /// 不変参照を返す。`Str::as_bytes` と同じくゼロコピー経路を提供する。
    pub fn as_slice(&self) -> &[u8] {
        &self.data
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// 指定した範囲を切り出して新しい `Bytes` を返す。
    pub fn slice(&self, range: Range<usize>) -> UnicodeResult<Self> {
        if range.end > self.data.len() || range.start > range.end {
            return Err(UnicodeError::new(
                UnicodeErrorKind::InvalidRange,
                "bytes slice out of bounds",
            ));
        }
        let view = &self.data[range.start..range.end];
        Ok(Self::from_slice_internal(view, true))
    }

    /// UTF-8 として解釈した `Str` を返す。仕様準拠の `DecodeError` へ繋ぐ予定。
    pub fn decode_utf8(&self) -> UnicodeResult<Str<'_>> {
        match std::str::from_utf8(&self.data) {
            Ok(s) => Ok(Str::from(s)),
            Err(error) => Err(UnicodeError::invalid_utf8(error.valid_up_to())),
        }
    }

    /// 所有権ごと UTF-8 文字列化する。`Str<'static>` を返すため長期保管に向く。
    pub fn into_utf8(self) -> UnicodeResult<Str<'static>> {
        match std::string::String::from_utf8(self.data) {
            Ok(string) => Ok(Str::owned(string)),
            Err(error) => Err(UnicodeError::invalid_utf8(error.utf8_error().valid_up_to())),
        }
    }

    /// 所有権を `Core.Text::String` に移すショートカット。
    pub fn into_string(self) -> UnicodeResult<TextString> {
        self.into_utf8().map(|s| s.into_owned())
    }

    fn from_slice_internal(slice: &[u8], track_effects: bool) -> Self {
        if track_effects {
            effects::record_mem_copy(slice.len());
        }
        Self {
            data: slice.to_vec(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::effects;

    #[test]
    fn from_slice_records_mem_effects() {
        effects::take_recorded_effects();
        let _ = Bytes::from_slice(b"hello");
        let effects = effects::take_recorded_effects();
        assert!(effects.contains_mem());
        assert_eq!(effects.mem_bytes(), 5);
    }

    #[test]
    fn slice_records_mem_effects() {
        effects::take_recorded_effects();
        let bytes = Bytes::from_slice(b"abcdef");
        effects::take_recorded_effects();
        let _ = bytes.slice(1..4).expect("slice ok");
        let effects = effects::take_recorded_effects();
        assert!(effects.contains_mem());
        assert_eq!(effects.mem_bytes(), 3);
    }

    #[test]
    fn from_vec_records_transfer_effect() {
        effects::take_recorded_effects();
        let _ = Bytes::from_vec(vec![1, 2, 3]).expect("from_vec ok");
        let effects = effects::take_recorded_effects();
        assert!(effects.contains_transfer());
    }
}
