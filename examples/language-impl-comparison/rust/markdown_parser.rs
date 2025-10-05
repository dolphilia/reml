// Markdown風軽量マークアップパーサー - Rust実装
//
// Unicode処理の注意点：
// - Rustの標準String/strは内部的にUTF-8だが、charはUnicodeスカラー値（コードポイント）
// - 書記素クラスター（Grapheme）の扱いにはunicode-segmentationクレートが必要
// - Remlの3層モデル（Byte/Char/Grapheme）と比較すると、Rustは明示的な区別が必要

use std::fmt;

/// Markdown AST のブロック要素。
#[derive(Debug, Clone, PartialEq)]
pub enum Block {
    Heading { level: usize, inline: Vec<Inline> },
    Paragraph { inline: Vec<Inline> },
    UnorderedList { items: Vec<Vec<Inline>> },
    OrderedList { items: Vec<Vec<Inline>> },
    CodeBlock { lang: Option<String>, code: String },
    HorizontalRule,
}

/// Markdown AST のインライン要素。
#[derive(Debug, Clone, PartialEq)]
pub enum Inline {
    Text(String),
    Strong(Vec<Inline>),
    Emphasis(Vec<Inline>),
    Code(String),
    Link { text: Vec<Inline>, url: String },
    LineBreak,
}

pub type Document = Vec<Block>;

#[derive(Debug)]
pub enum ParseError {
    UnexpectedEof,
    InvalidHeading(String),
    InvalidCodeBlock(String),
    Other(String),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ParseError::UnexpectedEof => write!(f, "予期しないファイル終端"),
            ParseError::InvalidHeading(msg) => write!(f, "見出しエラー: {}", msg),
            ParseError::InvalidCodeBlock(msg) => write!(f, "コードブロックエラー: {}", msg),
            ParseError::Other(msg) => write!(f, "{}", msg),
        }
    }
}

/// パーサー状態。
///
/// Unicode処理の課題：
/// - positionはバイト位置（Rustのstrスライスの制約）
/// - char単位の処理には.chars()イテレータが必要
/// - Grapheme単位の処理には外部クレート（unicode-segmentation）が必要
struct Parser<'a> {
    input: &'a str,
    position: usize,  // バイト位置（Rustの制約）
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Parser { input, position: 0 }
    }

    /// 現在位置の1文字（char = Unicodeスカラー値）を取得。
    ///
    /// 注意：Rustのcharはコードポイントだが、Graphemeではない。
    /// 「🇯🇵」のような国旗絵文字は2つのcharとして扱われる。
    fn peek(&self) -> Option<char> {
        self.input[self.position..].chars().next()
    }

    /// 1文字（char）を消費して進める。
    ///
    /// 課題：UTF-8の可変長エンコーディングのため、
    /// len_utf8()でバイト長を計算する必要がある。
    fn advance_char(&mut self) {
        if let Some(ch) = self.peek() {
            self.position += ch.len_utf8();
        }
    }

    /// 固定文字列をマッチ。
    fn match_string(&mut self, target: &str) -> bool {
        if self.input[self.position..].starts_with(target) {
            self.position += target.len();
            true
        } else {
            false
        }
    }

    /// 水平空白をスキップ。
    fn skip_hspace(&mut self) {
        while let Some(ch) = self.peek() {
            if ch == ' ' || ch == '\t' {
                self.advance_char();
            } else {
                break;
            }
        }
    }

    /// 行末まで読む。
    fn read_until_eol(&mut self) -> String {
        let start = self.position;
        while let Some(ch) = self.peek() {
            if ch == '\n' {
                break;
            }
            self.advance_char();
        }
        self.input[start..self.position].to_string()
    }

    /// 改行を消費。
    fn consume_newline(&mut self) -> bool {
        if self.peek() == Some('\n') {
            self.advance_char();
            true
        } else {
            false
        }
    }

    /// EOFチェック。
    fn is_eof(&self) -> bool {
        self.position >= self.input.len()
    }

    /// 見出し行のパース（`# Heading` 形式）。
    fn parse_heading(&mut self) -> Result<Block, ParseError> {
        self.skip_hspace();

        // `#` の連続をカウント
        let mut level = 0;
        while self.peek() == Some('#') {
            level += 1;
            self.advance_char();
        }

        if level == 0 || level > 6 {
            return Err(ParseError::InvalidHeading(
                "見出しレベルは1-6の範囲内である必要があります".to_string(),
            ));
        }

        self.skip_hspace();
        let text = self.read_until_eol();
        self.consume_newline();

        Ok(Block::Heading {
            level,
            inline: vec![Inline::Text(text.trim().to_string())],
        })
    }

    /// 水平線のパース（`---`, `***`, `___`）。
    fn parse_horizontal_rule(&mut self) -> Result<Block, ParseError> {
        self.skip_hspace();
        let text = self.read_until_eol();
        self.consume_newline();

        let trimmed = text.trim();
        let is_rule = (trimmed.chars().all(|c| c == '-') && trimmed.len() >= 3)
            || (trimmed.chars().all(|c| c == '*') && trimmed.len() >= 3)
            || (trimmed.chars().all(|c| c == '_') && trimmed.len() >= 3);

        if is_rule {
            Ok(Block::HorizontalRule)
        } else {
            Err(ParseError::Other("水平線として認識できません".to_string()))
        }
    }

    /// コードブロックのパース（```言語名）。
    fn parse_code_block(&mut self) -> Result<Block, ParseError> {
        if !self.match_string("```") {
            return Err(ParseError::InvalidCodeBlock(
                "コードブロック開始が見つかりません".to_string(),
            ));
        }

        let lang_line = self.read_until_eol();
        self.consume_newline();

        let lang = if lang_line.trim().is_empty() {
            None
        } else {
            Some(lang_line.trim().to_string())
        };

        // コードブロック内容を ```閉じまで読む
        let mut code_lines = Vec::new();
        loop {
            if self.is_eof() {
                break;
            }
            if self.match_string("```") {
                break;
            }
            let line = self.read_until_eol();
            code_lines.push(line);
            self.consume_newline();
        }

        let code = code_lines.join("\n");
        self.consume_newline();

        Ok(Block::CodeBlock { lang, code })
    }

    /// リスト項目のパース（簡易版：`-` または `*`）。
    fn parse_unordered_list(&mut self) -> Result<Block, ParseError> {
        let mut items = Vec::new();

        loop {
            self.skip_hspace();
            match self.peek() {
                Some('-') | Some('*') => {
                    self.advance_char();
                    self.skip_hspace();
                    let text = self.read_until_eol();
                    self.consume_newline();
                    items.push(vec![Inline::Text(text.trim().to_string())]);
                }
                _ => break,
            }
        }

        if items.is_empty() {
            Err(ParseError::Other("リスト項目が見つかりません".to_string()))
        } else {
            Ok(Block::UnorderedList { items })
        }
    }

    /// 段落のパース（簡易版：空行まで）。
    fn parse_paragraph(&mut self) -> Result<Block, ParseError> {
        let mut lines = Vec::new();

        loop {
            if self.is_eof() {
                break;
            }
            if self.peek() == Some('\n') {
                self.advance_char();
                if self.peek() == Some('\n') {
                    break;
                }
                lines.push("".to_string());
            } else {
                let line = self.read_until_eol();
                lines.push(line);
                self.consume_newline();
            }
        }

        let text = lines.join(" ");
        Ok(Block::Paragraph {
            inline: vec![Inline::Text(text.trim().to_string())],
        })
    }

    /// ブロック要素のパース（優先順位付き試行）。
    fn parse_block(&mut self) -> Result<Block, ParseError> {
        // 空行スキップ
        while self.peek() == Some('\n') {
            self.advance_char();
        }

        if self.is_eof() {
            return Err(ParseError::UnexpectedEof);
        }

        self.skip_hspace();

        match self.peek() {
            Some('#') => self.parse_heading(),
            Some('`') if self.input[self.position..].starts_with("```") => self.parse_code_block(),
            Some('-') | Some('*') | Some('_') => {
                // 水平線かリストか判定
                let saved_pos = self.position;
                match self.parse_horizontal_rule() {
                    Ok(block) => Ok(block),
                    Err(_) => {
                        self.position = saved_pos;
                        self.parse_unordered_list()
                    }
                }
            }
            _ => self.parse_paragraph(),
        }
    }
}

/// ドキュメント全体のパース。
pub fn parse(input: &str) -> Result<Document, ParseError> {
    let mut parser = Parser::new(input);
    let mut blocks = Vec::new();

    loop {
        match parser.parse_block() {
            Ok(block) => blocks.push(block),
            Err(ParseError::UnexpectedEof) => break,
            Err(e) => return Err(e),
        }
    }

    Ok(blocks)
}

/// 簡易的なレンダリング（検証用）。
pub fn render_to_string(doc: &Document) -> String {
    fn render_inline(inline: &[Inline]) -> String {
        inline
            .iter()
            .map(|i| match i {
                Inline::Text(s) => s.clone(),
                Inline::Strong(inner) => format!("**{}**", render_inline(inner)),
                Inline::Emphasis(inner) => format!("*{}*", render_inline(inner)),
                Inline::Code(s) => format!("`{}`", s),
                Inline::Link { text, url } => format!("[{}]({})", render_inline(text), url),
                Inline::LineBreak => "\n".to_string(),
            })
            .collect()
    }

    doc.iter()
        .map(|block| match block {
            Block::Heading { level, inline } => {
                format!("{} {}\n\n", "#".repeat(*level), render_inline(inline))
            }
            Block::Paragraph { inline } => format!("{}\n\n", render_inline(inline)),
            Block::UnorderedList { items } => {
                let mut result = String::new();
                for item in items {
                    result.push_str(&format!("- {}\n", render_inline(item)));
                }
                result.push('\n');
                result
            }
            Block::OrderedList { items } => {
                let mut result = String::new();
                for (i, item) in items.iter().enumerate() {
                    result.push_str(&format!("{}. {}\n", i + 1, render_inline(item)));
                }
                result.push('\n');
                result
            }
            Block::CodeBlock { lang, code } => {
                let lang_str = lang.as_deref().unwrap_or("");
                format!("```{}\n{}\n```\n\n", lang_str, code)
            }
            Block::HorizontalRule => "---\n\n".to_string(),
        })
        .collect()
}

// Rustでのグラフェムクラスター処理の課題を示すヘルパー関数
// （unicode-segmentationクレートが必要だが、ここではコメントで示す）

/// Unicode 3層モデル比較：
///
/// Remlでは Byte/Char/Grapheme が型レベルで区別されるが、
/// Rustでは明示的なクレート使用が必要：
///
/// ```rust
/// // use unicode_segmentation::UnicodeSegmentation;
///
/// // pub fn count_graphemes(text: &str) -> usize {
/// //     text.graphemes(true).count()
/// // }
///
/// // pub fn count_codepoints(text: &str) -> usize {
/// //     text.chars().count()
/// // }
///
/// // pub fn byte_length(text: &str) -> usize {
/// //     text.len()
/// // }
/// ```
///
/// この明示性の欠如が、絵文字や結合文字の扱いでバグを生む可能性がある。