//! Core.Numeric 仕様（`docs/spec/3-4-core-numeric-time.md` §1）に対応した
//! トレイトと基本統計ユーティリティ。
//! 仕様との整合が最優先であり、現時点では浮動小数点型を中心に提供する。

#[cfg(feature = "decimal")]
pub mod decimal;
mod effects;
pub mod error;
#[cfg(feature = "decimal")]
pub mod finance;
pub mod histogram;
mod iter;
pub mod precision;
pub mod statistics;
#[cfg(feature = "decimal")]
pub use decimal::Decimal;

use std::cmp::Ordering;
use std::collections::HashMap;
use std::hash::Hash;
use std::mem;
use std::ops::{Add, Div, Mul, Sub};

#[cfg(feature = "bigint")]
use num_bigint::{BigInt, BigUint};
#[cfg(feature = "ratio")]
use num_rational::BigRational;

use crate::prelude::iter::{EffectLabels, Iter};

pub use error::{NumericError, NumericErrorKind, StatisticsError, StatisticsErrorKind};
#[cfg(feature = "decimal")]
pub use finance::{compound_interest, currency_add, net_present_value, CurrencyCode};
pub use histogram::{histogram, HistogramBucket, HistogramBucketState};
pub use iter::{rolling_average, z_score};
pub use precision::{round_to, truncate_to, with_precision, Precision};
pub use statistics::{correlation, linear_regression, quantiles, LinearModel, QuantilePoint};

/// Core.Numeric の基礎トレイト。
pub trait Numeric: PartialOrd + Clone {
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
/// Core.Numeric のうち、統計演算を提供する型。
pub trait Floating:
    Numeric
    + Clone
    + PartialOrd
    + Add<Output = Self>
    + Sub<Output = Self>
    + Mul<Output = Self>
    + Div<Output = Self>
{
    fn from_usize(value: usize) -> Self;
    fn try_from_f64(value: f64) -> Option<Self>;
    fn to_f64(&self) -> f64;
    fn is_nan(&self) -> bool;
    fn is_infinite(&self) -> bool;
    fn total_cmp(&self, other: &Self) -> Ordering;
}

/// 線形補間。
pub fn lerp<T>(start: T, end: T, t: T) -> T
where
    T: Floating,
{
    let span = end - start.clone();
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
                let count_t = T::from_usize(new_count);
                let delta = value - mean.clone();
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
    let (count, _mean, m2) = iter
        .try_fold(
            (0usize, T::zero(), T::zero()),
            |(count, mean, m2), value| -> Result<_, ()> {
                let new_count = count + 1;
                let count_t = T::from_usize(new_count);
                let delta = value.clone() - mean.clone();
                let updated_mean = mean + delta.clone() / count_t;
                let delta2 = value - updated_mean.clone();
                let updated_m2 = m2 + delta * delta2;
                Ok((new_count, updated_mean, updated_m2))
            },
        )
        .ok()?;
    if count < 2 {
        return None;
    }
    let denom = T::from_usize(count);
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
        return values.first().cloned();
    }
    effects::record_mem_copy(values.len().saturating_mul(mem::size_of::<T>()));
    values.sort_by(|a, b| a.total_cmp(b));
    let p = percentile.clamp(T::zero(), T::one()).to_f64();
    let steps = (values.len() - 1) as f64;
    let rank = p * steps;
    let lower = rank.floor();
    let upper = rank.ceil();
    if lower == upper {
        return values.get(lower as usize).cloned();
    }
    let lower_value = values[lower as usize].clone();
    let upper_value = values[upper as usize].clone();
    let weight = T::try_from_f64(rank - lower)?;
    let span = upper_value.clone() - lower_value.clone();
    Some(lower_value + span * weight)
}

/// 中央値（偶数個は lower median）。
pub fn median<T>(iter: Iter<T>) -> Option<T>
where
    T: Numeric + PartialOrd + Clone,
{
    let mut values: Vec<T> = iter.into_iter().collect();
    if values.is_empty() {
        return None;
    }
    effects::record_mem_copy(values.len().saturating_mul(mem::size_of::<T>()));
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    let mid = values.len() / 2;
    if values.len() % 2 == 0 {
        values.get(mid - 1).cloned()
    } else {
        values.get(mid).cloned()
    }
}

/// 最頻値。複数ある場合は最初に観測した値を返す。
pub fn mode<T>(iter: Iter<T>) -> Option<T>
where
    T: Numeric + Eq + Hash + Clone,
{
    let mut counts: HashMap<T, (usize, usize)> = HashMap::new();
    let mut best_value: Option<T> = None;
    let mut best_count = 0usize;
    let mut best_first_seen = usize::MAX;

    for (index, value) in iter.into_iter().enumerate() {
        let entry = counts.entry(value.clone()).or_insert((0, index));
        entry.0 += 1;
        let count = entry.0;
        let first_seen = entry.1;
        if count > best_count || (count == best_count && first_seen < best_first_seen) {
            best_count = count;
            best_first_seen = first_seen;
            best_value = Some(value);
        }
    }

    best_value
}

/// 最小値と最大値のペア。
pub fn range<T>(iter: Iter<T>) -> Option<(T, T)>
where
    T: Numeric + Ord + Clone,
{
    let mut iterator = iter.into_iter();
    let first = iterator.next()?;
    let mut min_value = first.clone();
    let mut max_value = first;
    for value in iterator {
        if value < min_value {
            min_value = value.clone();
        }
        if value > max_value {
            max_value = value.clone();
        }
    }
    Some((min_value, max_value))
}

/// `Iter<T>` 拡張。
pub trait IterNumericExt<T>: Sized {
    fn mean(self) -> Option<T>
    where
        T: Floating;
    fn variance(self) -> Option<T>
    where
        T: Floating;
    fn percentile(self, percentile: T) -> Option<T>
    where
        T: Floating;
    fn median(self) -> Option<T>
    where
        T: Numeric + PartialOrd + Clone;
    fn mode(self) -> Option<T>
    where
        T: Numeric + Eq + Hash + Clone;
    fn range(self) -> Option<(T, T)>
    where
        T: Numeric + Ord + Clone;
}

impl<T> IterNumericExt<T> for Iter<T> {
    fn mean(self) -> Option<T>
    where
        T: Floating,
    {
        crate::numeric::mean(self)
    }

    fn variance(self) -> Option<T>
    where
        T: Floating,
    {
        crate::numeric::variance(self)
    }

    fn percentile(self, percentile: T) -> Option<T>
    where
        T: Floating,
    {
        crate::numeric::percentile(self, percentile)
    }

    fn median(self) -> Option<T>
    where
        T: Numeric + PartialOrd + Clone,
    {
        crate::numeric::median(self)
    }

    fn mode(self) -> Option<T>
    where
        T: Numeric + Eq + Hash + Clone,
    {
        crate::numeric::mode(self)
    }

    fn range(self) -> Option<(T, T)>
    where
        T: Numeric + Ord + Clone,
    {
        crate::numeric::range(self)
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

            impl Floating for $ty {
                fn from_usize(value: usize) -> Self {
                    value as $ty
                }

                fn try_from_f64(value: f64) -> Option<Self> {
                    if value.is_finite() {
                        Some(value as $ty)
                    } else {
                        None
                    }
                }

                fn to_f64(&self) -> f64 {
                    *self as f64
                }

                fn is_nan(&self) -> bool {
                    <$ty>::is_nan(*self)
                }

                fn is_infinite(&self) -> bool {
                    <$ty>::is_infinite(*self)
                }

                fn total_cmp(&self, other: &Self) -> Ordering {
                    <$ty>::total_cmp(self, other)
                }
            }
        )*
    };
}

impl_numeric_for_signed!(i8, i16, i32, i64, i128, isize);
impl_numeric_for_unsigned!(u8, u16, u32, u64, u128, usize);
impl_numeric_for_float!(f32, f64);

#[cfg(feature = "bigint")]
impl Numeric for BigInt {
    fn zero() -> Self {
        BigInt::from(0)
    }

    fn one() -> Self {
        BigInt::from(1)
    }

    fn abs(self) -> Self {
        if self < BigInt::from(0) {
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

#[cfg(feature = "bigint")]
impl Numeric for BigUint {
    fn zero() -> Self {
        BigUint::from(0u8)
    }

    fn one() -> Self {
        BigUint::from(1u8)
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

#[cfg(feature = "ratio")]
impl Numeric for BigRational {
    fn zero() -> Self {
        BigRational::from_integer(BigInt::from(0))
    }

    fn one() -> Self {
        BigRational::from_integer(BigInt::from(1))
    }

    fn abs(self) -> Self {
        if self < BigRational::from_integer(BigInt::from(0)) {
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

/// Numeric API が記録した効果を取得する。
pub fn take_numeric_effects_snapshot() -> EffectLabels {
    effects::take_recorded_effects().to_labels()
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "decimal")]
    use super::Decimal;
    use super::*;
    #[cfg(feature = "decimal")]
    use rust_decimal::prelude::FromPrimitive;

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

    #[test]
    fn median_mode_and_range_cover_basic_cases() {
        let data = vec![8i32, 2, 3, 2, 7, 9];
        let median_value = Iter::from_list(data.clone()).median().unwrap();
        assert_eq!(median_value, 3);

        let mode_value = Iter::from_list(data.clone()).mode().unwrap();
        assert_eq!(mode_value, 2);

        let range_values = Iter::from_list(data).range().unwrap();
        assert_eq!(range_values.0, 2);
        assert_eq!(range_values.1, 9);
    }

    #[cfg(feature = "decimal")]
    #[test]
    fn decimal_numeric_supports_basic_clamp() {
        let three = Decimal::new(3, 0);
        let five = Decimal::new(5, 0);
        assert_eq!(<Decimal as Numeric>::abs(Decimal::new(-3, 0)), three);
        assert_eq!(
            <Decimal as Numeric>::clamp(five, Decimal::new(0, 0), three),
            three
        );
    }

    #[cfg(feature = "bigint")]
    #[test]
    fn bigint_numeric_handles_sign() {
        let neg = BigInt::from(-10);
        assert_eq!(<BigInt as Numeric>::abs(neg.clone()), BigInt::from(10));
        assert_eq!(
            <BigInt as Numeric>::clamp(neg, BigInt::from(-5), BigInt::from(5)),
            BigInt::from(-5)
        );
    }

    #[cfg(feature = "ratio")]
    #[test]
    fn ratio_numeric_provides_bounds() {
        let half = BigRational::new(BigInt::from(1), BigInt::from(2));
        let two = BigRational::new(BigInt::from(2), BigInt::from(1));
        let four = BigRational::new(BigInt::from(4), BigInt::from(1));
        assert_eq!(<BigRational as Numeric>::abs(-half.clone()), half);
        assert_eq!(
            <BigRational as Numeric>::clamp(four, half.clone(), two.clone()),
            two
        );
    }

    #[cfg(feature = "decimal")]
    #[test]
    fn decimal_mean_and_variance_support_precision_ops() {
        let samples = vec![
            Decimal::new(10, 1),
            Decimal::new(30, 1),
            Decimal::new(50, 1),
            Decimal::new(70, 1),
        ];
        let mean_value = mean(Iter::from_list(samples.clone())).expect("mean");
        assert_eq!(mean_value, Decimal::from(4));

        let variance_value = variance(Iter::from_list(samples)).expect("variance");
        assert_eq!(variance_value, Decimal::from(5));
    }

    #[cfg(feature = "decimal")]
    #[test]
    fn currency_add_respects_scale_and_validates_code() {
        let usd = CurrencyCode::from("usd");
        let amount = currency_add(Decimal::new(12345, 3), Decimal::new(5005, 2), usd).unwrap();
        assert_eq!(amount, Decimal::new(6240, 2));

        let err = currency_add(
            Decimal::new(10, 0),
            Decimal::new(5, 0),
            CurrencyCode::from("zzz"),
        )
        .expect_err("unknown currency");
        assert_eq!(err.kind, NumericErrorKind::UnsupportedCurrency);
    }

    #[cfg(feature = "decimal")]
    #[test]
    fn compound_interest_and_npv_return_expected_values() {
        let principal = Decimal::new(1000, 0);
        let amount = compound_interest(principal, 0.05, 2).expect("compound interest");
        assert_eq!(amount.round_dp(2), Decimal::from_f64(1102.5).unwrap());

        let cashflows = Iter::from_list(vec![
            Decimal::new(-1000, 0),
            Decimal::new(400, 0),
            Decimal::new(400, 0),
            Decimal::new(400, 0),
        ]);
        let npv = net_present_value(cashflows, 0.1).expect("npv");
        assert!(npv < Decimal::ZERO);
    }
}
