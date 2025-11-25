/// Unicode 関連エラー。仕様の `UnicodeError` 相当。
#[derive(Debug, Clone)]
pub struct UnicodeError {
  kind: UnicodeErrorKind,
  message: String,
  offset: Option<usize>,
}

impl UnicodeError {
  pub fn new(kind: UnicodeErrorKind, message: impl Into<String>) -> Self {
    Self {
      kind,
      message: message.into(),
      offset: None,
    }
  }

  pub fn with_offset(mut self, offset: usize) -> Self {
    self.offset = Some(offset);
    self
  }

  pub fn invalid_utf8(offset: usize) -> Self {
    Self::new(
      UnicodeErrorKind::InvalidUtf8,
      "byte sequence is not valid UTF-8",
    )
    .with_offset(offset)
  }

  pub fn kind(&self) -> UnicodeErrorKind {
    self.kind
  }

  pub fn message(&self) -> &str {
    &self.message
  }

  pub fn offset(&self) -> Option<usize> {
    self.offset
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnicodeErrorKind {
  InvalidUtf8,
  UnsupportedScalar,
  UnsupportedLocale,
  InvalidRange,
  DecodeFailure,
  EncodeFailure,
}

pub type UnicodeResult<T> = Result<T, UnicodeError>;
