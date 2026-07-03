//! Register a backend on [`LogRouter`] and resolve destinations with evaluators.
//!
//! ```bash
//! cargo run -p continuum --example router --features mem
//! ```

use std::sync::Arc;

use continuum::backends::InMemoryLogBackend;
use continuum::router::{
    resolve_stream, LogFromDestination, LogResolverContext, LogRouter, LogTopicRouter,
};
use continuum::types::{AppendRecord, LogBackendKind, LogDestination, LogStreamId};
use continuum::{KeyHashEvaluator, LogEvaluator};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), continuum::LogError> {
    let dest = LogDestination::new("default", LogBackendKind::Memory);
    let backend = Arc::new(InMemoryLogBackend::new());
    let router = LogRouter::with_default(&dest, backend);

    let evaluator = LogFromDestination(dest.clone());
    let (resolved_dest, resolved_backend) = resolve_stream(
        &evaluator,
        &router,
        &LogResolverContext::default(),
        "events",
        Some("user-42".into()),
    )
    .await?;

    let stream = LogStreamId::new(resolved_dest, "events", Some("user-42".into()));
    let record = AppendRecord::new(Uuid::new_v4(), b"hello".to_vec());
    let seqs = resolved_backend.append(stream.clone(), &[record]).await?;
    println!(
        "routed stream {} appended at seq {}",
        stream.storage_key(),
        seqs[0].0
    );

    let metrics = LogDestination::new("metrics", LogBackendKind::Memory);
    let topic_router = LogTopicRouter::new(dest).prefix("metrics.", metrics.clone());
    let ctx = LogResolverContext::default();
    let got = topic_router
        .resolve_for_topic(&ctx, "metrics.cache_hits", None)
        .await?;
    println!("prefix route: {}", got.logical);

    let cells = vec![
        LogDestination::new("cell-0", LogBackendKind::Memory),
        LogDestination::new("cell-1", LogBackendKind::Memory),
    ];
    let hash_eval = KeyHashEvaluator::new(cells);
    let hashed = hash_eval
        .resolve_for_topic(&ctx, "events", Some("user-42"))
        .await?;
    println!("key-hash destination: {}", hashed.logical);

    Ok(())
}
