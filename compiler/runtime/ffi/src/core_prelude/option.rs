//! `Option` 型の正式実装。
//!
//! 仕様: `docs/spec/3-1-core-prelude-iteration.md` §2.1

use super::{
    never::Never,
    result::Result,
    try_support::{ControlFlow, Try},
};

/// Reml コアプレリュードで利用する Option 互換型。
#[must_use = "Option の戻り値を無視すると失敗を見逃します"]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Option<T> {
    /// 値を保持するケース。
    Some(T),
    /// 値が存在しないケース。
    None,
}

impl<T> Option<T> {
    /// `Some` であるかどうかを判定する。
    #[inline]
    pub const fn is_some(&self) -> bool {
        matches!(self, Self::Some(_))
    }

    /// `None` であるかどうかを判定する。
    #[inline]
    pub const fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    /// 値を写像し、新しい `Option` を返す。
    #[inline]
    #[must_use = "map の結果を無視すると計算が消失します"]
    pub fn map<U, F>(self, f: F) -> Option<U>
    where
        F: FnOnce(T) -> U,
    {
        match self {
            Self::Some(value) => Option::Some(f(value)),
            Self::None => Option::None,
        }
    }

    /// `Option` を連鎖させる（`flat_map`）。
    #[inline]
    #[must_use = "and_then の結果を無視すると副作用が起きません"]
    pub fn and_then<U, F>(self, f: F) -> Option<U>
    where
        F: FnOnce(T) -> Option<U>,
    {
        match self {
            Self::Some(value) => f(value),
            Self::None => Option::None,
        }
    }

    /// `Option` を `Result` へ昇格させる。
    #[inline]
    #[must_use = "ok_or の結果を使わない場合は失敗理由が捨てられます"]
    pub fn ok_or<E, F>(self, err: F) -> Result<T, E>
    where
        F: FnOnce() -> E,
    {
        match self {
            Self::Some(value) => Result::Ok(value),
            Self::None => Result::Err(err()),
        }
    }

    /// `None` のときに既定値を返す。
    #[inline]
    pub fn unwrap_or(self, default: T) -> T {
        match self {
            Self::Some(value) => value,
            Self::None => default,
        }
    }

    /// `None` のときに遅延評価した既定値を返す。
    #[inline]
    pub fn unwrap_or_else<F>(self, default: F) -> T
    where
        F: FnOnce() -> T,
    {
        match self {
            Self::Some(value) => value,
            Self::None => default(),
        }
    }

    /// `Some` を保証し、`None` の場合はデバッグ専用 panic を発生させる。
    #[inline]
    #[track_caller]
    pub fn expect(self, message: &str) -> T {
        match self {
            Self::Some(value) => value,
            Self::None => panic_option_expect(message),
        }
    }
}

#[cold]
#[inline(never)]
fn panic_option_expect(message: &str) -> ! {
    #[cfg(debug_assertions)]
    panic!("Reml Option.expect が None を検出: {message}");
    #[cfg(not(debug_assertions))]
    panic!("Reml Option.expect (release) が None を検出: {message}");
}

impl<T> Try for Option<T> {
    type Output = T;
    type Residual = Option<Never>;

    #[inline]
    fn from_output(output: Self::Output) -> Self {
        Self::Some(output)
    }

    #[inline]
    fn branch(self) -> ControlFlow<Self::Residual, Self::Output> {
        match self {
            Self::Some(value) => ControlFlow::Continue(value),
            Self::None => ControlFlow::Break(Option::None),
        }
    }
}
