//! Build [`LogBackend`] instances for benchmark dimensions.

use std::fmt::Debug;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use continuum::backend::LogBackend;
use continuum::backends::{InMemoryLogBackend, PostgresLogBackend, SqliteLogBackend, SurrealLocalLogBackend};
use continuum::types::LogBackendKind;
use continuum::{ConsoleTelemetry, InstrumentedLogBackend, NoTelemetry};
use surrealdb::engine::any::Any;
use surrealdb::Surreal;
use tempfile::TempDir;

use super::dimensions::{Storage, Telemetry, Topology};

/// Shared engine handle for co-tenancy experiments.
#[derive(Clone)]
pub enum SharedHandle {
    /// Shared Surreal client.
    Surreal(Arc<Surreal<Any>>),
    /// Shared `SQLite` pool.
    Sqlite(Arc<sqlx::SqlitePool>),
    /// Shared `PostgreSQL` pool.
    Postgres(Arc<sqlx::PgPool>),
}

/// Holds backend plus metadata needed for reporting and lifetime management.
pub struct BackendHandle {
    pub backend: BenchBackend,
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
        Storage::Mem => {
            let inner = InMemoryLogBackend::new();
            let backend = wrap_mem(inner, telemetry);
            Ok(BackendHandle {
                backend,
                engine_path: "mem://".into(),
                _temp_dir: None,
                _shared: None,
            })
        }
        Storage::SurrealMem | Storage::SurrealRocksdb => {
            build_surreal(storage, topology, telemetry, shared).await
        }
        Storage::Sqlite => build_sqlite(topology, telemetry, shared).await,
        Storage::Postgres => build_postgres(topology, telemetry, shared).await,
    }
}

fn wrap_mem(inner: InMemoryLogBackend, telemetry: Telemetry) -> BenchBackend {
    match telemetry {
        Telemetry::Off | Telemetry::Stub => {
            BenchBackend::MemOff(InstrumentedLogBackend::new(inner, NoTelemetry))
        }
        Telemetry::Console => {
            BenchBackend::MemConsole(InstrumentedLogBackend::new(inner, ConsoleTelemetry))
        }
    }
}

fn wrap_surreal(inner: SurrealLocalLogBackend, telemetry: Telemetry) -> BenchBackend {
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

fn wrap_sqlite(inner: SqliteLogBackend, telemetry: Telemetry) -> BenchBackend {
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

fn wrap_postgres(inner: PostgresLogBackend, telemetry: Telemetry) -> BenchBackend {
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

async fn build_surreal(
    storage: Storage,
    topology: Topology,
    telemetry: Telemetry,
    shared: Option<SharedHandle>,
) -> Result<BackendHandle> {
    let (db, engine_path, temp_dir) = if topology == Topology::SharedHandle {
        if let Some(SharedHandle::Surreal(db)) = shared {
            (db, "shared-surreal-handle".into(), None)
        } else {
            let (db, path, temp) = open_surreal(storage).await?;
            (db, path, temp)
        }
    } else if topology == Topology::RemoteSurreal {
        let url = std::env::var("CONTINUUM_BENCH_SURREAL_URL")
            .context("CONTINUUM_BENCH_SURREAL_URL not set for distributed topology")?;
        let db = open_surreal_url(&url).await?;
        (db, url, None)
    } else {
        let (db, path, temp) = open_surreal(storage).await?;
        (db, path, temp)
    };

    let backend = SurrealLocalLogBackend::new(Arc::clone(&db))
        .await
        .context("SurrealLocalLogBackend::new")?;

    Ok(BackendHandle {
        backend: wrap_surreal(backend, telemetry),
        engine_path,
        _temp_dir: temp_dir,
        _shared: Some(SharedHandle::Surreal(db)),
    })
}

async fn build_sqlite(
    topology: Topology,
    telemetry: Telemetry,
    shared: Option<SharedHandle>,
) -> Result<BackendHandle> {
    let (pool, engine_path, temp_dir) = if topology == Topology::SharedHandle {
        if let Some(SharedHandle::Sqlite(pool)) = shared {
            (pool, "shared-sqlite-pool".into(), None)
        } else {
            let (pool, path, temp) = open_sqlite_pool().await?;
            (pool, path, temp)
        }
    } else {
        let (pool, path, temp) = open_sqlite_pool().await?;
        (pool, path, temp)
    };

    let inner = SqliteLogBackend::from_pool((*pool).clone())
        .await
        .context("SqliteLogBackend::from_pool")?;

    Ok(BackendHandle {
        backend: wrap_sqlite(inner, telemetry),
        engine_path,
        _temp_dir: temp_dir,
        _shared: Some(SharedHandle::Sqlite(pool)),
    })
}

async fn build_postgres(
    topology: Topology,
    telemetry: Telemetry,
    shared: Option<SharedHandle>,
) -> Result<BackendHandle> {
    let url = std::env::var("CONTINUUM_BENCH_POSTGRES_URL")
        .context("CONTINUUM_BENCH_POSTGRES_URL not set for postgres storage")?;

    let pool = if topology == Topology::SharedHandle {
        if let Some(SharedHandle::Postgres(pool)) = shared {
            pool
        } else {
            open_postgres_pool(&url).await?
        }
    } else {
        open_postgres_pool(&url).await?
    };

    let inner = PostgresLogBackend::from_pool((*pool).clone())
        .await
        .context("PostgresLogBackend::from_pool")?;

    Ok(BackendHandle {
        backend: wrap_postgres(inner, telemetry),
        engine_path: url,
        _temp_dir: None,
        _shared: Some(SharedHandle::Postgres(pool)),
    })
}

async fn open_surreal(
    storage: Storage,
) -> Result<(Arc<Surreal<Any>>, String, Option<TempDir>)> {
    match storage {
        Storage::SurrealMem => {
            let db = open_surreal_url("mem://").await?;
            Ok((db, "mem://".into(), None))
        }
        Storage::SurrealRocksdb => {
            let dir = TempDir::new().context("tempdir for rocksdb")?;
            let path = dir.path().to_path_buf();
            let url = format!("rocksdb://{}", path.display());
            let db = open_surreal_url(&url).await?;
            Ok((db, url, Some(dir)))
        }
        _ => Err(anyhow!("not surreal storage")),
    }
}

async fn open_surreal_url(url: &str) -> Result<Arc<Surreal<Any>>> {
    let db: Surreal<Any> = Surreal::init();
    db.connect(url).await.context("surreal connect")?;
    db.use_ns("continuum")
        .use_db("bench")
        .await
        .context("surreal use_ns/db")?;
    Ok(Arc::new(db))
}

async fn open_sqlite_pool() -> Result<(Arc<sqlx::SqlitePool>, String, Option<TempDir>)> {
    let dir = TempDir::new().context("tempdir for sqlite")?;
    let path = dir.path().join("bench.db");
    let url = format!("sqlite://{}?mode=rwc", path.display());
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&url)
        .await
        .context("sqlite connect")?;
    Ok((Arc::new(pool), url, Some(dir)))
}

async fn open_postgres_pool(url: &str) -> Result<Arc<sqlx::PgPool>> {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(url)
        .await
        .context("postgres connect")?;
    Ok(Arc::new(pool))
}

/// Open a fresh shared Surreal handle for co-tenancy experiments.
pub async fn open_shared_surreal(storage: Storage) -> Result<(Arc<Surreal<Any>>, Option<TempDir>)> {
    let (db, _, temp) = open_surreal(storage).await?;
    Ok((db, temp))
}

/// Open a fresh shared `SQLite` pool for co-tenancy experiments.
pub async fn open_shared_sqlite() -> Result<(Arc<sqlx::SqlitePool>, TempDir)> {
    let (pool, _, temp) = open_sqlite_pool().await?;
    let temp = temp.context("sqlite tempdir")?;
    Ok((pool, temp))
}

/// Open a fresh shared `PostgreSQL` pool for co-tenancy experiments.
pub async fn open_shared_postgres() -> Result<Arc<sqlx::PgPool>> {
    let url = std::env::var("CONTINUUM_BENCH_POSTGRES_URL")
        .context("CONTINUUM_BENCH_POSTGRES_URL not set")?;
    open_postgres_pool(&url).await
}

/// Destination kind matching storage for stream construction.
pub fn backend_kind(storage: Storage) -> LogBackendKind {
    match storage {
        Storage::Mem => LogBackendKind::Memory,
        Storage::SurrealMem | Storage::SurrealRocksdb => LogBackendKind::SurrealLocal,
        Storage::Postgres => LogBackendKind::Postgres,
        Storage::Sqlite => LogBackendKind::Sqlite,
    }
}

/// Directory or file size in bytes (for on-disk storage growth metrics).
pub fn dir_size_bytes(path: &PathBuf) -> u64 {
    dir_size_recursive(path).unwrap_or(0)
}

fn dir_size_recursive(path: &PathBuf) -> Result<u64> {
    let mut total = 0u64;
    if path.is_file() {
        return Ok(path.metadata()?.len());
    }
    if path.is_dir() {
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            total += dir_size_recursive(&entry.path())?;
        }
    }
    Ok(total)
}

/// Parse an engine path into a local disk path when applicable.
pub fn storage_disk_path(engine_path: &str) -> Option<PathBuf> {
    if let Some(path) = engine_path.strip_prefix("rocksdb://") {
        return Some(PathBuf::from(path));
    }
    if let Some(path) = engine_path.strip_prefix("sqlite://") {
        let path = path.split('?').next()?;
        return Some(PathBuf::from(path));
    }
    None
}
