use std::mem;

use ordered_float::OrderedFloat;

use crate::prelude::{
    collectors::{List, Map},
    iter::Iter,
};

use super::{effects, error::StatisticsErrorKind, StatisticsError};

const QUANTILE_CONTEXT: &str = "core.numeric.statistics.quantiles";
const CORRELATION_CONTEXT: &str = "core.numeric.statistics.correlation";
const REGRESSION_CONTEXT: &str = "core.numeric.statistics.linear_regression";

/// `Map` のキーとして利用する量子化点。
pub type QuantilePoint = OrderedFloat<f64>;

/// 高度統計の結果として返す線形モデル。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LinearModel {
    pub slope: f64,
    pub intercept: f64,
    pub r_squared: f64,
}

impl LinearModel {
    /// 与えられた `x` に対する予測値を計算する。
    pub fn predict(&self, x: f64) -> f64 {
        self.slope.mul_add(x, self.intercept)
    }
}

/// 百分位を複数まとめて計算する。
pub fn quantiles(
    values: Iter<f64>,
    points: List<f64>,
) -> Result<Map<QuantilePoint, f64>, StatisticsError> {
    let mut samples: Vec<f64> = values.into_iter().collect();
    if samples.is_empty() {
        return Err(
            StatisticsError::insufficient_data("quantiles requires at least one sample")
                .with_context_code(QUANTILE_CONTEXT),
        );
    }
    for &value in &samples {
        if !value.is_finite() {
            return Err(StatisticsError::numerical_instability(format!(
                "encountered non-finite value {value} in quantiles input"
            ))
            .with_value(value)
            .with_context_code(QUANTILE_CONTEXT));
        }
    }
    effects::record_mem_copy(samples.len().saturating_mul(mem::size_of::<f64>()));
    samples.sort_by(|a, b| a.total_cmp(b));

    let mut query_points: Vec<f64> = points.iter().collect();
    if query_points.is_empty() {
        return Err(StatisticsError::invalid_parameter(
            "quantiles requires at least one percentile point",
        )
        .with_context_code(QUANTILE_CONTEXT));
    }
    for &point in &query_points {
        if !point.is_finite() {
            return Err(StatisticsError::invalid_parameter(format!(
                "percentile point {point} is not finite"
            ))
            .with_context_code(QUANTILE_CONTEXT));
        }
    }
    effects::record_mem_copy(query_points.len().saturating_mul(mem::size_of::<f64>()));
    query_points.sort_by(|a, b| a.total_cmp(b));

    let mut result = Map::new();
    for point in query_points {
        let clamped = point.clamp(0.0, 1.0);
        let value = percentile_from_sorted(&samples, clamped);
        result = result.insert(OrderedFloat::from(clamped), value);
    }
    Ok(result)
}

/// 二系列のピアソン相関係数を計算する。
pub fn correlation(x: Iter<f64>, y: Iter<f64>) -> Result<f64, StatisticsError> {
    let mut x_iter = x.into_iter();
    let mut y_iter = y.into_iter();

    let mut count = 0usize;
    let mut sum_x = 0.0;
    let mut sum_y = 0.0;
    let mut sum_xx = 0.0;
    let mut sum_yy = 0.0;
    let mut sum_xy = 0.0;

    loop {
        match (x_iter.next(), y_iter.next()) {
            (Some(xv), Some(yv)) => {
                if !xv.is_finite() || !yv.is_finite() {
                    return Err(StatisticsError::numerical_instability(
                        "correlation input must be finite",
                    )
                    .with_value(if !xv.is_finite() { xv } else { yv })
                    .with_context_code(CORRELATION_CONTEXT));
                }
                count += 1;
                sum_x += xv;
                sum_y += yv;
                sum_xx += xv * xv;
                sum_yy += yv * yv;
                sum_xy += xv * yv;
            }
            (None, None) => break,
            (Some(_), None) | (None, Some(_)) => {
                return Err(StatisticsError::invalid_parameter(
                    "correlation requires iterators with equal length",
                )
                .with_context_code(CORRELATION_CONTEXT));
            }
        }
    }

    if count < 2 {
        return Err(StatisticsError::insufficient_data(
            "correlation requires at least two samples",
        )
        .with_context_code(CORRELATION_CONTEXT));
    }

    let n = count as f64;
    let mean_x = sum_x / n;
    let mean_y = sum_y / n;
    let cov = (sum_xy / n) - (mean_x * mean_y);
    let var_x = (sum_xx / n) - mean_x * mean_x;
    let var_y = (sum_yy / n) - mean_y * mean_y;
    if var_x <= 0.0 || var_y <= 0.0 {
        return Err(
            StatisticsError::invalid_parameter("correlation variance must be positive")
                .with_context_code(CORRELATION_CONTEXT),
        );
    }
    let denom = (var_x * var_y).sqrt();
    if denom == 0.0 || !denom.is_finite() {
        return Err(
            StatisticsError::numerical_instability("correlation denominator became zero")
                .with_context_code(CORRELATION_CONTEXT),
        );
    }
    let corr = (cov / denom).clamp(-1.0, 1.0);
    if corr.is_nan() {
        return Err(
            StatisticsError::numerical_instability("correlation result is not a number")
                .with_context_code(CORRELATION_CONTEXT),
        );
    }
    Ok(corr)
}

/// 単回帰モデルを構築する。
pub fn linear_regression(points: Iter<(f64, f64)>) -> Result<LinearModel, StatisticsError> {
    let mut count = 0usize;
    let mut sum_x = 0.0;
    let mut sum_y = 0.0;
    let mut sum_xx = 0.0;
    let mut sum_yy = 0.0;
    let mut sum_xy = 0.0;

    for (x, y) in points.into_iter() {
        if !x.is_finite() || !y.is_finite() {
            return Err(StatisticsError::numerical_instability(
                "linear_regression input must be finite",
            )
            .with_value(if !x.is_finite() { x } else { y })
            .with_context_code(REGRESSION_CONTEXT));
        }
        count += 1;
        sum_x += x;
        sum_y += y;
        sum_xx += x * x;
        sum_yy += y * y;
        sum_xy += x * y;
    }

    if count < 2 {
        return Err(StatisticsError::insufficient_data(
            "linear_regression requires at least two samples",
        )
        .with_context_code(REGRESSION_CONTEXT));
    }

    let n = count as f64;
    let denominator = n * sum_xx - sum_x * sum_x;
    if denominator == 0.0 {
        return Err(StatisticsError::invalid_parameter(
            "linear_regression cannot fit a vertical line",
        )
        .with_context_code(REGRESSION_CONTEXT));
    }

    let numerator = n * sum_xy - sum_x * sum_y;
    let slope = numerator / denominator;
    if !slope.is_finite() {
        return Err(StatisticsError::numerical_instability(
            "linear_regression slope became non-finite",
        )
        .with_context_code(REGRESSION_CONTEXT));
    }
    let intercept = (sum_y - slope * sum_x) / n;
    if !intercept.is_finite() {
        return Err(StatisticsError::numerical_instability(
            "linear_regression intercept became non-finite",
        )
        .with_context_code(REGRESSION_CONTEXT));
    }

    let denom_r = ((n * sum_xx - sum_x * sum_x) * (n * sum_yy - sum_y * sum_y)).sqrt();
    let r = if denom_r == 0.0 {
        0.0
    } else {
        (numerator / denom_r).clamp(-1.0, 1.0)
    };
    let r_squared = r * r;

    Ok(LinearModel {
        slope,
        intercept,
        r_squared,
    })
}

fn percentile_from_sorted(samples: &[f64], percentile: f64) -> f64 {
    if samples.len() == 1 {
        return samples[0];
    }
    let steps = (samples.len() - 1) as f64;
    let rank = percentile * steps;
    let lower = rank.floor();
    let upper = rank.ceil();
    if lower == upper {
        return samples[lower as usize];
    }
    let lower_value = samples[lower as usize];
    let upper_value = samples[upper as usize];
    let weight = rank - lower;
    lower_value + (upper_value - lower_value) * weight
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::numeric::take_numeric_effects_snapshot;
    use crate::prelude::collectors::List;
    use crate::prelude::iter::Iter;

    fn iter_from(values: &[f64]) -> Iter<f64> {
        Iter::from_list(values.to_vec())
    }

    #[test]
    fn quantiles_returns_expected_percentiles() {
        let samples = iter_from(&[10.0, 30.0, 20.0, 40.0, 50.0]);
        let points = List::from_vec(vec![0.25, 0.50, 0.95]);
        let result = quantiles(samples, points).expect("quantiles succeeded");
        assert_eq!(result.len(), 3);
        let median = result.get(&OrderedFloat::from(0.5)).copied().unwrap();
        assert!((median - 30.0).abs() < 1e-9);

        let effects = take_numeric_effects_snapshot();
        assert!(effects.mem, "quantiles should record effect {{mem}}");
        assert!(
            effects.mem_bytes >= 5 * mem::size_of::<f64>(),
            "mem_bytes should reflect sample allocation"
        );
    }

    #[test]
    fn correlation_detects_relationship() {
        let x = iter_from(&[1.0, 2.0, 3.0, 4.0]);
        let y = iter_from(&[2.0, 4.0, 6.0, 8.0]);
        let corr = correlation(x, y).expect("correlation succeeded");
        assert!((corr - 1.0).abs() < 1e-9);
    }

    #[test]
    fn linear_regression_produces_reasonable_model() {
        let points = Iter::from_list(vec![(1.0, 2.0), (2.0, 4.1), (3.0, 6.2), (4.0, 8.2)]);
        let model = linear_regression(points).expect("linear_regression succeeded");
        assert!((model.slope - 2.06).abs() < 0.05);
        assert!(model.r_squared > 0.99);
        let predicted = model.predict(5.0);
        assert!((predicted - 10.3).abs() < 0.2);
    }

    #[test]
    fn correlation_rejects_mismatched_len() {
        let err = correlation(iter_from(&[1.0, 2.0]), iter_from(&[3.0]))
            .expect_err("length mismatch should error");
        assert_eq!(
            err.kind,
            StatisticsErrorKind::InvalidParameter,
            "expected invalid parameter error"
        );
    }
}
