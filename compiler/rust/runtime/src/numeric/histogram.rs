use std::collections::HashSet;

use crate::{
    numeric::error::StatisticsError,
    prelude::{
        collectors::List,
        ensure::{GuardDiagnostic, IntoDiagnostic},
        iter::Iter,
    },
};

const INVALID_BUCKET_CODE: &str = "data.stats.invalid_bucket";
const OUT_OF_RANGE_CODE: &str = "core.numeric.statistics.out_of_range";

/// 仕様上の `HistogramBucket`。
#[derive(Debug, Clone, PartialEq)]
pub struct HistogramBucket {
    pub label: String,
    pub min: f64,
    pub max: f64,
}

impl HistogramBucket {
    pub fn new(label: impl Into<String>, min: f64, max: f64) -> Self {
        Self {
            label: label.into(),
            min,
            max,
        }
    }

    fn contains(&self, value: f64, inclusive_max: bool) -> bool {
        if inclusive_max {
            value >= self.min && value <= self.max
        } else {
            value >= self.min && value < self.max
        }
    }
}

/// `HistogramBucket` の集計状態。
#[derive(Debug, Clone, PartialEq)]
pub struct HistogramBucketState {
    pub bucket: HistogramBucket,
    pub count: u64,
    pub sum: Option<f64>,
}

impl HistogramBucketState {
    pub fn new(bucket: HistogramBucket) -> Self {
        Self {
            bucket,
            count: 0,
            sum: None,
        }
    }

    fn record(&mut self, value: f64) {
        self.count = self.count.saturating_add(1);
        self.sum = Some(self.sum.unwrap_or(0.0) + value);
    }
}

/// 指定されたバケットでヒストグラムを構築する。
pub fn histogram(
    values: Iter<f64>,
    buckets: List<HistogramBucket>,
) -> Result<List<HistogramBucketState>, GuardDiagnostic> {
    if buckets.is_empty() {
        return Err(
            StatisticsError::invalid_parameter("histogram requires at least one bucket")
                .with_rule("H-01")
                .with_context_code(INVALID_BUCKET_CODE)
                .into_diagnostic(),
        );
    }

    let bucket_vec: Vec<HistogramBucket> = buckets.iter().collect();
    validate_buckets(&bucket_vec)?;

    let mut states: Vec<HistogramBucketState> = bucket_vec
        .into_iter()
        .map(HistogramBucketState::new)
        .collect();

    let mut sample_count = 0usize;

    for value in values.into_iter() {
        sample_count += 1;
        if !value.is_finite() {
            return Err(StatisticsError::numerical_instability(format!(
                "encountered non-finite value {value} in histogram input"
            ))
            .with_rule("H-05")
            .with_value(value)
            .into_diagnostic());
        }

        let len = states.len();
        let mut matched = false;
        for (index, state) in states.iter_mut().enumerate() {
            let inclusive_max = index == len - 1;
            if state.bucket.contains(value, inclusive_max) {
                state.record(value);
                matched = true;
                break;
            }
        }
        if !matched {
            return Err(StatisticsError::invalid_parameter(format!(
                "value {value} does not fit any histogram bucket"
            ))
            .with_rule("H-06")
            .with_context_code(OUT_OF_RANGE_CODE)
            .with_value(value)
            .into_diagnostic());
        }
    }

    if sample_count == 0 {
        return Err(StatisticsError::insufficient_data(
            "histogram requires at least one sample value",
        )
        .with_rule("H-07")
        .into_diagnostic());
    }

    Ok(List::from_vec(states))
}

fn validate_buckets(buckets: &[HistogramBucket]) -> Result<(), GuardDiagnostic> {
    let mut last_max = f64::NEG_INFINITY;
    let mut label_set: HashSet<String> = HashSet::new();
    let mut range_set: HashSet<(u64, u64)> = HashSet::new();

    for (index, bucket) in buckets.iter().enumerate() {
        if !bucket.min.is_finite() || !bucket.max.is_finite() {
            return Err(StatisticsError::invalid_parameter(format!(
                "bucket `{}` contains non-finite bounds ({}, {})",
                bucket.label, bucket.min, bucket.max
            ))
            .with_bucket_context(index, bucket.label.clone())
            .with_rule("H-02")
            .with_context_code(INVALID_BUCKET_CODE)
            .into_diagnostic());
        }
        if bucket.min >= bucket.max {
            return Err(StatisticsError::invalid_parameter(format!(
                "bucket `{}` must satisfy min < max (found {} >= {})",
                bucket.label, bucket.min, bucket.max
            ))
            .with_bucket_context(index, bucket.label.clone())
            .with_rule("H-02")
            .with_context_code(INVALID_BUCKET_CODE)
            .into_diagnostic());
        }
        if index > 0 && bucket.min < last_max {
            return Err(StatisticsError::invalid_parameter(format!(
                "bucket `{}` overlaps with previous bucket ({} < previous max {})",
                bucket.label, bucket.min, last_max
            ))
            .with_bucket_context(index, bucket.label.clone())
            .with_rule("H-03")
            .with_context_code(INVALID_BUCKET_CODE)
            .into_diagnostic());
        }
        if !label_set.insert(bucket.label.clone()) {
            return Err(StatisticsError::invalid_parameter(format!(
                "bucket label `{}` is duplicated",
                bucket.label
            ))
            .with_bucket_context(index, bucket.label.clone())
            .with_rule("H-04")
            .with_context_code(INVALID_BUCKET_CODE)
            .into_diagnostic());
        }
        let key = (bucket.min.to_bits(), bucket.max.to_bits());
        if !range_set.insert(key) {
            return Err(StatisticsError::invalid_parameter(format!(
                "bucket range [{}, {}) is duplicated",
                bucket.min, bucket.max
            ))
            .with_bucket_context(index, bucket.label.clone())
            .with_rule("H-04")
            .with_context_code(INVALID_BUCKET_CODE)
            .into_diagnostic());
        }

        last_max = bucket.max;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn iter_from(values: &[f64]) -> Iter<f64> {
        Iter::from_list(values.to_vec())
    }

    #[test]
    fn histogram_counts_values_and_sum() {
        let buckets = List::from_vec(vec![
            HistogramBucket::new("low", 0.0, 10.0),
            HistogramBucket::new("high", 10.0, 20.0),
        ]);
        let values = iter_from(&[1.5, 2.5, 15.0, 19.5, 10.0]);
        let states = histogram(values, buckets).unwrap();
        let vec = states.to_vec();
        assert_eq!(vec.len(), 2);
        assert_eq!(vec[0].bucket.label, "low");
        assert_eq!(vec[0].count, 2);
        assert!((vec[0].sum.unwrap() - 4.0).abs() < f64::EPSILON);
        assert_eq!(vec[1].bucket.label, "high");
        assert_eq!(vec[1].count, 3);
        assert!((vec[1].sum.unwrap() - 44.5).abs() < f64::EPSILON);
    }

    #[test]
    fn histogram_rejects_overlapping_buckets() {
        let buckets = List::from_vec(vec![
            HistogramBucket::new("a", 0.0, 5.0),
            HistogramBucket::new("b", 4.0, 10.0),
        ]);
        let diag = histogram(iter_from(&[1.0]), buckets).unwrap_err();
        assert_eq!(diag.code, INVALID_BUCKET_CODE);
        assert_eq!(
            diag.audit_metadata
                .get("numeric.statistics.rule")
                .and_then(Value::as_str),
            Some("H-03")
        );
    }

    #[test]
    fn histogram_requires_sample_values() {
        let buckets = List::from_vec(vec![HistogramBucket::new("single", 0.0, 5.0)]);
        let diag = histogram(iter_from(&[]), buckets).unwrap_err();
        assert_eq!(diag.code, "core.numeric.statistics.insufficient_data");
    }
}
