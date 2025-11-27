use crate::io::IoError;

/// Unicode 関連エラー。仕様の `UnicodeError` 相当。
#[derive(Debug, Clone)]
pub struct UnicodeError {
    kind: UnicodeErrorKind,
    message: String,
    offset: Option<usize>,
    phase: &'static str,
    source: Option<IoError>,
}

impl UnicodeError {
    pub fn new(kind: UnicodeErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            offset: None,
            phase: "unicode",
            source: None,
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

    pub fn with_phase(mut self, phase: &'static str) -> Self {
        self.phase = phase;
        self
    }

    pub fn with_source(mut self, source: IoError) -> Self {
        self.source = Some(source);
        self
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

    pub fn phase(&self) -> &'static str {
        self.phase
    }

    pub fn source(&self) -> Option<&IoError> {
        self.source.as_ref()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnicodeErrorKind {
    InvalidUtf8,
    UnsupportedScalar,
    UnsupportedLocale,
    InvalidIdentifier,
    InvalidRange,
    DecodeFailure,
    EncodeFailure,
}

#[derive(Debug, Clone, Copy)]
pub struct UnicodeEffectInfo {
    pub effect_mem: bool,
    pub effect_unicode: bool,
}

pub type UnicodeResult<T> = Result<T, UnicodeError>;
