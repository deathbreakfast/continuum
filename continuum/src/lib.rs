//! Append-only event/log substrate — public facade for the Continuum workspace.
//!
//! **Continuum** defines an append-only, sequenced log **storage port** ([`LogBackend`]) with
//! feature-gated backends. A stream ([`LogStreamId`]) is destination + topic + optional key;
//! sequences are strictly increasing per stream. The host owns encryption, routing policy,
//! and database handles — this crate provides storage semantics only.
//!
//! **Requires nightly Rust** (see the workspace `rust-toolchain.toml`). Stable is not supported
//! in v0.1.
//!
//! # Getting started
//!
//! Depend on this crate with an explicit backend feature (none are enabled by default),
//! construct a backend, then append and read on a [`LogStreamId`]:
//!
//! ```rust
//! # #[cfg(feature = "mem")]
//! use continuum::InMemoryLogBackend;
//! # #[cfg(not(feature = "mem"))]
//! # use continuum_backend_mem::InMemoryLogBackend;
//! use continuum::{
//!     AppendRecord, LogBackend, LogBackendKind, LogDestination, LogStreamId, Seq,
//! };
//! use uuid::Uuid;
//!
//! # #[tokio::main]
//! # async fn main() -> continuum::Result<()> {
//! let backend = InMemoryLogBackend::new();
//! let stream = LogStreamId::new(
//!     LogDestination::new("default", LogBackendKind::Memory),
//!     "events",
//!     None,
//! );
//! let seqs = backend
//!     .append(stream.clone(), &[AppendRecord::new(Uuid::new_v4(), vec![1, 2, 3])])
//!     .await?;
//! assert_eq!(backend.read_from(stream, Seq::ZERO, 10).await?.len(), 1);
//! assert_eq!(seqs.len(), 1);
//! # Ok(())
//! # }
//! ```
//!
//! # Documentation map
//!
//! Full snippets live on the linked items (not repeated here).
//!
//! - **Append and read** — write and read opaque payloads on a stream. Example on [`LogBackend`].
//! - **Route destinations** — register backends and resolve topics at boot. Examples on
//!   [`LogRouter`], [`LogFromDestination`], and [`router::resolve_stream`].
//! - **Topic-prefix routing** — longest-prefix rules with a fallback destination. Example on
//!   [`LogTopicRouter`].
//! - **Key-hash routing** — shard a topic across cells by partition key. Example on
//!   [`KeyHashEvaluator`].
//! - **Checkpoint and truncate** — durable consumer position and space reclaim. Example on
//!   [`LogBackend`] (runnable: `checkpoint_truncate`).
//! - **Instrument** — wrap any backend for timing hooks (`telemetry-console`). Example on
//!   `InstrumentedLogBackend`.
//! - **Backends** — enable one feature per engine; see [`backends`]. Connect examples on
//!   `PostgresLogBackend` / `SqliteLogBackend`; Surreal via the `surreal_embedded` binary.
//! - **Implement a backend** — honor the [`LogBackend`] contract; start from `InMemoryLogBackend`
//!   (`mem`).
//!
//! Runnable binaries: `quickstart`, `router`, `checkpoint_truncate`
//! (`cargo run -p continuum --example <name> --features mem`), and
//! `cargo run -p continuum-backend-surreal --example surreal_embedded`.
//! Backend wiring and [configuration](https://github.com/unified-field-dev/continuum/blob/main/continuum/README.md#configuration)
//! are in the crate and root READMEs.
//!
//! # Workspace
//!
//! | Crate | Role |
//! |-------|------|
//! | `continuum` (this crate) | Public facade — re-exports core + optional backends |
//! | `continuum-core` | [`LogBackend`] port, DTOs, [`LogRouter`], validation |
//! | `continuum-backend-*` | Per-engine [`LogBackend`] implementations |
//! | `continuum-telemetry` | Optional instrumentation decorator |
//!
//! # Design decisions
//!
//! - **Single facade crate** with feature-gated backends — depend with `default-features = false`
//!   for port + DTOs only
//! - **Encryption above the port** — payloads are opaque ciphertext in storage
//! - **Topic + key partitioning** — per-key ordering and load spreading
//! - **Transport log ≠ system of record** — truncate after delivery; canonical data lives elsewhere
//! - **No product constants** — logical destination names and DB handles are host-owned
//!
//! See also: [`LogTopicRouter`], [`LogFromDestination`], [`backends`].

pub use continuum_core::*;

#[cfg(feature = "telemetry-console")]
pub use continuum_telemetry::{
    AppendOutcome, AppendTelemetry, CheckpointTelemetry, ConsoleTelemetry, InstrumentedLogBackend,
    NoTelemetry, ReadOutcome, ReadTelemetry, TelemetryOp, TelemetrySink, TruncateTelemetry,
    telemetry_from_env,
};

/// Feature-gated [`LogBackend`] implementations.
///
/// No features are enabled by default. Enable backends explicitly:
///
/// | Feature | Type | Status |
/// |---------|------|--------|
/// | `mem` | `InMemoryLogBackend` | Ready — tests and local dev |
/// | `surreal-local` | `SurrealLocalLogBackend` | Ready — production (injected Surreal handle) |
/// | `postgres` | `PostgresLogBackend` | Ready — `PostgreSQL` transport log |
/// | `sqlite` | `SqliteLogBackend` | Ready — `SQLite` transport log |
/// | `scylla` | `ScyllaLogBackend` | Ready — native `ScyllaDB` transport log |
/// | `tikv-raw` | `TikvRawLogBackend` | Ready — native `TiKV` transport log |
/// | `telemetry-console` | `InstrumentedLogBackend` | Optional diagnostics wrapper |
///
/// Port-only build: `default-features = false` — trait, DTOs, and router without any backend linked.
pub mod backends {
    #[cfg(feature = "mem")]
    pub use continuum_backend_mem::*;

    #[cfg(feature = "surreal-local")]
    pub use continuum_backend_surreal::*;

    #[cfg(feature = "postgres")]
    pub use continuum_backend_postgres::*;

    #[cfg(feature = "sqlite")]
    pub use continuum_backend_sqlite::*;

    #[cfg(feature = "scylla")]
    pub use continuum_backend_scylla::*;

    #[cfg(feature = "tikv-raw")]
    pub use continuum_backend_tikv_raw::*;
}

#[cfg(feature = "mem")]
pub use backends::InMemoryLogBackend;

#[cfg(feature = "surreal-local")]
pub use backends::{SurrealLocalLogBackend, SurrealLogConfig};

#[cfg(feature = "postgres")]
pub use backends::PostgresLogBackend;

#[cfg(feature = "sqlite")]
pub use backends::SqliteLogBackend;

#[cfg(feature = "scylla")]
pub use backends::{ScyllaLogBackend, ScyllaLogConfig};

#[cfg(feature = "tikv-raw")]
pub use backends::{TikvRawLogBackend, TikvRawLogConfig};
