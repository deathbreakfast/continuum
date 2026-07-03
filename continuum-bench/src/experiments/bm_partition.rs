//! Multi-partition benchmark experiments BM-P1 and BM-P2.

use std::time::Instant;

use anyhow::Result;
use continuum::backend::LogBackend;
use continuum::types::LogStreamId;
use serde_json::{json, Value};

use crate::experiments::fixtures::{bench_record, bench_stream};
use crate::experiments::bm_core::ExperimentContext;
use crate::harness::ExperimentId;
use crate::metrics::LatencySamples;
use crate::util::{u64_to_f64, usize_to_f64};

fn partition_count() -> usize {
    std::env::var("CONTINUUM_BENCH_PARTITION_COUNT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10)
}

fn keyed_stream(ctx: &ExperimentContext, topic: &str, partition: usize) -> LogStreamId {
    let mut stream = bench_stream(ctx.storage, topic);
    stream.key = Some(format!("partition_{partition}"));
    stream
}

pub async fn run_bm_p1(ctx: &ExperimentContext) -> Result<Value> {
    let k = partition_count();
    let backend = &ctx.handle.backend;
    let total_ops = 5_000usize;
    let start = Instant::now();
    let mut samples = LatencySamples::new();
    let mut ops_ok = 0u64;

    for i in 0..total_ops {
        let stream = keyed_stream(ctx, "bm-p1", i % k);
        let op_start = Instant::now();
        backend.append(stream, &[bench_record()]).await?;
        samples.record(op_start.elapsed());
        ops_ok += 1;
    }

    let elapsed = start.elapsed().as_secs_f64();
    Ok(json!({
        "partition_count": k,
        "total_ops": total_ops,
        "achieved_ops_per_sec": u64_to_f64(ops_ok) / elapsed,
        "p50_ms": samples.p50(),
        "p95_ms": samples.p95(),
        "p99_ms": samples.p99(),
    }))
}

pub async fn run_bm_p2(ctx: &ExperimentContext) -> Result<Value> {
    let k = partition_count();
    let backend = &ctx.handle.backend;
    let per_partition = 100usize;
    let mut streams = Vec::with_capacity(k);

    for p in 0..k {
        let stream = keyed_stream(ctx, "bm-p2", p);
        for _ in 0..per_partition {
            backend.append(stream.clone(), &[bench_record()]).await?;
        }
        streams.push(stream);
    }

    let start = Instant::now();
    let mut read_total = 0usize;
    for stream in streams {
        let rows = backend.read_from(stream, continuum::types::Seq::ZERO, per_partition).await?;
        read_total += rows.len();
    }
    let elapsed = start.elapsed().as_secs_f64();

    Ok(json!({
        "partition_count": k,
        "rows_read": read_total,
        "read_ops_per_sec": usize_to_f64(read_total) / elapsed,
        "expected_rows": k * per_partition,
    }))
}

pub async fn run_partition(ctx: &ExperimentContext, id: ExperimentId) -> Result<Value> {
    match id {
        ExperimentId::BmP1 => run_bm_p1(ctx).await,
        ExperimentId::BmP2 => run_bm_p2(ctx).await,
        _ => unreachable!(),
    }
}
