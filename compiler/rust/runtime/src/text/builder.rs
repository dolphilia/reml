use super::{Bytes, Str, String as TextString, UnicodeResult};
use crate::prelude::iter::EffectSet;

/// `TextBuilder` は複数段階でテキストを構築する可変バッファ。
pub struct TextBuilder {
    buffer: Vec<u8>,
    effects: EffectSet,
}

impl TextBuilder {
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            effects: EffectSet::PURE,
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(capacity),
            effects: EffectSet::PURE.with_mem().with_mem_bytes(capacity),
        }
    }

    pub fn reserve(&mut self, additional: usize) {
        if additional > 0 {
            self.record_allocation(additional);
        }
        self.buffer.reserve(additional);
    }

    pub fn push_bytes(&mut self, bytes: &Bytes) {
        let len = bytes.len();
        self.record_write(len);
        self.buffer.extend_from_slice(bytes.as_slice());
    }

    pub fn push_str(&mut self, value: &Str<'_>) {
        let len = value.len_bytes();
        self.record_write(len);
        self.buffer.extend_from_slice(value.as_str().as_bytes());
    }

    pub fn push_grapheme(&mut self, cluster: &str) {
        let len = cluster.as_bytes().len();
        self.record_write(len);
        self.buffer.extend_from_slice(cluster.as_bytes());
    }

    /// 現在の効果計測値を取得する。`EffectSet` は `Copy` のため軽量に参照できる。
    pub fn effects(&self) -> EffectSet {
        self.effects
    }

    /// 構築を完了して `TextString` と計測済み効果を返す。
    pub fn finish_with_effects(self) -> UnicodeResult<(TextString, EffectSet)> {
        #[cfg(debug_assertions)]
        let ptr = self.buffer.as_ptr();
        let mut effects = self.effects;
        effects.mark_transfer();
        let string = Bytes::from_vec(self.buffer)?.into_string()?;
        #[cfg(debug_assertions)]
        {
            debug_assert_eq!(
                string.as_str().as_ptr(),
                ptr,
                "TextBuilder::finish は Vec<u8> のアロケーションを再利用する必要があります"
            );
        }
        Ok((string, effects))
    }

    /// 互換 API。効果が不要な場合はこちらを利用する。
    pub fn finish(self) -> UnicodeResult<TextString> {
        self.finish_with_effects().map(|(string, _)| string)
    }

    fn record_write(&mut self, bytes: usize) {
        self.record_allocation(bytes);
        if bytes > 0 {
            self.effects.mark_mut();
        }
    }

    fn record_allocation(&mut self, bytes: usize) {
        if bytes == 0 {
            return;
        }
        self.effects.mark_mem();
        self.effects.record_mem_bytes(bytes);
    }
}

pub fn builder() -> TextBuilder {
    TextBuilder::new()
}

impl Default for TextBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_mem_bytes_on_pushes() {
        let mut builder = TextBuilder::new();
        builder.push_bytes(&Bytes::from_slice(b"ab"));
        builder.push_str(&Str::from("cd"));
        builder.push_grapheme("ef");
        let effects = builder.effects();
        assert_eq!(effects.mem_bytes(), 6);
    }

    #[test]
    fn finish_reuses_allocation() {
        let mut builder = TextBuilder::with_capacity(4);
        builder.push_str(&Str::from("test"));
        let ptr = builder.buffer.as_ptr();
        let len = builder.buffer.len();
        let (string, effects) = builder.finish_with_effects().expect("finish ok");
        assert_eq!(string.as_str(), "test");
        assert!(effects.mem_bytes() >= len);
        assert_eq!(string.as_str().as_ptr(), ptr);
        assert!(effects.contains_transfer());
    }
}
