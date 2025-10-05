use std::collections::HashMap;

/// TOML v1.0.0準拠の簡易パーサー
///
/// 対応する構文：
/// - キーバリューペア: `key = "value"`
/// - テーブル: `[section]`
/// - 配列テーブル: `[[array_section]]`
/// - データ型: 文字列、整数、浮動小数点、真偽値、配列、インラインテーブル
/// - コメント: `# comment`
///
/// Rust実装の特徴：
/// - 所有権システムによる安全なメモリ管理
/// - Result型による明示的なエラーハンドリング
/// - パターンマッチによる直感的な構文解析
/// - ゼロコスト抽象化による高速な実行
/// - 型システムによるコンパイル時の保証
///
/// Remlとの比較ポイント：
/// - Reml: パーサーコンビネーターの統合により宣言的で簡潔
/// - Rust: 手動のバックトラック管理が必要だが、パフォーマンスが高い
/// - Reml: cut/commit/recoverによる高品質なエラーメッセージ
/// - Rust: Result型とカスタムエラー型で柔軟なエラーハンドリング

#[derive(Debug, Clone, PartialEq)]
pub enum TomlValue {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Array(Vec<TomlValue>),
    InlineTable(HashMap<String, TomlValue>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct TomlDocument {
    pub root: HashMap<String, TomlValue>,
    pub tables: HashMap<Vec<String>, HashMap<String, TomlValue>>,
}

#[derive(Debug, Clone)]
pub struct ParseError {
    message: String,
    position: usize,
}

impl ParseError {
    fn new(message: impl Into<String>, position: usize) -> Self {
        ParseError {
            message: message.into(),
            position,
        }
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "位置 {}: {}", self.position, self.message)
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

    fn current_pos(&self) -> usize {
        self.pos
    }

    fn peek(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }

    fn advance(&mut self) {
        if let Some(c) = self.peek() {
            self.pos += c.len_utf8();
        }
    }

    fn is_eof(&self) -> bool {
        self.pos >= self.input.len()
    }

    fn expect(&mut self, expected: char) -> Result<(), ParseError> {
        if self.peek() != Some(expected) {
            return Err(ParseError::new(
                format!("期待された文字 '{}' が見つかりません", expected),
                self.pos,
            ));
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

    /// 空白とコメントをスキップ。
    fn skip_whitespace_and_comments(&mut self) {
        loop {
            // 空白をスキップ
            while let Some(c) = self.peek() {
                if c == ' ' || c == '\t' || c == '\n' || c == '\r' {
                    self.advance();
                } else {
                    break;
                }
            }

            // コメントをスキップ
            if self.peek() == Some('#') {
                self.advance();
                while let Some(c) = self.peek() {
                    if c == '\n' {
                        self.advance();
                        break;
                    }
                    self.advance();
                }
            } else {
                break;
            }
        }
    }

    /// 水平空白のみをスキップ（改行は含まない）。
    fn skip_hspace(&mut self) {
        while let Some(c) = self.peek() {
            if c == ' ' || c == '\t' {
                self.advance();
            } else {
                break;
            }
        }
    }

    /// ベアキー（英数字・`-`・`_`のみ）のパース。
    fn parse_bare_key(&mut self) -> Result<String, ParseError> {
        let start = self.pos;
        let mut key = String::new();

        let first = self.peek().ok_or_else(|| {
            ParseError::new("キーが期待されます", self.pos)
        })?;

        if !first.is_alphanumeric() && first != '_' && first != '-' {
            return Err(ParseError::new("無効なキー文字", self.pos));
        }

        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' || c == '-' {
                key.push(c);
                self.advance();
            } else {
                break;
            }
        }

        if key.is_empty() {
            return Err(ParseError::new("キーが空です", start));
        }

        Ok(key)
    }

    /// 引用符付き文字列のパース。
    fn parse_quoted_string(&mut self) -> Result<String, ParseError> {
        self.expect('"')?;
        let mut str = String::new();

        while let Some(c) = self.peek() {
            if c == '"' {
                break;
            } else if c == '\\' {
                self.advance();
                match self.peek() {
                    Some('n') => {
                        str.push('\n');
                        self.advance();
                    }
                    Some('t') => {
                        str.push('\t');
                        self.advance();
                    }
                    Some('\\') => {
                        str.push('\\');
                        self.advance();
                    }
                    Some('"') => {
                        str.push('"');
                        self.advance();
                    }
                    _ => {
                        return Err(ParseError::new("無効なエスケープシーケンス", self.pos));
                    }
                }
            } else {
                str.push(c);
                self.advance();
            }
        }

        self.expect('"')?;
        Ok(str)
    }

    /// キー名のパース（ベアキーまたは引用符付き）。
    fn parse_key(&mut self) -> Result<String, ParseError> {
        if self.peek() == Some('"') {
            self.parse_quoted_string()
        } else {
            self.parse_bare_key()
        }
    }

    /// ドットで区切られたキーパスのパース。
    fn parse_key_path(&mut self) -> Result<Vec<String>, ParseError> {
        let mut path = vec![self.parse_key()?];

        while self.peek() == Some('.') {
            self.advance();
            path.push(self.parse_key()?);
        }

        Ok(path)
    }

    /// 文字列値のパース。
    fn parse_string_value(&mut self) -> Result<TomlValue, ParseError> {
        Ok(TomlValue::String(self.parse_quoted_string()?))
    }

    /// 整数値のパース。
    fn parse_integer(&mut self) -> Result<i64, ParseError> {
        let mut num_str = String::new();
        let is_negative = self.peek() == Some('-');

        if is_negative {
            num_str.push('-');
            self.advance();
        }

        let start = self.pos;
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                num_str.push(c);
                self.advance();
            } else if c == '_' {
                // アンダースコアは無視（TOMLの数値セパレーター）
                self.advance();
            } else {
                break;
            }
        }

        if num_str == "-" || num_str.is_empty() {
            return Err(ParseError::new("無効な整数", start));
        }

        num_str.parse::<i64>().map_err(|_| {
            ParseError::new("整数の解析に失敗しました", start)
        })
    }

    /// 浮動小数点値のパース。
    fn parse_float(&mut self, int_part: &str) -> Result<f64, ParseError> {
        let mut num_str = int_part.to_string();

        // 小数点
        if self.peek() == Some('.') {
            num_str.push('.');
            self.advance();

            while let Some(c) = self.peek() {
                if c.is_ascii_digit() {
                    num_str.push(c);
                    self.advance();
                } else if c == '_' {
                    self.advance();
                } else {
                    break;
                }
            }
        }

        // 指数部
        if let Some(c) = self.peek() {
            if c == 'e' || c == 'E' {
                num_str.push(c);
                self.advance();

                if let Some(sign) = self.peek() {
                    if sign == '+' || sign == '-' {
                        num_str.push(sign);
                        self.advance();
                    }
                }

                while let Some(c) = self.peek() {
                    if c.is_ascii_digit() {
                        num_str.push(c);
                        self.advance();
                    } else if c == '_' {
                        self.advance();
                    } else {
                        break;
                    }
                }
            }
        }

        num_str.parse::<f64>().map_err(|_| {
            ParseError::new("浮動小数点数の解析に失敗しました", self.pos)
        })
    }

    /// 数値（整数または浮動小数点）のパース。
    fn parse_number(&mut self) -> Result<TomlValue, ParseError> {
        let start = self.pos;
        let mut num_str = String::new();

        if self.peek() == Some('-') {
            num_str.push('-');
            self.advance();
        }

        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                num_str.push(c);
                self.advance();
            } else if c == '_' {
                self.advance();
            } else {
                break;
            }
        }

        // 浮動小数点かどうかを判定
        if self.peek() == Some('.') || self.peek() == Some('e') || self.peek() == Some('E') {
            self.pos = start;
            let int_part = num_str.clone();
            if self.peek() == Some('-') {
                self.advance();
            }
            let mut temp = String::new();
            while let Some(c) = self.peek() {
                if c.is_ascii_digit() || c == '_' {
                    if c != '_' {
                        temp.push(c);
                    }
                    self.advance();
                } else {
                    break;
                }
            }
            let float = self.parse_float(&format!("{}{}", if int_part.starts_with('-') { "-" } else { "" }, temp))?;
            Ok(TomlValue::Float(float))
        } else {
            let num = num_str.parse::<i64>().map_err(|_| {
                ParseError::new("整数の解析に失敗しました", start)
            })?;
            Ok(TomlValue::Integer(num))
        }
    }

    /// 真偽値のパース。
    fn parse_boolean(&mut self) -> Result<TomlValue, ParseError> {
        if self.input[self.pos..].starts_with("true") {
            self.expect_string("true")?;
            Ok(TomlValue::Boolean(true))
        } else if self.input[self.pos..].starts_with("false") {
            self.expect_string("false")?;
            Ok(TomlValue::Boolean(false))
        } else {
            Err(ParseError::new("真偽値が期待されます", self.pos))
        }
    }

    /// 配列のパース。
    fn parse_array(&mut self) -> Result<TomlValue, ParseError> {
        self.expect('[')?;
        self.skip_whitespace_and_comments();

        let mut items = Vec::new();

        while self.peek() != Some(']') {
            items.push(self.parse_value()?);
            self.skip_whitespace_and_comments();

            if self.peek() == Some(',') {
                self.advance();
                self.skip_whitespace_and_comments();
            } else {
                break;
            }
        }

        self.expect(']')?;
        Ok(TomlValue::Array(items))
    }

    /// インラインテーブルのパース。
    fn parse_inline_table(&mut self) -> Result<TomlValue, ParseError> {
        self.expect('{')?;
        self.skip_hspace();

        let mut table = HashMap::new();

        while self.peek() != Some('}') {
            let key = self.parse_key()?;
            self.skip_hspace();
            self.expect('=')?;
            self.skip_hspace();
            let value = self.parse_value()?;

            table.insert(key, value);
            self.skip_hspace();

            if self.peek() == Some(',') {
                self.advance();
                self.skip_hspace();
            } else {
                break;
            }
        }

        self.expect('}')?;
        Ok(TomlValue::InlineTable(table))
    }

    /// TOML値のパース。
    fn parse_value(&mut self) -> Result<TomlValue, ParseError> {
        self.skip_hspace();

        match self.peek() {
            Some('"') => self.parse_string_value(),
            Some('[') => self.parse_array(),
            Some('{') => self.parse_inline_table(),
            Some('t') | Some('f') => self.parse_boolean(),
            Some('-') | Some(c) if c.is_ascii_digit() => self.parse_number(),
            _ => Err(ParseError::new("値が期待されます", self.pos)),
        }
    }

    /// キーバリューペアのパース。
    fn parse_key_value(&mut self) -> Result<(Vec<String>, TomlValue), ParseError> {
        let path = self.parse_key_path()?;
        self.skip_hspace();
        self.expect('=')?;
        self.skip_hspace();
        let value = self.parse_value()?;

        Ok((path, value))
    }

    /// テーブルヘッダーのパース（`[section]`）。
    fn parse_table_header(&mut self) -> Result<Vec<String>, ParseError> {
        self.expect('[')?;
        let path = self.parse_key_path()?;
        self.expect(']')?;
        Ok(path)
    }

    /// 配列テーブルヘッダーのパース（`[[section]]`）。
    fn parse_array_table_header(&mut self) -> Result<Vec<String>, ParseError> {
        self.expect('[')?;
        self.expect('[')?;
        let path = self.parse_key_path()?;
        self.expect(']')?;
        self.expect(']')?;
        Ok(path)
    }

    /// ドキュメント全体のパース。
    fn parse_document(&mut self) -> Result<TomlDocument, ParseError> {
        let mut root = HashMap::new();
        let mut tables: HashMap<Vec<String>, HashMap<String, TomlValue>> = HashMap::new();
        let mut current_table: Vec<String> = Vec::new();

        self.skip_whitespace_and_comments();

        while !self.is_eof() {
            // 配列テーブルヘッダー
            if self.input[self.pos..].starts_with("[[") {
                current_table = self.parse_array_table_header()?;
                if !tables.contains_key(&current_table) {
                    tables.insert(current_table.clone(), HashMap::new());
                }
            }
            // テーブルヘッダー
            else if self.peek() == Some('[') {
                current_table = self.parse_table_header()?;
                if !tables.contains_key(&current_table) {
                    tables.insert(current_table.clone(), HashMap::new());
                }
            }
            // キーバリューペア
            else {
                let (path, value) = self.parse_key_value()?;

                if current_table.is_empty() {
                    // ルートテーブルに追加
                    insert_nested(&mut root, &path, value);
                } else {
                    // 現在のテーブルに追加
                    let table = tables.entry(current_table.clone()).or_insert_with(HashMap::new);
                    insert_nested(table, &path, value);
                }
            }

            self.skip_whitespace_and_comments();
        }

        Ok(TomlDocument { root, tables })
    }
}

/// ネストしたキーパスに値を挿入する補助関数。
fn insert_nested(table: &mut HashMap<String, TomlValue>, path: &[String], value: TomlValue) {
    if path.len() == 1 {
        table.insert(path[0].clone(), value);
    } else {
        let key = &path[0];
        let subtable = table
            .entry(key.clone())
            .or_insert_with(|| TomlValue::InlineTable(HashMap::new()));

        if let TomlValue::InlineTable(ref mut nested) = subtable {
            insert_nested(nested, &path[1..], value);
        }
    }
}

/// パブリックAPI：TOML文字列をパース。
pub fn parse_toml(input: &str) -> Result<TomlDocument, ParseError> {
    let mut parser = Parser::new(input);
    parser.parse_document()
}

/// 簡易的なレンダリング（検証用）。
pub fn render_to_string(doc: &TomlDocument) -> String {
    let mut output = String::new();

    // ルートテーブルをレンダリング
    for (key, value) in &doc.root {
        output.push_str(&format!("{} = {}\n", key, render_value(value)));
    }

    // 各セクションをレンダリング
    for (path, table) in &doc.tables {
        output.push_str(&format!("\n[{}]\n", path.join(".")));
        for (key, value) in table {
            output.push_str(&format!("{} = {}\n", key, render_value(value)));
        }
    }

    output
}

fn render_value(value: &TomlValue) -> String {
    match value {
        TomlValue::String(s) => format!("\"{}\"", s),
        TomlValue::Integer(n) => n.to_string(),
        TomlValue::Float(f) => f.to_string(),
        TomlValue::Boolean(b) => b.to_string(),
        TomlValue::Array(items) => {
            let items_str = items
                .iter()
                .map(render_value)
                .collect::<Vec<_>>()
                .join(", ");
            format!("[{}]", items_str)
        }
        TomlValue::InlineTable(entries) => {
            let entries_str = entries
                .iter()
                .map(|(k, v)| format!("{} = {}", k, render_value(v)))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{{ {} }}", entries_str)
        }
    }
}

/// テスト例。
pub fn test_examples() {
    let examples = vec![
        ("simple_kv", "key = \"value\""),
        ("integer", "port = 8080"),
        ("float", "pi = 3.14159"),
        ("boolean", "enabled = true"),
        ("array", "colors = [\"red\", \"green\", \"blue\"]"),
        ("inline_table", "point = { x = 1, y = 2 }"),
        (
            "table",
            r#"
[server]
host = "localhost"
port = 8080
"#,
        ),
        (
            "nested_table",
            r#"
[database.connection]
host = "localhost"
port = 5432
"#,
        ),
        (
            "array_table",
            r#"
[[plugins]]
name = "system"
version = "1.0"

[[plugins]]
name = "memory"
version = "1.0"
"#,
        ),
        (
            "full_document",
            r#"
# Reml パッケージ設定

[package]
name = "my_project"
version = "0.1.0"
authors = ["Author Name"]

[dependencies]
core = "1.0"

[dev-dependencies]
test_framework = "0.5"

[[plugins]]
name = "system"
version = "1.0"

[[plugins]]
name = "memory"
version = "1.0"
"#,
        ),
    ];

    for (name, toml_str) in examples {
        println!("--- {} ---", name);
        match parse_toml(toml_str) {
            Ok(doc) => {
                println!("パース成功:");
                println!("{}", render_to_string(&doc));
            }
            Err(err) => {
                println!("パースエラー: {}", err);
            }
        }
        println!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_key_value() {
        let input = "key = \"value\"";
        let result = parse_toml(input);
        assert!(result.is_ok());
        let doc = result.unwrap();
        assert_eq!(
            doc.root.get("key"),
            Some(&TomlValue::String("value".to_string()))
        );
    }

    #[test]
    fn test_integer() {
        let input = "port = 8080";
        let result = parse_toml(input);
        assert!(result.is_ok());
        let doc = result.unwrap();
        assert_eq!(doc.root.get("port"), Some(&TomlValue::Integer(8080)));
    }

    #[test]
    fn test_float() {
        let input = "pi = 3.14159";
        let result = parse_toml(input);
        assert!(result.is_ok());
        let doc = result.unwrap();
        if let Some(TomlValue::Float(f)) = doc.root.get("pi") {
            assert!((f - 3.14159).abs() < 0.00001);
        } else {
            panic!("Expected float value");
        }
    }

    #[test]
    fn test_boolean() {
        let input = "enabled = true";
        let result = parse_toml(input);
        assert!(result.is_ok());
        let doc = result.unwrap();
        assert_eq!(doc.root.get("enabled"), Some(&TomlValue::Boolean(true)));
    }

    #[test]
    fn test_array() {
        let input = "colors = [\"red\", \"green\", \"blue\"]";
        let result = parse_toml(input);
        assert!(result.is_ok());
        let doc = result.unwrap();
        if let Some(TomlValue::Array(arr)) = doc.root.get("colors") {
            assert_eq!(arr.len(), 3);
        } else {
            panic!("Expected array value");
        }
    }

    #[test]
    fn test_table() {
        let input = r#"
[server]
host = "localhost"
port = 8080
"#;
        let result = parse_toml(input);
        assert!(result.is_ok());
        let doc = result.unwrap();
        let table = doc.tables.get(&vec!["server".to_string()]);
        assert!(table.is_some());
    }

    #[test]
    fn test_comment() {
        let input = r#"
# This is a comment
key = "value"
"#;
        let result = parse_toml(input);
        assert!(result.is_ok());
    }
}

/// Rust実装の特徴と課題：
///
/// 1. **所有権システム**
///    - パーサー状態の安全な管理
///    - バックトラック時のメモリ安全性保証
///
/// 2. **Result型によるエラーハンドリング**
///    - 明示的なエラー伝播
///    - `?` 演算子による簡潔な記述
///
/// 3. **パターンマッチ**
///    - 値の型に応じた処理の分岐
///    - コンパイル時の網羅性チェック
///
/// 4. **ゼロコスト抽象化**
///    - インライン化による最適化
///    - 実行時オーバーヘッドの最小化
///
/// Remlとの比較：
///
/// - **Rustの利点**:
///   - コンパイル時の型安全性と最適化
///   - 明示的なメモリ管理によるパフォーマンス
///   - 豊富なエコシステムとツール
///
/// - **Rustの課題**:
///   - 手動でのバックトラック管理が煩雑
///   - パーサーコンビネーターライブラリが分散
///   - エラーメッセージの品質向上に追加の実装が必要
///
/// - **Remlの利点**:
///   - 標準ライブラリの統合パーサーコンビネーター
///   - cut/commit/recoverによる高品質なエラー診断
///   - 宣言的な構文による可読性の向上
///   - 字句レイヤの柔軟性