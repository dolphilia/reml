/// JSON拡張版：コメント・トレーリングカンマ対応。
///
/// 標準JSONからの拡張点：
/// 1. コメント対応（`//` 行コメント、`/* */` ブロックコメント）
/// 2. トレーリングカンマ許可（配列・オブジェクトの最後の要素の後）
/// 3. より詳細なエラーメッセージ
///
/// 実用的な設定ファイル形式として：
/// - `package.json` 風の設定ファイル
/// - `.babelrc`, `.eslintrc` など開発ツールの設定
/// - VS Code の `settings.json`

use std::collections::HashMap;

// 型定義

#[derive(Debug, Clone, PartialEq)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<JsonValue>),
    Object(HashMap<String, JsonValue>),
}

#[derive(Debug)]
pub enum ParseError {
    UnexpectedEOF,
    InvalidValue(String),
    UnclosedString,
    UnclosedBlockComment,
    ExpectedChar(char),
    InvalidNumber(String),
}

struct State {
    input: String,
    pos: usize,
}

// パース

pub fn parse(input: &str) -> Result<JsonValue, ParseError> {
    let mut state = State {
        input: input.to_string(),
        pos: 0,
    };

    skip_whitespace_and_comments(&mut state)?;
    let value = parse_value(&mut state)?;
    skip_whitespace_and_comments(&mut state)?;

    if state.pos >= state.input.len() {
        Ok(value)
    } else {
        Err(ParseError::InvalidValue(
            "入力の終端に到達していません".to_string(),
        ))
    }
}

// 空白とコメントをスキップ

fn skip_whitespace_and_comments(state: &mut State) -> Result<(), ParseError> {
    loop {
        skip_ws(state);
        if state.pos >= state.input.len() {
            return Ok(());
        }

        let remaining = &state.input[state.pos..];
        if remaining.starts_with("//") {
            skip_line_comment(state);
        } else if remaining.starts_with("/*") {
            skip_block_comment(state)?;
        } else {
            return Ok(());
        }
    }
}

fn skip_ws(state: &mut State) {
    while state.pos < state.input.len() {
        match state.input.chars().nth(state.pos) {
            Some(' ') | Some('\n') | Some('\t') | Some('\r') => state.pos += 1,
            _ => break,
        }
    }
}

fn skip_line_comment(state: &mut State) {
    state.pos += 2; // "//" をスキップ
    while state.pos < state.input.len() {
        if state.input.chars().nth(state.pos) == Some('\n') {
            state.pos += 1;
            break;
        }
        state.pos += 1;
    }
}

fn skip_block_comment(state: &mut State) -> Result<(), ParseError> {
    state.pos += 2; // "/*" をスキップ
    while state.pos + 1 < state.input.len() {
        if &state.input[state.pos..state.pos + 2] == "*/" {
            state.pos += 2;
            return Ok(());
        }
        state.pos += 1;
    }
    Err(ParseError::UnclosedBlockComment)
}

// 値のパース

fn parse_value(state: &mut State) -> Result<JsonValue, ParseError> {
    skip_whitespace_and_comments(state)?;

    if state.pos >= state.input.len() {
        return Err(ParseError::UnexpectedEOF);
    }

    let remaining = &state.input[state.pos..];

    if remaining.starts_with("null") {
        state.pos += 4;
        Ok(JsonValue::Null)
    } else if remaining.starts_with("true") {
        state.pos += 4;
        Ok(JsonValue::Bool(true))
    } else if remaining.starts_with("false") {
        state.pos += 5;
        Ok(JsonValue::Bool(false))
    } else if remaining.starts_with('"') {
        parse_string(state)
    } else if remaining.starts_with('[') {
        parse_array(state)
    } else if remaining.starts_with('{') {
        parse_object(state)
    } else {
        parse_number(state)
    }
}

// 文字列リテラルのパース

fn parse_string(state: &mut State) -> Result<JsonValue, ParseError> {
    state.pos += 1; // '"' をスキップ
    let mut result = String::new();

    while state.pos < state.input.len() {
        match state.input.chars().nth(state.pos) {
            Some('"') => {
                state.pos += 1;
                return Ok(JsonValue::String(result));
            }
            Some('\\') if state.pos + 1 < state.input.len() => {
                state.pos += 1;
                let escaped = match state.input.chars().nth(state.pos) {
                    Some('n') => '\n',
                    Some('t') => '\t',
                    Some('r') => '\r',
                    Some('\\') => '\\',
                    Some('"') => '"',
                    Some(ch) => ch,
                    None => return Err(ParseError::UnclosedString),
                };
                result.push(escaped);
                state.pos += 1;
            }
            Some(ch) => {
                result.push(ch);
                state.pos += 1;
            }
            None => return Err(ParseError::UnclosedString),
        }
    }

    Err(ParseError::UnclosedString)
}

// 数値のパース

fn parse_number(state: &mut State) -> Result<JsonValue, ParseError> {
    let start = state.pos;

    while state.pos < state.input.len() {
        match state.input.chars().nth(state.pos) {
            Some(ch) if ch == '-' || ch == '+' || ch == '.' || ch == 'e' || ch == 'E' || ch.is_ascii_digit() => {
                state.pos += 1;
            }
            _ => break,
        }
    }

    let num_str = &state.input[start..state.pos];
    num_str
        .parse::<f64>()
        .map(JsonValue::Number)
        .map_err(|_| ParseError::InvalidNumber(num_str.to_string()))
}

// 配列のパース（トレーリングカンマ対応）

fn parse_array(state: &mut State) -> Result<JsonValue, ParseError> {
    state.pos += 1; // '[' をスキップ
    skip_whitespace_and_comments(state)?;

    if state.pos < state.input.len() && state.input.chars().nth(state.pos) == Some(']') {
        state.pos += 1;
        return Ok(JsonValue::Array(Vec::new()));
    }

    let mut items = Vec::new();

    loop {
        let value = parse_value(state)?;
        items.push(value);
        skip_whitespace_and_comments(state)?;

        if state.pos >= state.input.len() {
            return Err(ParseError::UnexpectedEOF);
        }

        match state.input.chars().nth(state.pos) {
            Some(',') => {
                state.pos += 1;
                skip_whitespace_and_comments(state)?;

                // トレーリングカンマチェック
                if state.pos < state.input.len() && state.input.chars().nth(state.pos) == Some(']') {
                    state.pos += 1;
                    return Ok(JsonValue::Array(items));
                }
            }
            Some(']') => {
                state.pos += 1;
                return Ok(JsonValue::Array(items));
            }
            _ => return Err(ParseError::ExpectedChar(',')),
        }
    }
}

// オブジェクトのパース（トレーリングカンマ対応）

fn parse_object(state: &mut State) -> Result<JsonValue, ParseError> {
    state.pos += 1; // '{' をスキップ
    skip_whitespace_and_comments(state)?;

    if state.pos < state.input.len() && state.input.chars().nth(state.pos) == Some('}') {
        state.pos += 1;
        return Ok(JsonValue::Object(HashMap::new()));
    }

    let mut pairs = HashMap::new();

    loop {
        let key_value = parse_string(state)?;
        let key = match key_value {
            JsonValue::String(s) => s,
            _ => {
                return Err(ParseError::InvalidValue(
                    "オブジェクトのキーは文字列である必要があります".to_string(),
                ))
            }
        };

        skip_whitespace_and_comments(state)?;

        if state.pos >= state.input.len() || state.input.chars().nth(state.pos) != Some(':') {
            return Err(ParseError::ExpectedChar(':'));
        }
        state.pos += 1;

        skip_whitespace_and_comments(state)?;

        let value = parse_value(state)?;
        pairs.insert(key, value);

        skip_whitespace_and_comments(state)?;

        if state.pos >= state.input.len() {
            return Err(ParseError::UnexpectedEOF);
        }

        match state.input.chars().nth(state.pos) {
            Some(',') => {
                state.pos += 1;
                skip_whitespace_and_comments(state)?;

                // トレーリングカンマチェック
                if state.pos < state.input.len() && state.input.chars().nth(state.pos) == Some('}') {
                    state.pos += 1;
                    return Ok(JsonValue::Object(pairs));
                }
            }
            Some('}') => {
                state.pos += 1;
                return Ok(JsonValue::Object(pairs));
            }
            _ => return Err(ParseError::ExpectedChar(',')),
        }
    }
}

// レンダリング

pub fn render_to_string(value: &JsonValue, indent_level: usize) -> String {
    let indent = "  ".repeat(indent_level);
    let next_indent = "  ".repeat(indent_level + 1);

    match value {
        JsonValue::Null => "null".to_string(),
        JsonValue::Bool(true) => "true".to_string(),
        JsonValue::Bool(false) => "false".to_string(),
        JsonValue::Number(num) => num.to_string(),
        JsonValue::String(s) => format!("\"{}\"", s),
        JsonValue::Array(items) => {
            if items.is_empty() {
                "[]".to_string()
            } else {
                let items_str = items
                    .iter()
                    .map(|item| format!("{}{}", next_indent, render_to_string(item, indent_level + 1)))
                    .collect::<Vec<_>>()
                    .join(",\n");
                format!("[\n{}\n{}]", items_str, indent)
            }
        }
        JsonValue::Object(pairs) => {
            if pairs.is_empty() {
                "{}".to_string()
            } else {
                let pairs_str = pairs
                    .iter()
                    .map(|(key, val)| {
                        format!(
                            "{}\"{}\": {}",
                            next_indent,
                            key,
                            render_to_string(val, indent_level + 1)
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(",\n");
                format!("{{\n{}\n{}}}", pairs_str, indent)
            }
        }
    }
}

// テスト

pub fn test_extended_json() {
    let test_cases = vec![
        (
            "コメント対応",
            r#"
{
  // これは行コメント
  "name": "test",
  /* これは
     ブロックコメント */
  "version": "1.0"
}
"#,
        ),
        (
            "トレーリングカンマ",
            r#"
{
  "items": [
    1,
    2,
    3,
  ],
  "config": {
    "debug": true,
    "port": 8080,
  }
}
"#,
        ),
    ];

    for (name, json_str) in test_cases {
        println!("--- {} ---", name);
        match parse(json_str) {
            Ok(value) => {
                println!("パース成功:");
                println!("{}", render_to_string(&value, 0));
            }
            Err(err) => {
                println!("パースエラー: {:?}", err);
            }
        }
        println!();
    }
}