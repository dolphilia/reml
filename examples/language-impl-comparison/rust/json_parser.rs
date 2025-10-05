use std::collections::BTreeMap;

#[derive(Debug, PartialEq)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<JsonValue>),
    Object(BTreeMap<String, JsonValue>),
}

#[derive(Debug)]
pub enum ParseError {
    UnexpectedEof,
    UnexpectedChar(char),
    InvalidNumber(String),
}

pub fn parse(source: &str) -> Result<JsonValue, ParseError> {
    let mut parser = Parser::new(source);
    let value = parser.parse_value()?;
    parser.skip_whitespace();
    if parser.peek().is_some() {
        return Err(ParseError::UnexpectedChar(parser.peek().unwrap()));
    }
    Ok(value)
}

struct Parser<'a> {
    chars: std::str::Chars<'a>,
    lookahead: Option<char>,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        let mut chars = input.chars();
        let lookahead = chars.next();
        Parser { chars, lookahead }
    }

    fn peek(&self) -> Option<char> {
        self.lookahead
    }

    fn bump(&mut self) -> Option<char> {
        let current = self.lookahead;
        self.lookahead = self.chars.next();
        current
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some(c) if c.is_whitespace()) {
            self.bump();
        }
    }

    fn parse_value(&mut self) -> Result<JsonValue, ParseError> {
        self.skip_whitespace();
        match self.peek() {
            Some('n') => self.parse_null(),
            Some('t') | Some('f') => self.parse_bool(),
            Some('"') => self.parse_string().map(JsonValue::String),
            Some('[') => self.parse_array(),
            Some('{') => self.parse_object(),
            Some('-') | Some('0'..='9') => self.parse_number().map(JsonValue::Number),
            Some(c) => Err(ParseError::UnexpectedChar(c)),
            None => Err(ParseError::UnexpectedEof),
        }
    }

    fn parse_null(&mut self) -> Result<JsonValue, ParseError> {
        self.expect_literal("null")?;
        Ok(JsonValue::Null)
    }

    fn parse_bool(&mut self) -> Result<JsonValue, ParseError> {
        if self.expect_literal("true").is_ok() {
            Ok(JsonValue::Bool(true))
        } else if self.expect_literal("false").is_ok() {
            Ok(JsonValue::Bool(false))
        } else {
            Err(ParseError::UnexpectedChar(self.peek().unwrap_or('\0')))
        }
    }

    fn parse_number(&mut self) -> Result<f64, ParseError> {
        let mut literal = String::new();
        if self.peek() == Some('-') {
            literal.push('-');
            self.bump();
        }
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() || c == '.' || c == 'e' || c == 'E' || c == '+' || c == '-' {
                literal.push(c);
                self.bump();
            } else {
                break;
            }
        }
        literal.parse::<f64>().map_err(|_| ParseError::InvalidNumber(literal))
    }

    fn parse_string(&mut self) -> Result<String, ParseError> {
        self.expect('"')?;
        let mut buf = String::new();
        while let Some(c) = self.bump() {
            match c {
                '"' => return Ok(buf),
                '\\' => {
                    let escaped = self.bump().ok_or(ParseError::UnexpectedEof)?;
                    buf.push(match escaped {
                        '"' => '"',
                        '\\' => '\\',
                        '/' => '/',
                        'b' => '\u{0008}',
                        'f' => '\u{000C}',
                        'n' => '\n',
                        'r' => '\r',
                        't' => '\t',
                        'u' => self.parse_unicode_escape()?,
                        other => return Err(ParseError::UnexpectedChar(other)),
                    });
                }
                other => buf.push(other),
            }
        }
        Err(ParseError::UnexpectedEof)
    }

    fn parse_unicode_escape(&mut self) -> Result<char, ParseError> {
        let mut code = 0u32;
        for _ in 0..4 {
            let digit = self.bump().ok_or(ParseError::UnexpectedEof)?;
            code = code * 16
                + digit.to_digit(16).ok_or(ParseError::UnexpectedChar(digit))?;
        }
        std::char::from_u32(code).ok_or(ParseError::UnexpectedChar('\0'))
    }

    fn parse_array(&mut self) -> Result<JsonValue, ParseError> {
        self.expect('[')?;
        let mut items = Vec::new();
        self.skip_whitespace();
        if self.peek() == Some(']') {
            self.bump();
            return Ok(JsonValue::Array(items));
        }
        loop {
            let value = self.parse_value()?;
            items.push(value);
            self.skip_whitespace();
            match self.peek() {
                Some(',') => {
                    self.bump();
                }
                Some(']') => {
                    self.bump();
                    break;
                }
                Some(c) => return Err(ParseError::UnexpectedChar(c)),
                None => return Err(ParseError::UnexpectedEof),
            }
        }
        Ok(JsonValue::Array(items))
    }

    fn parse_object(&mut self) -> Result<JsonValue, ParseError> {
        self.expect('{')?;
        let mut map = BTreeMap::new();
        self.skip_whitespace();
        if self.peek() == Some('}') {
            self.bump();
            return Ok(JsonValue::Object(map));
        }
        loop {
            let key = self.parse_string()?;
            self.skip_whitespace();
            self.expect(':')?;
            let value = self.parse_value()?;
            map.insert(key, value);
            self.skip_whitespace();
            match self.peek() {
                Some(',') => {
                    self.bump();
                }
                Some('}') => {
                    self.bump();
                    break;
                }
                Some(c) => return Err(ParseError::UnexpectedChar(c)),
                None => return Err(ParseError::UnexpectedEof),
            }
        }
        Ok(JsonValue::Object(map))
    }

    fn expect(&mut self, target: char) -> Result<(), ParseError> {
        match self.bump() {
            Some(c) if c == target => Ok(()),
            Some(c) => Err(ParseError::UnexpectedChar(c)),
            None => Err(ParseError::UnexpectedEof),
        }
    }

    fn expect_literal(&mut self, literal: &str) -> Result<(), ParseError> {
        for expected in literal.chars() {
            if self.bump() != Some(expected) {
                return Err(ParseError::UnexpectedChar(expected));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_object() {
        let json = parse("{\"number\": 42, \"ok\": true}").unwrap();
        if let JsonValue::Object(map) = json {
            assert_eq!(map["number"], JsonValue::Number(42.0));
            assert_eq!(map["ok"], JsonValue::Bool(true));
        } else {
            panic!("オブジェクトが得られる想定");
        }
    }
}
