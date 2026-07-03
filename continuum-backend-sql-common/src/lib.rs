//! Shared SQL [`LogBackend`] for `PostgreSQL` and `SQLite`.
//!
//! Uses `continuum_event`, `continuum_stream`, and `continuum_checkpoint` tables
//! (bootstrapped on connect).

mod error_map;
mod schema;

use std::fmt;

use chrono::{DateTime, Utc};
use sqlx::{Executor, Pool, Postgres, Row, Sqlite};
use uuid::Uuid;

use continuum_core::backend::LogBackend;
use continuum_core::error::Result;
use continuum_core::types::{AppendRecord, EventRecord, LogStreamId, Seq, SubscriptionId};
use continuum_core::validation::{validate_read_limit, validate_topic};

use error_map::map_err;

fn usize_as_i64(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

/// `SQLite` uses `?` placeholders; `PostgreSQL` uses `$1`, `$2`, …
fn bind_sql(dialect: SqlDialect, sql: &str) -> String {
    match dialect {
        SqlDialect::Sqlite => sql.to_string(),
        SqlDialect::Postgres => {
            let mut out = String::with_capacity(sql.len());
            let mut n = 1u32;
            for ch in sql.chars() {
                if ch == '?' {
                    out.push('$');
                    out.push_str(&n.to_string());
                    n += 1;
                } else {
                    out.push(ch);
                }
            }
            out
        }
    }
}

/// SQL dialect for query variants (locking).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SqlDialect {
    /// `PostgreSQL`.
    Postgres,
    /// `SQLite`.
    Sqlite,
}

/// Connection pool for a SQL backend.
#[derive(Clone)]
pub enum SqlPool {
    /// `SQLite` pool.
    Sqlite(Pool<Sqlite>),
    /// `PostgreSQL` pool.
    Postgres(Pool<Postgres>),
}

impl fmt::Debug for SqlPool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sqlite(_) => f.debug_tuple("SqlPool::Sqlite").finish(),
            Self::Postgres(_) => f.debug_tuple("SqlPool::Postgres").finish(),
        }
    }
}

/// SQL-backed transport log (`PostgreSQL` or `SQLite`).
pub struct SqlLogBackend {
    pool: SqlPool,
    dialect: SqlDialect,
}

impl SqlLogBackend {
    /// Open a `SQLite` pool, bootstrap schema, and return a backend.
    ///
    /// # Errors
    ///
    /// Returns an error if the connection or schema bootstrap fails.
    pub async fn connect_sqlite(url: &str) -> Result<Self> {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(5)
            .connect(url)
            .await
            .map_err(map_err)?;
        Self::from_sqlite_pool(pool).await
    }

    /// Open a `PostgreSQL` pool, bootstrap schema, and return a backend.
    ///
    /// # Errors
    ///
    /// Returns an error if the connection or schema bootstrap fails.
    pub async fn connect_postgres(url: &str) -> Result<Self> {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(5)
            .connect(url)
            .await
            .map_err(map_err)?;
        Self::from_postgres_pool(pool).await
    }

    /// Wrap an existing `SQLite` pool (schema bootstrap runs).
    ///
    /// # Errors
    ///
    /// Returns an error if schema bootstrap fails.
    pub async fn from_sqlite_pool(pool: Pool<Sqlite>) -> Result<Self> {
        let backend = Self {
            pool: SqlPool::Sqlite(pool),
            dialect: SqlDialect::Sqlite,
        };
        schema::ensure_schema(&backend).await?;
        Ok(backend)
    }

    /// Wrap an existing `PostgreSQL` pool (schema bootstrap runs).
    ///
    /// # Errors
    ///
    /// Returns an error if schema bootstrap fails.
    pub async fn from_postgres_pool(pool: Pool<Postgres>) -> Result<Self> {
        let backend = Self {
            pool: SqlPool::Postgres(pool),
            dialect: SqlDialect::Postgres,
        };
        schema::ensure_schema(&backend).await?;
        Ok(backend)
    }

    /// Underlying connection pool (for shared-handle benchmarks).
    #[must_use]
    pub const fn pool(&self) -> &SqlPool {
        &self.pool
    }

    /// Engine dialect.
    #[must_use]
    pub const fn dialect(&self) -> SqlDialect {
        self.dialect
    }

    async fn existing_seq(&self, stream_key: &str, event_id: &Uuid) -> Result<Option<Seq>> {
        let sql = bind_sql(
            self.dialect,
            "SELECT seq FROM continuum_event WHERE stream_key = ? AND event_id = ? LIMIT 1",
        );
        match &self.pool {
            SqlPool::Sqlite(pool) => {
                let row = sqlx::query(&sql)
                    .bind(stream_key)
                    .bind(event_id.to_string())
                    .fetch_optional(pool)
                    .await
                    .map_err(map_err)?;
                Ok(row.map(|r| Seq(r.get::<i64, _>("seq"))))
            }
            SqlPool::Postgres(pool) => {
                let row = sqlx::query(&sql)
                    .bind(stream_key)
                    .bind(event_id.to_string())
                    .fetch_optional(pool)
                    .await
                    .map_err(map_err)?;
                Ok(row.map(|r| Seq(r.get::<i64, _>("seq"))))
            }
        }
    }

    async fn allocate_seq_batch(&self, stream_key: &str, count: usize) -> Result<Vec<Seq>> {
        if count == 0 {
            return Ok(vec![]);
        }

        let select_sql = bind_sql(
            self.dialect,
            match self.dialect {
                SqlDialect::Postgres => {
                    "SELECT next_seq FROM continuum_stream WHERE stream_key = ? FOR UPDATE"
                }
                SqlDialect::Sqlite => {
                    "SELECT next_seq FROM continuum_stream WHERE stream_key = ?"
                }
            },
        );

        let upsert_sql = bind_sql(
            self.dialect,
            "INSERT INTO continuum_stream (stream_key, next_seq) VALUES (?, ?)
             ON CONFLICT (stream_key) DO UPDATE SET next_seq = excluded.next_seq",
        );

        let seqs = match &self.pool {
            SqlPool::Sqlite(pool) => {
                let mut tx = pool.begin().await.map_err(map_err)?;
                let row = sqlx::query(&select_sql)
                    .bind(stream_key)
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(map_err)?;
                let current = row.map_or(0, |r| r.get::<i64, _>("next_seq"));
                let end = current + usize_as_i64(count);
                sqlx::query(&upsert_sql)
                    .bind(stream_key)
                    .bind(end)
                    .execute(&mut *tx)
                    .await
                    .map_err(map_err)?;
                tx.commit().await.map_err(map_err)?;
                (1..=usize_as_i64(count))
                    .map(|offset| Seq(current + offset))
                    .collect()
            }
            SqlPool::Postgres(pool) => {
                let mut tx = pool.begin().await.map_err(map_err)?;
                let row = sqlx::query(&select_sql)
                    .bind(stream_key)
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(map_err)?;
                let current = row.map_or(0, |r| r.get::<i64, _>("next_seq"));
                let end = current + usize_as_i64(count);
                sqlx::query(&upsert_sql)
                    .bind(stream_key)
                    .bind(end)
                    .execute(&mut *tx)
                    .await
                    .map_err(map_err)?;
                tx.commit().await.map_err(map_err)?;
                (1..=usize_as_i64(count))
                    .map(|offset| Seq(current + offset))
                    .collect()
            }
        };

        Ok(seqs)
    }

    async fn insert_event(
        &self,
        stream_key: &str,
        seq: Seq,
        rec: &AppendRecord,
    ) -> Result<()> {
        let sql = bind_sql(
            self.dialect,
            "INSERT INTO continuum_event
             (stream_key, seq, event_id, ts_millis, attempt, actor_ref, payload_ciphertext)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        );
        match &self.pool {
            SqlPool::Sqlite(pool) => {
                sqlx::query(&sql)
                    .bind(stream_key)
                    .bind(seq.as_i64())
                    .bind(rec.event_id.to_string())
                    .bind(rec.ts.timestamp_millis())
                    .bind(rec.attempt.cast_signed())
                    .bind(rec.actor_ref.as_deref())
                    .bind(rec.payload_ciphertext.as_slice())
                    .execute(pool)
                    .await
                    .map_err(map_err)?;
            }
            SqlPool::Postgres(pool) => {
                sqlx::query(&sql)
                    .bind(stream_key)
                    .bind(seq.as_i64())
                    .bind(rec.event_id.to_string())
                    .bind(rec.ts.timestamp_millis())
                    .bind(rec.attempt.cast_signed())
                    .bind(rec.actor_ref.as_deref())
                    .bind(rec.payload_ciphertext.as_slice())
                    .execute(pool)
                    .await
                    .map_err(map_err)?;
            }
        }
        Ok(())
    }

    async fn execute_ddl(&self, ddl: &str) -> Result<()> {
        match &self.pool {
            SqlPool::Sqlite(pool) => {
                pool.execute(ddl).await.map_err(map_err)?;
            }
            SqlPool::Postgres(pool) => {
                pool.execute(ddl).await.map_err(map_err)?;
            }
        }
        Ok(())
    }
}

impl SqlLogBackend {
    pub(crate) async fn run_ddl(&self, ddl: &str) -> Result<()> {
        self.execute_ddl(ddl).await
    }
}

#[async_trait::async_trait]
impl LogBackend for SqlLogBackend {
    async fn append(
        &self,
        stream: LogStreamId,
        records: &[AppendRecord],
    ) -> Result<Vec<Seq>> {
        if records.is_empty() {
            return Ok(vec![]);
        }
        validate_topic(&stream.topic)?;

        let stream_key = stream.storage_key();
        let mut out = Vec::with_capacity(records.len());
        let mut new_records: Vec<(usize, &AppendRecord)> = Vec::new();

        for (idx, rec) in records.iter().enumerate() {
            if let Some(seq) = self.existing_seq(&stream_key, &rec.event_id).await? {
                out.push(seq);
            } else {
                new_records.push((idx, rec));
                out.push(Seq(0));
            }
        }

        if new_records.is_empty() {
            return Ok(out);
        }

        let seqs = self
            .allocate_seq_batch(&stream_key, new_records.len())
            .await?;

        for ((idx, rec), seq) in new_records.into_iter().zip(seqs) {
            out[idx] = seq;
            self.insert_event(&stream_key, seq, rec).await?;
        }

        Ok(out)
    }

    async fn read_from(
        &self,
        stream: LogStreamId,
        after: Seq,
        limit: usize,
    ) -> Result<Vec<EventRecord>> {
        validate_read_limit(limit)?;
        if limit == 0 {
            return Ok(vec![]);
        }

        let stream_key = stream.storage_key();
        let sql = bind_sql(
            self.dialect,
            "SELECT seq, event_id, ts_millis, attempt, actor_ref, payload_ciphertext
             FROM continuum_event
             WHERE stream_key = ? AND seq > ?
             ORDER BY seq ASC
             LIMIT ?",
        );

        match &self.pool {
            SqlPool::Sqlite(pool) => {
                let rows = sqlx::query(&sql)
                    .bind(&stream_key)
                    .bind(after.as_i64())
                    .bind(usize_as_i64(limit))
                    .fetch_all(pool)
                    .await
                    .map_err(map_err)?;
                Ok(rows
                    .into_iter()
                    .filter_map(|row| sqlite_row_to_event(&stream, &row, stream.key.clone()))
                    .collect())
            }
            SqlPool::Postgres(pool) => {
                let rows = sqlx::query(&sql)
                    .bind(&stream_key)
                    .bind(after.as_i64())
                    .bind(usize_as_i64(limit))
                    .fetch_all(pool)
                    .await
                    .map_err(map_err)?;
                Ok(rows
                    .into_iter()
                    .filter_map(|row| pg_row_to_event(&stream, &row, stream.key.clone()))
                    .collect())
            }
        }
    }

    async fn read_from_topic(
        &self,
        stream: LogStreamId,
        topic_key: Option<&str>,
        after: Seq,
        limit: usize,
    ) -> Result<Vec<EventRecord>> {
        validate_read_limit(limit)?;
        if limit == 0 {
            return Ok(vec![]);
        }

        if let Some(key) = topic_key {
            return self
                .read_from(
                    LogStreamId::new(
                        stream.destination.clone(),
                        stream.topic.clone(),
                        Some(key.to_string()),
                    ),
                    after,
                    limit,
                )
                .await;
        }

        let topic_prefix = format!(
            "{}{}{}{}",
            stream.destination.router_key(),
            continuum_core::types::STORAGE_KEY_SEP,
            stream.topic,
            continuum_core::types::STORAGE_KEY_SEP,
        );
        let like_prefix = format!("{topic_prefix}%");

        let sql = bind_sql(
            self.dialect,
            "SELECT seq, event_id, ts_millis, attempt, actor_ref, payload_ciphertext, stream_key
             FROM continuum_event
             WHERE stream_key LIKE ? AND seq > ?
             ORDER BY seq ASC
             LIMIT ?",
        );

        match &self.pool {
            SqlPool::Sqlite(pool) => {
                let rows = sqlx::query(&sql)
                    .bind(&like_prefix)
                    .bind(after.as_i64())
                    .bind(usize_as_i64(limit))
                    .fetch_all(pool)
                    .await
                    .map_err(map_err)?;
                Ok(rows
                    .into_iter()
                    .filter_map(|row| {
                        let stream_key: String = row.get("stream_key");
                        let key = stream_key.strip_prefix(&topic_prefix).and_then(|rest| {
                            if rest.is_empty() {
                                None
                            } else {
                                Some(rest.to_string())
                            }
                        });
                        sqlite_row_to_event(&stream, &row, key)
                    })
                    .collect())
            }
            SqlPool::Postgres(pool) => {
                let rows = sqlx::query(&sql)
                    .bind(&like_prefix)
                    .bind(after.as_i64())
                    .bind(usize_as_i64(limit))
                    .fetch_all(pool)
                    .await
                    .map_err(map_err)?;
                Ok(rows
                    .into_iter()
                    .filter_map(|row| {
                        let stream_key: String = row.get("stream_key");
                        let key = stream_key.strip_prefix(&topic_prefix).and_then(|rest| {
                            if rest.is_empty() {
                                None
                            } else {
                                Some(rest.to_string())
                            }
                        });
                        pg_row_to_event(&stream, &row, key)
                    })
                    .collect())
            }
        }
    }

    async fn commit_checkpoint(
        &self,
        subscription: &SubscriptionId,
        stream: LogStreamId,
        seq: Seq,
    ) -> Result<()> {
        let stream_key = stream.storage_key();
        let existing = self.load_checkpoint(subscription, stream.clone()).await?;
        if let Some(current) = existing {
            if seq <= current {
                return Ok(());
            }
        }

        let sql = bind_sql(
            self.dialect,
            "INSERT INTO continuum_checkpoint (subscription, stream_key, seq)
             VALUES (?, ?, ?)
             ON CONFLICT (subscription, stream_key) DO UPDATE SET seq = excluded.seq",
        );

        match &self.pool {
            SqlPool::Sqlite(pool) => {
                sqlx::query(&sql)
                    .bind(subscription.0.as_str())
                    .bind(&stream_key)
                    .bind(seq.as_i64())
                    .execute(pool)
                    .await
                    .map_err(map_err)?;
            }
            SqlPool::Postgres(pool) => {
                sqlx::query(&sql)
                    .bind(subscription.0.as_str())
                    .bind(&stream_key)
                    .bind(seq.as_i64())
                    .execute(pool)
                    .await
                    .map_err(map_err)?;
            }
        }
        Ok(())
    }

    async fn load_checkpoint(
        &self,
        subscription: &SubscriptionId,
        stream: LogStreamId,
    ) -> Result<Option<Seq>> {
        let stream_key = stream.storage_key();
        let sql = bind_sql(
            self.dialect,
            "SELECT seq FROM continuum_checkpoint
             WHERE subscription = ? AND stream_key = ?
             LIMIT 1",
        );

        match &self.pool {
            SqlPool::Sqlite(pool) => {
                let row = sqlx::query(&sql)
                    .bind(subscription.0.as_str())
                    .bind(&stream_key)
                    .fetch_optional(pool)
                    .await
                    .map_err(map_err)?;
                Ok(row.map(|r| Seq(r.get::<i64, _>("seq"))))
            }
            SqlPool::Postgres(pool) => {
                let row = sqlx::query(&sql)
                    .bind(subscription.0.as_str())
                    .bind(&stream_key)
                    .fetch_optional(pool)
                    .await
                    .map_err(map_err)?;
                Ok(row.map(|r| Seq(r.get::<i64, _>("seq"))))
            }
        }
    }

    async fn truncate_before(&self, stream: LogStreamId, seq: Seq) -> Result<u64> {
        let stream_key = stream.storage_key();
        let count_sql = bind_sql(
            self.dialect,
            "SELECT COUNT(*) AS count FROM continuum_event
             WHERE stream_key = ? AND seq < ?",
        );
        let delete_sql = bind_sql(
            self.dialect,
            "DELETE FROM continuum_event WHERE stream_key = ? AND seq < ?",
        );

        let removed: i64 = match &self.pool {
            SqlPool::Sqlite(pool) => {
                let count_row = sqlx::query(&count_sql)
                    .bind(&stream_key)
                    .bind(seq.as_i64())
                    .fetch_one(pool)
                    .await
                    .map_err(map_err)?;
                let removed = count_row.get::<i64, _>("count");
                sqlx::query(&delete_sql)
                    .bind(&stream_key)
                    .bind(seq.as_i64())
                    .execute(pool)
                    .await
                    .map_err(map_err)?;
                removed
            }
            SqlPool::Postgres(pool) => {
                let count_row = sqlx::query(&count_sql)
                    .bind(&stream_key)
                    .bind(seq.as_i64())
                    .fetch_one(pool)
                    .await
                    .map_err(map_err)?;
                let removed = count_row.get::<i64, _>("count");
                sqlx::query(&delete_sql)
                    .bind(&stream_key)
                    .bind(seq.as_i64())
                    .execute(pool)
                    .await
                    .map_err(map_err)?;
                removed
            }
        };

        Ok(removed.cast_unsigned())
    }
}

impl fmt::Debug for SqlLogBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SqlLogBackend")
            .field("dialect", &self.dialect)
            .finish_non_exhaustive()
    }
}

fn sqlite_row_to_event(
    stream: &LogStreamId,
    row: &sqlx::sqlite::SqliteRow,
    key: Option<String>,
) -> Option<EventRecord> {
    let event_id = Uuid::parse_str(row.get::<String, _>("event_id").as_str()).ok()?;
    Some(EventRecord {
        destination: stream.destination.clone(),
        event_id,
        topic: stream.topic.clone(),
        key,
        seq: Seq(row.get::<i64, _>("seq")),
        ts: DateTime::from_timestamp_millis(row.get::<i64, _>("ts_millis"))
            .unwrap_or_else(Utc::now),
        attempt: row.get::<i32, _>("attempt").cast_unsigned(),
        actor_ref: row.get::<Option<String>, _>("actor_ref"),
        payload_ciphertext: row.get::<Vec<u8>, _>("payload_ciphertext"),
    })
}

fn pg_row_to_event(
    stream: &LogStreamId,
    row: &sqlx::postgres::PgRow,
    key: Option<String>,
) -> Option<EventRecord> {
    let event_id = Uuid::parse_str(row.get::<String, _>("event_id").as_str()).ok()?;
    Some(EventRecord {
        destination: stream.destination.clone(),
        event_id,
        topic: stream.topic.clone(),
        key,
        seq: Seq(row.get::<i64, _>("seq")),
        ts: DateTime::from_timestamp_millis(row.get::<i64, _>("ts_millis"))
            .unwrap_or_else(Utc::now),
        attempt: row.get::<i32, _>("attempt").cast_unsigned(),
        actor_ref: row.get::<Option<String>, _>("actor_ref"),
        payload_ciphertext: row.get::<Vec<u8>, _>("payload_ciphertext"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use continuum_core::backend::LogBackend;
    use continuum_core::types::{LogBackendKind, LogDestination};

    #[tokio::test]
    async fn schema_and_append() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.db");
        let url = format!("sqlite://{}?mode=rwc", path.display());
        let backend = SqlLogBackend::connect_sqlite(&url)
            .await
            .expect("backend");
        let stream = LogStreamId::new(
            LogDestination::new("default", LogBackendKind::Sqlite),
            "topic",
            None,
        );
        let rec = AppendRecord::new(Uuid::new_v4(), vec![1]);
        let seqs = backend
            .append(stream.clone(), std::slice::from_ref(&rec))
            .await
            .unwrap();
        assert_eq!(seqs[0], Seq(1));
        let rows = backend.read_from(stream, Seq::ZERO, 10).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].payload_ciphertext, vec![1]);
    }
}
