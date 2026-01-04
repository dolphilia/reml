use super::{effects, Bytes, Str, UnicodeResult};

/// 所有文字列。仕様上の `String` 名に揃える。
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct String {
    inner: std::string::String,
}

impl String {
    pub fn new() -> Self {
        Self {
            inner: std::string::String::new(),
        }
    }

    pub fn from_std(inner: std::string::String) -> Self {
        Self { inner }
    }

    pub fn as_str(&self) -> &str {
        &self.inner
    }

    pub fn from_str(value: &str) -> Self {
        effects::record_mem_copy(value.len());
        Self::from_std(value.to_owned())
    }

    pub fn push_str(&mut self, value: &str) {
        self.inner.push_str(value);
    }

    pub fn into_std(self) -> std::string::String {
        self.inner
    }

    pub fn to_bytes(&self) -> Bytes {
        Bytes::from_slice(self.inner.as_bytes())
    }

    pub fn into_bytes(self) -> UnicodeResult<Bytes> {
        effects::record_transfer();
        Bytes::from_vec(self.inner.into_bytes())
    }

    pub fn from_bytes(bytes: Bytes) -> UnicodeResult<Self> {
        bytes.into_utf8().map(|s| s.into_owned())
    }

    pub fn normalize(self, form: super::NormalizationForm) -> UnicodeResult<Self> {
        super::normalize(self, form)
    }
}

impl From<std::string::String> for String {
    fn from(value: std::string::String) -> Self {
        Self::from_std(value)
    }
}

impl From<&str> for String {
    fn from(value: &str) -> Self {
        Self::from_str(value)
    }
}

impl From<Str<'_>> for String {
    fn from(value: Str<'_>) -> Self {
        value.into_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::effects;

    #[test]
    fn from_str_records_mem_effects() {
        effects::take_recorded_effects();
        let _ = String::from_str("text");
        let effects = effects::take_recorded_effects();
        assert!(effects.contains_mem());
        assert_eq!(effects.mem_bytes(), 4);
    }

    #[test]
    fn into_bytes_records_transfer() {
        let string = String::from_str("text");
        effects::take_recorded_effects();
        let _ = string.into_bytes().expect("into_bytes ok");
        let effects = effects::take_recorded_effects();
        assert!(effects.contains_transfer());
    }
}
