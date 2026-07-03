//! Continuum core: the [`LogBackend`] port, DTOs, routing, and validation.
//!
//! # Overview
//!
//! This crate defines an append-only, sequenced event log **storage port** and the types
//! needed to use it. It has no storage engine dependencies — backends live in separate
//! `continuum-backend-*` crates and are wired at runtime via [`LogRouter`].
//!
//! **Goals:**
//!
//! - [`LogBackend`] with append-only, sequenced, partitioned storage and durable consumer
//!   checkpoints
//! - Partition by topic + optional key for per-key ordering and load spreading at scale
//! - Opaque ciphertext payloads: the host encrypts and decrypts above the port
//! - Batched async append, idempotent dedupe on [`AppendRecord::event_id`], and logical
//!   truncate for short-lived transport logs
//!
//! # Non-goals
//!
//! - Canonical system-of-record or classified data storage
//! - ORM, privacy evaluation, or schema codegen
//! - User-facing ops UI (build projection tables above the log)
//! - Opening database connections (the host injects handles into backends)
//!
//! # Modules
//!
//! - [`backend`] — [`LogBackend`] trait and storage contract
//! - [`types`] — streams, records, sequences, checkpoints, destinations
//! - [`router`] — destination resolution and backend registration
//! - [`validation`] — topic and read-limit guards used by backends
//! - [`error`] — [`LogError`] and [`Result`]
//!
//! See also: [`LogStreamId`], [`LogTopicRouter`], [`LogFromDestination`].

pub mod backend;
pub mod error;
pub mod router;
pub mod types;
pub mod validation;

pub use backend::LogBackend;
pub use error::{LogError, Result};
pub use router::{
    log_router_key, KeyHashEvaluator, LogEvaluator, LogFromDestination, LogResolverContext,
    LogRouter, LogTopicRouter,
};
pub use types::{
    AppendRecord, CheckpointKey, EventRecord, LogBackendKind, LogDestination, LogStreamId,
    PartitionId, Seq, SubscriptionId,
};
