//! Append-only log storage port.
//!
//! The [`LogBackend`] trait is the sole persistence boundary. Implementations assign
//! monotonic [`crate::Seq`] values per [`crate::LogStreamId`], store opaque payload bytes,
//! and support durable consumer checkpoints.
//!
//! Core dependencies are limited to `serde`, `async-trait`, `thiserror`, and `uuid` — no
//! database drivers or storage engines in this module.
//!
//! See also: [`crate::types::AppendRecord`], [`crate::types::LogStreamId`], [`crate::router`].

pub mod log_backend;

pub use log_backend::LogBackend;
