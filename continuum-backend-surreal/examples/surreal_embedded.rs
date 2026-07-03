//! Surreal-local backend with an in-process `mem://` handle.
//!
//! ```bash
//! cargo run -p continuum-backend-surreal --example surreal_embedded
//! ```

use std::sync::Arc;

use continuum_backend_surreal::SurrealLocalLogBackend;
use continuum_core::backend::LogBackend;
use continuum_core::types::{AppendRecord, LogBackendKind, LogDestination, LogStreamId, Seq};
use continuum_core::LogError;
use surrealdb::engine::any::Any;
use surrealdb::Surreal;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), LogError> {
    let db: Surreal<Any> = Surreal::init();
    db.connect("mem://")
        .await
        .map_err(|e| LogError::Backend(e.to_string()))?;
    db.use_ns("continuum")
        .use_db("transport")
        .await
        .map_err(|e| LogError::Backend(e.to_string()))?;

    let backend = SurrealLocalLogBackend::new(Arc::new(db)).await?;
    let stream = LogStreamId::new(
        LogDestination::new("default", LogBackendKind::SurrealLocal),
        "events",
        None,
    );

    let record = AppendRecord::new(Uuid::new_v4(), b"surreal".to_vec());
    let seqs = backend.append(stream.clone(), &[record]).await?;
    let events = backend.read_from(stream, Seq::ZERO, 10).await?;
    println!("appended seq {}, read {} event(s)", seqs[0].0, events.len());
    Ok(())
}
