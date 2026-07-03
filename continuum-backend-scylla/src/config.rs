//! Scylla backend tuning types (configured via [`crate::ScyllaLogConfig`], not env vars).

use std::collections::HashMap;

use scylla::statement::Consistency;

/// Per-append idempotency via `event_id IF NOT EXISTS` lightweight transactions (LWT).
///
/// Lightweight transactions (LWT) use compare-and-set semantics so a duplicate
/// `event_id` returns the existing sequence without inserting a second row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IdempotencyMode {
    /// Exactly-once append: LWT on `event_id` (default).
    #[default]
    Lwt,
    /// At-least-once append: skip LWT for higher throughput; callers must tolerate duplicates.
    None,
}

/// Idempotency policy: one mode for all topics, or per-topic overrides.
#[derive(Debug, Clone)]
pub enum IdempotencyPolicy {
    /// Apply the same [`IdempotencyMode`] to every topic.
    Global(IdempotencyMode),
    /// Use `default` unless `overrides` names a topic-specific mode.
    PerTopic {
        /// Mode for topics not listed in `overrides`.
        default: IdempotencyMode,
        /// Topic name → mode overrides.
        overrides: HashMap<String, IdempotencyMode>,
    },
}

impl IdempotencyPolicy {
    /// Resolve idempotency mode for a topic name.
    #[must_use]
    pub fn mode_for(&self, topic: &str) -> IdempotencyMode {
        match self {
            IdempotencyPolicy::Global(m) => *m,
            IdempotencyPolicy::PerTopic { default, overrides } => {
                overrides.get(topic).copied().unwrap_or(*default)
            }
        }
    }
}

impl Default for IdempotencyPolicy {
    fn default() -> Self {
        Self::Global(IdempotencyMode::Lwt)
    }
}

/// Parse write consistency from a string (`one`, `local_one`, `quorum`, `local_quorum`).
#[must_use]
pub fn consistency_from_str(s: &str) -> Option<Consistency> {
    match s.trim().to_ascii_lowercase().as_str() {
        "one" => Some(Consistency::One),
        "local_one" => Some(Consistency::LocalOne),
        "quorum" | "local_quorum" => Some(Consistency::LocalQuorum),
        _ => None,
    }
}

/// Cache key for in-process topic+stream index entries (`topic_prefix|stream_key`).
#[must_use]
pub fn stream_index_cache_key(topic_prefix: &str, stream_key: &str) -> String {
    format!("{topic_prefix}|{stream_key}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idempotency_policy_global() {
        let p = IdempotencyPolicy::Global(IdempotencyMode::None);
        assert_eq!(p.mode_for("any"), IdempotencyMode::None);
    }

    #[test]
    fn idempotency_policy_per_topic_override() {
        let mut overrides = HashMap::new();
        overrides.insert("telemetry".into(), IdempotencyMode::None);
        let p = IdempotencyPolicy::PerTopic {
            default: IdempotencyMode::Lwt,
            overrides,
        };
        assert_eq!(p.mode_for("payments"), IdempotencyMode::Lwt);
        assert_eq!(p.mode_for("telemetry"), IdempotencyMode::None);
    }

    #[test]
    fn idempotency_policy_default_is_lwt() {
        assert_eq!(
            IdempotencyPolicy::default().mode_for("t"),
            IdempotencyMode::Lwt
        );
    }

    #[test]
    fn write_consistency_parsing() {
        assert_eq!(consistency_from_str("one"), Some(Consistency::One));
        assert_eq!(consistency_from_str("local_one"), Some(Consistency::LocalOne));
        assert_eq!(consistency_from_str(""), None);
    }

    #[test]
    fn stream_index_cache_key_format() {
        assert_eq!(
            stream_index_cache_key("router|topic|", "router|topic|key1"),
            "router|topic||router|topic|key1"
        );
    }
}
