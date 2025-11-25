use unicode_normalization::{is_nfc, is_nfd, is_nfkc, is_nfkd, UnicodeNormalization};

use super::{Str, String as TextString, UnicodeResult};

#[derive(Debug, Clone, Copy)]
pub enum NormalizationForm {
  Nfc,
  Nfd,
  Nfkc,
  Nfkd,
}

pub fn normalize(string: TextString, form: NormalizationForm) -> UnicodeResult<TextString> {
  let normalized = match form {
    NormalizationForm::Nfc => string.into_std().nfc().collect::<std::string::String>(),
    NormalizationForm::Nfd => string.into_std().nfd().collect::<std::string::String>(),
    NormalizationForm::Nfkc => string.into_std().nfkc().collect::<std::string::String>(),
    NormalizationForm::Nfkd => string.into_std().nfkd().collect::<std::string::String>(),
  };
  Ok(TextString::from_std(normalized))
}

pub fn is_normalized(str_ref: &Str<'_>, form: NormalizationForm) -> bool {
  match form {
    NormalizationForm::Nfc => is_nfc(str_ref.as_str()),
    NormalizationForm::Nfd => is_nfd(str_ref.as_str()),
    NormalizationForm::Nfkc => is_nfkc(str_ref.as_str()),
    NormalizationForm::Nfkd => is_nfkd(str_ref.as_str()),
  }
}
