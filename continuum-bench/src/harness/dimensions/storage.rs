//! Storage backend dimension from EXPERIMENTS.md.

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

/// Storage backend dimension from EXPERIMENTS.md.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum Storage {
    Mem,
    #[value(name = "surreal-mem")]
    SurrealMem,
    #[value(name = "surreal-rocksdb")]
    SurrealRocksdb,
    #[value(name = "surreal-tikv")]
    SurrealTikv,
    Postgres,
    Sqlite,
    #[value(name = "scylla")]
    Scylla,
    #[value(name = "tikv-raw")]
    TikvRaw,
}

impl Storage {
    /// Whether this storage backend is implemented in v0.1.
    pub const fn is_supported(self) -> bool {
        matches!(
            self,
            Self::Mem
                | Self::SurrealMem
                | Self::SurrealRocksdb
                | Self::SurrealTikv
                | Self::Sqlite
                | Self::Postgres
                | Self::Scylla
                | Self::TikvRaw
        )
    }

    /// Whether this storage requires a remote Scylla cluster.
    pub const fn needs_remote_scylla(self) -> bool {
        matches!(self, Self::Scylla)
    }

    /// Whether this storage requires a remote `TiKV` PD endpoint (raw client).
    pub const fn needs_remote_tikv_raw(self) -> bool {
        matches!(self, Self::TikvRaw)
    }

    /// Whether this storage requires a remote Surreal endpoint.
    pub const fn needs_remote_surreal(self) -> bool {
        matches!(self, Self::SurrealTikv)
    }

    /// Short slug for report filenames.
    pub const fn slug(self) -> &'static str {
        match self {
            Self::Mem => "mem",
            Self::SurrealMem => "surreal-mem",
            Self::SurrealRocksdb => "surreal-rocksdb",
            Self::SurrealTikv => "surreal-tikv",
            Self::Postgres => "postgres",
            Self::Sqlite => "sqlite",
            Self::Scylla => "scylla",
            Self::TikvRaw => "tikv-raw",
        }
    }
}
