//! Decimal 型サポート。
//!
//! Reml 仕様の `Decimal` は `rust_decimal::Decimal` を土台にし、
//! `core_numeric` feature から有効化する。

#[cfg(feature = "decimal")]
pub use rust_decimal::Decimal;

#[cfg(feature = "decimal")]
use super::{Floating, Numeric};
#[cfg(feature = "decimal")]
use rust_decimal::prelude::{FromPrimitive, ToPrimitive};

#[cfg(feature = "decimal")]
impl Numeric for Decimal {
    fn zero() -> Self {
        Decimal::ZERO
    }

    fn one() -> Self {
        Decimal::ONE
    }

    fn abs(self) -> Self {
        if self.is_sign_negative() {
            -self
        } else {
            self
        }
    }

    fn clamp(self, min: Self, max: Self) -> Self {
        debug_assert!(min <= max, "min must be <= max");
        if self < min {
            min
        } else if self > max {
            max
        } else {
            self
        }
    }
}

#[cfg(feature = "decimal")]
impl Floating for Decimal {
    fn from_usize(value: usize) -> Self {
        Decimal::from(value as u64)
    }

    fn try_from_f64(value: f64) -> Option<Self> {
        Decimal::from_f64(value)
    }

    fn to_f64(&self) -> f64 {
        ToPrimitive::to_f64(self).unwrap_or_else(|| {
            if self.is_sign_negative() {
                f64::NEG_INFINITY
            } else {
                f64::INFINITY
            }
        })
    }

    fn is_nan(&self) -> bool {
        false
    }

    fn is_infinite(&self) -> bool {
        false
    }

    fn total_cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.cmp(other)
    }
}
