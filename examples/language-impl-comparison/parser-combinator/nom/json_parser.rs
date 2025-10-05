use std::collections::BTreeMap;

use nom::branch::alt;
use nom::bytes::complete::{escaped_transform, is_not, tag, take_while_m_n};
use nom::character::complete::{char, multispace0};
use nom::combinator::{cut, map, map_res, value};
use nom::multi::{separated_list0};
use nom::number::complete::double;
use nom::sequence::{delimited, preceded, separated_pair};
use nom::IResult;

#[derive(Debug, PartialEq)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<JsonValue>),
    Object(BTreeMap<String, JsonValue>),
}

pub fn parse_json(input: &str) -> Result<JsonValue, String> {
    let parser = preceded(multispace0, json_value);
    match parser(input) {
        Ok((rest, value)) if rest.trim().is_empty() => Ok(value),
        Ok((_rest, _)) => Err("未消費文字が残っています".into()),
        Err(err) => Err(format!("解析に失敗しました: {err}")),
    }
}

fn json_value(input: &str) -> IResult<&str, JsonValue> {
    alt((
        value(JsonValue::Null, tag("null")),
        map(tag("true"), |_| JsonValue::Bool(true)),
        map(tag("false"), |_| JsonValue::Bool(false)),
        map(json_string, JsonValue::String),
        map(double, JsonValue::Number),
        json_array,
        json_object,
    ))(input)
}

fn json_string(input: &str) -> IResult<&str, String> {
    let inner = escaped_transform(is_not("\\\""), '\\', escape_sequence);
    delimited(char('"'), cut(inner), char('"'))(input)
}

fn escape_sequence(input: &str) -> IResult<&str, String> {
    alt((
        value("\"".into(), char('"')),
        value("\\".into(), char('\\')),
        value("/".into(), char('/')),
        value("\u{0008}".into(), char('b')),
        value("\u{000C}".into(), char('f')),
        value("\n".into(), char('n')),
        value("\r".into(), char('r')),
        value("\t".into(), char('t')),
        map(hex_escape, |c| c.to_string()),
    ))(input)
}

fn hex_escape(input: &str) -> IResult<&str, char> {
    map_res(preceded(char('u'), take_while_m_n(4, 4, |c: char| c.is_ascii_hexdigit())), |hex: &str| {
        let code = u32::from_str_radix(hex, 16).map_err(|_| "Unicode エスケープが不正です")?;
        char::from_u32(code).ok_or("Unicode エスケープが不正です")
    })(input)
}

fn json_array(input: &str) -> IResult<&str, JsonValue> {
    map(
        delimited(
            preceded(multispace0, char('[')),
            separated_list0(preceded(multispace0, char(',')), preceded(multispace0, json_value)),
            preceded(multispace0, char(']')),
        ),
        JsonValue::Array,
    )(input)
}

fn json_object(input: &str) -> IResult<&str, JsonValue> {
    map(
        delimited(
            preceded(multispace0, char('{')),
            separated_list0(
                preceded(multispace0, char(',')),
                preceded(
                    multispace0,
                    separated_pair(json_string, preceded(multispace0, char(':')), preceded(multispace0, json_value)),
                ),
            ),
            preceded(multispace0, char('}')),
        ),
        |entries| JsonValue::Object(entries.into_iter().collect()),
    )(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_object() {
        let json = parse_json("{\"ok\": true, \"num\": 12}").unwrap();
        if let JsonValue::Object(map) = json {
            assert_eq!(map["ok"], JsonValue::Bool(true));
            assert_eq!(map["num"], JsonValue::Number(12.0));
        } else {
            panic!("オブジェクトを期待しました");
        }
    }
}
