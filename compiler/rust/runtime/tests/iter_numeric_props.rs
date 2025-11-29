#![cfg(feature = "core-numeric")]

#[cfg(feature = "decimal")]
use reml_runtime::numeric::Decimal;
use reml_runtime::numeric::{
    median, rolling_average, take_numeric_effects_snapshot, z_score, IterNumericExt,
};
use reml_runtime::prelude::iter::Iter;

fn collect_values(iter: Iter<f64>) -> Vec<f64> {
    let (core_vec, _) = iter
        .collect_numeric()
        .expect("numeric collector should succeed")
        .into_parts();
    core_vec.into_inner()
}

fn manual_rolling_average(values: &[f64], window: usize) -> Vec<f64> {
    if window == 0 || values.len() < window {
        return Vec::new();
    }
    values
        .windows(window)
        .map(|slice| slice.iter().sum::<f64>() / window as f64)
        .collect()
}

fn sample_sequence(seed: u64, len: usize) -> Vec<f64> {
    let mut state = seed;
    let mut output = Vec::with_capacity(len);
    for _ in 0..len {
        state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let fraction = ((state >> 12) & ((1u64 << 52) - 1)) as f64 / (1u64 << 52) as f64;
        output.push(fraction * 20_000.0 - 10_000.0);
    }
    output
}

fn manual_lower_median(mut values: Vec<f64>) -> f64 {
    values.sort_by(|a, b| a.partial_cmp(b).expect("no NaN in sample sequence"));
    let mid = values.len() / 2;
    if values.len() % 2 == 0 {
        values[mid - 1]
    } else {
        values[mid]
    }
}

#[test]
fn rolling_average_matches_manual_samples() {
    for seed in 1..=6 {
        let values = sample_sequence(seed, 12 + seed as usize);
        for window in 1..=5 {
            if window > values.len() {
                continue;
            }
            let iter = Iter::from_list(values.clone());
            let derived = collect_values(rolling_average(window, iter));
            let expected = manual_rolling_average(&values, window);
            assert_eq!(
                derived.len(),
                expected.len(),
                "seed={seed}, window={window}"
            );
            for (a, b) in derived.iter().zip(expected.iter()) {
                assert!(
                    (a - b).abs() < 1e-8,
                    "seed={seed}, window={window}, a={a}, b={b}"
                );
            }
        }
    }
}

#[test]
fn z_score_matches_reference_samples() {
    let samples = [
        (0.0, 0.0, 1.0),
        (12.5, 10.0, 2.5),
        (-4.0, -2.0, 0.5),
        (1e-6, -1e-6, 1e-3),
        (0.0, -5000.0, 1000.0),
    ];
    for (value, mean, stddev) in samples {
        let expected = (value - mean) / stddev;
        let actual = z_score(value, mean, stddev).expect("valid z_score");
        assert!((actual - expected).abs() < 1e-12);
    }
}

#[test]
fn z_score_rejects_invalid_cases() {
    assert!(z_score(1.0, 0.0, 0.0).is_none());
    assert!(z_score(1.0, 0.0, -10.0).is_none());
    assert!(z_score(f64::INFINITY, 0.0, 1.0).is_none());
}

#[test]
fn rolling_average_records_mem_effect() {
    let iter = rolling_average(3, Iter::from_list(vec![1.0, 2.0, 3.0, 4.0]));
    // 評価せずともメモリ確保が effect に記録されることを確認する。
    let _ = iter;
    let effects = take_numeric_effects_snapshot();
    assert!(effects.mem, "effect {{mem}} should be set");
    assert!(
        effects.mem_bytes >= 3 * std::mem::size_of::<f64>(),
        "mem_bytes should reflect buffer allocation"
    );
}

#[test]
fn median_matches_manual_lower_median() {
    for seed in 1..=10u64 {
        for len in 3..=17 {
            let values = sample_sequence(seed * 19, len);
            let expected = manual_lower_median(values.clone());
            let actual = median(Iter::from_list(values)).expect("non-empty data");
            assert!(
                (actual - expected).abs() < 1e-9,
                "seed={seed}, len={len}, expected={expected}, actual={actual}"
            );
        }
    }
}

#[cfg(feature = "decimal")]
#[test]
fn decimal_median_records_mem_effect() {
    let values = vec![
        Decimal::new(10, 1),
        Decimal::new(20, 1),
        Decimal::new(30, 1),
        Decimal::new(40, 1),
    ];
    let _ = take_numeric_effects_snapshot();
    let iter = Iter::from_list(values.clone());
    let _ = iter.median();
    let effects = take_numeric_effects_snapshot();
    assert!(
        effects.mem,
        "effect {{mem}} should be recorded for decimal median"
    );
    assert!(
        effects.mem_bytes > 0,
        "mem_bytes should record allocation size"
    );
}
