//! Pass/fail evaluators aligned with EXPERIMENTS.md criteria.

use super::latency::LatencySamples;
use crate::harness::ExperimentId;

/// Evaluate pass criteria for an experiment given collected metrics.
pub fn evaluate_pass(id: ExperimentId, metrics: &serde_json::Value) -> bool {
    match id {
        ExperimentId::BmC0 => {
            let p50 = metrics.get("p50_ms").and_then(serde_json::Value::as_f64).unwrap_or(0.0);
            let p95 = metrics.get("p95_ms").and_then(serde_json::Value::as_f64).unwrap_or(0.0);
            let slope = metrics
                .get("slope_vs_index")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0);
            p50 > 0.0 && p95 / p50 < 10.0 && slope.abs() < 0.01
        }
        ExperimentId::BmC1 => {
            let b1 = metrics
                .get("events_per_sec_batch_1")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0);
            let b1000 = metrics
                .get("events_per_sec_batch_1000")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0);
            b1000 >= b1 * 0.5
        }
        ExperimentId::BmC2 => {
            let p95_1k = metrics
                .get("p95_poll_ms_1k")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(f64::MAX);
            let p95_100k = metrics
                .get("p95_poll_ms_100k")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(f64::MAX);
            p95_100k <= p95_1k * 2.0
        }
        ExperimentId::BmC3 => {
            let slope = metrics
                .get("decile_p95_slope")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0);
            slope.abs() < 0.05
        }
        ExperimentId::BmC4 => {
            let pre = metrics
                .get("pre_truncate_p95_ms")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(1.0);
            let post = metrics
                .get("post_truncate_p95_ms")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(f64::MAX);
            post <= pre * 1.5
        }
        ExperimentId::BmC5 => {
            let ratio = metrics
                .get("growth_ratio_same_vs_isolated")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(1.0);
            ratio > 1.0
        }
        ExperimentId::BmC6 => {
            let ratio = metrics
                .get("growth_ratio")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(f64::MAX);
            ratio < 2.0
        }
        ExperimentId::BmL0
        | ExperimentId::BmL1
        | ExperimentId::BmL2
        | ExperimentId::BmL3 => {
            let error_rate = metrics
                .get("error_rate")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(1.0);
            error_rate < 0.001
        }
        ExperimentId::BmP1 => {
            let ops = metrics
                .get("achieved_ops_per_sec")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0);
            ops > 0.0
        }
        ExperimentId::BmP2 => {
            let expected = metrics
                .get("expected_rows")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);
            let read = metrics
                .get("rows_read")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);
            read == expected && expected > 0
        }
        ExperimentId::BmM1 | ExperimentId::BmM2 => {
            let error_rate = metrics
                .get("error_rate")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(1.0);
            error_rate < 0.001
        }
    }
}

/// Build a summary string for EXPERIMENTS.md Results column.
pub fn results_summary(id: ExperimentId, metrics: &serde_json::Value, pass: bool) -> String {
    let status = if pass { "PASS" } else { "FAIL" };
    match id {
        ExperimentId::BmC0 => format!(
            "p50={:.3}ms p95={:.3}ms {}",
            metrics.get("p50_ms").and_then(serde_json::Value::as_f64).unwrap_or(0.0),
            metrics.get("p95_ms").and_then(serde_json::Value::as_f64).unwrap_or(0.0),
            status
        ),
        ExperimentId::BmC1 => format!(
            "1={:.0}/s 1000={:.0}/s {}",
            metrics
                .get("events_per_sec_batch_1")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0),
            metrics
                .get("events_per_sec_batch_1000")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0),
            status
        ),
        ExperimentId::BmC2 => format!(
            "p95@100k={:.3}ms {}",
            metrics
                .get("p95_poll_ms_100k")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0),
            status
        ),
        ExperimentId::BmC3 => format!(
            "p95={:.3}ms {}",
            metrics.get("p95_ms").and_then(serde_json::Value::as_f64).unwrap_or(0.0),
            status
        ),
        ExperimentId::BmC4 => format!(
            "post/pre={:.2}x {}",
            metrics
                .get("post_truncate_p95_ms")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0)
                / metrics
                    .get("pre_truncate_p95_ms")
                    .and_then(serde_json::Value::as_f64)
                    .unwrap_or(1.0),
            status
        ),
        ExperimentId::BmC5 => format!(
            "ratio={:.2} {}",
            metrics
                .get("growth_ratio_same_vs_isolated")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0),
            status
        ),
        ExperimentId::BmC6 => format!(
            "growth={:.2}x {}",
            metrics
                .get("growth_ratio")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0),
            status
        ),
        ExperimentId::BmL0
        | ExperimentId::BmL1
        | ExperimentId::BmL2
        | ExperimentId::BmL3 => format!(
            "p99={:.3}ms {:.0}/s err={:.4}% {}",
            metrics.get("p99_ms").and_then(serde_json::Value::as_f64).unwrap_or(0.0),
            metrics
                .get("achieved_ops_per_sec")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0),
            metrics
                .get("error_rate")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0)
                * 100.0,
            status
        ),
        ExperimentId::BmP1 => format!(
            "K={} {:.0}/s {}",
            metrics
                .get("partition_count")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0),
            metrics
                .get("achieved_ops_per_sec")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0),
            status
        ),
        ExperimentId::BmP2 => format!(
            "read={}/{} {}",
            metrics
                .get("rows_read")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0),
            metrics
                .get("expected_rows")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0),
            status
        ),
        ExperimentId::BmM1 | ExperimentId::BmM2 => format!(
            "C={} {:.0}/s err={:.4}% {}",
            metrics
                .get("client_count")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0),
            metrics
                .get("achieved_ops_per_sec")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0),
            metrics
                .get("error_rate")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0)
                * 100.0,
            status
        ),
    }
}

/// Helper to merge latency stats into JSON object.
pub fn latency_to_json(samples: &LatencySamples) -> serde_json::Value {
    serde_json::json!({
        "p50_ms": samples.p50(),
        "p95_ms": samples.p95(),
        "p99_ms": samples.p99(),
        "mean_ms": samples.mean(),
        "sample_count": samples.len(),
        "slope_vs_index": samples.slope_vs_index(),
        "decile_p95_slope": samples.decile_p95_slope(),
    })
}
