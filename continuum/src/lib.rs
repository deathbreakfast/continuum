//! Append-only event/log substrate — public facade for the Continuum workspace.
//!
//! **Continuum** defines an append-only, sequenced log **storage port** ([`LogBackend`]) with
//! feature-gated backends. A stream ([`LogStreamId`]) is destination + topic + optional key;
//! sequences are strictly increasing per stream. The host owns encryption, routing policy,
//! and database handles — this crate provides storage semantics only.
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
/// | `scylla` | `ScyllaLogBackend` | Ready — native ScyllaDB transport log |
/// | `tikv-raw` | `TikvRawLogBackend` | Ready — native TiKV transport log |
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
