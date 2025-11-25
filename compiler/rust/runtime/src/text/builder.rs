use super::{Bytes, Str, String as TextString, UnicodeResult};

/// `TextBuilder` は複数段階でテキストを構築する可変バッファ。
#[derive(Default)]
pub struct TextBuilder {
  buffer: Vec<u8>,
}

impl TextBuilder {
  pub fn new() -> Self {
    Self { buffer: Vec::new() }
  }

  pub fn with_capacity(capacity: usize) -> Self {
    Self {
      buffer: Vec::with_capacity(capacity),
    }
  }

  pub fn reserve(&mut self, additional: usize) {
    self.buffer.reserve(additional);
  }

  pub fn push_bytes(&mut self, bytes: &Bytes) {
    self.buffer.extend_from_slice(bytes.as_slice());
  }

  pub fn push_str(&mut self, value: &Str<'_>) {
    self.buffer.extend_from_slice(value.as_str().as_bytes());
  }

  pub fn push_grapheme(&mut self, cluster: &str) {
    self.buffer.extend_from_slice(cluster.as_bytes());
  }

  pub fn finish(self) -> UnicodeResult<TextString> {
    Bytes::from_vec(self.buffer)?.into_string()
  }
}

pub fn builder() -> TextBuilder {
  TextBuilder::new()
}
