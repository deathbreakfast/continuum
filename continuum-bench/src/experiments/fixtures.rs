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

/// Append `count` single-record batches and return final sequence.
pub async fn preload_stream(
    backend: &dyn LogBackend,
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
