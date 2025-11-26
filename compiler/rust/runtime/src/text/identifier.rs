use unicode_normalization::is_nfc;

use super::{
    locale::{ensure_locale_supported, LocaleId, LocaleScope},
    Str, String as TextString, UnicodeError, UnicodeErrorKind, UnicodeResult,
};

const FORBIDDEN_BIDI_RANGES: &[(u32, u32)] = &[
    (0x200E, 0x200F), // LRM, RLM
    (0x202A, 0x202E), // LRE..RLO
    (0x2066, 0x2069), // LRI..PDI
];

/// `prepare_identifier` の既定実装。ロケールは `und` 扱い。
pub fn prepare_identifier(str_ref: &Str<'_>) -> UnicodeResult<TextString> {
    prepare_identifier_with_locale(str_ref, None)
}

/// ロケール指定付き `prepare_identifier`。
pub fn prepare_identifier_with_locale(
    str_ref: &Str<'_>,
    locale: Option<&LocaleId>,
) -> UnicodeResult<TextString> {
    if let Some(locale) = locale {
        ensure_locale_supported(locale, LocaleScope::Case)?;
    }
    let value = str_ref.as_str();
    if !is_nfc(value) {
        return Err(UnicodeError::new(
            UnicodeErrorKind::InvalidIdentifier,
            "identifier must be NFC normalized",
        )
        .with_phase("lex"));
    }
    if let Some((offset, ch)) = value.char_indices().find(|(_, ch)| is_forbidden_bidi(*ch)) {
        return Err(UnicodeError::new(
            UnicodeErrorKind::InvalidIdentifier,
            format!(
                "identifier contains forbidden bidi control U+{:04X}",
                ch as u32
            ),
        )
        .with_offset(offset));
    }
    Ok(TextString::from_str(value))
}

fn is_forbidden_bidi(ch: char) -> bool {
    let code = ch as u32;
    FORBIDDEN_BIDI_RANGES.iter().any(|(start, end)| {
        let start = *start;
        let end = *end;
        (start..=end).contains(&code)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use unicode_normalization::UnicodeNormalization;

    #[test]
    fn rejects_non_nfc_identifiers() {
        let raw = "cafe\u{0301}";
        let str_ref = Str::from(raw);
        let err = prepare_identifier(&str_ref).unwrap_err();
        assert_eq!(err.kind(), UnicodeErrorKind::InvalidIdentifier);
    }

    #[test]
    fn rejects_bidi_controls() {
        let raw = format!("foo\u{202E}bar");
        let str_ref = Str::from(raw.as_str());
        let err = prepare_identifier(&str_ref).unwrap_err();
        assert_eq!(err.kind(), UnicodeErrorKind::InvalidIdentifier);
        assert_eq!(err.offset(), Some(3));
    }

    #[test]
    fn accepts_normalized_identifier() {
        let raw = "ユニコード";
        let str_ref = Str::from(raw);
        let prepared = prepare_identifier(&str_ref).expect("identifier");
        assert_eq!(prepared.as_str(), raw.nfc().collect::<String>());
    }

    #[test]
    fn locale_check_runs_before_conversion() {
        let raw = "foo";
        let str_ref = Str::from(raw);
        let locale = LocaleId::parse("az-Latn").expect("locale");
        let err = prepare_identifier_with_locale(&str_ref, Some(&locale)).unwrap_err();
        assert_eq!(err.kind(), UnicodeErrorKind::UnsupportedLocale);
    }
}
