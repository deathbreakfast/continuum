//! Logical log destination (opaque name + engine kind).
//!
//! Host wiring chooses destination names at boot. This crate does not embed product-specific
//! constants — see [`LogDestination::logical`] and [`LogBackendKind`].

use serde::{Deserialize, Serialize};

/// Engine kind for compound router keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum LogBackendKind {
    /// `SurrealDB` transport log (embedded or remote-on-TiKV via injected handle).
    SurrealLocal,
    /// In-process test backend.
    Memory,
    /// `PostgreSQL` transport log.
    Postgres,
    /// `SQLite` transport log.
    Sqlite,
}

/// Logical destination — host chooses `logical`; no product presets in this crate.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LogDestination {
    /// Opaque logical name registered by the host at boot.
    pub logical: String,
    /// Storage engine kind for router key prefix.
    pub kind: LogBackendKind,
}

impl LogDestination {
    /// Construct at wiring time.
    pub fn new(logical: impl Into<String>, kind: LogBackendKind) -> Self {
        Self {
            logical: logical.into(),
            kind,
        }
    }

    /// Compound router key: `"{kind_prefix}:{logical}"`.
    #[must_use]
    pub fn router_key(&self) -> String {
        crate::router::log_router_key(&self.logical, self.kind)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn router_key_format() {
        let d = LogDestination::new("default", LogBackendKind::SurrealLocal);
        assert_eq!(d.router_key(), "surreal:default");
    }
}
