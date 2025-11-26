use std::str::FromStr;

use super::{UnicodeError, UnicodeErrorKind, UnicodeResult};

/// Core.Text で利用するロケール ID。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocaleId {
    canonical: String,
}

impl LocaleId {
    pub fn parse(raw: impl AsRef<str>) -> UnicodeResult<Self> {
        let canonical = canonicalize_locale(raw.as_ref())
            .ok_or_else(|| unsupported_locale("locale id is empty or invalid", None))?;
        Ok(Self { canonical })
    }

    pub fn canonical(&self) -> &str {
        &self.canonical
    }

    pub fn und() -> Self {
        Self {
            canonical: "und".to_string(),
        }
    }
}

impl Default for LocaleId {
    fn default() -> Self {
        LocaleId::und()
    }
}

impl FromStr for LocaleId {
    type Err = UnicodeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        LocaleId::parse(s)
    }
}

impl AsRef<str> for LocaleId {
    fn as_ref(&self) -> &str {
        self.canonical()
    }
}

pub(crate) fn ensure_locale_supported(locale: &LocaleId, scope: LocaleScope) -> UnicodeResult<()> {
    let Some(entry) = find_entry(locale.canonical()) else {
        return Err(unsupported_locale(
            &format!(
                "locale `{}` is not registered for {} operations",
                locale.canonical(),
                scope.as_str()
            ),
            None,
        ));
    };
    if !entry.scope.supports(scope) {
        return Err(unsupported_locale(
            &format!(
                "locale `{}` does not opt into {} operations",
                locale.canonical(),
                scope.as_str()
            ),
            entry.fallback,
        ));
    }
    if entry.status != LocaleSupportStatus::Supported {
        return Err(unsupported_locale(
            &format!(
                "locale `{}` is only {} for {} operations",
                locale.canonical(),
                entry.status,
                scope.as_str()
            ),
            entry.fallback,
        ));
    }
    Ok(())
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LocaleScope {
    Case,
    Width,
}

impl LocaleScope {
    fn as_str(&self) -> &'static str {
        match self {
            LocaleScope::Case => "case",
            LocaleScope::Width => "width",
        }
    }
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LocaleSupportStatus {
    Supported,
    Partial,
    Planned,
}

impl std::fmt::Display for LocaleSupportStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LocaleSupportStatus::Supported => write!(f, "supported"),
            LocaleSupportStatus::Partial => write!(f, "partial"),
            LocaleSupportStatus::Planned => write!(f, "planned"),
        }
    }
}

struct LocaleEntry {
    id: &'static str,
    scope: LocaleScopeSet,
    status: LocaleSupportStatus,
    fallback: Option<&'static str>,
}

#[derive(Clone, Copy)]
struct LocaleScopeSet {
    case: bool,
    width: bool,
}

impl LocaleScopeSet {
    const fn case() -> Self {
        Self {
            case: true,
            width: false,
        }
    }

    const fn width() -> Self {
        Self {
            case: false,
            width: true,
        }
    }

    const fn both() -> Self {
        Self {
            case: true,
            width: true,
        }
    }

    fn supports(&self, scope: LocaleScope) -> bool {
        match scope {
            LocaleScope::Case => self.case,
            LocaleScope::Width => self.width,
        }
    }
}

const LOCALE_TABLE: &[LocaleEntry] = &[
    LocaleEntry {
        id: "und",
        scope: LocaleScopeSet::both(),
        status: LocaleSupportStatus::Supported,
        fallback: None,
    },
    LocaleEntry {
        id: "ja-JP",
        scope: LocaleScopeSet::both(),
        status: LocaleSupportStatus::Supported,
        fallback: Some("und"),
    },
    LocaleEntry {
        id: "tr-TR",
        scope: LocaleScopeSet::case(),
        status: LocaleSupportStatus::Supported,
        fallback: Some("und"),
    },
    LocaleEntry {
        id: "az-Latn",
        scope: LocaleScopeSet::case(),
        status: LocaleSupportStatus::Planned,
        fallback: Some("tr-TR"),
    },
    LocaleEntry {
        id: "zh-TW",
        scope: LocaleScopeSet::width(),
        status: LocaleSupportStatus::Planned,
        fallback: Some("ja-JP"),
    },
];

fn find_entry(id: &str) -> Option<&'static LocaleEntry> {
    LOCALE_TABLE
        .iter()
        .find(|entry| entry.id.eq_ignore_ascii_case(id))
}

fn canonicalize_locale(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let mut parts = Vec::new();
    for (index, part) in trimmed.split('-').filter(|seg| !seg.is_empty()).enumerate() {
        let canonical = if index == 0 {
            let lower = part.to_ascii_lowercase();
            if !(2..=8).contains(&lower.len()) || !lower.chars().all(|c| c.is_ascii_alphabetic()) {
                return None;
            }
            lower
        } else if part.len() == 2 && part.chars().all(|c| c.is_ascii_alphabetic()) {
            part.to_ascii_uppercase()
        } else if part.len() == 4 && part.chars().all(|c| c.is_ascii_alphabetic()) {
            let mut chars = part.chars();
            let mut script = String::new();
            if let Some(first) = chars.next() {
                script.push(first.to_ascii_uppercase());
            }
            script.extend(chars.map(|ch| ch.to_ascii_lowercase()));
            script
        } else if part.chars().all(|c| c.is_ascii_digit()) {
            part.to_string()
        } else {
            part.to_ascii_lowercase()
        };
        parts.push(canonical);
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("-"))
    }
}

fn unsupported_locale(message: &str, fallback: Option<&'static str>) -> UnicodeError {
    let mut full_message = String::from(message);
    if let Some(fallback) = fallback {
        full_message.push_str(&format!(" (try `{fallback}` instead)"));
    }
    UnicodeError::new(UnicodeErrorKind::UnsupportedLocale, full_message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonicalizes_basic_ids() {
        let id = LocaleId::parse("  JA-jp ").expect("locale parse");
        assert_eq!(id.canonical(), "ja-JP");
    }

    #[test]
    fn rejects_invalid_ids() {
        assert!(LocaleId::parse("").is_err());
        assert!(LocaleId::parse("1").is_err());
    }

    #[test]
    fn errors_for_planned_locale() {
        let id = LocaleId::parse("az-Latn").expect("locale parse");
        let err = ensure_locale_supported(&id, LocaleScope::Case).unwrap_err();
        assert_eq!(err.kind(), UnicodeErrorKind::UnsupportedLocale);
        assert!(err.message().contains("partial") || err.message().contains("planned"));
    }
}
