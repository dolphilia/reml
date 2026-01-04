use unicode_normalization::{is_nfc, is_nfd, is_nfkc, is_nfkd, UnicodeNormalization};

use super::{effects, Str, String as TextString, UnicodeResult};

#[derive(Debug, Clone, Copy)]
pub enum NormalizationForm {
    Nfc,
    Nfd,
    Nfkc,
    Nfkd,
}

pub fn normalize(string: TextString, form: NormalizationForm) -> UnicodeResult<TextString> {
    let owned = string.into_std();
    if is_normalized_str(owned.as_str(), form) {
        return Ok(TextString::from_std(owned));
    }
    let normalized = match form {
        NormalizationForm::Nfc => owned.nfc().collect::<std::string::String>(),
        NormalizationForm::Nfd => owned.nfd().collect::<std::string::String>(),
        NormalizationForm::Nfkc => owned.nfkc().collect::<std::string::String>(),
        NormalizationForm::Nfkd => owned.nfkd().collect::<std::string::String>(),
    };
    effects::record_mem_copy(normalized.len());
    Ok(TextString::from_std(normalized))
}

pub fn is_normalized(str_ref: &Str<'_>, form: NormalizationForm) -> bool {
    is_normalized_str(str_ref.as_str(), form)
}

fn is_normalized_str(value: &str, form: NormalizationForm) -> bool {
    match form {
        NormalizationForm::Nfc => is_nfc(value),
        NormalizationForm::Nfd => is_nfd(value),
        NormalizationForm::Nfkc => is_nfkc(value),
        NormalizationForm::Nfkd => is_nfkd(value),
    }
}
