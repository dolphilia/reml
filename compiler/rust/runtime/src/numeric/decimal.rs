//! Decimal 型サポート。
//!
//! Reml 仕様の `Decimal` は `rust_decimal::Decimal` を土台にし、
//! `core_numeric` feature から有効化する。

#[cfg(feature = "decimal")]
pub use rust_decimal::Decimal;

#[cfg(feature = "decimal")]
use super::Numeric;

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
