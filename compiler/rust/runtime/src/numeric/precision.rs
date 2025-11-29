//! Precision / rounding helpers for Core.Numeric §6.

use crate::numeric::error::NumericError;
use crate::numeric::Numeric;

#[cfg(feature = "decimal")]
use rust_decimal::prelude::{FromPrimitive, ToPrimitive};
#[cfg(feature = "decimal")]
use rust_decimal::{Decimal, RoundingStrategy};

/// Precision kinds supported by Core.Numeric.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Precision {
    Float32,
    Float64,
    Decimal { scale: u8, precision: u8 },
    Arbitrary,
}

/// Apply precision limits to the provided value.
pub fn with_precision<T>(value: T, precision: Precision) -> Result<T, NumericError>
where
    T: PrecisionValue,
{
    value.apply_precision(&precision)
}

/// Round `value` to the specified decimal places.
pub fn round_to<T>(value: T, places: u8) -> T
where
    T: Numeric + PrecisionValue,
{
    value.round_decimal_places(places)
}

/// Truncate `value` to the specified decimal places.
pub fn truncate_to<T>(value: T, places: u8) -> T
where
    T: Numeric + PrecisionValue,
{
    value.truncate_decimal_places(places)
}

/// Internal trait used to adapt concrete numeric types to the precision helpers.
pub trait PrecisionValue: Sized {
    fn apply_precision(self, precision: &Precision) -> Result<Self, NumericError>;
    fn round_decimal_places(self, places: u8) -> Self;
    fn truncate_decimal_places(self, places: u8) -> Self;
}

macro_rules! impl_precision_for_float {
    ($ty:ty) => {
        impl PrecisionValue for $ty {
            fn apply_precision(self, precision: &Precision) -> Result<Self, NumericError> {
                match precision {
                    Precision::Float32 => Ok((self as f32) as $ty),
                    Precision::Float64 => Ok((self as f64) as $ty),
                    Precision::Decimal { scale, precision } => {
                        #[cfg(feature = "decimal")]
                        {
                            apply_decimal_precision_to_float(self as f64, *scale, *precision)
                                .map(|value| value as $ty)
                        }
                        #[cfg(not(feature = "decimal"))]
                        {
                            Err(NumericError::unsupported_precision(
                                "decimal precision requires the `decimal` feature",
                            )
                            .with_precision_kind("decimal")
                            .with_precision_scale(*scale)
                            .with_precision_digits(*precision))
                        }
                    }
                    Precision::Arbitrary => Ok(self),
                }
            }

            fn round_decimal_places(self, places: u8) -> Self {
                round_float(self as f64, places) as $ty
            }

            fn truncate_decimal_places(self, places: u8) -> Self {
                truncate_float(self as f64, places) as $ty
            }
        }
    };
}

macro_rules! impl_precision_for_signed {
    ($($ty:ty),* $(,)?) => {
        $(
            impl PrecisionValue for $ty {
                fn apply_precision(self, precision: &Precision) -> Result<Self, NumericError> {
                    match precision {
                        Precision::Float32 | Precision::Float64 | Precision::Arbitrary => Ok(self),
                        Precision::Decimal { scale, precision } => Err(
                            NumericError::unsupported_precision(
                                "decimal precision is not available for integral types",
                            )
                            .with_precision_kind("decimal")
                            .with_precision_scale(*scale)
                            .with_precision_digits(*precision),
                        ),
                    }
                }

                fn round_decimal_places(self, _places: u8) -> Self {
                    self
                }

                fn truncate_decimal_places(self, _places: u8) -> Self {
                    self
                }
            }
        )*
    };
}

macro_rules! impl_precision_for_unsigned {
    ($($ty:ty),* $(,)?) => {
        $(
            impl PrecisionValue for $ty {
                fn apply_precision(self, precision: &Precision) -> Result<Self, NumericError> {
                    match precision {
                        Precision::Float32 | Precision::Float64 | Precision::Arbitrary => Ok(self),
                        Precision::Decimal { scale, precision } => Err(
                            NumericError::unsupported_precision(
                                "decimal precision is not available for integral types",
                            )
                            .with_precision_kind("decimal")
                            .with_precision_scale(*scale)
                            .with_precision_digits(*precision),
                        ),
                    }
                }

                fn round_decimal_places(self, _places: u8) -> Self {
                    self
                }

                fn truncate_decimal_places(self, _places: u8) -> Self {
                    self
                }
            }
        )*
    };
}

impl_precision_for_float!(f32);
impl_precision_for_float!(f64);
impl_precision_for_signed!(i8, i16, i32, i64, i128, isize);
impl_precision_for_unsigned!(u8, u16, u32, u64, u128, usize);

#[cfg(feature = "decimal")]
impl PrecisionValue for Decimal {
    fn apply_precision(self, precision: &Precision) -> Result<Self, NumericError> {
        match precision {
            Precision::Float32 => {
                let float_value = self.to_f32().ok_or_else(|| {
                    NumericError::conversion_failed("Decimal -> f32 conversion failed")
                        .with_precision_kind("float32")
                })?;
                Decimal::from_f32(float_value)
                    .ok_or_else(|| {
                        NumericError::conversion_failed("f32 -> Decimal conversion failed")
                            .with_precision_kind("float32")
                    })
                    .map(|value| value.normalize())
            }
            Precision::Float64 => {
                let float_value = self.to_f64().ok_or_else(|| {
                    NumericError::conversion_failed("Decimal -> f64 conversion failed")
                        .with_precision_kind("float64")
                })?;
                Decimal::from_f64(float_value)
                    .ok_or_else(|| {
                        NumericError::conversion_failed("f64 -> Decimal conversion failed")
                            .with_precision_kind("float64")
                    })
                    .map(|value| value.normalize())
            }
            Precision::Decimal { scale, precision } => {
                apply_decimal_precision_decimal(self, *scale, *precision)
            }
            Precision::Arbitrary => Ok(self),
        }
    }

    fn round_decimal_places(self, places: u8) -> Self {
        self.round_dp_with_strategy(places as u32, RoundingStrategy::MidpointNearestEven)
            .normalize()
    }

    fn truncate_decimal_places(self, places: u8) -> Self {
        self.round_dp_with_strategy(places as u32, RoundingStrategy::ToZero)
            .normalize()
    }
}

fn round_float(value: f64, places: u8) -> f64 {
    let factor = 10f64.powi(i32::from(places));
    (value * factor).round() / factor
}

fn truncate_float(value: f64, places: u8) -> f64 {
    let factor = 10f64.powi(i32::from(places));
    (value * factor).trunc() / factor
}

#[cfg(feature = "decimal")]
fn apply_decimal_precision_to_float(
    value: f64,
    scale: u8,
    digits: u8,
) -> Result<f64, NumericError> {
    let decimal = Decimal::from_f64(value).ok_or_else(|| {
        NumericError::conversion_failed("f64 -> Decimal conversion failed")
            .with_precision_kind("decimal")
            .with_precision_scale(scale)
            .with_precision_digits(digits)
            .with_value_repr(value.to_string())
    })?;
    apply_decimal_precision_decimal(decimal, scale, digits)?
        .to_f64()
        .ok_or_else(|| {
            NumericError::conversion_failed("Decimal -> f64 conversion failed")
                .with_precision_kind("decimal")
                .with_precision_scale(scale)
                .with_precision_digits(digits)
                .with_value_repr(value.to_string())
        })
}

#[cfg(feature = "decimal")]
fn apply_decimal_precision_decimal(
    value: Decimal,
    scale: u8,
    digits: u8,
) -> Result<Decimal, NumericError> {
    validate_decimal_precision(scale, digits)?;
    let rounded = value.round_dp_with_strategy(scale as u32, RoundingStrategy::MidpointNearestEven);
    let normalized = rounded.normalize();
    let digit_count = count_significant_digits(&normalized);
    if digit_count > u32::from(digits) {
        return Err(
            NumericError::precision_overflow("value exceeds requested precision")
                .with_precision_kind("decimal")
                .with_precision_scale(scale)
                .with_precision_digits(digits)
                .with_value_repr(normalized.to_string()),
        );
    }
    Ok(normalized)
}

#[cfg(feature = "decimal")]
fn validate_decimal_precision(scale: u8, digits: u8) -> Result<(), NumericError> {
    const DECIMAL_MAX_PRECISION: u8 = 28;
    if digits == 0 {
        return Err(
            NumericError::invalid_precision("precision must be greater than zero")
                .with_precision_kind("decimal")
                .with_precision_digits(digits),
        );
    }
    if scale > digits {
        return Err(
            NumericError::invalid_precision("scale cannot exceed precision")
                .with_precision_kind("decimal")
                .with_precision_scale(scale)
                .with_precision_digits(digits),
        );
    }
    if digits > DECIMAL_MAX_PRECISION {
        return Err(
            NumericError::invalid_precision("precision exceeds Decimal::MAX_PRECISION")
                .with_precision_kind("decimal")
                .with_precision_digits(digits),
        );
    }
    Ok(())
}

#[cfg(feature = "decimal")]
fn count_significant_digits(value: &Decimal) -> u32 {
    value
        .normalize()
        .to_string()
        .chars()
        .filter(|c| c.is_ascii_digit())
        .count() as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "decimal")]
    use crate::numeric::error::NumericErrorKind;

    #[test]
    fn round_and_truncate_f64_values() {
        let value = 12.34567_f64;
        assert!((round_to(value, 2) - 12.35).abs() < f64::EPSILON);
        assert!((truncate_to(value, 2) - 12.34).abs() < f64::EPSILON);
    }

    #[cfg(feature = "decimal")]
    #[test]
    fn decimal_precision_enforces_digit_limits() {
        let value = Decimal::from_f64(1234.56789).expect("decimal");
        let adjusted = with_precision(
            value,
            Precision::Decimal {
                scale: 3,
                precision: 8,
            },
        )
        .unwrap();
        assert_eq!(adjusted.to_string(), "1234.568");
    }

    #[cfg(feature = "decimal")]
    #[test]
    fn decimal_precision_overflow_returns_error() {
        let value = Decimal::from_f64(999999999.0).expect("decimal");
        let err = with_precision(
            value,
            Precision::Decimal {
                scale: 2,
                precision: 4,
            },
        )
        .expect_err("should fail");
        assert_eq!(err.kind, NumericErrorKind::PrecisionOverflow);
    }
}
