//! `SQLite` [`LogBackend`] for the continuum transport log.
//!
//! Enable via the `sqlite` feature on the [`continuum`](https://docs.rs/continuum) facade.
//! See [Getting started](https://docs.rs/continuum/latest/continuum/index.html#getting-started)
//! and the [documentation map](https://docs.rs/continuum/latest/continuum/index.html#documentation-map).

use std::path::Path;

use async_trait::async_trait;
use continuum_backend_sql_common::SqlLogBackend;
use continuum_core::backend::LogBackend;
use continuum_core::error::Result;
use continuum_core::types::{AppendRecord, EventRecord, LogStreamId, Seq, SubscriptionId};
use sqlx::SqlitePool;

/// SQLite-backed transport log.
pub struct SqliteLogBackend {
    inner: SqlLogBackend,
}

impl SqliteLogBackend {
    /// Open a `SQLite` database at `path` (creates the file if missing).
    ///
    /// # Errors
    ///
    /// Returns an error if the connection or schema bootstrap fails.
    pub async fn new(path: impl AsRef<Path>) -> Result<Self> {
        let url = format!("sqlite://{}?mode=rwc", path.as_ref().display());
        Self::connect(&url).await
    }

    /// Connect using a `SQLite` connection URL.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # async fn run() -> continuum_core::Result<()> {
    /// use continuum_backend_sqlite::SqliteLogBackend;
    ///
    /// let backend = SqliteLogBackend::connect("sqlite://continuum.db?mode=rwc").await?;
    /// let _ = backend;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the connection or schema bootstrap fails.
    pub async fn connect(url: &str) -> Result<Self> {
        let inner = SqlLogBackend::connect_sqlite(url).await?;
        Ok(Self { inner })
    }

    /// Wrap an existing pool (schema bootstrap runs).
    ///
    /// # Errors
    ///
    /// Returns an error if schema bootstrap fails.
    pub async fn from_pool(pool: SqlitePool) -> Result<Self> {
        let inner = SqlLogBackend::from_sqlite_pool(pool).await?;
        Ok(Self { inner })
    }

    /// Underlying connection pool (for shared-handle benchmarks).
    ///
    /// # Panics
    ///
    /// Panics if the inner pool is not a `SQLite` pool.
    #[must_use]
    pub fn pool(&self) -> &SqlitePool {
        match self.inner.pool() {
            continuum_backend_sql_common::SqlPool::Sqlite(pool) => pool,
            continuum_backend_sql_common::SqlPool::Postgres(_) => {
                panic!("sqlite backend has non-sqlite pool")
            }
        }
    }
}

#[async_trait]
impl LogBackend for SqliteLogBackend {
    async fn append(
        &self,
        stream: LogStreamId,
        records: &[AppendRecord],
    ) -> Result<Vec<Seq>> {
        self.inner.append(stream, records).await
    }

    async fn read_from(
        &self,
        stream: LogStreamId,
        after: Seq,
        limit: usize,
    ) -> Result<Vec<EventRecord>> {
        self.inner.read_from(stream, after, limit).await
    }

    async fn commit_checkpoint(
        &self,
        subscription: &SubscriptionId,
        stream: LogStreamId,
        seq: Seq,
    ) -> Result<()> {
        self.inner
            .commit_checkpoint(subscription, stream, seq)
            .await
    }

    async fn load_checkpoint(
        &self,
        subscription: &SubscriptionId,
        stream: LogStreamId,
    ) -> Result<Option<Seq>> {
        self.inner.load_checkpoint(subscription, stream).await
    }

    async fn read_from_topic(
        &self,
        stream: LogStreamId,
        topic_key: Option<&str>,
        after: Seq,
        limit: usize,
    ) -> Result<Vec<EventRecord>> {
        self.inner
            .read_from_topic(stream, topic_key, after, limit)
            .await
    }

    async fn truncate_before(&self, stream: LogStreamId, seq: Seq) -> Result<u64> {
        self.inner.truncate_before(stream, seq).await
    }
}

impl std::fmt::Debug for SqliteLogBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SqliteLogBackend").finish_non_exhaustive()
    }
}
