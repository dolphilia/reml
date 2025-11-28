use std::collections::VecDeque;
use std::mem;

use crate::numeric::effects;
use crate::prelude::iter::Iter;

/// 直近 `window` 件の平均を返す遅延 `Iter`。
/// `window = 0` の場合は空の `Iter` を返す。
pub fn rolling_average(window: usize, values: Iter<f64>) -> Iter<f64> {
    if window == 0 {
        return Iter::empty();
    }
    effects::record_mem_copy(window.saturating_mul(mem::size_of::<f64>()));
    let state = RollingWindowState::new(window);
    values.scan(state, move |state, value| state.push(value))
}

/// Z スコアを計算する。`stddev <= 0` や非有限値の場合は `None`。
pub fn z_score(value: f64, mean: f64, stddev: f64) -> Option<f64> {
    if stddev <= 0.0 || !stddev.is_finite() {
        return None;
    }
    let delta = value - mean;
    if !delta.is_finite() {
        return None;
    }
    let score = delta / stddev;
    if score.is_finite() {
        Some(score)
    } else {
        None
    }
}

#[derive(Clone, Debug)]
struct RollingWindowState {
    window: usize,
    sum: f64,
    values: VecDeque<f64>,
}

impl RollingWindowState {
    fn new(window: usize) -> Self {
        Self {
            window,
            sum: 0.0,
            values: VecDeque::with_capacity(window),
        }
    }

    fn push(&mut self, value: f64) -> Option<f64> {
        if !value.is_finite() {
            self.values.clear();
            self.sum = f64::NAN;
            return Some(f64::NAN);
        }
        self.values.push_back(value);
        self.sum += value;
        if self.values.len() > self.window {
            if let Some(removed) = self.values.pop_front() {
                self.sum -= removed;
            }
        }
        if self.values.len() == self.window {
            Some(self.sum / self.window as f64)
        } else {
            None
        }
    }
}
