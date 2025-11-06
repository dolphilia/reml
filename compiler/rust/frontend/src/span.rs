//! ソースコード上の位置情報を表現するユーティリティ。

use serde::Serialize;
use std::fmt;

/// 半開区間で表現したソースコード上の範囲。
/// `start` はバイトオフセット、`end` は `start` 以上である必要がある。
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

impl Span {
    /// 新しい `Span` を生成する。`end < start` の場合は自動で `start` に丸める。
    pub fn new(start: u32, end: u32) -> Self {
        if end < start {
            Self { start, end: start }
        } else {
            Self { start, end }
        }
    }

    /// `Span` の長さ（バイト数）を返す。
    pub fn len(&self) -> u32 {
        self.end.saturating_sub(self.start)
    }

    /// 空範囲かどうかを返す。
    pub fn is_empty(&self) -> bool {
        self.start >= self.end
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}..{})", self.start, self.end)
    }
}

/// 値と `Span` をペアにしたユーティリティ。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SpanTagged<T> {
    pub span: Span,
    pub value: T,
}

impl<T> SpanTagged<T> {
    /// 値と `Span` をまとめる。
    pub fn new(value: T, span: Span) -> Self {
        Self { span, value }
    }

    /// `SpanTagged` を別の値へ写像する。
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> SpanTagged<U> {
        SpanTagged {
            span: self.span,
            value: f(self.value),
        }
    }
}
