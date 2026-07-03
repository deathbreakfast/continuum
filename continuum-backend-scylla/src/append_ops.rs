//! Optional per-append round-trip / op counters (env `CONTINUUM_APPEND_DEBUG_OPS=1`).

use std::sync::atomic::{AtomicU64, Ordering};

static ROUND_TRIPS: AtomicU64 = AtomicU64::new(0);
static OPS: AtomicU64 = AtomicU64::new(0);

fn enabled() -> bool {
    std::env::var("CONTINUUM_APPEND_DEBUG_OPS")
        .ok()
        .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true"))
}

/// Record one storage round-trip comprising `ops` CQL statements.
pub fn record_round_trip(ops: u64) {
    if enabled() {
        ROUND_TRIPS.fetch_add(1, Ordering::Relaxed);
        OPS.fetch_add(ops.max(1), Ordering::Relaxed);
    }
}

/// Reset counters (for micro-bench tests).
pub fn reset() {
    ROUND_TRIPS.store(0, Ordering::Relaxed);
    OPS.store(0, Ordering::Relaxed);
}

/// Snapshot `(round_trips, ops)`.
#[must_use]
pub fn snapshot() -> (u64, u64) {
    (
        ROUND_TRIPS.load(Ordering::Relaxed),
        OPS.load(Ordering::Relaxed),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counters_disabled_by_default() {
        reset();
        record_round_trip(3);
        assert_eq!(snapshot(), (0, 0));
    }

    #[test]
    fn counters_when_enabled() {
        std::env::set_var("CONTINUUM_APPEND_DEBUG_OPS", "1");
        reset();
        record_round_trip(2);
        record_round_trip(1);
        assert_eq!(snapshot(), (2, 3));
        std::env::remove_var("CONTINUUM_APPEND_DEBUG_OPS");
    }
}
