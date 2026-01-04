use reml_runtime::text::{UnicodeError, UnicodeErrorKind};

/// Diagnostics/Parser で共有する Unicode エラーの付加情報。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnicodeDetail {
    kind: UnicodeErrorKind,
    offset: Option<u32>,
    phase: Option<String>,
    raw: Option<String>,
    locale: Option<String>,
    profile: Option<String>,
}

impl UnicodeDetail {
    /// `UnicodeError` から詳細情報を抽出する。
    pub fn from_error(error: &UnicodeError) -> Self {
        Self {
            kind: error.kind(),
            offset: error.offset().and_then(|value| u32::try_from(value).ok()),
            phase: Some(error.phase().to_string()),
            raw: None,
            locale: None,
            profile: None,
        }
    }

    pub fn new(kind: UnicodeErrorKind) -> Self {
        Self {
            kind,
            offset: None,
            phase: None,
            raw: None,
            locale: None,
            profile: None,
        }
    }

    pub fn with_phase(mut self, phase: impl Into<String>) -> Self {
        self.phase = Some(phase.into());
        self
    }

    pub fn with_raw(mut self, raw: impl Into<String>) -> Self {
        self.raw = Some(raw.into());
        self
    }

    pub fn with_locale(mut self, locale: impl Into<String>) -> Self {
        self.locale = Some(locale.into());
        self
    }

    pub fn with_profile(mut self, profile: impl Into<String>) -> Self {
        self.profile = Some(profile.into());
        self
    }

    pub fn with_offset(mut self, offset: Option<u32>) -> Self {
        self.offset = offset;
        self
    }

    pub fn kind(&self) -> UnicodeErrorKind {
        self.kind
    }

    pub fn kind_label(&self) -> &'static str {
        unicode_kind_label(self.kind)
    }

    pub fn offset(&self) -> Option<u32> {
        self.offset
    }

    pub fn phase(&self) -> &str {
        self.phase
            .as_deref()
            .unwrap_or_else(|| default_unicode_phase(self.kind))
    }

    pub fn raw(&self) -> Option<&str> {
        self.raw.as_deref()
    }

    pub fn locale(&self) -> Option<&str> {
        self.locale.as_deref()
    }

    pub fn profile(&self) -> Option<&str> {
        self.profile.as_deref()
    }
}

fn default_unicode_phase(kind: UnicodeErrorKind) -> &'static str {
    match kind {
        UnicodeErrorKind::InvalidIdentifier => "lex.identifier",
        UnicodeErrorKind::UnsupportedLocale => "lex.locale",
        _ => "unicode",
    }
}

fn unicode_kind_label(kind: UnicodeErrorKind) -> &'static str {
    match kind {
        UnicodeErrorKind::InvalidUtf8 => "invalid_utf8",
        UnicodeErrorKind::UnsupportedScalar => "unsupported_scalar",
        UnicodeErrorKind::UnsupportedLocale => "unsupported_locale",
        UnicodeErrorKind::InvalidIdentifier => "invalid_identifier",
        UnicodeErrorKind::InvalidRange => "invalid_range",
        UnicodeErrorKind::DecodeFailure => "decode_failure",
        UnicodeErrorKind::EncodeFailure => "encode_failure",
    }
}

pub fn unicode_diagnostic_code(kind: UnicodeErrorKind) -> &'static str {
    match kind {
        UnicodeErrorKind::InvalidUtf8 => "unicode.invalid_utf8",
        UnicodeErrorKind::UnsupportedScalar => "unicode.unsupported_scalar",
        UnicodeErrorKind::UnsupportedLocale => "unicode.unsupported_locale",
        UnicodeErrorKind::InvalidIdentifier => "unicode.invalid_identifier",
        UnicodeErrorKind::InvalidRange => "unicode.invalid_range",
        UnicodeErrorKind::DecodeFailure => "unicode.decode_failure",
        UnicodeErrorKind::EncodeFailure => "unicode.encode_failure",
    }
}
