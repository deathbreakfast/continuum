//! Shared fixtures for benchmark workloads.

use continuum::backend::LogBackend;
use continuum::types::{AppendRecord, LogDestination, LogStreamId, Seq};
use uuid::Uuid;

use crate::harness::backend_kind;
use crate::harness::Storage;

/// Fixed ciphertext payload size for comparability across experiments.
pub const PAYLOAD_BYTES: usize = 256;

/// Build a bench append record with random event id.
pub fn bench_record() -> AppendRecord {
    AppendRecord::new(Uuid::new_v4(), vec![0u8; PAYLOAD_BYTES])
}

/// Build N distinct bench records.
pub fn bench_records(n: usize) -> Vec<AppendRecord> {
    (0..n).map(|_| bench_record()).collect()
}

/// Standard stream for an experiment.
pub fn bench_stream(storage: Storage, topic: &str) -> LogStreamId {
    LogStreamId::new(
        LogDestination::new("bench", backend_kind(storage)),
        topic,
        None,
    )
}

/// Partition count for BM-L* load tests (`CONTINUUM_BENCH_LOAD_PARTITION_COUNT`). `1` = hot stream.
pub fn load_partition_count() -> usize {
    std::env::var("CONTINUUM_BENCH_LOAD_PARTITION_COUNT")
        .ok()
        .and_then(|s| s.parse().ok())
        .filter(|&k| k >= 1)
        .unwrap_or(1)
}

/// Stream for BM-L* with optional round-robin partition keys when `load_partition_count() > 1`.
pub fn bench_stream_for_load(storage: Storage, topic: &str, op_index: u64) -> LogStreamId {
    let mut stream = bench_stream(storage, topic);
    let k = load_partition_count();
    if k > 1 {
        stream.key = Some(format!(
            "partition_{}",
            usize::try_from(op_index).unwrap_or(0) % k
        ));
    }
    stream
}

/// Append `count` single-record batches and return final sequence.
pub async fn preload_stream<B: LogBackend + ?Sized>(
    backend: &B,
    stream: LogStreamId,
    count: usize,
) -> anyhow::Result<Seq> {
    let mut last = Seq::ZERO;
    for _ in 0..count {
        let rec = bench_record();
        let seqs = backend.append(stream.clone(), &[rec]).await?;
        last = seqs[0];
    }
    Ok(last)
}
