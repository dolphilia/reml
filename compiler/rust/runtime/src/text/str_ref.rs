use std::borrow::Cow;

use super::{effects, Bytes, GraphemeIter, UnicodeResult};

/// UTF-8 スライスを表す参照型。仕様上の `Str` に相当する。
#[derive(Clone, Debug)]
pub struct Str<'a> {
    inner: Cow<'a, str>,
}

impl<'a> Str<'a> {
    pub fn from_cow(inner: Cow<'a, str>) -> Self {
        Self { inner }
    }

    pub fn as_str(&self) -> &str {
        &self.inner
    }

    pub fn len_bytes(&self) -> usize {
        self.inner.as_bytes().len()
    }

    pub fn to_bytes(&self) -> Bytes {
        effects::record_mem_copy(self.len_bytes());
        Bytes::from_slice_untracked(self.inner.as_bytes())
    }

    pub fn into_owned(self) -> super::String {
        super::String::from_std(self.inner.into_owned())
    }

    /// Unicode 拡張書記素クラスター単位でのイテレータを返す。
    pub fn iter_graphemes(&self) -> GraphemeIter<'_> {
        GraphemeIter::new(self.as_str())
    }

    pub fn from_bytes(bytes: Bytes) -> UnicodeResult<Str<'static>> {
        bytes.into_utf8()
    }
}

impl<'a> From<&'a str> for Str<'a> {
    fn from(value: &'a str) -> Self {
        Self::from_cow(Cow::Borrowed(value))
    }
}

impl Str<'static> {
    pub fn owned(value: std::string::String) -> Self {
        Str::from_cow(Cow::Owned(value))
    }
}

impl From<std::string::String> for Str<'static> {
    fn from(value: std::string::String) -> Self {
        Str::owned(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::effects;

    #[test]
    fn to_bytes_records_mem_effects_once() {
        effects::take_recorded_effects();
        let s = Str::from("abc");
        let _ = s.to_bytes();
        let effects = effects::take_recorded_effects();
        assert!(effects.contains_mem());
        assert_eq!(effects.mem_bytes(), 3);
    }
}
