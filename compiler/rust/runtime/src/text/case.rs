use super::{
    effects,
    locale::{ensure_locale_supported, LocaleId, LocaleScope},
    String as TextString, UnicodeResult,
};

pub fn to_upper(string: TextString, locale: &LocaleId) -> UnicodeResult<TextString> {
    map_case(string, locale, CaseOperation::Upper)
}

pub fn to_lower(string: TextString, locale: &LocaleId) -> UnicodeResult<TextString> {
    map_case(string, locale, CaseOperation::Lower)
}

enum CaseOperation {
    Upper,
    Lower,
}

fn map_case(
    string: TextString,
    locale: &LocaleId,
    operation: CaseOperation,
) -> UnicodeResult<TextString> {
    ensure_locale_supported(locale, LocaleScope::Case)?;
    let owned = string.into_std();
    let transformed = apply_case(owned.as_str(), locale, operation);
    if transformed == owned {
        return Ok(TextString::from_std(owned));
    }
    effects::record_mem_copy(transformed.len());
    Ok(TextString::from_std(transformed))
}

fn apply_case(input: &str, locale: &LocaleId, operation: CaseOperation) -> String {
    if locale.canonical().starts_with("tr") {
        return match operation {
            CaseOperation::Upper => map_turkish_upper(input),
            CaseOperation::Lower => map_turkish_lower(input),
        };
    }
    default_case(input, operation)
}

fn default_case(input: &str, operation: CaseOperation) -> String {
    let mut buffer = String::with_capacity(input.len());
    for ch in input.chars() {
        match operation {
            CaseOperation::Upper => buffer.extend(ch.to_uppercase()),
            CaseOperation::Lower => buffer.extend(ch.to_lowercase()),
        }
    }
    buffer
}

fn map_turkish_upper(input: &str) -> String {
    let mut buffer = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            'i' => buffer.push('İ'),
            'ı' => buffer.push('I'),
            _ => buffer.extend(ch.to_uppercase()),
        }
    }
    buffer
}

fn map_turkish_lower(input: &str) -> String {
    let mut buffer = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            'I' => buffer.push('ı'),
            'İ' => buffer.push('i'),
            _ => buffer.extend(ch.to_lowercase()),
        }
    }
    buffer
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_locale_uses_unicode_case_mapping() {
        let locale = LocaleId::und();
        let input = TextString::from_str("core πß");
        let upper = to_upper(input.clone(), &locale).expect("upper");
        assert_eq!(upper.as_str(), "CORE ΠSS");
        let lower = to_lower(upper, &locale).expect("lower");
        assert_eq!(lower.as_str(), "core πss");
    }

    #[test]
    fn turkish_locale_handles_dotted_i() {
        let locale = LocaleId::parse("tr-TR").expect("locale");
        let mut input = TextString::from_str("iıIİ");
        let upper = to_upper(input.clone(), &locale).expect("upper");
        assert_eq!(upper.as_str(), "İIIİ");
        input = TextString::from_str("Iİ");
        let lower = to_lower(input, &locale).expect("lower");
        assert_eq!(lower.as_str(), "ıi");
    }

    #[test]
    fn unsupported_locale_errors() {
        let locale = LocaleId::parse("az-Latn").expect("locale");
        let err = to_upper(TextString::from_str("test"), &locale).unwrap_err();
        assert_eq!(
            err.kind(),
            super::super::UnicodeErrorKind::UnsupportedLocale
        );
    }
}
