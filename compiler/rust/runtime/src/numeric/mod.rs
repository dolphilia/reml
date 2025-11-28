//! Core.Numeric 仕様（`docs/spec/3-4-core-numeric-time.md` §1）に対応した
//! トレイトと基本統計ユーティリティ。
//! 仕様との整合が最優先であり、現時点では浮動小数点型を中心に提供する。

mod effects;
pub mod error;
pub mod histogram;
pub mod statistics;

use std::cmp::Ordering;
use std::ops::{Add, Div, Mul, Sub};

use crate::prelude::iter::{EffectLabels, Iter};

pub use error::{StatisticsError, StatisticsErrorKind};
pub use histogram::{histogram, HistogramBucket, HistogramBucketState};
pub use statistics::{correlation, linear_regression, quantiles, LinearModel, QuantilePoint};

/// Core.Numeric の基礎トレイト。
pub trait Numeric: Copy + PartialOrd {
    /// 加法単位元。
    fn zero() -> Self;
    /// 乗法単位元。
    fn one() -> Self;
    /// 絶対値。
    fn abs(self) -> Self;
    /// 範囲クランプ。
    fn clamp(self, min: Self, max: Self) -> Self;
}

/// `NaN` を含む全順序比較ヘルパ。
pub trait OrderedFloat: Copy {
    fn is_nan(self) -> bool;
    fn is_infinite(self) -> bool;
    fn total_cmp(self, other: Self) -> Ordering;
}

/// Core.Numeric のうち、浮動小数点演算を提供する型。
pub trait Floating:
    Numeric
    + OrderedFloat
    + Add<Output = Self>
    + Sub<Output = Self>
    + Mul<Output = Self>
    + Div<Output = Self>
{
    fn from_f64(value: f64) -> Self;
    fn to_f64(self) -> f64;
}

/// 線形補間。
pub fn lerp<T>(start: T, end: T, t: T) -> T
where
    T: Floating,
{
    let span = end - start;
    start + span * t
}

/// 平均値（Welford 法ベース）。
pub fn mean<T>(iter: Iter<T>) -> Option<T>
where
    T: Floating,
{
    let (count, mean) = iter
        .try_fold(
            (0usize, T::zero()),
            |(count, mean), value| -> Result<_, ()> {
                let new_count = count + 1;
                let count_t = T::from_f64(new_count as f64);
                let delta = value - mean;
                let next_mean = mean + delta / count_t;
                Ok((new_count, next_mean))
            },
        )
        .ok()?;
    if count == 0 {
        None
    } else {
        Some(mean)
    }
}

/// 分散（母分散、Welford 法）。
pub fn variance<T>(iter: Iter<T>) -> Option<T>
where
    T: Floating,
{
    let (count, mean, m2) = iter
        .try_fold(
            (0usize, T::zero(), T::zero()),
            |(count, mean, m2), value| -> Result<_, ()> {
                let new_count = count + 1;
                let count_t = T::from_f64(new_count as f64);
                let delta = value - mean;
                let updated_mean = mean + delta / count_t;
                let delta2 = value - updated_mean;
                let updated_m2 = m2 + delta * delta2;
                Ok((new_count, updated_mean, updated_m2))
            },
        )
        .ok()?;
    if count < 2 {
        return None;
    }
    let denom = T::from_f64(count as f64);
    Some(m2 / denom)
}

/// 百分位点（線形補間付き nearest-rank）。
pub fn percentile<T>(iter: Iter<T>, percentile: T) -> Option<T>
where
    T: Floating,
{
    if percentile.is_nan() {
        return None;
    }
    let mut values: Vec<T> = iter.into_iter().collect();
    if values.is_empty() {
        return None;
    }
    if values.len() == 1 {
        return values.first().copied();
    }
    values.sort_by(|a, b| (*a).total_cmp(*b));
    let p = percentile.clamp(T::zero(), T::one()).to_f64();
    let steps = (values.len() - 1) as f64;
    let rank = p * steps;
    let lower = rank.floor();
    let upper = rank.ceil();
    if lower == upper {
        return values.get(lower as usize).copied();
    }
    let lower_value = values[lower as usize];
    let upper_value = values[upper as usize];
    let weight = T::from_f64(rank - lower);
    Some(lower_value + (upper_value - lower_value) * weight)
}

/// `Iter<T>` 拡張。
pub trait IterNumericExt<T>
where
    T: Floating,
{
    fn mean(self) -> Option<T>;
    fn variance(self) -> Option<T>;
    fn percentile(self, percentile: T) -> Option<T>;
}

impl<T> IterNumericExt<T> for Iter<T>
where
    T: Floating,
{
    fn mean(self) -> Option<T> {
        crate::numeric::mean(self)
    }

    fn variance(self) -> Option<T> {
        crate::numeric::variance(self)
    }

    fn percentile(self, percentile: T) -> Option<T> {
        crate::numeric::percentile(self, percentile)
    }
}

macro_rules! impl_numeric_for_signed {
    ($($ty:ty),* $(,)?) => {
        $(
            impl Numeric for $ty {
                fn zero() -> Self {
                    0
                }

                fn one() -> Self {
                    1
                }

                fn abs(self) -> Self {
                    <$ty>::abs(self)
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
        )*
    };
}

macro_rules! impl_numeric_for_unsigned {
    ($($ty:ty),* $(,)?) => {
        $(
            impl Numeric for $ty {
                fn zero() -> Self {
                    0
                }

                fn one() -> Self {
                    1
                }

                fn abs(self) -> Self {
                    self
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
        )*
    };
}

macro_rules! impl_numeric_for_float {
    ($($ty:ty),* $(,)?) => {
        $(
            impl Numeric for $ty {
                fn zero() -> Self {
                    0.0
                }

                fn one() -> Self {
                    1.0
                }

                fn abs(self) -> Self {
                    <$ty>::abs(self)
                }

                fn clamp(self, min: Self, max: Self) -> Self {
                    <$ty>::clamp(self, min, max)
                }
            }

            impl OrderedFloat for $ty {
                fn is_nan(self) -> bool {
                    <$ty>::is_nan(self)
                }

                fn is_infinite(self) -> bool {
                    <$ty>::is_infinite(self)
                }

                fn total_cmp(self, other: Self) -> Ordering {
                    <$ty>::total_cmp(&self, &other)
                }
            }

            impl Floating for $ty {
                fn from_f64(value: f64) -> Self {
                    value as $ty
                }

                fn to_f64(self) -> f64 {
                    self as f64
                }
            }
        )*
    };
}

impl_numeric_for_signed!(i8, i16, i32, i64, i128, isize);
impl_numeric_for_unsigned!(u8, u16, u32, u64, u128, usize);
impl_numeric_for_float!(f32, f64);

/// Numeric API が記録した効果を取得する。
pub fn take_numeric_effects_snapshot() -> EffectLabels {
    effects::take_recorded_effects().to_labels()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn iter_from_slice(values: &[f64]) -> Iter<f64> {
        Iter::from_list(values.to_vec())
    }

    #[test]
    fn numeric_trait_basics() {
        assert_eq!(i64::zero(), 0);
        assert_eq!(f32::one(), 1.0);
        assert_eq!((-3i32).abs(), 3);
        assert_eq!(<u32 as Numeric>::clamp(5, 0, 3), 3);
        assert!(((-5.5f64).abs() - 5.5).abs() < f64::EPSILON);
    }

    #[test]
    fn lerp_midpoint() {
        let result = lerp(0.0f64, 10.0, 0.5);
        assert!((result - 5.0).abs() < 1e-9);
    }

    #[test]
    fn mean_and_variance() {
        let samples = iter_from_slice(&[1.0, 2.0, 3.0, 4.0]);
        let avg = mean(samples).unwrap();
        assert!((avg - 2.5).abs() < 1e-9);

        let samples = iter_from_slice(&[1.0, 2.0, 3.0, 4.0]);
        let var = variance(samples).unwrap();
        assert!((var - 1.25).abs() < 1e-9);
    }

    #[test]
    fn percentile_linear_interpolation() {
        let samples = iter_from_slice(&[10.0, 30.0, 20.0, 40.0]);
        let median = percentile(samples, 0.5).unwrap();
        assert!((median - 25.0).abs() < 1e-9);
    }

    #[test]
    fn iter_extension_methods() {
        let samples = iter_from_slice(&[2.0, 4.0, 6.0]);
        let iter = samples;
        let mean_value = iter.clone().mean().unwrap();
        assert!((mean_value - 4.0).abs() < 1e-9);

        let variance_value = iter_from_slice(&[2.0, 4.0, 6.0]).variance().unwrap();
        assert!((variance_value - 2.6666666666666665).abs() < 1e-9);

        let percentile_value = iter_from_slice(&[2.0, 4.0, 6.0]).percentile(0.75).unwrap();
        assert!((percentile_value - 5.0).abs() < 1e-9);
    }
}
