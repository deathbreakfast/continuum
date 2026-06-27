//! Topic + key ordering slice within one destination.
//!
//! Default partition identity is **topic + optional key**. Per-key ordering applies for
//! keyed topics (e.g. `user.notifications` + `user_id`). Topic-only partitioning uses
//! `key = None` and can become a single hot partition at very high volume — prefer keyed
//! partitions when load spreading matters.

use serde::{Deserialize, Serialize};

/// Ordering partition (topic + optional key) within one destination.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PartitionId {
    /// Topic name for routing and storage.
    pub topic: String,
    /// Optional partition key for per-key ordering and load spreading.
    pub key: Option<String>,
}

/// Wire separator for composite storage keys (must not contain NUL — Surreal-safe).
pub const STORAGE_KEY_SEP: char = '\x1f';

impl PartitionId {
    /// Create a partition identifier.
    pub fn new(topic: impl Into<String>, key: Option<String>) -> Self {
        Self {
            topic: topic.into(),
            key,
        }
    }

    /// Stable wire key: `{topic}{SEP}{key_or_empty}`.
    #[must_use]
    pub fn storage_key(&self) -> String {
        format!(
            "{}{}{}",
            self.topic,
            STORAGE_KEY_SEP,
            self.key.as_deref().unwrap_or("")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distinct_keys_for_some_vs_none() {
        let a = PartitionId::new("t", Some("k".into()));
        let b = PartitionId::new("t", None);
        assert_ne!(a.storage_key(), b.storage_key());
    }
}
