//! Shared stream and record builders for backend contract tests.

use continuum_core::types::{AppendRecord, LogBackendKind, LogDestination, LogStreamId};
use uuid::Uuid;

/// Backend-specific destination kind and default logical name.
#[derive(Debug, Clone, Copy)]
pub struct BackendEnv {
    /// Storage engine kind for constructed destinations.
    pub kind: LogBackendKind,
    /// Default logical destination name.
    pub logical_dest: &'static str,
}

impl BackendEnv {
    /// In-memory backend test environment.
    pub const MEMORY: Self = Self {
        kind: LogBackendKind::Memory,
        logical_dest: "default",
    };

    /// Surreal local backend test environment.
    pub const SURREAL: Self = Self {
        kind: LogBackendKind::SurrealLocal,
        logical_dest: "default",
    };

    /// `PostgreSQL` backend test environment.
    pub const POSTGRES: Self = Self {
        kind: LogBackendKind::Postgres,
        logical_dest: "default",
    };

    /// `SQLite` backend test environment.
    pub const SQLITE: Self = Self {
        kind: LogBackendKind::Sqlite,
        logical_dest: "default",
    };

    /// Scylla backend test environment.
    pub const SCYLLA: Self = Self {
        kind: LogBackendKind::Scylla,
        logical_dest: "default",
    };

    /// Raw `TiKV` backend test environment.
    pub const TIKV_RAW: Self = Self {
        kind: LogBackendKind::TikvRaw,
        logical_dest: "default",
    };

    /// Default destination for this environment.
    #[must_use]
    pub fn destination(&self) -> LogDestination {
        LogDestination::new(self.logical_dest, self.kind)
    }

    /// Named destination with this environment's engine kind.
    #[must_use]
    pub fn destination_named(&self, logical: &str) -> LogDestination {
        LogDestination::new(logical, self.kind)
    }

    /// Topic-only stream on the default destination.
    #[must_use]
    pub fn stream(&self, topic: &str) -> LogStreamId {
        LogStreamId::new(self.destination(), topic, None)
    }

    /// Keyed stream on the default destination.
    #[must_use]
    pub fn stream_with_key(&self, topic: &str, key: &str) -> LogStreamId {
        LogStreamId::new(self.destination(), topic, Some(key.into()))
    }
}

/// Sample append record with random event id.
#[must_use]
pub fn sample_record() -> AppendRecord {
    AppendRecord::new(Uuid::new_v4(), vec![7, 8, 9])
}
