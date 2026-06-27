//! In-memory [`LogBackend`](continuum_core::backend::LogBackend) for unit tests and local development.
//!
//! Not durable — state lives in process memory only. Implements the full
//! [`LogBackend`](continuum_core::backend::LogBackend) contract including idempotent append
//! and checkpoint monotonicity.
//!
//! Enable via the `mem` feature on the `continuum` facade crate.
//!
//! # Examples
//!
//! ```
//! use continuum_backend_mem::InMemoryLogBackend;
//! use continuum_core::{AppendRecord, LogBackend, LogBackendKind, LogDestination, LogStreamId, Seq};
//! use uuid::Uuid;
//!
//! # #[tokio::main]
//! # async fn main() -> continuum_core::Result<()> {
//! let backend = InMemoryLogBackend::new();
//! let stream = LogStreamId::new(
//!     LogDestination::new("default", LogBackendKind::Memory),
//!     "events",
//!     None,
//! );
//! let seqs = backend
//!     .append(stream.clone(), &[AppendRecord::new(Uuid::new_v4(), vec![1])])
//!     .await?;
//! assert_eq!(backend.read_from(stream, Seq::ZERO, 10).await?.len(), 1);
//! assert_eq!(seqs.len(), 1);
//! # Ok(())
//! # }
//! ```

mod memory;

pub use memory::InMemoryLogBackend;
