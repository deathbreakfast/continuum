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
    pub fn is_supported(self) -> bool {
        matches!(
            self,
            Storage::Mem
                | Storage::SurrealMem
                | Storage::SurrealRocksdb
                | Storage::SurrealTikv
                | Storage::Sqlite
                | Storage::Postgres
                | Storage::Scylla
                | Storage::TikvRaw
        )
    }

    /// Whether this storage requires a remote Scylla cluster.
    pub fn needs_remote_scylla(self) -> bool {
        matches!(self, Storage::Scylla)
    }

    /// Whether this storage requires a remote TiKV PD endpoint (raw client).
    pub fn needs_remote_tikv_raw(self) -> bool {
        matches!(self, Storage::TikvRaw)
    }

    /// Whether this storage requires a remote Surreal endpoint.
    pub fn needs_remote_surreal(self) -> bool {
        matches!(self, Storage::SurrealTikv)
    }

    /// Short slug for report filenames.
    pub fn slug(self) -> &'static str {
        match self {
            Storage::Mem => "mem",
            Storage::SurrealMem => "surreal-mem",
            Storage::SurrealRocksdb => "surreal-rocksdb",
            Storage::SurrealTikv => "surreal-tikv",
            Storage::Postgres => "postgres",
            Storage::Sqlite => "sqlite",
            Storage::Scylla => "scylla",
            Storage::TikvRaw => "tikv-raw",
        }
    }
}
