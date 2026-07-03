//! Minimal append + read against the in-memory backend.
//!
//! ```bash
//! cargo run -p continuum --example quickstart --features mem
//! ```

use continuum::backend::LogBackend;
use continuum::backends::InMemoryLogBackend;
use continuum::types::{AppendRecord, LogBackendKind, LogDestination, LogStreamId, Seq};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), continuum::LogError> {
    let backend = InMemoryLogBackend::new();
    let stream = LogStreamId::new(
        LogDestination::new("default", LogBackendKind::Memory),
        "events",
        None,
    );

    let record = AppendRecord::new(Uuid::new_v4(), vec![1, 2, 3]);
    let seqs = backend.append(stream.clone(), &[record]).await?;
    println!("appended at seq {}", seqs[0].0);

    let events = backend.read_from(stream, Seq::ZERO, 10).await?;
    println!("read {} event(s)", events.len());
    Ok(())
}
