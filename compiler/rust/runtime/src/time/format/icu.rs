use super::{TimeError, TimeResult};
use crate::text::LocaleId;
use std::iter::Peekable;
use std::str::Chars;

pub(crate) fn resolve_custom_pattern(pattern: &str, locale: &LocaleId) -> TimeResult<String> {
    if pattern.contains('[') {
        return Ok(pattern.to_string());
    }
    translate_icu_pattern(pattern, locale)
}

fn translate_icu_pattern(pattern: &str, locale: &LocaleId) -> TimeResult<String> {
    let mut output = String::new();
    let mut chars = pattern.chars().peekable();
    while let Some(&ch) = chars.peek() {
        if ch == '\'' {
            consume_literal(&mut chars, &mut output)?;
        } else if ch.is_ascii_alphabetic() {
            consume_component(&mut chars, &mut output, locale)?;
        } else {
            output.push(ch);
            chars.next();
        }
    }
    Ok(output)
}

fn consume_literal(chars: &mut Peekable<Chars<'_>>, output: &mut String) -> TimeResult<()> {
    chars.next(); // opening quote
    let mut literal = String::new();
    while let Some(ch) = chars.next() {
        if ch == '\'' {
            if let Some('\'') = chars.peek() {
                chars.next();
                literal.push('\'');
                continue;
            }
            break;
        }
        literal.push(ch);
    }
    output.push_str(&literal);
    Ok(())
}

fn consume_component(
    chars: &mut Peekable<Chars<'_>>,
    output: &mut String,
    locale: &LocaleId,
) -> TimeResult<()> {
    let ch = chars.next().expect("component start");
    let mut len = 1;
    while matches!(chars.peek(), Some(&next) if next == ch) {
        chars.next();
        len += 1;
    }
    let component = build_component(ch, len, locale)?;
    output.push_str(&component);
    Ok(())
}

fn build_component(ch: char, len: usize, locale: &LocaleId) -> TimeResult<String> {
    let component = match ch {
        'y' => match len {
            2 => "[year repr:last_two_digits]".to_string(),
            _ => "[year repr:full]".to_string(),
        },
        'M' => match len {
            1 => "[month repr:numerical padding:none]".to_string(),
            2 => "[month repr:numerical padding:zero]".to_string(),
            _ => {
                return Err(icu_translation_error(
                    "textual month patterns are not supported",
                    ch,
                    len,
                    locale,
                ))
            }
        },
        'd' => match len {
            1 => "[day padding:none]".to_string(),
            _ => "[day padding:zero]".to_string(),
        },
        'H' => match len {
            1 => "[hour repr:24 padding:none]".to_string(),
            _ => "[hour repr:24 padding:zero]".to_string(),
        },
        'm' => match len {
            1 => "[minute padding:none]".to_string(),
            _ => "[minute padding:zero]".to_string(),
        },
        's' => match len {
            1 => "[second padding:none]".to_string(),
            _ => "[second padding:zero]".to_string(),
        },
        'S' => {
            let digits = len.min(9);
            format!("[subsecond digits:{digits}]")
        }
        other => {
            return Err(icu_translation_error(
                "unsupported ICU token",
                other,
                len,
                locale,
            ))
        }
    };
    Ok(component)
}

fn icu_translation_error(reason: &str, token: char, len: usize, locale: &LocaleId) -> TimeError {
    TimeError::invalid_format(format!(
        "{reason}: `{token}` (length {len}) cannot be translated to Core.Time pattern"
    ))
    .with_locale(locale.canonical().to_string())
}
