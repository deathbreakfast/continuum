//! Consumer checkpoint and truncate lifecycle.
//!
//! ```bash
//! cargo run -p continuum --example checkpoint_truncate --features mem
//! ```

use continuum::backend::LogBackend;
use continuum::backends::InMemoryLogBackend;
use continuum::types::{
    AppendRecord, LogBackendKind, LogDestination, LogStreamId, Seq, SubscriptionId,
};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), continuum::LogError> {
    let backend = InMemoryLogBackend::new();
    let stream = LogStreamId::new(
        LogDestination::new("default", LogBackendKind::Memory),
        "events",
        None,
    );
    let sub = SubscriptionId::new("worker-1");

    let record = AppendRecord::new(Uuid::new_v4(), vec![9, 9, 9]);
    let seqs = backend.append(stream.clone(), &[record]).await?;
    let seq = seqs[0];

    backend
        .commit_checkpoint(&sub, stream.clone(), seq)
        .await?;
    let loaded = backend.load_checkpoint(&sub, stream.clone()).await?;
    println!("checkpoint: {loaded:?}");

    let events = backend
        .read_from_topic(stream.clone(), None, Seq::ZERO, 10)
        .await?;
    println!("topic events: {}", events.len());

    let removed = backend.truncate_before(stream, seq.next()).await?;
    println!("truncated removed {removed} record(s)");
    Ok(())
}
