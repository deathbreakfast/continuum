//! Build [`LogBackend`] instances for benchmark dimensions.
//!
//! **Internal — performance engineers.** Dispatches to per-storage builders in
//! [`surreal`], [`sqlite`], and [`postgres`]. Remote TiKV-backed Surreal uses
//! `CONTINUUM_BENCH_SURREAL_URL` (see [`surreal::build_surreal_tikv`]).

mod disk;
pub mod postgres;
pub mod scylla;
pub mod sqlite;
pub mod surreal;
pub mod tikv_raw;

use std::fmt::Debug;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use continuum::backend::LogBackend;
use continuum::backends::{
    InMemoryLogBackend, PostgresLogBackend, ScyllaLogBackend, SqliteLogBackend,
    SurrealLocalLogBackend, TikvRawLogBackend,
};
use continuum::types::LogBackendKind;
use continuum::{ConsoleTelemetry, InstrumentedLogBackend, NoTelemetry};
use surrealdb::engine::any::Any;
use surrealdb::Surreal;
use tempfile::TempDir;

use super::dimensions::{Storage, Telemetry, Topology};

pub use disk::{dir_size_bytes, storage_disk_path};
pub use postgres::open_shared_postgres;
pub use scylla::open_shared_scylla;
pub use sqlite::open_shared_sqlite;
pub use surreal::open_shared_surreal;
pub use tikv_raw::open_shared_tikv_raw;

/// Shared engine handle for co-tenancy experiments.
#[derive(Clone)]
pub enum SharedHandle {
    /// Shared Surreal client.
    Surreal(Arc<Surreal<Any>>),
    /// Shared `SQLite` pool.
    Sqlite(Arc<sqlx::SqlitePool>),
    /// Shared `PostgreSQL` pool.
    Postgres(Arc<sqlx::PgPool>),
    /// Shared Scylla backend.
    Scylla(Arc<ScyllaLogBackend>),
    /// Shared raw TiKV backend.
    TikvRaw(Arc<TikvRawLogBackend>),
}

/// Holds backend plus metadata needed for reporting and lifetime management.
pub struct BackendHandle {
    pub backend: Arc<BenchBackend>,
    pub engine_path: String,
    _temp_dir: Option<TempDir>,
    pub _shared: Option<SharedHandle>,
}

/// Type-erased instrumented backend for benchmark runs.
pub enum BenchBackend {
    MemOff(InstrumentedLogBackend<InMemoryLogBackend, NoTelemetry>),
    MemConsole(InstrumentedLogBackend<InMemoryLogBackend, ConsoleTelemetry>),
    SurrealOff(InstrumentedLogBackend<SurrealLocalLogBackend, NoTelemetry>),
    SurrealConsole(InstrumentedLogBackend<SurrealLocalLogBackend, ConsoleTelemetry>),
    SqliteOff(InstrumentedLogBackend<SqliteLogBackend, NoTelemetry>),
    SqliteConsole(InstrumentedLogBackend<SqliteLogBackend, ConsoleTelemetry>),
    PostgresOff(InstrumentedLogBackend<PostgresLogBackend, NoTelemetry>),
    PostgresConsole(InstrumentedLogBackend<PostgresLogBackend, ConsoleTelemetry>),
    ScyllaOff(InstrumentedLogBackend<ScyllaLogBackend, NoTelemetry>),
    ScyllaConsole(InstrumentedLogBackend<ScyllaLogBackend, ConsoleTelemetry>),
    TikvRawOff(InstrumentedLogBackend<TikvRawLogBackend, NoTelemetry>),
    TikvRawConsole(InstrumentedLogBackend<TikvRawLogBackend, ConsoleTelemetry>),
}

impl BenchBackend {
    fn log(&self) -> &dyn LogBackend {
        match self {
            BenchBackend::MemOff(b) => b,
            BenchBackend::MemConsole(b) => b,
            BenchBackend::SurrealOff(b) => b,
            BenchBackend::SurrealConsole(b) => b,
            BenchBackend::SqliteOff(b) => b,
            BenchBackend::SqliteConsole(b) => b,
            BenchBackend::PostgresOff(b) => b,
            BenchBackend::PostgresConsole(b) => b,
            BenchBackend::ScyllaOff(b) => b,
            BenchBackend::ScyllaConsole(b) => b,
            BenchBackend::TikvRawOff(b) => b,
            BenchBackend::TikvRawConsole(b) => b,
        }
    }
}

#[async_trait::async_trait]
impl LogBackend for BenchBackend {
    async fn append(
        &self,
        stream: continuum::types::LogStreamId,
        records: &[continuum::types::AppendRecord],
    ) -> continuum::error::Result<Vec<continuum::types::Seq>> {
        self.log().append(stream, records).await
    }

    async fn read_from(
        &self,
        stream: continuum::types::LogStreamId,
        after: continuum::types::Seq,
        limit: usize,
    ) -> continuum::error::Result<Vec<continuum::types::EventRecord>> {
        self.log().read_from(stream, after, limit).await
    }

    async fn commit_checkpoint(
        &self,
        subscription: &continuum::types::SubscriptionId,
        stream: continuum::types::LogStreamId,
        seq: continuum::types::Seq,
    ) -> continuum::error::Result<()> {
        self.log()
            .commit_checkpoint(subscription, stream, seq)
            .await
    }

    async fn load_checkpoint(
        &self,
        subscription: &continuum::types::SubscriptionId,
        stream: continuum::types::LogStreamId,
    ) -> continuum::error::Result<Option<continuum::types::Seq>> {
        self.log().load_checkpoint(subscription, stream).await
    }

    async fn read_from_topic(
        &self,
        stream: continuum::types::LogStreamId,
        topic_key: Option<&str>,
        after: continuum::types::Seq,
        limit: usize,
    ) -> continuum::error::Result<Vec<continuum::types::EventRecord>> {
        self.log()
            .read_from_topic(stream, topic_key, after, limit)
            .await
    }

    async fn truncate_before(
        &self,
        stream: continuum::types::LogStreamId,
        seq: continuum::types::Seq,
    ) -> continuum::error::Result<u64> {
        self.log().truncate_before(stream, seq).await
    }
}

impl Debug for BenchBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BenchBackend").finish_non_exhaustive()
    }
}

/// Build a backend for the given storage/topology/telemetry combination.
pub async fn build_backend(
    storage: Storage,
    topology: Topology,
    telemetry: Telemetry,
    shared: Option<SharedHandle>,
) -> Result<BackendHandle> {
    if !storage.is_supported() {
        return Err(anyhow!("storage {storage:?} is not supported in v0.1"));
    }
    if !telemetry.is_supported() {
        return Err(anyhow!("telemetry {telemetry:?} is not supported"));
    }

    match storage {
        Storage::Mem => Ok(build_mem(telemetry)),
        Storage::SurrealMem | Storage::SurrealRocksdb => {
            surreal::build_surreal(storage, topology, telemetry, shared).await
        }
        Storage::SurrealTikv => surreal::build_surreal_tikv(telemetry, shared).await,
        Storage::Sqlite => sqlite::build_sqlite(topology, telemetry, shared).await,
        Storage::Postgres => postgres::build_postgres(topology, telemetry, shared).await,
        Storage::Scylla => scylla::build_scylla(topology, telemetry, shared).await,
        Storage::TikvRaw => tikv_raw::build_tikv_raw(topology, telemetry, shared).await,
    }
}

fn build_mem(telemetry: Telemetry) -> BackendHandle {
    let inner = InMemoryLogBackend::new();
    BackendHandle {
        backend: Arc::new(wrap_mem(inner, telemetry)),
        engine_path: "mem://".into(),
        _temp_dir: None,
        _shared: None,
    }
}

pub(crate) fn wrap_mem(inner: InMemoryLogBackend, telemetry: Telemetry) -> BenchBackend {
    match telemetry {
        Telemetry::Off | Telemetry::Stub => {
            BenchBackend::MemOff(InstrumentedLogBackend::new(inner, NoTelemetry))
        }
        Telemetry::Console => {
            BenchBackend::MemConsole(InstrumentedLogBackend::new(inner, ConsoleTelemetry))
        }
    }
}

pub(crate) fn wrap_surreal(inner: SurrealLocalLogBackend, telemetry: Telemetry) -> BenchBackend {
    match telemetry {
        Telemetry::Off | Telemetry::Stub => {
            BenchBackend::SurrealOff(InstrumentedLogBackend::new(inner, NoTelemetry))
        }
        Telemetry::Console => BenchBackend::SurrealConsole(InstrumentedLogBackend::new(
            inner,
            ConsoleTelemetry,
        )),
    }
}

pub(crate) fn wrap_sqlite(inner: SqliteLogBackend, telemetry: Telemetry) -> BenchBackend {
    match telemetry {
        Telemetry::Off | Telemetry::Stub => {
            BenchBackend::SqliteOff(InstrumentedLogBackend::new(inner, NoTelemetry))
        }
        Telemetry::Console => BenchBackend::SqliteConsole(InstrumentedLogBackend::new(
            inner,
            ConsoleTelemetry,
        )),
    }
}

pub(crate) fn wrap_postgres(inner: PostgresLogBackend, telemetry: Telemetry) -> BenchBackend {
    match telemetry {
        Telemetry::Off | Telemetry::Stub => {
            BenchBackend::PostgresOff(InstrumentedLogBackend::new(inner, NoTelemetry))
        }
        Telemetry::Console => BenchBackend::PostgresConsole(InstrumentedLogBackend::new(
            inner,
            ConsoleTelemetry,
        )),
    }
}

pub(crate) fn wrap_tikv_raw(inner: TikvRawLogBackend, telemetry: Telemetry) -> BenchBackend {
    match telemetry {
        Telemetry::Off | Telemetry::Stub => {
            BenchBackend::TikvRawOff(InstrumentedLogBackend::new(inner, NoTelemetry))
        }
        Telemetry::Console => BenchBackend::TikvRawConsole(InstrumentedLogBackend::new(
            inner,
            ConsoleTelemetry,
        )),
    }
}

pub(crate) fn wrap_scylla(inner: ScyllaLogBackend, telemetry: Telemetry) -> BenchBackend {
    match telemetry {
        Telemetry::Off | Telemetry::Stub => {
            BenchBackend::ScyllaOff(InstrumentedLogBackend::new(inner, NoTelemetry))
        }
        Telemetry::Console => BenchBackend::ScyllaConsole(InstrumentedLogBackend::new(
            inner,
            ConsoleTelemetry,
        )),
    }
}

/// Destination kind matching storage for stream construction.
pub fn backend_kind(storage: Storage) -> LogBackendKind {
    match storage {
        Storage::Mem => LogBackendKind::Memory,
        Storage::SurrealMem | Storage::SurrealRocksdb | Storage::SurrealTikv => {
            LogBackendKind::SurrealLocal
        }
        Storage::Postgres => LogBackendKind::Postgres,
        Storage::Sqlite => LogBackendKind::Sqlite,
        Storage::Scylla => LogBackendKind::Scylla,
        Storage::TikvRaw => LogBackendKind::TikvRaw,
    }
}
