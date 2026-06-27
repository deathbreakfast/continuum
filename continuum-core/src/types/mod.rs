//! DTOs for streams, records, checkpoints, and destinations.
//!
//! A **stream** ([`LogStreamId`]) is the unit of ordering: destination + topic + optional key.
//! Sequences ([`Seq`]) are scoped to one stream. Records carry opaque ciphertext payloads;
//! the host encrypts before append and decrypts after read.
//!
//! See also: [`crate::backend::LogBackend`], [`crate::router`], [`PartitionId`].

mod checkpoint;
mod destination;
mod partition;
mod record;
mod seq;
mod stream;

pub use checkpoint::{CheckpointKey, SubscriptionId};
pub use destination::{LogBackendKind, LogDestination};
pub use partition::{PartitionId, STORAGE_KEY_SEP};
pub use record::{AppendRecord, EventRecord};
pub use seq::Seq;
pub use stream::LogStreamId;
