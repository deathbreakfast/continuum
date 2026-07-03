//! Surreal-local [`LogBackend`](continuum_core::backend::LogBackend) (injected embedded or remote client).
//!
//! The host owns the database handle (embedded `RocksDB` or remote `TiKV`-backed client).
//! Continuum does not open connections — wrap an existing `Arc<Surreal<…>>` with
//! [`SurrealLocalLogBackend::new`] or [`SurrealLocalLogBackend::new_embedded_local`].
//!
//! Enable via the `surreal-local` feature on the [`continuum`](https://docs.rs/continuum) facade.
//! See [Getting started](https://docs.rs/continuum/latest/continuum/index.html#getting-started)
//! and the [documentation map](https://docs.rs/continuum/latest/continuum/index.html#documentation-map).
//!
//! See also: [`SurrealLogConfig`], [`continuum_core::LogBackend`].

mod surreal_local;

pub use surreal_local::{SurrealLocalLogBackend, SurrealLogConfig};
