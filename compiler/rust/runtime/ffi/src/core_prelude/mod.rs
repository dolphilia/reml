#![allow(dead_code)]

//! Reml コアプレリュード API（Option/Result/Never/Try 群）の雛形実装。
//!
//! WBS 2.1a では API の型構造とモジュール区切りのみを確立し、
//! 具体的なメソッド実装や効果タグ検証は WBS 2.1b 以降で実装する。

pub mod never;
pub mod option;
pub mod result;
pub mod try_support;

pub use never::Never;
pub use option::Option;
pub use result::Result;
pub use try_support::{Try, TryContext};
