//! Surreal-local [`LogBackend`](continuum_core::backend::LogBackend) (injected embedded or remote client).
//!
//! The host owns the database handle (embedded `RocksDB` or remote `TiKV`-backed client).
//! Continuum does not open connections — wrap an existing `Arc<Surreal<…>>` with
//! [`SurrealLocalLogBackend::new`] or [`SurrealLocalLogBackend::new_embedded_local`].
//!
//! Enable via the `surreal-local` feature on the `continuum` facade crate.
//!
//! See also: [`SurrealLogConfig`], [`continuum_core::LogBackend`].

mod surreal_local;

pub use surreal_local::{SurrealLocalLogBackend, SurrealLogConfig};
