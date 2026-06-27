//! Durable subscription checkpoint identifiers.
//!
//! Checkpoints record how far a consumer has processed a stream. Each subscription maintains
//! an independent cursor per stream, keyed by [`CheckpointKey`].

use serde::{Deserialize, Serialize};

/// Stable subscription name for durable checkpoint ownership.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SubscriptionId(
    /// Opaque subscription label chosen by the host (e.g. handler or consumer group name).
    pub String,
);

impl SubscriptionId {
    /// Wrap a subscription name.
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }
}

/// Checkpoint map key: subscription + stream.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CheckpointKey {
    /// Subscription that owns the checkpoint.
    pub subscription: SubscriptionId,
    /// [`crate::LogStreamId::storage_key`] for the stream.
    pub stream_key: String,
}

impl CheckpointKey {
    /// Build a checkpoint key for a subscription + stream.
    #[must_use]
    pub fn new(subscription: &SubscriptionId, stream_key: String) -> Self {
        Self {
            subscription: subscription.clone(),
            stream_key,
        }
    }

    /// Stable wire key for persistence maps.
    #[must_use]
    pub fn wire_key(&self) -> String {
        format!("{}\0{}", self.subscription.0, self.stream_key)
    }
}
