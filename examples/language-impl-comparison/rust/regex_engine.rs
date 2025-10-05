// 正規表現エンジン：パース + 評価の両方を実装。
//
// 対応する正規表現構文（簡易版）：
// - リテラル: `abc`
// - 連結: `ab`
// - 選択: `a|b`
// - 繰り返し: `a*`, `a+`, `a?`, `a{2,5}`
// - グループ: `(abc)`
// - 文字クラス: `[a-z]`, `[^0-9]`, `\d`, `\w`, `\s`
// - アンカー: `^`, `$`
// - ドット: `.` (任意の1文字)

use std::fmt;

// 正規表現のAST
#[derive(Debug, Clone)]
enum Regex {
    Literal(String),
    CharClass(CharSet),
    Dot,
    Concat(Vec<Regex>),
    Alternation(Vec<Regex>),
    Repeat(Box<Regex>, RepeatKind),
    Group(Box<Regex>),
    Anchor(AnchorKind),
}

#[derive(Debug, Clone)]
enum CharSet {
    CharRange(char, char),
    CharList(Vec<char>),
    Predefined(PredefinedClass),
    Negated(Box<CharSet>),
    Union(Vec<CharSet>),
}

#[derive(Debug, Clone)]
enum PredefinedClass {
    Digit,
    Word,
    Whitespace,
    NotDigit,
    NotWord,
    NotWhitespace,
}

#[derive(Debug, Clone)]
enum RepeatKind {
    ZeroOrMore,
    OneOrMore,
    ZeroOrOne,
    Exactly(usize),
    Range(usize, Option<usize>),
}

#[derive(Debug, Clone)]
enum AnchorKind {
    Start,
    End,
}

// パーサー型
type ParseResult<T> = Result<(T, String), String>;

// パーサーコンビネーター
fn ok<T>(value: T, rest: String) -> ParseResult<T> {
    Ok((value, rest))
}

fn fail<T>(message: &str) -> ParseResult<T> {
    Err(message.to_string())
}

fn choice<T>(parsers: Vec<fn(String) -> ParseResult<T>>, input: String) -> ParseResult<T> {
    for parser in parsers {
        if let Ok(result) = parser(input.clone()) {
            return Ok(result);
        }
    }
    fail("no choice matched")
}

fn many<T, F>(parser: F, input: String) -> ParseResult<Vec<T>>
where
    F: Fn(String) -> ParseResult<T>,
{
    let mut results = Vec::new();
    let mut current = input;

    loop {
        match parser(current.clone()) {
            Ok((value, rest)) => {
                results.push(value);
                current = rest;
            }
            Err(_) => break,
        }
    }

    ok(results, current)
}

fn many1<T, F>(parser: F, input: String) -> ParseResult<Vec<T>>
where
    F: Fn(String) -> ParseResult<T>,
{
    let (first, rest1) = parser(input)?;
    let (mut others, rest2) = many(&parser, rest1)?;
    let mut results = vec![first];
    results.append(&mut others);
    ok(results, rest2)
}

fn optional<T, F>(parser: F, input: String) -> ParseResult<Option<T>>
where
    F: Fn(String) -> ParseResult<T>,
{
    match parser(input.clone()) {
        Ok((value, rest)) => ok(Some(value), rest),
        Err(_) => ok(None, input),
    }
}

fn char(c: char, input: String) -> ParseResult<char> {
    if let Some(first) = input.chars().next() {
        if first == c {
            return ok(c, input[c.len_utf8()..].to_string());
        }
    }
    fail(&format!("expected {}", c))
}

fn string(s: &str, input: String) -> ParseResult<String> {
    if input.starts_with(s) {
        ok(s.to_string(), input[s.len()..].to_string())
    } else {
        fail(&format!("expected {}", s))
    }
}

fn satisfy<F>(pred: F, input: String) -> ParseResult<char>
where
    F: Fn(char) -> bool,
{
    if let Some(c) = input.chars().next() {
        if pred(c) {
            return ok(c, input[c.len_utf8()..].to_string());
        }
    }
    fail("predicate failed")
}

fn digit(input: String) -> ParseResult<char> {
    satisfy(|c| c.is_ascii_digit(), input)
}

fn integer(input: String) -> ParseResult<usize> {
    let (digits, rest) = many1(digit, input)?;
    let num: usize = digits.iter().collect::<String>().parse().unwrap_or(0);
    ok(num, rest)
}

fn sep_by1<T, S, F, G>(parser: F, sep: G, input: String) -> ParseResult<Vec<T>>
where
    F: Fn(String) -> ParseResult<T>,
    G: Fn(String) -> ParseResult<S>,
{
    let (first, rest1) = parser(input)?;
    let (mut others, rest2) = many(
        |inp: String| {
            let (_, rest) = sep(inp)?;
            parser(rest)
        },
        rest1,
    )?;
    let mut results = vec![first];
    results.append(&mut others);
    ok(results, rest2)
}

// 正規表現パーサー
fn parse_regex(input: &str) -> Result<Regex, String> {
    match regex_expr(input.to_string()) {
        Ok((regex, rest)) if rest.is_empty() => Ok(regex),
        Ok((_, rest)) => Err(format!("unexpected input: {}", rest)),
        Err(err) => Err(err),
    }
}

fn regex_expr(input: String) -> ParseResult<Regex> {
    alternation_expr(input)
}

fn alternation_expr(input: String) -> ParseResult<Regex> {
    let (alts, rest) = sep_by1(concat_expr, |inp| string("|", inp), input)?;
    if alts.len() == 1 {
        ok(alts[0].clone(), rest)
    } else {
        ok(Regex::Alternation(alts), rest)
    }
}

fn concat_expr(input: String) -> ParseResult<Regex> {
    let (terms, rest) = many1(postfix_term, input)?;
    if terms.len() == 1 {
        ok(terms[0].clone(), rest)
    } else {
        ok(Regex::Concat(terms), rest)
    }
}

fn postfix_term(input: String) -> ParseResult<Regex> {
    let (base, rest1) = atom(input)?;
    let (repeat_opt, rest2) = optional(repeat_suffix, rest1)?;

    match repeat_opt {
        Some(kind) => ok(Regex::Repeat(Box::new(base), kind), rest2),
        None => ok(base, rest2),
    }
}

fn atom(input: String) -> ParseResult<Regex> {
    // 括弧グループ
    if let Ok((_, rest1)) = string("(", input.clone()) {
        let (inner, rest2) = regex_expr(rest1)?;
        let (_, rest3) = string(")", rest2)?;
        return ok(Regex::Group(Box::new(inner)), rest3);
    }

    // アンカー
    if let Ok((_, rest)) = string("^", input.clone()) {
        return ok(Regex::Anchor(AnchorKind::Start), rest);
    }
    if let Ok((_, rest)) = string("$", input.clone()) {
        return ok(Regex::Anchor(AnchorKind::End), rest);
    }

    // ドット
    if let Ok((_, rest)) = string(".", input.clone()) {
        return ok(Regex::Dot, rest);
    }

    // 文字クラス
    if let Ok(result) = char_class(input.clone()) {
        return Ok(result);
    }

    // 定義済みクラス
    if let Ok(result) = predefined_class(input.clone()) {
        return Ok(result);
    }

    // エスケープ文字
    if let Ok(result) = escape_char(input.clone()) {
        return Ok(result);
    }

    // 通常のリテラル
    let (c, rest) = satisfy(
        |ch| !matches!(ch, '(' | ')' | '[' | ']' | '{' | '}' | '*' | '+' | '?' | '.' | '|' | '^' | '$' | '\\'),
        input,
    )?;
    ok(Regex::Literal(c.to_string()), rest)
}

fn escape_char(input: String) -> ParseResult<Regex> {
    let (_, rest1) = string("\\", input)?;
    let (c, rest2) = satisfy(
        |ch| matches!(ch, 'n' | 't' | 'r' | '\\' | '(' | ')' | '[' | ']' | '{' | '}' | '*' | '+' | '?' | '.' | '|' | '^' | '$'),
        rest1,
    )?;

    let lit = match c {
        'n' => "\n",
        't' => "\t",
        'r' => "\r",
        _ => return ok(Regex::Literal(c.to_string()), rest2),
    };
    ok(Regex::Literal(lit.to_string()), rest2)
}

fn predefined_class(input: String) -> ParseResult<Regex> {
    let (_, rest1) = string("\\", input)?;

    if let Ok((_, rest2)) = char('d', rest1.clone()) {
        return ok(Regex::CharClass(CharSet::Predefined(PredefinedClass::Digit)), rest2);
    }
    if let Ok((_, rest2)) = char('w', rest1.clone()) {
        return ok(Regex::CharClass(CharSet::Predefined(PredefinedClass::Word)), rest2);
    }
    if let Ok((_, rest2)) = char('s', rest1.clone()) {
        return ok(Regex::CharClass(CharSet::Predefined(PredefinedClass::Whitespace)), rest2);
    }
    if let Ok((_, rest2)) = char('D', rest1.clone()) {
        return ok(Regex::CharClass(CharSet::Predefined(PredefinedClass::NotDigit)), rest2);
    }
    if let Ok((_, rest2)) = char('W', rest1.clone()) {
        return ok(Regex::CharClass(CharSet::Predefined(PredefinedClass::NotWord)), rest2);
    }
    if let Ok((_, rest2)) = char('S', rest1.clone()) {
        return ok(Regex::CharClass(CharSet::Predefined(PredefinedClass::NotWhitespace)), rest2);
    }

    fail("expected predefined class")
}

fn char_class(input: String) -> ParseResult<Regex> {
    let (_, rest1) = string("[", input)?;
    let (negated, rest2) = optional(|inp| string("^", inp), rest1)?;
    let (items, rest3) = many1(char_class_item, rest2)?;
    let (_, rest4) = string("]", rest3)?;

    let union_set = CharSet::Union(items);
    let cs = if negated.is_some() {
        CharSet::Negated(Box::new(union_set))
    } else {
        union_set
    };

    ok(Regex::CharClass(cs), rest4)
}

fn char_class_item(input: String) -> ParseResult<CharSet> {
    let (start, rest1) = satisfy(|c| c != ']' && c != '-', input)?;
    let (end_opt, rest2) = optional(
        |inp: String| {
            let (_, r1) = string("-", inp)?;
            satisfy(|c| c != ']', r1)
        },
        rest1,
    )?;

    match end_opt {
        Some(end) => ok(CharSet::CharRange(start, end), rest2),
        None => ok(CharSet::CharList(vec![start]), rest2),
    }
}

fn repeat_suffix(input: String) -> ParseResult<RepeatKind> {
    if let Ok((_, rest)) = string("*", input.clone()) {
        return ok(RepeatKind::ZeroOrMore, rest);
    }
    if let Ok((_, rest)) = string("+", input.clone()) {
        return ok(RepeatKind::OneOrMore, rest);
    }
    if let Ok((_, rest)) = string("?", input.clone()) {
        return ok(RepeatKind::ZeroOrOne, rest);
    }

    // {n,m} 形式
    let (_, rest1) = string("{", input)?;
    let (n, rest2) = integer(rest1)?;
    let (range_opt, rest3) = optional(
        |inp: String| {
            let (_, r1) = string(",", inp)?;
            optional(integer, r1)
        },
        rest2,
    )?;
    let (_, rest4) = string("}", rest3)?;

    match range_opt {
        None => ok(RepeatKind::Exactly(n), rest4),
        Some(None) => ok(RepeatKind::Range(n, None), rest4),
        Some(Some(m)) => ok(RepeatKind::Range(n, Some(m)), rest4),
    }
}

// マッチングエンジン
fn match_regex(regex: &Regex, text: &str) -> bool {
    match_from_pos(regex, text, 0)
}

fn match_from_pos(regex: &Regex, text: &str, pos: usize) -> bool {
    match regex {
        Regex::Literal(s) => {
            if pos + s.len() <= text.len() {
                &text[pos..pos + s.len()] == s
            } else {
                false
            }
        }

        Regex::CharClass(cs) => {
            if pos < text.len() {
                if let Some(c) = text[pos..].chars().next() {
                    char_matches_class(c, cs)
                } else {
                    false
                }
            } else {
                false
            }
        }

        Regex::Dot => pos < text.len(),

        Regex::Concat(terms) => {
            let mut current_pos = pos;
            for term in terms {
                if match_from_pos(term, text, current_pos) {
                    current_pos += 1;
                } else {
                    return false;
                }
            }
            true
        }

        Regex::Alternation(alts) => alts.iter().any(|alt| match_from_pos(alt, text, pos)),

        Regex::Repeat(inner, kind) => match kind {
            RepeatKind::ZeroOrMore => match_repeat_loop(inner, text, pos, 0, 0, 999999),
            RepeatKind::OneOrMore => {
                if match_from_pos(inner, text, pos) {
                    match_repeat_loop(inner, text, pos + 1, 1, 1, 999999)
                } else {
                    false
                }
            }
            RepeatKind::ZeroOrOne => match_from_pos(inner, text, pos) || true,
            RepeatKind::Exactly(n) => match_repeat_loop(inner, text, pos, 0, *n, *n),
            RepeatKind::Range(min, max_opt) => {
                let max = max_opt.unwrap_or(999999);
                match_repeat_loop(inner, text, pos, 0, *min, max)
            }
        },

        Regex::Group(inner) => match_from_pos(inner, text, pos),

        Regex::Anchor(kind) => match kind {
            AnchorKind::Start => pos == 0,
            AnchorKind::End => pos >= text.len(),
        },
    }
}

fn char_matches_class(c: char, cs: &CharSet) -> bool {
    match cs {
        CharSet::CharRange(start, end) => c >= *start && c <= *end,

        CharSet::CharList(chars) => chars.contains(&c),

        CharSet::Predefined(cls) => match cls {
            PredefinedClass::Digit => c.is_ascii_digit(),
            PredefinedClass::Word => c.is_alphanumeric() || c == '_',
            PredefinedClass::Whitespace => c.is_whitespace(),
            PredefinedClass::NotDigit => !c.is_ascii_digit(),
            PredefinedClass::NotWord => !(c.is_alphanumeric() || c == '_'),
            PredefinedClass::NotWhitespace => !c.is_whitespace(),
        },

        CharSet::Negated(inner) => !char_matches_class(c, inner),

        CharSet::Union(sets) => sets.iter().any(|set| char_matches_class(c, set)),
    }
}

fn match_repeat_loop(inner: &Regex, text: &str, pos: usize, count: usize, min: usize, max: usize) -> bool {
    if count == max {
        true
    } else if count >= min && !match_from_pos(inner, text, pos) {
        true
    } else if match_from_pos(inner, text, pos) {
        match_repeat_loop(inner, text, pos + 1, count + 1, min, max)
    } else if count >= min {
        true
    } else {
        false
    }
}

// テスト例
fn test_examples() {
    let examples = vec![
        ("a+", "aaa", true),
        ("a+", "b", false),
        ("[0-9]+", "123", true),
        ("[0-9]+", "abc", false),
        ("\\d{2,4}", "12", true),
        ("\\d{2,4}", "12345", true),
        ("(abc)+", "abcabc", true),
        ("a|b", "a", true),
        ("a|b", "b", true),
        ("a|b", "c", false),
        ("^hello$", "hello", true),
        ("^hello$", "hello world", false),
    ];

    for (pattern, text, expected) in examples {
        match parse_regex(pattern) {
            Ok(regex) => {
                let result = match_regex(&regex, text);
                let status = if result == expected { "✓" } else { "✗" };
                println!(
                    "{} パターン: '{}', テキスト: '{}', 期待: {}, 結果: {}",
                    status, pattern, text, expected, result
                );
            }
            Err(err) => {
                println!("✗ パーサーエラー: {} - {}", pattern, err);
            }
        }
    }
}

fn main() {
    test_examples();
}