use std::collections::HashMap;

/// YAML風パーサー：インデント管理が重要な題材。
///
/// 対応する構文（簡易版）：
/// - スカラー値: 文字列、数値、真偽値、null
/// - リスト: `- item1`
/// - マップ: `key: value`
/// - ネストしたインデント構造
///
/// インデント処理の特徴：
/// - Rustの所有権システムとResultを活用したパーサー実装
/// - エラー回復機能でインデントミスを報告しつつ継続

#[derive(Debug, Clone, PartialEq)]
pub enum YamlValue {
    Scalar(String),
    List(Vec<YamlValue>),
    Map(HashMap<String, YamlValue>),
    Null,
}

pub type Document = YamlValue;

#[derive(Debug, Clone)]
pub struct ParseError {
    message: String,
}

impl ParseError {
    fn new(message: impl Into<String>) -> Self {
        ParseError {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ParseError {}

pub struct Parser<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> Parser<'a> {
    pub fn new(input: &'a str) -> Self {
        Parser { input, pos: 0 }
    }

    fn peek(&self) -> Option<char> {
        self.input.chars().nth(self.pos)
    }

    fn advance(&mut self) {
        if self.pos < self.input.len() {
            self.pos += self.input.chars().nth(self.pos).unwrap().len_utf8();
        }
    }

    fn is_eof(&self) -> bool {
        self.pos >= self.input.len()
    }

    fn expect(&mut self, expected: char) -> Result<(), ParseError> {
        if self.peek() != Some(expected) {
            return Err(ParseError::new(format!(
                "期待された文字 '{}' が見つかりません",
                expected
            )));
        }
        self.advance();
        Ok(())
    }

    fn expect_string(&mut self, expected: &str) -> Result<(), ParseError> {
        for c in expected.chars() {
            self.expect(c)?;
        }
        Ok(())
    }

    /// 水平空白のみをスキップ（改行は含まない）。
    fn hspace(&mut self) {
        while let Some(c) = self.peek() {
            if c == ' ' || c == '\t' {
                self.advance();
            } else {
                break;
            }
        }
    }

    /// 改行をスキップ。
    fn newline(&mut self) {
        if self.peek() == Some('\n') {
            self.advance();
        } else if self.peek() == Some('\r') {
            self.advance();
            if self.peek() == Some('\n') {
                self.advance();
            }
        }
    }

    /// コメントのスキップ（`#` から行末まで）。
    fn comment(&mut self) {
        if self.peek() == Some('#') {
            self.advance();
            while let Some(c) = self.peek() {
                if c == '\n' {
                    break;
                }
                self.advance();
            }
        }
    }

    /// 空行またはコメント行をスキップ。
    fn blank_or_comment(&mut self) {
        self.hspace();
        self.comment();
        self.newline();
    }

    /// 特定のインデントレベルを期待する。
    fn expect_indent(&mut self, level: usize) -> Result<(), ParseError> {
        let mut spaces = 0;
        while self.peek() == Some(' ') {
            spaces += 1;
            self.advance();
        }

        if spaces != level {
            return Err(ParseError::new(format!(
                "インデント不一致: 期待 {}, 実際 {}",
                level, spaces
            )));
        }

        Ok(())
    }

    /// 現在よりも深いインデントを検出。
    fn deeper_indent(&mut self, current: usize) -> Result<usize, ParseError> {
        let mut spaces = 0;
        while self.peek() == Some(' ') {
            spaces += 1;
            self.advance();
        }

        if spaces <= current {
            return Err(ParseError::new(format!(
                "深いインデントが期待されます: 現在 {}, 実際 {}",
                current, spaces
            )));
        }

        Ok(spaces)
    }

    /// スカラー値のパース。
    fn scalar_value(&mut self) -> Result<YamlValue, ParseError> {
        // null
        if self.input[self.pos..].starts_with("null") {
            self.expect_string("null")?;
            return Ok(YamlValue::Null);
        }

        if self.peek() == Some('~') {
            self.advance();
            return Ok(YamlValue::Null);
        }

        // 真偽値
        if self.input[self.pos..].starts_with("true") {
            self.expect_string("true")?;
            return Ok(YamlValue::Scalar("true".to_string()));
        }

        if self.input[self.pos..].starts_with("false") {
            self.expect_string("false")?;
            return Ok(YamlValue::Scalar("false".to_string()));
        }

        // 数値（簡易実装）
        let mut num_str = String::new();
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                num_str.push(c);
                self.advance();
            } else {
                break;
            }
        }

        if !num_str.is_empty() {
            return Ok(YamlValue::Scalar(num_str));
        }

        // 文字列（引用符付き）
        if self.peek() == Some('"') {
            self.advance();
            let mut str = String::new();
            while let Some(c) = self.peek() {
                if c == '"' {
                    break;
                }
                str.push(c);
                self.advance();
            }
            self.expect('"')?;
            return Ok(YamlValue::Scalar(str));
        }

        // 文字列（引用符なし：行末または `:` まで）
        let mut str = String::new();
        while let Some(c) = self.peek() {
            if c == '\n' || c == ':' || c == '#' {
                break;
            }
            str.push(c);
            self.advance();
        }

        Ok(YamlValue::Scalar(str.trim().to_string()))
    }

    /// リスト項目のパース（`- value` 形式）。
    fn parse_list_item(&mut self, indent: usize) -> Result<YamlValue, ParseError> {
        self.expect_indent(indent)?;
        self.expect('-')?;
        self.hspace();
        self.parse_value(indent + 2)
    }

    /// リスト全体のパース。
    fn parse_list(&mut self, indent: usize) -> Result<YamlValue, ParseError> {
        let mut items = Vec::new();

        loop {
            let saved_pos = self.pos;
            match self.parse_list_item(indent) {
                Ok(item) => {
                    items.push(item);
                    if self.peek() == Some('\n') {
                        self.newline();
                    } else {
                        break;
                    }
                }
                Err(_) => {
                    self.pos = saved_pos;
                    break;
                }
            }
        }

        if items.is_empty() {
            return Err(ParseError::new("リストが空です"));
        }

        Ok(YamlValue::List(items))
    }

    /// マップのキーバリューペアのパース（`key: value` 形式）。
    fn parse_map_entry(&mut self, indent: usize) -> Result<(String, YamlValue), ParseError> {
        self.expect_indent(indent)?;

        let mut key = String::new();
        while let Some(c) = self.peek() {
            if c == ':' || c == '\n' {
                break;
            }
            key.push(c);
            self.advance();
        }

        let key = key.trim().to_string();
        self.expect(':')?;
        self.hspace();

        // 同じ行に値があるか、次の行にネストされているか
        let value = if self.peek() == Some('\n') {
            self.newline();
            self.parse_value(indent + 2)?
        } else {
            self.parse_value(indent)?
        };

        Ok((key, value))
    }

    /// マップ全体のパース。
    fn parse_map(&mut self, indent: usize) -> Result<YamlValue, ParseError> {
        let mut entries = HashMap::new();

        loop {
            let saved_pos = self.pos;
            match self.parse_map_entry(indent) {
                Ok((key, value)) => {
                    entries.insert(key, value);
                    if self.peek() == Some('\n') {
                        self.newline();
                    } else {
                        break;
                    }
                }
                Err(_) => {
                    self.pos = saved_pos;
                    break;
                }
            }
        }

        if entries.is_empty() {
            return Err(ParseError::new("マップが空です"));
        }

        Ok(YamlValue::Map(entries))
    }

    /// YAML値のパース（再帰的）。
    fn parse_value(&mut self, indent: usize) -> Result<YamlValue, ParseError> {
        let saved_pos = self.pos;

        // リストを試行
        if let Ok(list) = self.parse_list(indent) {
            return Ok(list);
        }

        self.pos = saved_pos;

        // マップを試行
        if let Ok(map) = self.parse_map(indent) {
            return Ok(map);
        }

        self.pos = saved_pos;

        // スカラー
        self.scalar_value()
    }

    /// ドキュメント全体のパース。
    fn document(&mut self) -> Result<Document, ParseError> {
        // 空行やコメントをスキップ
        while !self.is_eof() {
            let saved_pos = self.pos;
            self.blank_or_comment();
            if self.pos == saved_pos {
                break;
            }
        }

        let doc = self.parse_value(0)?;

        // 末尾の空行やコメントをスキップ
        while !self.is_eof() {
            let saved_pos = self.pos;
            self.blank_or_comment();
            if self.pos == saved_pos {
                break;
            }
        }

        if !self.is_eof() {
            return Err(ParseError::new("ドキュメントの終端が期待されます"));
        }

        Ok(doc)
    }
}

/// パブリックAPI：YAML文字列をパース。
pub fn parse_yaml(input: &str) -> Result<Document, ParseError> {
    let mut parser = Parser::new(input);
    parser.document()
}

/// 簡易的なレンダリング（検証用）。
pub fn render_to_string(doc: &Document) -> String {
    fn render_value(value: &YamlValue, indent: usize) -> String {
        let indent_str = " ".repeat(indent);

        match value {
            YamlValue::Scalar(s) => s.clone(),
            YamlValue::Null => "null".to_string(),
            YamlValue::List(items) => items
                .iter()
                .map(|item| format!("{}- {}", indent_str, render_value(item, indent + 2)))
                .collect::<Vec<_>>()
                .join("\n"),
            YamlValue::Map(entries) => {
                let mut lines = Vec::new();
                for (key, val) in entries.iter() {
                    match val {
                        YamlValue::Scalar(_) | YamlValue::Null => {
                            lines.push(format!("{}{}: {}", indent_str, key, render_value(val, 0)));
                        }
                        _ => {
                            lines.push(format!(
                                "{}{}:\n{}",
                                indent_str,
                                key,
                                render_value(val, indent + 2)
                            ));
                        }
                    }
                }
                lines.join("\n")
            }
        }
    }

    render_value(doc, 0)
}

/// テスト例。
pub fn test_examples() {
    let examples = vec![
        ("simple_scalar", "hello"),
        ("simple_list", "- item1\n- item2\n- item3"),
        ("simple_map", "key1: value1\nkey2: value2"),
        ("nested_map", "parent:\n  child1: value1\n  child2: value2"),
        ("nested_list", "items:\n  - item1\n  - item2"),
        ("mixed", "name: John\nage: 30\nhobbies:\n  - reading\n  - coding"),
    ];

    for (name, yaml_str) in examples {
        println!("--- {} ---", name);
        match parse_yaml(yaml_str) {
            Ok(doc) => {
                println!("パース成功:");
                println!("{}", render_to_string(&doc));
            }
            Err(err) => {
                println!("パースエラー: {}", err);
            }
        }
    }
}

/// インデント処理の課題と解決策：
///
/// 1. **インデントレベルの追跡**
///    - パーサー引数としてインデントレベルを渡す
///    - Rustの所有権システムでパーサー状態を管理
///
/// 2. **エラー回復**
///    - Resultでバックトラックを制御
///    - ParseError型で分かりやすいエラーメッセージを提供
///
/// 3. **空白の扱い**
///    - hspaceで水平空白のみをスキップ（改行は構文の一部）
///    - newlineでCR/LF/CRLFを正規化
///
/// Remlとの比較：
///
/// - **Rustの利点**:
///   - ゼロコスト抽象化と高速な実行
///   - 強力な所有権システムによるメモリ安全性
///
/// - **Rustの課題**:
///   - パーサーコンビネーターライブラリがRemlほど充実していない
///   - 手動のバックトラック管理が煩雑
///
/// - **Remlの利点**:
///   - 字句レイヤの柔軟性により、インデント処理が自然に表現できる
///   - cut/commitによるエラー品質の向上
///   - recoverによる部分的なパース継続が可能