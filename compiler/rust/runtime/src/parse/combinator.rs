use crate::run_config::RunConfig;
use crate::text::Str;
use std::any::Any;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

/// Packrat メモキー。
pub type MemoKey = (ParserId, usize);

/// Packrat メモ値（型消去して格納する）。
pub type MemoEntry = Box<dyn Any + Send + Sync>;

/// Packrat メモテーブル。
pub type MemoTable = HashMap<MemoKey, MemoEntry>;

/// パーサー ID。診断や Packrat キーに利用する。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ParserId(u32);

impl ParserId {
    /// 新しい ID を生成する。`rule` などで固定 ID を与える場合は `from_raw` を利用する。
    pub fn fresh() -> Self {
        static NEXT_ID: AtomicU32 = AtomicU32::new(1);
        Self(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }

    /// 外部で決めた ID を固定化する。
    pub fn from_raw(value: u32) -> Self {
        Self(value)
    }

    pub fn as_u32(self) -> u32 {
        self.0
    }
}

/// 現在位置を表す。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InputPosition {
    pub byte: usize,
    pub line: usize,
    pub column: usize,
}

/// 入力ビュー。Arc で共有しつつオフセットのみを進める。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Input {
    source: Arc<str>,
    byte_offset: usize,
    line: usize,
    column: usize,
}

impl Input {
    pub fn new(source: impl AsRef<str>) -> Self {
        let source = Arc::<str>::from(source.as_ref());
        Self {
            source,
            byte_offset: 0,
            line: 1,
            column: 1,
        }
    }

    pub fn remaining(&self) -> &str {
        &self.source[self.byte_offset..]
    }

    pub fn is_empty(&self) -> bool {
        self.byte_offset >= self.source.len()
    }

    pub fn position(&self) -> InputPosition {
        InputPosition {
            byte: self.byte_offset,
            line: self.line,
            column: self.column,
        }
    }

    pub fn byte_offset(&self) -> usize {
        self.byte_offset
    }

    pub fn line(&self) -> usize {
        self.line
    }

    pub fn column(&self) -> usize {
        self.column
    }

    /// 指定バイト数だけ入力を進めた新しいビューを返す。
    pub fn advance(&self, bytes: usize) -> Self {
        let available = self.source.len().saturating_sub(self.byte_offset);
        let step = bytes.min(available);
        let slice = &self.source[self.byte_offset..self.byte_offset + step];
        let mut line = self.line;
        let mut column = self.column;
        let mut last_break = 0usize;
        for (idx, ch) in slice.char_indices() {
            if ch == '\n' {
                line += 1;
                column = 1;
                last_break = idx + ch.len_utf8();
            }
        }
        let tail = &slice[last_break..];
        let graphemes = Str::from(tail).iter_graphemes().count();
        column += graphemes;
        Self {
            source: Arc::clone(&self.source),
            byte_offset: self.byte_offset + step,
            line,
            column,
        }
    }

    pub fn span_to(&self, end: &Input) -> Span {
        Span::new(self.position(), end.position())
    }
}

/// 入力範囲を表す。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Span {
    pub start: InputPosition,
    pub end: InputPosition,
}

impl Span {
    pub fn new(start: InputPosition, end: InputPosition) -> Self {
        Self { start, end }
    }
}

/// パースエラーの骨組み。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    pub message: String,
    pub position: InputPosition,
}

impl ParseError {
    pub fn new(message: impl Into<String>, position: InputPosition) -> Self {
        Self {
            message: message.into(),
            position,
        }
    }
}

/// Parser から返される Reply。consumed/committed を明示する。
#[derive(Clone, Debug)]
pub enum Reply<T> {
    Ok {
        value: T,
        span: Span,
        consumed: bool,
        rest: Input,
    },
    Err {
        error: ParseError,
        consumed: bool,
        committed: bool,
    },
}

/// ランナーが外部へ返す結果。
#[derive(Clone, Debug)]
pub struct ParseResult<T> {
    pub value: Option<T>,
    pub span: Option<Span>,
    pub diagnostics: Vec<ParseError>,
    pub recovered: bool,
    pub legacy_error: Option<ParseError>,
}

impl<T> ParseResult<T> {
    pub fn from_value(value: T, span: Span) -> Self {
        Self {
            value: Some(value),
            span: Some(span),
            diagnostics: Vec::new(),
            recovered: false,
            legacy_error: None,
        }
    }

    pub fn from_error(error: ParseError, legacy_result: bool) -> Self {
        Self {
            value: None,
            span: None,
            diagnostics: vec![error.clone()],
            recovered: false,
            legacy_error: legacy_result.then_some(error),
        }
    }
}

/// パーサー本体。`ParserId` と実行クロージャを保持する。
pub struct Parser<T> {
    id: ParserId,
    f: Arc<dyn Fn(&mut ParseState) -> Reply<T> + Send + Sync>,
}

impl<T> Clone for Parser<T> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            f: Arc::clone(&self.f),
        }
    }
}

impl<T> Parser<T> {
    pub fn new<F>(f: F) -> Self
    where
        F: Fn(&mut ParseState) -> Reply<T> + Send + Sync + 'static,
    {
        Self {
            id: ParserId::fresh(),
            f: Arc::new(f),
        }
    }

    pub fn with_id<F>(id: ParserId, f: F) -> Self
    where
        F: Fn(&mut ParseState) -> Reply<T> + Send + Sync + 'static,
    {
        Self { id, f: Arc::new(f) }
    }

    pub fn id(&self) -> ParserId {
        self.id
    }

    pub fn parse(&self, state: &mut ParseState) -> Reply<T> {
        (self.f)(state)
    }
}

/// パース実行時の可変状態。
#[derive(Clone, Debug)]
pub struct ParseState {
    input: Input,
    pub run_config: RunConfig,
    pub memo: MemoTable,
    pub recovered: bool,
}

impl ParseState {
    pub fn new(source: &str, run_config: RunConfig) -> Self {
        Self {
            input: Input::new(source),
            run_config,
            memo: MemoTable::new(),
            recovered: false,
        }
    }

    pub fn input(&self) -> &Input {
        &self.input
    }

    pub fn set_input(&mut self, input: Input) {
        self.input = input;
    }

    pub fn packrat_enabled(&self) -> bool {
        self.run_config.packrat
    }
}

/// バッチランナー。`require_eof` と Packrat 設定を反映する。
pub fn run<T>(parser: &Parser<T>, input: &str, cfg: &RunConfig) -> ParseResult<T> {
    let mut state = ParseState::new(input, cfg.clone());
    let reply = parser.parse(&mut state);
    match reply {
        Reply::Ok {
            value, span, rest, ..
        } => {
            state.set_input(rest);
            if cfg.require_eof && !state.input().is_empty() {
                let error = ParseError::new("未消費の入力が残っています", state.input().position());
                ParseResult::from_error(error, cfg.legacy_result)
            } else {
                ParseResult::from_value(value, span)
            }
        }
        Reply::Err { error, .. } => ParseResult::from_error(error, cfg.legacy_result),
    }
}

/// RunConfig を指定しない場合のエイリアス。
pub fn run_with_default<T>(parser: &Parser<T>, input: &str) -> ParseResult<T> {
    run(parser, input, &RunConfig::default())
}
