//! Shared helpers for benchmark metrics.

/// Convert `u64` to `f64` for ratio math (RSS/disk bytes may exceed `f64` mantissa precision).
#[expect(clippy::cast_precision_loss)]
pub fn u64_to_f64(v: u64) -> f64 {
    v as f64
}

/// Convert `usize` to `f64` for sample statistics.
pub fn usize_to_f64(v: usize) -> f64 {
    f64::from(u32::try_from(v).unwrap_or(u32::MAX))
}
