//! Full stream identity: destination + topic + key.
//!
//! [`LogStreamId`] combines a [`LogDestination`] with a [`PartitionId`] view (topic + key).
//! Sequence numbers from [`crate::LogBackend::append`] are scoped to this type — two streams
//! with the same topic but different keys have independent sequence counters.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::{LogDestination, PartitionId, STORAGE_KEY_SEP};
use crate::backend::LogBackend;
use crate::error::Result;
use crate::router::LogRouter;

/// Full stream identity; sequence numbers are scoped to this type.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LogStreamId {
    /// Logical log destination (opaque name + engine kind).
    pub destination: LogDestination,
    /// Topic name.
    pub topic: String,
    /// Optional partition key for per-key ordering within the topic.
    pub key: Option<String>,
}

impl LogStreamId {
    /// Create a stream identifier.
    pub fn new(
        destination: LogDestination,
        topic: impl Into<String>,
        key: Option<String>,
    ) -> Self {
        Self {
            destination,
            topic: topic.into(),
            key,
        }
    }

    /// Topic + key sub-view.
    #[must_use]
    pub fn partition(&self) -> PartitionId {
        PartitionId::new(&self.topic, self.key.clone())
    }

    /// Full storage key including destination prefix.
    #[must_use]
    pub fn storage_key(&self) -> String {
        format!(
            "{}{}{}",
            self.destination.router_key(),
            STORAGE_KEY_SEP,
            self.partition().storage_key()
        )
    }

    /// Resolve the backend registered for this stream's destination.
    ///
    /// # Errors
    ///
    /// Returns an error if backend lookup fails (see [`LogRouter::resolve_backend`]).
    pub fn resolve_backend(&self, router: &LogRouter) -> Result<Arc<dyn LogBackend>> {
        router.resolve_backend(&self.destination)
    }
}
