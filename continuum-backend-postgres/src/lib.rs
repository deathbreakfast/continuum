//! `PostgreSQL` [`LogBackend`] for the continuum transport log.
//!
//! Enable via the `postgres` feature on the [`continuum`](https://docs.rs/continuum) facade.
//! See [Getting started](https://docs.rs/continuum/latest/continuum/index.html#getting-started)
//! and the [documentation map](https://docs.rs/continuum/latest/continuum/index.html#documentation-map).

use async_trait::async_trait;
use continuum_backend_sql_common::SqlLogBackend;
use continuum_core::backend::LogBackend;
use continuum_core::error::Result;
use continuum_core::types::{AppendRecord, EventRecord, LogStreamId, Seq, SubscriptionId};
use sqlx::PgPool;

/// PostgreSQL-backed transport log.
pub struct PostgresLogBackend {
    inner: SqlLogBackend,
}

impl PostgresLogBackend {
    /// Connect to `PostgreSQL` at `url`.
    ///
    /// # Errors
    ///
    /// Returns an error if the connection or schema bootstrap fails.
    pub async fn new(url: &str) -> Result<Self> {
        Self::connect(url).await
    }

    /// Connect using a `PostgreSQL` connection URL.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # async fn run() -> continuum_core::Result<()> {
    /// use continuum_backend_postgres::PostgresLogBackend;
    ///
    /// let backend = PostgresLogBackend::connect("postgres://localhost/continuum").await?;
    /// let _ = backend;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the connection or schema bootstrap fails.
    pub async fn connect(url: &str) -> Result<Self> {
        let inner = SqlLogBackend::connect_postgres(url).await?;
        Ok(Self { inner })
    }

    /// Wrap an existing pool (schema bootstrap runs).
    ///
    /// # Errors
    ///
    /// Returns an error if schema bootstrap fails.
    pub async fn from_pool(pool: PgPool) -> Result<Self> {
        let inner = SqlLogBackend::from_postgres_pool(pool).await?;
        Ok(Self { inner })
    }

    /// Underlying connection pool (for shared-handle benchmarks).
    ///
    /// # Panics
    ///
    /// Panics if the inner pool is not a `PostgreSQL` pool.
    #[must_use]
    pub fn pool(&self) -> &PgPool {
        match self.inner.pool() {
            continuum_backend_sql_common::SqlPool::Postgres(pool) => pool,
            continuum_backend_sql_common::SqlPool::Sqlite(_) => {
                panic!("postgres backend has non-postgres pool")
            }
        }
    }
}

#[async_trait]
impl LogBackend for PostgresLogBackend {
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

impl std::fmt::Debug for PostgresLogBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PostgresLogBackend").finish_non_exhaustive()
    }
}
