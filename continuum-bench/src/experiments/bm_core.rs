//! Core benchmark experiments BM-C0 through BM-C6.

use std::time::{Duration, Instant};

use anyhow::Result;
use continuum::backend::LogBackend;
use continuum::types::{Seq, SubscriptionId};
use serde_json::{json, Value};
use tokio::time::sleep;

use crate::experiments::fixtures::{bench_record, bench_records, bench_stream, preload_stream};
use crate::harness::{build_backend, open_shared_postgres, open_shared_sqlite, open_shared_surreal, BackendHandle, BenchBackend, SharedHandle, Storage};
use crate::harness::{RunDimensions, Topology};
use crate::metrics::{growth_ratio, latency_to_json, process_rss_bytes, LatencySamples};
use crate::util::{u64_to_f64, usize_to_f64};

pub struct ExperimentContext {
    pub handle: BackendHandle,
    pub storage: Storage,
}

pub async fn prepare_context(dims: RunDimensions) -> Result<ExperimentContext> {
    let shared = if dims.topology == Topology::SharedHandle {
        match dims.storage {
            Storage::SurrealMem | Storage::SurrealRocksdb => {
                let (db, temp) = open_shared_surreal(dims.storage).await?;
                std::mem::forget(temp);
                Some(SharedHandle::Surreal(db))
            }
            Storage::Sqlite => {
                let (pool, temp) = open_shared_sqlite().await?;
                std::mem::forget(temp);
                Some(SharedHandle::Sqlite(pool))
            }
            Storage::Postgres => Some(SharedHandle::Postgres(open_shared_postgres().await?)),
            Storage::Mem => None,
        }
    } else {
        None
    };

    let handle = build_backend(dims.storage, dims.topology, dims.telemetry, shared).await?;
    Ok(ExperimentContext {
        handle,
        storage: dims.storage,
    })
}

pub async fn run_bm_c0(ctx: &ExperimentContext) -> Result<Value> {
    let backend = &ctx.handle.backend;
    let stream = bench_stream(ctx.storage, "bm-c0");
    let mut samples = LatencySamples::with_capacity(5000);

    for _ in 0..5000 {
        let start = Instant::now();
        backend
            .append(stream.clone(), &[bench_record()])
            .await?;
        samples.record(start.elapsed());
    }

    Ok(latency_to_json(&samples))
}

pub async fn run_bm_c1(ctx: &ExperimentContext) -> Result<Value> {
    let backend = &ctx.handle.backend;
    let total_events = 10_000usize;
    let mut out = json!({});

    for batch_size in [1usize, 10, 100, 1000] {
        let stream = bench_stream(ctx.storage, &format!("bm-c1-{batch_size}"));
        let batches = total_events / batch_size;
        let start = Instant::now();
        for _ in 0..batches {
            let recs = bench_records(batch_size);
            backend.append(stream.clone(), &recs).await?;
        }
        let elapsed = start.elapsed().as_secs_f64();
        let eps = usize_to_f64(total_events) / elapsed;
        out[format!("events_per_sec_batch_{batch_size}")] = json!(eps);
    }

    Ok(out)
}

pub async fn run_bm_c2(ctx: &ExperimentContext) -> Result<Value> {
    let backend = &ctx.handle.backend;
    let mut out = json!({});

    for size in [1_000usize, 10_000, 100_000] {
        let stream = bench_stream(ctx.storage, &format!("bm-c2-{size}"));
        let last = preload_stream(backend, stream.clone(), size).await?;
        let mut samples = LatencySamples::with_capacity(200);
        for _ in 0..200 {
            let start = Instant::now();
            backend.read_from(stream.clone(), last, 100).await?;
            samples.record(start.elapsed());
        }
        out[format!("p95_poll_ms_{}", size_label(size))] = json!(samples.p95());
        out[format!("p50_poll_ms_{}", size_label(size))] = json!(samples.p50());
    }

    Ok(out)
}

fn size_label(n: usize) -> &'static str {
    match n {
        1_000 => "1k",
        10_000 => "10k",
        100_000 => "100k",
        _ => "other",
    }
}

pub async fn run_bm_c3(ctx: &ExperimentContext) -> Result<Value> {
    let backend = &ctx.handle.backend;
    let stream = bench_stream(ctx.storage, "bm-c3");
    let sub = SubscriptionId::new("bm-c3-sub");

    let seqs = backend
        .append(stream.clone(), &bench_records(10_000))
        .await?;
    let mut samples = LatencySamples::with_capacity(10_000);

    for seq in seqs {
        let start = Instant::now();
        backend
            .commit_checkpoint(&sub, stream.clone(), seq)
            .await?;
        samples.record(start.elapsed());
    }

    let mut metrics = latency_to_json(&samples);
    if let Some(obj) = metrics.as_object_mut() {
        obj.insert("decile_p95_slope".into(), json!(samples.decile_p95_slope()));
    }
    Ok(metrics)
}

pub async fn run_bm_c4(ctx: &ExperimentContext) -> Result<Value> {
    let backend = &ctx.handle.backend;
    let stream = bench_stream(ctx.storage, "bm-c4");
    let count = 50_000usize;
    let last = preload_stream(backend, stream.clone(), count).await?;
    let mid = Seq(last.0 / 2);

    let mut pre_samples = LatencySamples::with_capacity(100);
    for _ in 0..100 {
        let start = Instant::now();
        backend.read_from(stream.clone(), last, 100).await?;
        pre_samples.record(start.elapsed());
    }

    let removed = backend.truncate_before(stream.clone(), mid).await?;
    let remaining_count = count_rows(backend, stream.clone(), Seq::ZERO).await?;

    let mut post_samples = LatencySamples::with_capacity(100);
    let tail = if remaining_count > 0 {
        last_row_seq(backend, stream.clone()).await?
    } else {
        Seq::ZERO
    };
    for _ in 0..100 {
        let start = Instant::now();
        backend.read_from(stream.clone(), tail, 100).await?;
        post_samples.record(start.elapsed());
    }

    let mut disk_bytes_after = 0u64;
    if matches!(ctx.storage, Storage::SurrealRocksdb | Storage::Sqlite) {
        if let Some(path) = crate::harness::backend::storage_disk_path(&ctx.handle.engine_path) {
            disk_bytes_after = crate::harness::backend::dir_size_bytes(&path);
        }
    }

    Ok(json!({
        "pre_truncate_p95_ms": pre_samples.p95(),
        "post_truncate_p95_ms": post_samples.p95(),
        "removed_rows": removed,
        "remaining_rows": remaining_count,
        "disk_bytes_after": disk_bytes_after,
    }))
}

async fn count_rows(
    backend: &BenchBackend,
    stream: continuum::types::LogStreamId,
    after: Seq,
) -> anyhow::Result<usize> {
    let mut total = 0usize;
    let mut cursor = after;
    loop {
        let batch = backend.read_from(stream.clone(), cursor, 10_000).await?;
        if batch.is_empty() {
            break;
        }
        total += batch.len();
        cursor = batch.last().map_or(cursor, |r| r.seq);
    }
    Ok(total)
}

async fn last_row_seq(
    backend: &BenchBackend,
    stream: continuum::types::LogStreamId,
) -> anyhow::Result<Seq> {
    let mut cursor = Seq::ZERO;
    let mut last = Seq::ZERO;
    loop {
        let batch = backend.read_from(stream.clone(), cursor, 10_000).await?;
        if batch.is_empty() {
            break;
        }
        last = batch.last().map_or(last, |r| r.seq);
        cursor = last;
    }
    Ok(last)
}

pub async fn run_bm_c5(ctx: &ExperimentContext) -> Result<Value> {
    let n_streams = 10usize;
    let ops_per_stream = 500usize;

    let isolated_growth = measure_isolated_growth(ctx.storage, n_streams, ops_per_stream).await?;
    let same_growth =
        measure_same_handle_growth(ctx.storage, n_streams, ops_per_stream).await?;

    Ok(json!({
        "isolated_rss_delta_bytes": isolated_growth,
        "same_handle_rss_delta_bytes": same_growth,
        "growth_ratio_same_vs_isolated": growth_ratio(isolated_growth.max(1), same_growth.max(1)),
    }))
}

async fn measure_isolated_growth(
    storage: Storage,
    n_streams: usize,
    ops: usize,
) -> Result<u64> {
    if storage == Storage::Mem {
        let before = process_rss_bytes();
        for i in 0..n_streams {
            let handle = build_backend(storage, Topology::IsolatedLab, crate::harness::Telemetry::Off, None).await?;
            let stream = bench_stream(storage, &format!("iso-{i}"));
            for _ in 0..ops {
                handle.backend.append(stream.clone(), &[bench_record()]).await?;
            }
        }
        let after = process_rss_bytes();
        return Ok(after.saturating_sub(before));
    }

    let mut total = 0u64;
    for i in 0..n_streams {
        let handle = build_backend(storage, Topology::IsolatedLab, crate::harness::Telemetry::Off, None).await?;
        let before = disk_or_rss(&handle);
        let stream = bench_stream(storage, &format!("iso-{i}"));
        for _ in 0..ops {
            handle.backend.append(stream.clone(), &[bench_record()]).await?;
        }
        let after = disk_or_rss(&handle);
        total += after.saturating_sub(before);
    }
    Ok(total)
}

async fn measure_same_handle_growth(
    storage: Storage,
    n_streams: usize,
    ops: usize,
) -> Result<u64> {
    let (shared, temp) = if storage == Storage::Mem {
        (None, None)
    } else {
        match storage {
            Storage::SurrealMem | Storage::SurrealRocksdb => {
                let (db, temp) = open_shared_surreal(storage).await?;
                (Some(SharedHandle::Surreal(db)), temp)
            }
            Storage::Sqlite => {
                let (pool, temp) = open_shared_sqlite().await?;
                (Some(SharedHandle::Sqlite(pool)), Some(temp))
            }
            Storage::Postgres => (
                Some(SharedHandle::Postgres(open_shared_postgres().await?)),
                None,
            ),
            Storage::Mem => (None, None),
        }
    };

    let handle = build_backend(
        storage,
        Topology::SharedHandle,
        crate::harness::Telemetry::Off,
        shared.clone(),
    )
    .await?;

    let before = disk_or_rss(&handle);
    for i in 0..n_streams {
        let stream = bench_stream(storage, &format!("same-{i}"));
        for _ in 0..ops {
            handle.backend.append(stream.clone(), &[bench_record()]).await?;
        }
    }
    let after = disk_or_rss(&handle);
    drop(handle);
    drop(shared);
    drop(temp);
    Ok(after.saturating_sub(before))
}

fn disk_or_rss(handle: &BackendHandle) -> u64 {
    if let Some(path) = crate::harness::backend::storage_disk_path(&handle.engine_path) {
        crate::harness::backend::dir_size_bytes(&path)
    } else {
        process_rss_bytes()
    }
}

pub async fn run_bm_c6(ctx: &ExperimentContext, duration_secs: u64) -> Result<Value> {
    let storage = ctx.storage;
    let duration = Duration::from_secs(duration_secs);
    let interval = Duration::from_secs(1);

    let baseline_task = async move {
        let handle =
            build_backend(storage, Topology::IsolatedLab, crate::harness::Telemetry::Off, None)
                .await?;
        let before = disk_or_rss(&handle);
        sleep(duration).await;
        let after = disk_or_rss(&handle);
        Ok::<_, anyhow::Error>(after.saturating_sub(before))
    };

    let active_task = async {
        let backend = &ctx.handle.backend;
        let active = bench_stream(ctx.storage, "bm-c6-active");
        let before_active = disk_or_rss(&ctx.handle);
        let start = Instant::now();
        let mut ops = 0u64;

        while start.elapsed() < duration {
            let tick = Instant::now();
            backend.append(active.clone(), &[bench_record()]).await?;
            ops += 1;
            let elapsed = tick.elapsed();
            if let Some(sleep_for) = interval.checked_sub(elapsed) {
                sleep(sleep_for).await;
            }
        }

        let after_active = disk_or_rss(&ctx.handle);
        Ok::<_, anyhow::Error>((ops, after_active.saturating_sub(before_active)))
    };

    let (baseline_delta, (ops, active_delta)) = tokio::try_join!(baseline_task, active_task)?;
    let baseline_delta = baseline_delta.max(1);

    Ok(json!({
        "duration_secs": duration_secs,
        "ops": ops,
        "active_growth_bytes": active_delta,
        "baseline_growth_bytes": baseline_delta,
        "growth_ratio": u64_to_f64(active_delta) / u64_to_f64(baseline_delta),
    }))
}

