//! Load benchmark experiments BM-L0 through BM-L3.

use std::time::{Duration, Instant};

use anyhow::Result;
use continuum::backend::LogBackend;
use serde_json::{json, Value};
use tokio::time::sleep;

use crate::experiments::fixtures::{bench_record, bench_stream};
use crate::experiments::bm_core::ExperimentContext;
use crate::harness::ExperimentId;
use crate::metrics::LatencySamples;
use crate::util::u64_to_f64;

const LOAD_DURATION_SECS: u64 = 60;

pub async fn run_load(ctx: &ExperimentContext, id: ExperimentId) -> Result<Value> {
    let target_ops = match id {
        ExperimentId::BmL0 => 100,
        ExperimentId::BmL1 => 1_000,
        ExperimentId::BmL2 => 10_000,
        ExperimentId::BmL3 => 100_000,
        _ => unreachable!(),
    };

    let backend = &ctx.handle.backend;
    let stream = bench_stream(ctx.storage, id.slug());
    let duration = Duration::from_secs(LOAD_DURATION_SECS);
    let start = Instant::now();
    let mut samples = LatencySamples::new();
    let mut ops_ok = 0u64;
    let mut ops_err = 0u64;
    let mut next_tick = Instant::now();

    while start.elapsed() < duration {
        if Instant::now() < next_tick {
            sleep(next_tick - Instant::now()).await;
        }
        next_tick += Duration::from_nanos(1_000_000_000 / u64::try_from(target_ops).unwrap_or(1));

        let op_start = Instant::now();
        match backend.append(stream.clone(), &[bench_record()]).await {
            Ok(_) => {
                ops_ok += 1;
                samples.record(op_start.elapsed());
            }
            Err(_) => ops_err += 1,
        }
    }

    let elapsed = start.elapsed().as_secs_f64();
    let total = ops_ok + ops_err;
    let error_rate = if total == 0 {
        0.0
    } else {
        u64_to_f64(ops_err) / u64_to_f64(total)
    };

    Ok(json!({
        "target_ops_per_sec": target_ops,
        "achieved_ops_per_sec": u64_to_f64(ops_ok) / elapsed,
        "duration_secs": elapsed,
        "ops_ok": ops_ok,
        "ops_err": ops_err,
        "error_rate": error_rate,
        "p50_ms": samples.p50(),
        "p95_ms": samples.p95(),
        "p99_ms": samples.p99(),
    }))
}
