use crate::run_config::RunConfig;
use crate::text::Str;
use serde_json::Value;
use std::any::Any;
use std::collections::HashMap;
use std::fmt;
use std::hash::{Hash, Hasher};
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

fn empty_span(input: &Input) -> Span {
    let pos = input.position();
    Span::new(pos, pos)
}

fn span_from_inputs(start: &Input, end: &Input) -> Span {
    Span::new(start.position(), end.position())
}

fn parser_id_from_name(name: &str) -> ParserId {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    name.hash(&mut hasher);
    let raw = hasher.finish() as u32;
    // 0 を避けるため 1 を足す。
    ParserId::from_raw(raw.wrapping_add(1))
}

/// 簡易的な識別子継続文字の判定。
fn is_ident_continue(ch: char, profile: IdentifierProfile) -> bool {
    match profile {
        IdentifierProfile::Unicode => ch == '_' || ch.is_alphanumeric(),
        IdentifierProfile::AsciiCompat => ch == '_' || ch.is_ascii_alphanumeric(),
    }
}

/// `RunConfig.extensions["lex"].identifier_profile` から派生した識別子プロファイル。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IdentifierProfile {
    Unicode,
    AsciiCompat,
}

impl IdentifierProfile {
    fn from_run_config(cfg: &RunConfig) -> Self {
        cfg.extensions
            .get("lex")
            .and_then(|ext| ext.get("identifier_profile"))
            .and_then(|value| value.as_str())
            .map(|label| match label {
                "ascii-compat" => IdentifierProfile::AsciiCompat,
                _ => IdentifierProfile::Unicode,
            })
            .unwrap_or(IdentifierProfile::Unicode)
    }
}

fn decode_lex_space(run_config: &RunConfig) -> Option<Parser<()>> {
    let lex = run_config.extensions.get("lex")?;
    let ascii_only = lex
        .get("profile")
        .and_then(Value::as_str)
        .map(|label| label == "ascii-compat")
        .unwrap_or(false);
    let parser_id = lex
        .get("space_id")
        .and_then(Value::as_u64)
        .map(|raw| ParserId::from_raw(raw as u32))
        .unwrap_or_else(ParserId::fresh);
    let space = Parser::with_id(parser_id, move |state| {
        let start = state.input().clone();
        let mut last = None;
        for (idx, ch) in start.remaining().char_indices() {
            let is_ws = if ascii_only {
                ch.is_ascii_whitespace()
            } else {
                ch.is_whitespace()
            };
            if is_ws {
                last = Some(idx + ch.len_utf8());
            } else {
                break;
            }
        }

        if let Some(boundary) = last {
            let rest = start.advance(boundary);
            state.set_input(rest.clone());
            Reply::Ok {
                value: (),
                span: span_from_inputs(&start, &rest),
                consumed: true,
                rest,
            }
        } else {
            Reply::Ok {
                value: (),
                span: empty_span(&start),
                consumed: false,
                rest: start,
            }
        }
    });
    Some(space)
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

impl<T> fmt::Debug for Parser<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Parser").field("id", &self.id).finish()
    }
}

impl<T: Send + Sync + 'static> Parser<T> {
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

    /// 既定の空白パーサーをスコープに設定する。
    pub fn with_space(self, space: Parser<()>) -> Parser<T> {
        Parser::with_id(self.id, move |state| {
            let previous = state.space();
            state.set_space(Some(space.clone()));
            let result = self.parse(state);
            state.set_space(previous);
            result
        })
    }

    /// 値を変換する。
    pub fn map<U, F>(self, f: F) -> Parser<U>
    where
        U: Send + Sync + 'static,
        F: Fn(T) -> U + Send + Sync + 'static,
    {
        Parser::with_id(self.id, move |state| match self.parse(state) {
            Reply::Ok {
                value,
                span,
                consumed,
                rest,
            } => {
                state.set_input(rest.clone());
                Reply::Ok {
                    value: f(value),
                    span,
                    consumed,
                    rest,
                }
            }
            Reply::Err {
                error,
                consumed,
                committed,
            } => Reply::Err {
                error,
                consumed,
                committed,
            },
        })
    }

    /// 直列合成。
    pub fn then<U>(self, other: Parser<U>) -> Parser<(T, U)>
    where
        U: Send + Sync + 'static,
    {
        Parser::new(move |state| {
            let start_input = state.input().clone();
            match self.parse(state) {
                Reply::Ok {
                    value: left,
                    span: _,
                    consumed: left_consumed,
                    rest: left_rest,
                } => {
                    state.set_input(left_rest.clone());
                    match other.parse(state) {
                        Reply::Ok {
                            value: right,
                            span: _right_span,
                            consumed: right_consumed,
                            rest,
                        } => {
                            let span = span_from_inputs(&start_input, &rest);
                            state.set_input(rest.clone());
                            Reply::Ok {
                                value: (left, right),
                                span,
                                consumed: left_consumed || right_consumed,
                                rest,
                            }
                        }
                        Reply::Err {
                            error,
                            consumed,
                            committed,
                        } => {
                            state.set_input(start_input);
                            Reply::Err {
                                error,
                                consumed: left_consumed || consumed,
                                committed,
                            }
                        }
                    }
                }
                Reply::Err {
                    error,
                    consumed,
                    committed,
                } => {
                    state.set_input(start_input);
                    Reply::Err {
                        error,
                        consumed,
                        committed,
                    }
                }
            }
        })
    }

    /// flatMap 相当。
    pub fn and_then<U, F>(self, f: F) -> Parser<U>
    where
        U: Send + Sync + 'static,
        F: Fn(T) -> Parser<U> + Send + Sync + 'static,
    {
        Parser::new(move |state| {
            let start_input = state.input().clone();
            match self.parse(state) {
                Reply::Ok {
                    value,
                    consumed,
                    rest,
                    ..
                } => {
                    state.set_input(rest.clone());
                    let next = f(value);
                    match next.parse(state) {
                        Reply::Ok {
                            value: out,
                            span,
                            consumed: next_consumed,
                            rest,
                        } => {
                            state.set_input(rest.clone());
                            Reply::Ok {
                                value: out,
                                span,
                                consumed: consumed || next_consumed,
                                rest,
                            }
                        }
                        Reply::Err {
                            error,
                            consumed: next_consumed,
                            committed,
                        } => {
                            state.set_input(start_input);
                            Reply::Err {
                                error,
                                consumed: consumed || next_consumed,
                                committed,
                            }
                        }
                    }
                }
                Reply::Err {
                    error,
                    consumed,
                    committed,
                } => {
                    state.set_input(start_input);
                    Reply::Err {
                        error,
                        consumed,
                        committed,
                    }
                }
            }
        })
    }

    /// 左側を捨てて右側を返す。
    pub fn skip_l<U>(self, other: Parser<U>) -> Parser<U>
    where
        U: Send + Sync + 'static,
    {
        self.then(other).map(|(_, r)| r)
    }

    /// 右側を捨てて左側を返す。
    pub fn skip_r<U>(self, other: Parser<U>) -> Parser<T>
    where
        U: Send + Sync + 'static,
    {
        self.then(other).map(|(l, _)| l)
    }

    /// 左優先の選択。
    pub fn or(self, other: Parser<T>) -> Parser<T> {
        Parser::new(move |state| {
            let start_input = state.input().clone();
            match self.parse(state) {
                ok @ Reply::Ok { .. } => ok,
                Reply::Err {
                    error,
                    consumed,
                    committed,
                } => {
                    if consumed || committed {
                        state.set_input(start_input);
                        Reply::Err {
                            error,
                            consumed,
                            committed,
                        }
                    } else {
                        state.set_input(start_input.clone());
                        other.parse(state)
                    }
                }
            }
        })
    }

    /// 値を変換しつつ committed を付与する。
    pub fn cut(self) -> Parser<T> {
        Parser::with_id(self.id, move |state| match self.parse(state) {
            Reply::Ok {
                value,
                span,
                consumed,
                rest,
            } => {
                state.set_input(rest.clone());
                Reply::Ok {
                    value,
                    span,
                    consumed,
                    rest,
                }
            }
            Reply::Err {
                error,
                consumed,
                committed: _,
            } => Reply::Err {
                error,
                consumed,
                committed: true,
            },
        })
    }

    /// 直近の消費を巻き戻し、空失敗に変換する。
    pub fn attempt(self) -> Parser<T> {
        Parser::with_id(self.id, move |state| {
            let start_input = state.input().clone();
            match self.parse(state) {
                Reply::Ok {
                    value,
                    span,
                    consumed,
                    rest,
                } => {
                    state.set_input(rest.clone());
                    Reply::Ok {
                        value,
                        span,
                        consumed,
                        rest,
                    }
                }
                Reply::Err { error, .. } => {
                    state.set_input(start_input);
                    Reply::Err {
                        error,
                        consumed: false,
                        committed: false,
                    }
                }
            }
        })
    }

    /// 失敗時に until まで読み飛ばし、with の値で継続する。
    pub fn recover(self, until: Parser<()>, with: T) -> Parser<T>
    where
        T: Clone + 'static,
    {
        Parser::new(move |state| {
            let start_input = state.input().clone();
            match self.parse(state) {
                ok @ Reply::Ok { .. } => ok,
                Reply::Err {
                    error,
                    consumed,
                    committed,
                } => {
                    if committed {
                        state.set_input(start_input);
                        return Reply::Err {
                            error,
                            consumed,
                            committed,
                        };
                    }

                    state.set_input(start_input.clone());
                    let mut cursor = start_input.clone();
                    loop {
                        state.set_input(cursor.clone());
                        match until.parse(state) {
                            Reply::Ok {
                                rest,
                                consumed: until_consumed,
                                ..
                            } => {
                                if !until_consumed {
                                    let err = ParseError::new(
                                        "recover until が空成功しました",
                                        cursor.position(),
                                    );
                                    state.set_input(start_input);
                                    return Reply::Err {
                                        error: err,
                                        consumed: false,
                                        committed: false,
                                    };
                                }
                                state.set_input(rest.clone());
                                state.recovered = true;
                                let span = span_from_inputs(&start_input, &rest);
                                return Reply::Ok {
                                    value: with.clone(),
                                    span,
                                    consumed: true,
                                    rest,
                                };
                            }
                            Reply::Err {
                                consumed: until_consumed,
                                committed: until_committed,
                                error: until_err,
                            } => {
                                if until_consumed || until_committed {
                                    state.set_input(start_input);
                                    return Reply::Err {
                                        error: until_err,
                                        consumed: true,
                                        committed: until_committed,
                                    };
                                }
                            }
                        }

                        if cursor.is_empty() {
                            state.set_input(start_input);
                            return Reply::Err {
                                error,
                                consumed,
                                committed: false,
                            };
                        }

                        if let Some((idx, ch)) = cursor.remaining().char_indices().next() {
                            let step = ch.len_utf8().max(1);
                            cursor = cursor.advance(idx + step);
                        } else {
                            state.set_input(start_input);
                            return Reply::Err {
                                error,
                                consumed,
                                committed: false,
                            };
                        }
                    }
                }
            }
        })
    }

    /// トレースフラグを尊重して内側を実行する（現状は透過）。
    pub fn trace(self) -> Parser<T> {
        Parser::with_id(self.id, move |state| self.parse(state))
    }

    /// 0 回または 1 回。
    pub fn opt(self) -> Parser<Option<T>> {
        Parser::new(move |state| {
            let start_input = state.input().clone();
            match self.parse(state) {
                Reply::Ok {
                    value,
                    span,
                    consumed,
                    rest,
                } => {
                    state.set_input(rest.clone());
                    Reply::Ok {
                        value: Some(value),
                        span,
                        consumed,
                        rest,
                    }
                }
                Reply::Err {
                    consumed,
                    committed,
                    error,
                } => {
                    if consumed || committed {
                        state.set_input(start_input);
                        Reply::Err {
                            error,
                            consumed,
                            committed,
                        }
                    } else {
                        state.set_input(start_input.clone());
                        Reply::Ok {
                            value: None,
                            span: empty_span(&start_input),
                            consumed: false,
                            rest: start_input,
                        }
                    }
                }
            }
        })
    }

    /// 0 回以上。
    pub fn many(self) -> Parser<Vec<T>> {
        Parser::new(move |state| {
            let mut values = Vec::new();
            let start_input = state.input().clone();
            let mut current_input = start_input.clone();
            let mut any_consumed = false;

            loop {
                state.set_input(current_input.clone());
                match self.parse(state) {
                    Reply::Ok {
                        value,
                        consumed,
                        rest,
                        ..
                    } => {
                        if !consumed {
                            let err = ParseError::new(
                                "繰り返し本体が空成功しました",
                                current_input.position(),
                            );
                            state.set_input(start_input);
                            return Reply::Err {
                                error: err,
                                consumed: false,
                                committed: false,
                            };
                        }
                        any_consumed = true;
                        current_input = rest.clone();
                        values.push(value);
                    }
                    Reply::Err {
                        consumed,
                        committed,
                        error,
                    } => {
                        if consumed || committed {
                            state.set_input(start_input);
                            return Reply::Err {
                                error,
                                consumed,
                                committed,
                            };
                        } else {
                            state.set_input(current_input.clone());
                            let span = span_from_inputs(&start_input, &current_input);
                            return Reply::Ok {
                                value: values,
                                span,
                                consumed: any_consumed,
                                rest: current_input,
                            };
                        }
                    }
                }
            }
        })
    }

    /// 1 回以上。
    pub fn many1(self) -> Parser<Vec<T>> {
        Parser::new(move |state| {
            let start_input = state.input().clone();
            match self.clone().many().parse(state) {
                Reply::Ok {
                    value,
                    consumed,
                    rest,
                    span,
                } => {
                    if value.is_empty() {
                        state.set_input(start_input);
                        let err = ParseError::new(
                            "1 回以上の繰り返しが必要です",
                            state.input().position(),
                        );
                        Reply::Err {
                            error: err,
                            consumed: false,
                            committed: false,
                        }
                    } else {
                        state.set_input(rest.clone());
                        Reply::Ok {
                            value,
                            span,
                            consumed,
                            rest,
                        }
                    }
                }
                err @ Reply::Err { .. } => err,
            }
        })
    }

    /// 繰り返し回数を指定する。
    pub fn repeat(self, min: usize, max: Option<usize>) -> Parser<Vec<T>> {
        Parser::new(move |state| {
            let mut values = Vec::new();
            let start_input = state.input().clone();
            let mut current_input = start_input.clone();
            let mut any_consumed = false;

            loop {
                if let Some(limit) = max {
                    if values.len() >= limit {
                        state.set_input(current_input.clone());
                        let span = span_from_inputs(&start_input, &current_input);
                        return Reply::Ok {
                            value: values,
                            span,
                            consumed: any_consumed,
                            rest: current_input,
                        };
                    }
                }

                state.set_input(current_input.clone());
                match self.parse(state) {
                    Reply::Ok {
                        value,
                        consumed,
                        rest,
                        ..
                    } => {
                        if !consumed {
                            let err = ParseError::new(
                                "繰り返し本体が空成功しました",
                                current_input.position(),
                            );
                            state.set_input(start_input);
                            return Reply::Err {
                                error: err,
                                consumed: false,
                                committed: false,
                            };
                        }
                        any_consumed = true;
                        current_input = rest.clone();
                        values.push(value);
                    }
                    Reply::Err {
                        consumed,
                        committed,
                        error,
                    } => {
                        if consumed || committed {
                            state.set_input(start_input);
                            return Reply::Err {
                                error,
                                consumed,
                                committed,
                            };
                        } else if values.len() >= min {
                            state.set_input(current_input.clone());
                            let span = span_from_inputs(&start_input, &current_input);
                            return Reply::Ok {
                                value: values,
                                span,
                                consumed: any_consumed,
                                rest: current_input,
                            };
                        } else {
                            state.set_input(start_input);
                            let err = ParseError::new(
                                "指定回数の繰り返しに失敗しました",
                                current_input.position(),
                            );
                            return Reply::Err {
                                error: err,
                                consumed: false,
                                committed: false,
                            };
                        }
                    }
                }
            }
        })
    }

    /// セパレータ区切り（0 回以上）。
    pub fn sep_by<U>(self, sep: Parser<U>) -> Parser<Vec<T>>
    where
        U: Send + Sync + 'static,
    {
        Parser::new(move |state| {
            let start_input = state.input().clone();
            let mut values = Vec::new();
            let mut current_input = start_input.clone();

            state.set_input(current_input.clone());
            match self.parse(state) {
                Reply::Ok {
                    value,
                    consumed,
                    rest,
                    ..
                } => {
                    if !consumed {
                        let err = ParseError::new(
                            "繰り返し本体が空成功しました",
                            current_input.position(),
                        );
                        state.set_input(start_input);
                        return Reply::Err {
                            error: err,
                            consumed: false,
                            committed: false,
                        };
                    }
                    values.push(value);
                    current_input = rest.clone();
                }
                Reply::Err {
                    consumed,
                    committed,
                    error,
                } => {
                    if consumed || committed {
                        state.set_input(start_input);
                        return Reply::Err {
                            error,
                            consumed,
                            committed,
                        };
                    } else {
                        state.set_input(start_input.clone());
                        return Reply::Ok {
                            value: values,
                            span: empty_span(&start_input),
                            consumed: false,
                            rest: start_input,
                        };
                    }
                }
            }

            loop {
                state.set_input(current_input.clone());
                match sep.parse(state) {
                    Reply::Ok {
                        consumed: sep_consumed,
                        rest: sep_rest,
                        ..
                    } => {
                        if !sep_consumed {
                            let err = ParseError::new(
                                "セパレータが空成功しました",
                                current_input.position(),
                            );
                            state.set_input(start_input);
                            return Reply::Err {
                                error: err,
                                consumed: false,
                                committed: false,
                            };
                        }
                        current_input = sep_rest.clone();
                    }
                    Reply::Err {
                        consumed: sep_consumed,
                        committed: sep_committed,
                        error: sep_error,
                    } => {
                        if sep_consumed || sep_committed {
                            state.set_input(start_input);
                            return Reply::Err {
                                error: sep_error,
                                consumed: sep_consumed,
                                committed: sep_committed,
                            };
                        } else {
                            state.set_input(current_input.clone());
                            let span = span_from_inputs(&start_input, &current_input);
                            return Reply::Ok {
                                value: values,
                                span,
                                consumed: true,
                                rest: current_input,
                            };
                        }
                    }
                }

                state.set_input(current_input.clone());
                match self.parse(state) {
                    Reply::Ok {
                        value,
                        consumed,
                        rest,
                        ..
                    } => {
                        if !consumed {
                            let err = ParseError::new(
                                "繰り返し本体が空成功しました",
                                current_input.position(),
                            );
                            state.set_input(start_input);
                            return Reply::Err {
                                error: err,
                                consumed: false,
                                committed: false,
                            };
                        }
                        values.push(value);
                        current_input = rest.clone();
                    }
                    Reply::Err {
                        consumed: item_consumed,
                        committed: item_committed,
                        error: item_error,
                    } => {
                        if item_consumed || item_committed {
                            state.set_input(start_input);
                            return Reply::Err {
                                error: item_error,
                                consumed: item_consumed,
                                committed: item_committed,
                            };
                        } else {
                            state.set_input(start_input);
                            let err = ParseError::new(
                                "区切りの後に要素が見つかりません",
                                current_input.position(),
                            );
                            return Reply::Err {
                                error: err,
                                consumed: true,
                                committed: false,
                            };
                        }
                    }
                }
            }
        })
    }

    /// セパレータ区切り（1 回以上）。
    pub fn sep_by1<U>(self, sep: Parser<U>) -> Parser<Vec<T>>
    where
        U: Send + Sync + 'static,
    {
        Parser::new(
            move |state| match self.clone().sep_by(sep.clone()).parse(state) {
                Reply::Ok {
                    value,
                    consumed,
                    rest,
                    span,
                } => {
                    if value.is_empty() {
                        let err = ParseError::new(
                            "1 回以上の繰り返しが必要です",
                            state.input().position(),
                        );
                        Reply::Err {
                            error: err,
                            consumed: false,
                            committed: false,
                        }
                    } else {
                        Reply::Ok {
                            value,
                            consumed,
                            rest,
                            span,
                        }
                    }
                }
                err @ Reply::Err { .. } => err,
            },
        )
    }

    /// end まで読み続ける。
    pub fn many_till<U>(self, end: Parser<U>) -> Parser<Vec<T>>
    where
        U: Send + Sync + 'static,
    {
        Parser::new(move |state| {
            let mut values = Vec::new();
            let start_input = state.input().clone();
            let mut current_input = start_input.clone();

            loop {
                state.set_input(current_input.clone());
                match end.parse(state) {
                    Reply::Ok {
                        rest,
                        consumed: end_consumed,
                        ..
                    } => {
                        state.set_input(rest.clone());
                        let span = span_from_inputs(&start_input, &rest);
                        let consumed_flag = !values.is_empty() || end_consumed;
                        return Reply::Ok {
                            value: values,
                            span,
                            consumed: consumed_flag,
                            rest,
                        };
                    }
                    Reply::Err {
                        consumed: end_consumed,
                        committed: end_committed,
                        error: end_error,
                    } => {
                        if end_consumed || end_committed {
                            state.set_input(start_input);
                            return Reply::Err {
                                error: end_error,
                                consumed: end_consumed,
                                committed: end_committed,
                            };
                        }
                    }
                }

                state.set_input(current_input.clone());
                match self.parse(state) {
                    Reply::Ok {
                        value,
                        consumed,
                        rest,
                        ..
                    } => {
                        if !consumed {
                            let err = ParseError::new(
                                "繰り返し本体が空成功しました",
                                current_input.position(),
                            );
                            state.set_input(start_input);
                            return Reply::Err {
                                error: err,
                                consumed: false,
                                committed: false,
                            };
                        }
                        current_input = rest.clone();
                        values.push(value);
                    }
                    Reply::Err {
                        consumed,
                        committed,
                        error,
                    } => {
                        if consumed || committed {
                            state.set_input(start_input);
                            return Reply::Err {
                                error,
                                consumed,
                                committed,
                            };
                        } else {
                            state.set_input(start_input);
                            return Reply::Err {
                                error,
                                consumed: false,
                                committed: false,
                            };
                        }
                    }
                }
            }
        })
    }

    /// 先読み（非消費）。
    pub fn lookahead(self) -> Parser<T>
    where
        T: Clone + 'static,
    {
        Parser::new(move |state| {
            let start_input = state.input().clone();
            match self.parse(state) {
                Reply::Ok { value, span, .. } => {
                    state.set_input(start_input.clone());
                    Reply::Ok {
                        value: value.clone(),
                        span,
                        consumed: false,
                        rest: start_input,
                    }
                }
                Reply::Err {
                    error, committed, ..
                } => {
                    state.set_input(start_input);
                    Reply::Err {
                        error,
                        consumed: false,
                        committed,
                    }
                }
            }
        })
    }

    /// 否定先読み。
    pub fn not_followed_by(self) -> Parser<()> {
        Parser::new(move |state| {
            let start_input = state.input().clone();
            match self.parse(state) {
                Reply::Ok { span, .. } => {
                    state.set_input(start_input.clone());
                    let err = ParseError::new("後続禁止のパターンにマッチしました", span.start);
                    Reply::Err {
                        error: err,
                        consumed: false,
                        committed: false,
                    }
                }
                Reply::Err {
                    consumed,
                    committed,
                    error,
                } => {
                    state.set_input(start_input.clone());
                    if consumed || committed {
                        Reply::Err {
                            error,
                            consumed,
                            committed,
                        }
                    } else {
                        Reply::Ok {
                            value: (),
                            span: empty_span(&start_input),
                            consumed: false,
                            rest: start_input,
                        }
                    }
                }
            }
        })
    }

    /// 値とスパンを返す。
    pub fn spanned(self) -> Parser<(T, Span)> {
        Parser::new(move |state| match self.parse(state) {
            Reply::Ok {
                value,
                span,
                consumed,
                rest,
            } => {
                state.set_input(rest.clone());
                Reply::Ok {
                    value: (value, span.clone()),
                    span,
                    consumed,
                    rest,
                }
            }
            Reply::Err {
                error,
                consumed,
                committed,
            } => Reply::Err {
                error,
                consumed,
                committed,
            },
        })
    }

    /// 左結合のチェーン。
    pub fn chainl1<F>(self, op: Parser<F>) -> Parser<T>
    where
        T: Clone,
        F: Fn(T, T) -> T + Send + Sync + 'static,
    {
        Parser::new(move |state| {
            let start_input = state.input().clone();
            let mut current_input = start_input.clone();
            state.set_input(current_input.clone());
            let mut reply = match self.parse(state) {
                Reply::Ok {
                    value,
                    span,
                    consumed,
                    rest,
                } => {
                    current_input = rest.clone();
                    (value, span, consumed)
                }
                err @ Reply::Err { .. } => {
                    state.set_input(start_input);
                    return err;
                }
            };

            let mut acc = reply.0;
            let mut acc_span = reply.1;
            let mut any_consumed = reply.2;

            loop {
                let iter_input = current_input.clone();
                state.set_input(iter_input.clone());
                let step = op.clone().then(self.clone()).attempt();
                match step.parse(state) {
                    Reply::Ok {
                        value: (f, rhs),
                        span: rhs_span,
                        consumed,
                        rest,
                    } => {
                        if !consumed {
                            let err = ParseError::new(
                                "繰り返し本体が空成功しました",
                                iter_input.position(),
                            );
                            state.set_input(start_input);
                            return Reply::Err {
                                error: err,
                                consumed: false,
                                committed: false,
                            };
                        }
                        acc = f(acc, rhs);
                        acc_span = Span::new(acc_span.start, rhs_span.end);
                        any_consumed = true;
                        current_input = rest.clone();
                    }
                    Reply::Err {
                        consumed,
                        committed,
                        error,
                    } => {
                        state.set_input(iter_input);
                        if consumed || committed {
                            state.set_input(start_input);
                            return Reply::Err {
                                error,
                                consumed,
                                committed,
                            };
                        } else {
                            state.set_input(current_input.clone());
                            let span = Span::new(acc_span.start, current_input.position());
                            return Reply::Ok {
                                value: acc,
                                span,
                                consumed: any_consumed,
                                rest: current_input,
                            };
                        }
                    }
                }
            }
        })
    }

    /// 右結合のチェーン。
    pub fn chainr1<F>(self, op: Parser<F>) -> Parser<T>
    where
        T: Clone,
        F: Fn(T, T) -> T + Send + Sync + 'static,
    {
        Parser::new(move |state| {
            let start_input = state.input().clone();
            state.set_input(start_input.clone());
            let first = match self.parse(state) {
                Reply::Ok {
                    value,
                    consumed,
                    rest,
                    ..
                } => (value, consumed, rest),
                err @ Reply::Err { .. } => return err,
            };

            let mut operands = vec![first.0];
            let mut operators: Vec<F> = Vec::new();
            let mut consumed_any = first.1;
            let mut current_input = first.2.clone();

            loop {
                state.set_input(current_input.clone());
                let step = op.clone().then(self.clone()).attempt();
                match step.parse(state) {
                    Reply::Ok {
                        value: (f, rhs),
                        consumed,
                        rest,
                        ..
                    } => {
                        if !consumed {
                            let err = ParseError::new(
                                "繰り返し本体が空成功しました",
                                current_input.position(),
                            );
                            state.set_input(start_input);
                            return Reply::Err {
                                error: err,
                                consumed: false,
                                committed: false,
                            };
                        }
                        operators.push(f);
                        operands.push(rhs);
                        consumed_any = true;
                        current_input = rest.clone();
                    }
                    Reply::Err {
                        consumed,
                        committed,
                        error,
                    } => {
                        if consumed || committed {
                            state.set_input(start_input);
                            return Reply::Err {
                                error,
                                consumed,
                                committed,
                            };
                        }
                        break;
                    }
                }
            }

            let mut result = operands
                .pop()
                .expect("chainr1 で operands が空になることはありません");
            while let Some(lhs) = operands.pop() {
                if let Some(op_fn) = operators.pop() {
                    result = op_fn(lhs, result);
                }
            }

            let span = span_from_inputs(&start_input, &current_input);
            state.set_input(current_input.clone());
            Reply::Ok {
                value: result,
                span,
                consumed: consumed_any,
                rest: current_input,
            }
        })
    }
}

impl Parser<()> {
    /// 空白パーサーの安定 ID を取得する。
    pub fn space_id(&self) -> ParserId {
        // ID 割り当ての安定性は `rule` 由来に依存する。Lex ブリッジ連携時に共有する。
        self.id
    }
}

/// 非消費で成功するパーサー。
pub fn ok<T>(value: T) -> Parser<T>
where
    T: Clone + Send + Sync + 'static,
{
    Parser::new(move |state| Reply::Ok {
        value: value.clone(),
        span: empty_span(state.input()),
        consumed: false,
        rest: state.input().clone(),
    })
}

/// 非消費で失敗するパーサー。
pub fn fail<T>(message: impl Into<String>) -> Parser<T>
where
    T: Send + Sync + 'static,
{
    let msg = message.into();
    Parser::new(move |state| Reply::Err {
        error: ParseError::new(msg.clone(), state.input().position()),
        consumed: false,
        committed: false,
    })
}

/// 入力終端のみ成功する。
pub fn eof() -> Parser<()> {
    Parser::new(|state| {
        if state.input().is_empty() {
            Reply::Ok {
                value: (),
                span: empty_span(state.input()),
                consumed: false,
                rest: state.input().clone(),
            }
        } else {
            Reply::Err {
                error: ParseError::new("入力の終端を期待しました", state.input().position()),
                consumed: false,
                committed: false,
            }
        }
    })
}

/// 名前付きパーサー（ParserId を固定化する）。
pub fn rule<T>(name: impl AsRef<str>, parser: Parser<T>) -> Parser<T>
where
    T: Send + Sync + 'static,
{
    let id = parser_id_from_name(name.as_ref());
    Parser::with_id(id, move |state| parser.parse(state))
}

/// エラー時のラベルを差し替える。
pub fn label<T>(name: impl Into<String>, parser: Parser<T>) -> Parser<T>
where
    T: Send + Sync + 'static,
{
    let label = name.into();
    Parser::new(move |state| match parser.parse(state) {
        Reply::Ok {
            value,
            span,
            consumed,
            rest,
        } => {
            state.set_input(rest.clone());
            Reply::Ok {
                value,
                span,
                consumed,
                rest,
            }
        }
        Reply::Err {
            consumed,
            committed,
            ..
        } => Reply::Err {
            error: ParseError::new(label.clone(), state.input().position()),
            consumed,
            committed,
        },
    })
}

/// 選択肢の列を左から試す。
pub fn choice<T>(parsers: Vec<Parser<T>>) -> Parser<T>
where
    T: Send + Sync + 'static,
{
    parsers
        .into_iter()
        .reduce(|acc, p| acc.or(p))
        .unwrap_or_else(|| fail("選択肢がありません"))
}

/// ゼロ幅コミット。
pub fn cut_here() -> Parser<()> {
    Parser::new(|state| Reply::Ok {
        value: (),
        span: empty_span(state.input()),
        consumed: true,
        rest: state.input().clone(),
    })
}

/// 2 つのパーサーの間に挟む。
pub fn between<A>(open: Parser<()>, parser: Parser<A>, close: Parser<()>) -> Parser<A>
where
    A: Send + Sync + 'static,
{
    open.skip_l(parser).skip_r(close)
}

/// 前置パーサーを読み捨てる。
pub fn preceded<A, B>(pre: Parser<A>, parser: Parser<B>) -> Parser<B>
where
    A: Send + Sync + 'static,
    B: Send + Sync + 'static,
{
    pre.skip_l(parser)
}

/// 後置パーサーを読み捨てる。
pub fn terminated<A, B>(parser: Parser<A>, post: Parser<B>) -> Parser<A>
where
    A: Send + Sync + 'static,
    B: Send + Sync + 'static,
{
    parser.skip_r(post)
}

/// a b c の中央だけを返す。
pub fn delimited<A, B, C>(a: Parser<A>, b: Parser<B>, c: Parser<C>) -> Parser<B>
where
    A: Send + Sync + 'static,
    B: Send + Sync + 'static,
    C: Send + Sync + 'static,
{
    a.skip_l(b).skip_r(c)
}

/// 先読み。
pub fn lookahead<T>(parser: Parser<T>) -> Parser<T>
where
    T: Clone + Send + Sync + 'static,
{
    parser.lookahead()
}

/// 否定先読み。
pub fn not_followed_by<T>(parser: Parser<T>) -> Parser<()>
where
    T: Send + Sync + 'static,
{
    parser.not_followed_by()
}

/// 後続の空白をまとめて処理する。
pub fn lexeme<A, S>(space: S, parser: Parser<A>) -> Parser<A>
where
    A: Send + Sync + 'static,
    S: Into<Option<Parser<()>>>,
{
    let space = space.into();
    Parser::new(move |state| {
        let start_input = state.input().clone();
        match parser.parse(state) {
            Reply::Ok {
                value,
                span,
                consumed,
                rest,
            } => {
                state.set_input(rest.clone());
                let mut tail_input = rest.clone();
                let mut consumed_flag = consumed;
                if let Some(space_parser) = space.clone().or_else(|| state.space()) {
                    state.set_input(tail_input.clone());
                    match space_parser.parse(state) {
                        Reply::Ok {
                            rest: space_rest,
                            consumed: space_consumed,
                            ..
                        } => {
                            consumed_flag = consumed_flag || space_consumed;
                            tail_input = space_rest.clone();
                            state.set_input(space_rest);
                        }
                        Reply::Err {
                            error,
                            consumed: space_consumed,
                            committed,
                        } => {
                            if space_consumed || committed {
                                state.set_input(start_input);
                                return Reply::Err {
                                    error,
                                    consumed: true,
                                    committed,
                                };
                            } else {
                                state.set_input(tail_input.clone());
                            }
                        }
                    }
                }
                Reply::Ok {
                    value,
                    span,
                    consumed: consumed_flag,
                    rest: tail_input,
                }
            }
            Reply::Err {
                error,
                consumed,
                committed,
            } => Reply::Err {
                error,
                consumed,
                committed,
            },
        }
    })
}

/// 記号を読み取り、後続の空白もまとめて消費する。
pub fn symbol<S>(space: S, text: impl AsRef<str>) -> Parser<()>
where
    S: Into<Option<Parser<()>>>,
{
    let text = text.as_ref().to_string();
    let space = space.into();
    lexeme(
        space,
        Parser::new(move |state| {
            if text.is_empty() {
                return Reply::Err {
                    error: ParseError::new(
                        "空の記号は許可されていません",
                        state.input().position(),
                    ),
                    consumed: false,
                    committed: false,
                };
            }
            let start_input = state.input().clone();
            let remaining = start_input.remaining();
            if remaining.starts_with(&text) {
                let rest = start_input.advance(text.len());
                state.set_input(rest.clone());
                Reply::Ok {
                    value: (),
                    span: span_from_inputs(&start_input, &rest),
                    consumed: true,
                    rest,
                }
            } else {
                Reply::Err {
                    error: ParseError::new(
                        format!("期待した記号: {}", text),
                        state.input().position(),
                    ),
                    consumed: false,
                    committed: false,
                }
            }
        }),
    )
}

/// キーワードを読み取り、識別子境界を検査する。
pub fn keyword<S>(space: S, kw: impl AsRef<str>) -> Parser<()>
where
    S: Into<Option<Parser<()>>>,
{
    let kw = kw.as_ref().to_string();
    let space = space.into();
    lexeme(
        space,
        Parser::new(move |state| {
            if kw.is_empty() {
                return Reply::Err {
                    error: ParseError::new(
                        "空のキーワードは許可されていません",
                        state.input().position(),
                    ),
                    consumed: false,
                    committed: false,
                };
            }
            let start_input = state.input().clone();
            let remaining = start_input.remaining();
            if remaining.starts_with(&kw) {
                let rest = start_input.advance(kw.len());
                if let Some(ch) = rest.remaining().chars().next() {
                    if is_ident_continue(ch, state.identifier_profile()) {
                        state.set_input(start_input);
                        return Reply::Err {
                            error: ParseError::new(
                                format!("キーワード '{}' の後ろに識別子が続いています", kw),
                                rest.position(),
                            ),
                            consumed: true,
                            committed: false,
                        };
                    }
                }
                state.set_input(rest.clone());
                Reply::Ok {
                    value: (),
                    span: span_from_inputs(&start_input, &rest),
                    consumed: true,
                    rest,
                }
            } else {
                Reply::Err {
                    error: ParseError::new(
                        format!("期待したキーワード: {}", kw),
                        state.input().position(),
                    ),
                    consumed: false,
                    committed: false,
                }
            }
        }),
    )
}

/// 位置パーサー。
pub fn position() -> Parser<Span> {
    Parser::new(|state| {
        let span = empty_span(state.input());
        Reply::Ok {
            value: span.clone(),
            span,
            consumed: false,
            rest: state.input().clone(),
        }
    })
}

/// 値とスパンを返す。
pub fn spanned<T>(parser: Parser<T>) -> Parser<(T, Span)>
where
    T: Send + Sync + 'static,
{
    parser.spanned()
}

/// 左結合チェーン。
pub fn chainl1<T, F>(term: Parser<T>, op: Parser<F>) -> Parser<T>
where
    T: Clone + Send + Sync + 'static,
    F: Fn(T, T) -> T + Send + Sync + 'static,
{
    term.chainl1(op)
}

/// 右結合チェーン。
pub fn chainr1<T, F>(term: Parser<T>, op: Parser<F>) -> Parser<T>
where
    T: Clone + Send + Sync + 'static,
    F: Fn(T, T) -> T + Send + Sync + 'static,
{
    term.chainr1(op)
}

/// パース実行時の可変状態。
#[derive(Debug)]
pub struct ParseState {
    input: Input,
    pub run_config: RunConfig,
    pub memo: MemoTable,
    pub recovered: bool,
    space: Option<Parser<()>>,
    identifier_profile: IdentifierProfile,
}

impl ParseState {
    pub fn new(source: &str, run_config: RunConfig) -> Self {
        let identifier_profile = IdentifierProfile::from_run_config(&run_config);
        let space = decode_lex_space(&run_config);
        Self {
            input: Input::new(source),
            run_config,
            memo: MemoTable::new(),
            recovered: false,
            space,
            identifier_profile,
        }
    }

    pub fn input(&self) -> &Input {
        &self.input
    }

    pub fn set_input(&mut self, input: Input) {
        self.input = input;
    }

    pub fn space(&self) -> Option<Parser<()>> {
        self.space.clone()
    }

    pub fn set_space(&mut self, space: Option<Parser<()>>) {
        self.space = space;
    }

    pub fn identifier_profile(&self) -> IdentifierProfile {
        self.identifier_profile
    }

    pub fn packrat_enabled(&self) -> bool {
        self.run_config.packrat
    }
}

/// バッチランナー。`require_eof` と Packrat 設定を反映する。
pub fn run<T>(parser: &Parser<T>, input: &str, cfg: &RunConfig) -> ParseResult<T>
where
    T: Send + Sync + 'static,
{
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
pub fn run_with_default<T>(parser: &Parser<T>, input: &str) -> ParseResult<T>
where
    T: Send + Sync + 'static,
{
    run(parser, input, &RunConfig::default())
}
