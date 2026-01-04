//! `Result` 型の正式実装。
//!
//! 仕様: `docs/spec/3-1-core-prelude-iteration.md` §2.1

use super::{
    never::Never,
    option::Option,
    try_support::{ControlFlow, Try},
};
use std::fmt::Display;

/// Reml コアプレリュードで利用する Result 互換型。
#[must_use = "Result の戻り値を無視すると失敗を見逃します"]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Result<T, E> {
    /// 正常系。
    Ok(T),
    /// 異常系。
    Err(E),
}

impl<T, E> Result<T, E> {
    /// `Ok` であるかどうかを返す。
    #[inline]
    pub const fn is_ok(&self) -> bool {
        matches!(self, Self::Ok(_))
    }

    /// `Err` であるかどうかを返す。
    #[inline]
    pub const fn is_err(&self) -> bool {
        matches!(self, Self::Err(_))
    }

    /// 正常値に写像を適用する。
    #[inline]
    #[must_use = "map の結果を利用しないと変換が失われます"]
    pub fn map<U, F>(self, f: F) -> Result<U, E>
    where
        F: FnOnce(T) -> U,
    {
        match self {
            Self::Ok(value) => Result::Ok(f(value)),
            Self::Err(err) => Result::Err(err),
        }
    }

    /// エラー値に写像を適用する。
    #[inline]
    #[must_use = "map_err の結果を利用しないと変換が失われます"]
    pub fn map_err<F, O>(self, f: F) -> Result<T, O>
    where
        F: FnOnce(E) -> O,
    {
        match self {
            Self::Ok(value) => Result::Ok(value),
            Self::Err(err) => Result::Err(f(err)),
        }
    }

    /// 正常値を `Result` で連鎖させる。
    #[inline]
    #[must_use = "and_then の結果を利用しないと副作用が起きません"]
    pub fn and_then<U, F>(self, f: F) -> Result<U, E>
    where
        F: FnOnce(T) -> Result<U, E>,
    {
        match self {
            Self::Ok(value) => f(value),
            Self::Err(err) => Result::Err(err),
        }
    }

    /// エラー時に代替計算を行う。
    #[inline]
    #[must_use = "or_else の結果を利用しないと回復処理が無効になります"]
    pub fn or_else<F, O>(self, f: F) -> Result<T, O>
    where
        F: FnOnce(E) -> Result<T, O>,
    {
        match self {
            Self::Ok(value) => Result::Ok(value),
            Self::Err(err) => f(err),
        }
    }

    /// エラー時に既定値を返す。
    #[inline]
    pub fn unwrap_or(self, default: T) -> T {
        match self {
            Self::Ok(value) => value,
            Self::Err(_) => default,
        }
    }

    /// エラー時に遅延評価した値を返す。
    #[inline]
    pub fn unwrap_or_else<F>(self, default: F) -> T
    where
        F: FnOnce(E) -> T,
    {
        match self {
            Self::Ok(value) => value,
            Self::Err(err) => default(err),
        }
    }

    /// 成功値を取得し、エラー時はデバッグ専用 panic を発生させる。
    #[inline]
    #[track_caller]
    pub fn expect(self, message: &str) -> T
    where
        E: Display,
    {
        match self {
            Self::Ok(value) => value,
            Self::Err(err) => panic_result_expect(message, err),
        }
    }

    /// `Result` から `Option` へ変換する（エラー情報を破棄）。
    #[inline]
    #[must_use = "to_option の結果を利用しないと値が失われます"]
    pub fn to_option(self) -> Option<T> {
        match self {
            Self::Ok(value) => Option::Some(value),
            Self::Err(_) => Option::None,
        }
    }

    /// `Option` から `Result` へ昇格させる。
    #[inline]
    #[must_use = "from_option の結果を利用しないと失敗理由が捨てられます"]
    pub fn from_option(opt: Option<T>, err: E) -> Self {
        match opt {
            Option::Some(value) => Result::Ok(value),
            Option::None => Result::Err(err),
        }
    }
}

#[cold]
#[inline(never)]
fn panic_result_expect<E: Display>(message: &str, err: E) -> ! {
    #[cfg(debug_assertions)]
    panic!("Reml Result.expect が Err({err}) を検出: {message}");
    #[cfg(not(debug_assertions))]
    panic!("Reml Result.expect (release) が Err({err}) を検出: {message}");
}

impl<T, E> Try for Result<T, E> {
    type Output = T;
    type Residual = Result<Never, E>;

    #[inline]
    fn from_output(output: Self::Output) -> Self {
        Self::Ok(output)
    }

    #[inline]
    fn branch(self) -> ControlFlow<Self::Residual, Self::Output> {
        match self {
            Self::Ok(value) => ControlFlow::Continue(value),
            Self::Err(err) => ControlFlow::Break(Result::Err(err)),
        }
    }
}
