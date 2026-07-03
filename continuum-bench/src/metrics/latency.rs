//! Latency sample collection and percentile computation.

use crate::util::usize_to_f64;

/// Collect wall-clock samples in milliseconds.
#[derive(Debug, Default, Clone)]
pub struct LatencySamples {
    samples_ms: Vec<f64>,
}

impl LatencySamples {
    pub const fn new() -> Self {
        Self {
            samples_ms: Vec::new(),
        }
    }

    pub fn with_capacity(cap: usize) -> Self {
        Self {
            samples_ms: Vec::with_capacity(cap),
        }
    }

    pub fn record(&mut self, duration: std::time::Duration) {
        self.samples_ms.push(duration.as_secs_f64() * 1000.0);
    }

    pub const fn len(&self) -> usize {
        self.samples_ms.len()
    }

    pub fn percentile(&self, p: f64) -> f64 {
        if self.samples_ms.is_empty() {
            return 0.0;
        }
        let mut sorted = self.samples_ms.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let max_idx = sorted.len() - 1;
        let idx = percentile_index(max_idx, p);
        sorted[idx.min(max_idx)]
    }

    pub fn p50(&self) -> f64 {
        self.percentile(50.0)
    }

    pub fn p95(&self) -> f64 {
        self.percentile(95.0)
    }

    pub fn p99(&self) -> f64 {
        self.percentile(99.0)
    }

    pub fn mean(&self) -> f64 {
        if self.samples_ms.is_empty() {
            return 0.0;
        }
        self.samples_ms.iter().sum::<f64>() / usize_to_f64(self.samples_ms.len())
    }

    /// Simple linear regression slope of sample index vs latency.
    pub fn slope_vs_index(&self) -> f64 {
        let n = self.samples_ms.len();
        if n < 2 {
            return 0.0;
        }
        let n_f = usize_to_f64(n);
        let sum_x: f64 = (0..n).map(usize_to_f64).sum();
        let sum_y: f64 = self.samples_ms.iter().sum();
        let sum_cross: f64 = self
            .samples_ms
            .iter()
            .enumerate()
            .map(|(i, y)| usize_to_f64(i) * y)
            .sum();
        let sum_x2: f64 = (0..n).map(|i| usize_to_f64(i).powi(2)).sum();
        let denom = sum_x.mul_add(-sum_x, n_f * sum_x2);
        if denom.abs() < f64::EPSILON {
            return 0.0;
        }
        sum_x.mul_add(-sum_y, n_f * sum_cross) / denom
    }

    /// Slope of decile midpoints vs decile p95 values.
    pub fn decile_p95_slope(&self) -> f64 {
        if self.samples_ms.len() < 20 {
            return 0.0;
        }
        let chunk = (self.samples_ms.len() / 10).max(1);
        let mut xs = Vec::new();
        let mut ys = Vec::new();
        for (i, window) in self.samples_ms.chunks(chunk).enumerate().take(10) {
            let sub = Self {
                samples_ms: window.to_vec(),
            };
            xs.push(usize_to_f64(i));
            ys.push(sub.p95());
        }
        linear_slope(&xs, &ys)
    }
}

#[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn percentile_index(max_idx: usize, p: f64) -> usize {
    (p / 100.0 * usize_to_f64(max_idx)).round() as usize
}

fn linear_slope(xs: &[f64], ys: &[f64]) -> f64 {
    let n = xs.len();
    if n < 2 {
        return 0.0;
    }
    let n_f = usize_to_f64(n);
    let sum_x: f64 = xs.iter().sum();
    let sum_y: f64 = ys.iter().sum();
    let sum_cross: f64 = xs.iter().zip(ys).map(|(x, y)| x * y).sum();
    let sum_x2: f64 = xs.iter().map(|x| x * x).sum();
    let denom = sum_x.mul_add(-sum_x, n_f * sum_x2);
    if denom.abs() < f64::EPSILON {
        return 0.0;
    }
    sum_x.mul_add(-sum_y, n_f * sum_cross) / denom
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn percentiles_ordered() {
        let mut s = LatencySamples::new();
        for i in 1..=100 {
            s.record(Duration::from_millis(u64::try_from(i).expect("test index")));
        }
        assert!((s.p50() - 50.0).abs() < 2.0);
        assert!((s.p95() - 95.0).abs() < 2.0);
        assert!((s.p99() - 99.0).abs() < 2.0);
    }

    #[test]
    fn flat_slope_near_zero() {
        let mut s = LatencySamples::new();
        for _ in 0..100 {
            s.record(Duration::from_millis(1));
        }
        assert!(s.slope_vs_index().abs() < 0.01);
    }
}
