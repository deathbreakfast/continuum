//! Multi-client benchmark experiments BM-M1 through BM-M4.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;
use continuum::backend::LogBackend;
use continuum::types::LogStreamId;
use serde_json::{json, Value};
use tokio::task::JoinSet;

use crate::experiments::fixtures::{bench_record, bench_stream};
use crate::experiments::bm_core::ExperimentContext;
use crate::harness::backend::BenchBackend;
use crate::harness::ExperimentId;
use crate::metrics::LatencySamples;
use crate::util::u64_to_f64;

const LOAD_DURATION_SECS: u64 = 30;

fn client_count(default: usize) -> usize {
    std::env::var("CONTINUUM_BENCH_CLIENT_COUNT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

fn partition_count(default: usize) -> usize {
    std::env::var("CONTINUUM_BENCH_PARTITION_COUNT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

fn partition_offset() -> usize {
    std::env::var("CONTINUUM_BENCH_PARTITION_OFFSET")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}

fn worker_stream(ctx: &ExperimentContext, topic: &str, worker: usize) -> LogStreamId {
    let mut stream = bench_stream(ctx.storage, topic);
    stream.key = Some(format!("worker_{worker}"));
    stream
}

/// Shared hot stream (`key=None`) — all clients contend on one partition.
fn hot_stream(ctx: &ExperimentContext, topic: &str) -> LogStreamId {
    bench_stream(ctx.storage, topic)
}

/// Round-robin partition key for concurrent spread-key workload.
fn partition_stream(ctx: &ExperimentContext, topic: &str, worker: usize, k: usize) -> LogStreamId {
    let mut stream = bench_stream(ctx.storage, topic);
    let offset = partition_offset();
    let idx = offset + (worker % k);
    stream.key = Some(format!("partition_{idx}"));
    stream
}

pub async fn run_bm_m1(ctx: &ExperimentContext) -> Result<Value> {
    let clients = client_count(8);
    let backend = Arc::clone(&ctx.handle.backend);
    let duration = Duration::from_secs(LOAD_DURATION_SECS);
    let ops_ok = Arc::new(AtomicU64::new(0));
    let ops_err = Arc::new(AtomicU64::new(0));
    let start = Instant::now();
    let mut set = JoinSet::new();

    for worker in 0..clients {
        let backend = Arc::clone(&backend);
        let ops_ok = Arc::clone(&ops_ok);
        let ops_err = Arc::clone(&ops_err);
        let stream = worker_stream(ctx, "bm-m1", worker);
        set.spawn(async move {
            let worker_start = Instant::now();
            while worker_start.elapsed() < duration {
                match backend.append(stream.clone(), &[bench_record()]).await {
                    Ok(_) => {
                        ops_ok.fetch_add(1, Ordering::Relaxed);
                    }
                    Err(_) => {
                        ops_err.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }
        });
    }

    while set.join_next().await.is_some() {}

    let elapsed = start.elapsed().as_secs_f64();
    let ok = ops_ok.load(Ordering::Relaxed);
    let err = ops_err.load(Ordering::Relaxed);
    let total = ok + err;
    Ok(json!({
        "client_count": clients,
        "duration_secs": elapsed,
        "ops_ok": ok,
        "ops_err": err,
        "achieved_ops_per_sec": u64_to_f64(ok) / elapsed,
        "error_rate": if total == 0 { 0.0 } else { u64_to_f64(err) / u64_to_f64(total) },
    }))
}

pub async fn run_bm_m2(ctx: &ExperimentContext) -> Result<Value> {
    let clients = client_count(64);
    let backend = Arc::clone(&ctx.handle.backend);
    let duration = Duration::from_secs(LOAD_DURATION_SECS);
    let start = Instant::now();
    let samples = Arc::new(tokio::sync::Mutex::new(LatencySamples::new()));
    let ops_ok = Arc::new(AtomicU64::new(0));
    let ops_err = Arc::new(AtomicU64::new(0));
    let mut set = JoinSet::new();

    for worker in 0..clients {
        let backend: Arc<BenchBackend> = Arc::clone(&backend);
        let ops_ok = Arc::clone(&ops_ok);
        let ops_err = Arc::clone(&ops_err);
        let samples = Arc::clone(&samples);
        let stream = worker_stream(ctx, "bm-m2", worker);
        set.spawn(async move {
            let worker_start = Instant::now();
            while worker_start.elapsed() < duration {
                let op_start = Instant::now();
                match backend.append(stream.clone(), &[bench_record()]).await {
                    Ok(_) => {
                        ops_ok.fetch_add(1, Ordering::Relaxed);
                        samples.lock().await.record(op_start.elapsed());
                    }
                    Err(_) => {
                        ops_err.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }
        });
    }

    while set.join_next().await.is_some() {}

    let elapsed = start.elapsed().as_secs_f64();
    let ok = ops_ok.load(Ordering::Relaxed);
    let err = ops_err.load(Ordering::Relaxed);
    let total = ok + err;
    let samples = samples.lock().await;

    Ok(json!({
        "client_count": clients,
        "partitions_modeled": clients,
        "clients_modeled": clients,
        "duration_secs": elapsed,
        "ops_ok": ok,
        "ops_err": err,
        "achieved_ops_per_sec": u64_to_f64(ok) / elapsed,
        "error_rate": if total == 0 { 0.0 } else { u64_to_f64(err) / u64_to_f64(total) },
        "p50_ms": samples.p50(),
        "p95_ms": samples.p95(),
        "p99_ms": samples.p99(),
    }))
}

/// BM-M3: concurrent clients on a single hot stream (`key=None`).
pub async fn run_bm_m3(ctx: &ExperimentContext) -> Result<Value> {
    let clients = client_count(64);
    let backend = Arc::clone(&ctx.handle.backend);
    let duration = Duration::from_secs(LOAD_DURATION_SECS);
    let start = Instant::now();
    let samples = Arc::new(tokio::sync::Mutex::new(LatencySamples::new()));
    let ops_ok = Arc::new(AtomicU64::new(0));
    let ops_err = Arc::new(AtomicU64::new(0));
    let stream = hot_stream(ctx, "bm-m3");
    let mut set = JoinSet::new();

    for _worker in 0..clients {
        let backend: Arc<BenchBackend> = Arc::clone(&backend);
        let ops_ok = Arc::clone(&ops_ok);
        let ops_err = Arc::clone(&ops_err);
        let samples = Arc::clone(&samples);
        let stream = stream.clone();
        set.spawn(async move {
            let worker_start = Instant::now();
            while worker_start.elapsed() < duration {
                let op_start = Instant::now();
                match backend.append(stream.clone(), &[bench_record()]).await {
                    Ok(_) => {
                        ops_ok.fetch_add(1, Ordering::Relaxed);
                        samples.lock().await.record(op_start.elapsed());
                    }
                    Err(_) => {
                        ops_err.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }
        });
    }

    while set.join_next().await.is_some() {}

    let elapsed = start.elapsed().as_secs_f64();
    let ok = ops_ok.load(Ordering::Relaxed);
    let err = ops_err.load(Ordering::Relaxed);
    let total = ok + err;
    let samples = samples.lock().await;

    Ok(json!({
        "client_count": clients,
        "clients_modeled": clients,
        "hot_stream": true,
        "duration_secs": elapsed,
        "ops_ok": ok,
        "ops_err": err,
        "achieved_ops_per_sec": u64_to_f64(ok) / elapsed,
        "error_rate": if total == 0 { 0.0 } else { u64_to_f64(err) / u64_to_f64(total) },
        "p50_ms": samples.p50(),
        "p95_ms": samples.p95(),
        "p99_ms": samples.p99(),
    }))
}

/// BM-M4: concurrent clients spread across K partition keys.
pub async fn run_bm_m4(ctx: &ExperimentContext) -> Result<Value> {
    let clients = client_count(64);
    let k = partition_count(clients);
    let backend = Arc::clone(&ctx.handle.backend);
    let duration = Duration::from_secs(LOAD_DURATION_SECS);
    let start = Instant::now();
    let samples = Arc::new(tokio::sync::Mutex::new(LatencySamples::new()));
    let ops_ok = Arc::new(AtomicU64::new(0));
    let ops_err = Arc::new(AtomicU64::new(0));
    let mut set = JoinSet::new();

    for worker in 0..clients {
        let backend: Arc<BenchBackend> = Arc::clone(&backend);
        let ops_ok = Arc::clone(&ops_ok);
        let ops_err = Arc::clone(&ops_err);
        let samples = Arc::clone(&samples);
        let stream = partition_stream(ctx, "bm-m4", worker, k);
        set.spawn(async move {
            let worker_start = Instant::now();
            while worker_start.elapsed() < duration {
                let op_start = Instant::now();
                match backend.append(stream.clone(), &[bench_record()]).await {
                    Ok(_) => {
                        ops_ok.fetch_add(1, Ordering::Relaxed);
                        samples.lock().await.record(op_start.elapsed());
                    }
                    Err(_) => {
                        ops_err.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }
        });
    }

    while set.join_next().await.is_some() {}

    let elapsed = start.elapsed().as_secs_f64();
    let ok = ops_ok.load(Ordering::Relaxed);
    let err = ops_err.load(Ordering::Relaxed);
    let total = ok + err;
    let samples = samples.lock().await;

    Ok(json!({
        "client_count": clients,
        "clients_modeled": clients,
        "partition_count": k,
        "partitions_modeled": k,
        "partition_offset": partition_offset(),
        "duration_secs": elapsed,
        "ops_ok": ok,
        "ops_err": err,
        "achieved_ops_per_sec": u64_to_f64(ok) / elapsed,
        "error_rate": if total == 0 { 0.0 } else { u64_to_f64(err) / u64_to_f64(total) },
        "p50_ms": samples.p50(),
        "p95_ms": samples.p95(),
        "p99_ms": samples.p99(),
    }))
}

pub async fn run_multi_client(ctx: &ExperimentContext, id: ExperimentId) -> Result<Value> {
    match id {
        ExperimentId::BmM1 => run_bm_m1(ctx).await,
        ExperimentId::BmM2 => run_bm_m2(ctx).await,
        ExperimentId::BmM3 => run_bm_m3(ctx).await,
        ExperimentId::BmM4 => run_bm_m4(ctx).await,
        _ => unreachable!(),
    }
}
