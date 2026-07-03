//! Append, read, checkpoint, and truncate against Surreal tables.
//!
//! Uses `continuum_event`, `continuum_stream`, and `continuum_checkpoint` tables
//! (bootstrapped on connect). The host injects an embedded or remote Surreal handle —
//! see [`SurrealLocalLogBackend::new`] and [`SurrealLocalLogBackend::new_embedded_local`].

mod config;
mod db_conn;
mod error_map;
mod schema;

pub use config::SurrealLogConfig;

use std::sync::Arc;

use chrono::{DateTime, Utc};
use surrealdb::Surreal;
use surrealdb::engine::any::Any;
use surrealdb::engine::local::Db;
use uuid::Uuid;

use continuum_core::error::Result;
use continuum_core::types::{AppendRecord, EventRecord, LogStreamId, Seq, SubscriptionId};
use continuum_core::validation::{validate_read_limit, validate_topic};

use db_conn::DbConn;
use error_map::map_err;
use schema::ensure_schema;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct NewEvent {
    stream_key: String,
    seq: i64,
    event_id: String,
    ts_millis: i64,
    attempt: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    actor_ref: Option<String>,
    payload_ciphertext: Vec<u8>,
}

#[derive(Debug, serde::Deserialize)]
struct EventSeqRow {
    seq: i64,
}

#[derive(Debug, serde::Deserialize)]
struct SeqRow {
    next_seq: i64,
}

#[derive(Debug, serde::Deserialize)]
struct EventRow {
    seq: i64,
    event_id: String,
    ts_millis: i64,
    attempt: u32,
    actor_ref: Option<String>,
    payload_ciphertext: Vec<u8>,
}

#[derive(Debug, serde::Deserialize)]
struct CheckpointRow {
    seq: i64,
}

#[derive(Debug, serde::Deserialize)]
struct TopicEventRow {
    seq: i64,
    event_id: String,
    ts_millis: i64,
    attempt: u32,
    actor_ref: Option<String>,
    payload_ciphertext: Vec<u8>,
    #[serde(default)]
    stream_key: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct CountRow {
    count: i64,
}

/// Surreal-backed [`LogBackend`](continuum_core::backend::LogBackend).
pub struct SurrealLocalLogBackend {
    db: DbConn,
}

impl SurrealLocalLogBackend {
    /// Wrap an injected dynamic Surreal client (remote `TiKV` or in-memory tests).
    ///
    /// Runs schema bootstrap before returning.
    ///
    /// # Errors
    ///
    /// Returns an error when schema bootstrap or Surreal queries fail.
    pub async fn new(db: Arc<Surreal<Any>>) -> Result<Self> {
        let conn = DbConn::any(db);
        ensure_schema(&conn).await?;
        Ok(Self { db: conn })
    }

    /// Wrap an embedded local `RocksDB` Surreal handle.
    ///
    /// Equivalent behavior to [`Self::new`] — use whichever matches the injected client type.
    ///
    /// # Errors
    ///
    /// Returns an error when schema bootstrap or Surreal queries fail.
    pub async fn new_embedded_local(db: Arc<Surreal<Db>>) -> Result<Self> {
        let conn = DbConn::local(db);
        ensure_schema(&conn).await?;
        Ok(Self { db: conn })
    }

    /// Access the underlying dynamic client when constructed via [`Self::new`].
    #[must_use]
    pub fn db(&self) -> Option<&Surreal<Any>> {
        match &self.db {
            DbConn::Any(db) => Some(db),
            DbConn::Local(_) => None,
        }
    }

    async fn existing_seq(&self, stream_key: &str, event_id: &Uuid) -> Result<Option<Seq>> {
        let mut resp = self
            .db
            .query_sk_id(
                "SELECT seq FROM continuum_event WHERE stream_key = $sk AND event_id = $id LIMIT 1",
                stream_key.to_string(),
                event_id.to_string(),
            )
            .await?;
        let rows: Vec<EventSeqRow> = error_map::take_rows(&mut resp, 0)?;
        Ok(rows.into_iter().next().map(|r| Seq(r.seq)))
    }

    async fn allocate_seq_batch(&self, stream_key: &str, count: usize) -> Result<Vec<Seq>> {
        if count == 0 {
            return Ok(vec![]);
        }
        let mut resp = self
            .db
            .query_sk(
                "SELECT next_seq FROM continuum_stream WHERE stream_key = $sk LIMIT 1",
                stream_key.to_string(),
            )
            .await?;
        let rows: Vec<SeqRow> = error_map::take_rows(&mut resp, 0)?;
        let current = rows.first().map_or(0, |r| r.next_seq);
        let count_i64 = i64::try_from(count)
            .map_err(|_| continuum_core::error::LogError::Validation(format!("batch size {count} exceeds i64::MAX")))?;
        let end = current + count_i64;
        self.db
            .upsert_stream_seq(stream_key.to_string(), end)
            .await?;
        Ok((1..=count_i64)
            .map(|offset| Seq(current + offset))
            .collect())
    }

    async fn insert_event(&self, content: NewEvent) -> Result<()> {
        let payload = serde_json::to_value(content).map_err(|e| error_map::map_serde(&e))?;
        match &self.db {
            DbConn::Any(db) => {
                let _: Option<serde_json::Value> = db
                    .create("continuum_event")
                    .content(payload)
                    .await
                    .map_err(|e| map_err(&e))?;
            }
            DbConn::Local(db) => {
                let _: Option<serde_json::Value> = db
                    .create("continuum_event")
                    .content(payload)
                    .await
                    .map_err(|e| map_err(&e))?;
            }
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl continuum_core::backend::LogBackend for SurrealLocalLogBackend {
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
                out.push(Seq(0)); // placeholder; filled after batch allocate
            }
        }

        if new_records.is_empty() {
            return Ok(out);
        }

        let seqs = self
            .allocate_seq_batch(&stream_key, new_records.len())
            .await?;
        let mut insert_futs = Vec::with_capacity(new_records.len());
        for ((idx, rec), seq) in new_records.into_iter().zip(seqs) {
            out[idx] = seq;
            let content = NewEvent {
                stream_key: stream_key.clone(),
                seq: seq.as_i64(),
                event_id: rec.event_id.to_string(),
                ts_millis: rec.ts.timestamp_millis(),
                attempt: i64::from(rec.attempt),
                actor_ref: rec.actor_ref.clone(),
                payload_ciphertext: rec.payload_ciphertext.clone(),
            };
            insert_futs.push(self.insert_event(content));
        }
        futures::future::try_join_all(insert_futs).await?;

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
        let limit_i64 = i64::try_from(limit)
            .map_err(|_| continuum_core::error::LogError::Validation(format!("read limit {limit} exceeds i64::MAX")))?;
        let mut resp = self
            .db
            .query_stream_read(stream_key, after.as_i64(), limit_i64)
            .await?;

        let rows: Vec<EventRow> = error_map::take_rows(&mut resp, 0)?;
        Ok(rows
            .into_iter()
            .filter_map(|row| {
                let event_id = Uuid::parse_str(&row.event_id).ok()?;
                Some(EventRecord {
                    destination: stream.destination.clone(),
                    event_id,
                    topic: stream.topic.clone(),
                    key: stream.key.clone(),
                    seq: Seq(row.seq),
                    ts: DateTime::from_timestamp_millis(row.ts_millis).unwrap_or_else(Utc::now),
                    attempt: row.attempt,
                    actor_ref: row.actor_ref,
                    payload_ciphertext: row.payload_ciphertext,
                })
            })
            .collect())
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

        let mut resp = self
            .db
            .query_topic_read(
                topic_prefix.clone(),
                after.as_i64(),
                i64::try_from(limit).map_err(|_| {
                    continuum_core::error::LogError::Validation(format!(
                        "read limit {limit} exceeds i64::MAX"
                    ))
                })?,
            )
            .await?;

        let rows: Vec<TopicEventRow> = error_map::take_rows(&mut resp, 0)?;
        Ok(rows
            .into_iter()
            .filter_map(|row| {
                let event_id = Uuid::parse_str(&row.event_id).ok()?;
                let key = row.stream_key.as_ref().and_then(|sk| {
                    sk.strip_prefix(&topic_prefix).map(|rest| {
                        if rest.is_empty() {
                            None
                        } else {
                            Some(rest.to_string())
                        }
                    })
                }).flatten();
                Some(EventRecord {
                    destination: stream.destination.clone(),
                    event_id,
                    topic: stream.topic.clone(),
                    key,
                    seq: Seq(row.seq),
                    ts: DateTime::from_timestamp_millis(row.ts_millis).unwrap_or_else(Utc::now),
                    attempt: row.attempt,
                    actor_ref: row.actor_ref,
                    payload_ciphertext: row.payload_ciphertext,
                })
            })
            .collect())
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
        self.db
            .upsert_checkpoint(
                subscription.0.clone(),
                stream_key,
                seq.as_i64(),
            )
            .await?;
        Ok(())
    }

    async fn load_checkpoint(
        &self,
        subscription: &SubscriptionId,
        stream: LogStreamId,
    ) -> Result<Option<Seq>> {
        let stream_key = stream.storage_key();
        let mut resp = self
            .db
            .query_checkpoint(subscription.0.clone(), stream_key)
            .await?;
        let rows: Vec<CheckpointRow> = error_map::take_rows(&mut resp, 0)?;
        Ok(rows.into_iter().next().map(|r| Seq(r.seq)))
    }

    async fn truncate_before(&self, stream: LogStreamId, seq: Seq) -> Result<u64> {
        let stream_key = stream.storage_key();
        let mut count_resp = self
            .db
            .count_truncate(stream_key.clone(), seq.as_i64())
            .await?;
        let counts: Vec<CountRow> = error_map::take_rows(&mut count_resp, 0)?;
        let removed = counts
            .first()
            .map_or(0, |c| c.count.cast_unsigned());

        self.db
            .delete_truncate(stream_key, seq.as_i64())
            .await?;
        Ok(removed)
    }
}

impl std::fmt::Debug for SurrealLocalLogBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SurrealLocalLogBackend").finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use continuum_core::backend::LogBackend;
    use continuum_core::types::{LogBackendKind, LogDestination};
    use surrealdb::engine::any::Any;

    pub async fn test_db() -> Arc<Surreal<Any>> {
        let db: Surreal<Any> = Surreal::init();
        db.connect("mem://")
            .await
            .expect("connect mem");
        db.use_ns("continuum")
            .use_db("test")
            .await
            .expect("ns/db");
        Arc::new(db)
    }

    #[tokio::test]
    async fn schema_and_append() {
        let db = test_db().await;
        let backend = SurrealLocalLogBackend::new(db).await.expect("backend");
        let stream = LogStreamId::new(
            LogDestination::new("default", LogBackendKind::SurrealLocal),
            "topic",
            None,
        );
        let rec = AppendRecord::new(Uuid::new_v4(), vec![1]);
        let seqs = backend.append(stream.clone(), std::slice::from_ref(&rec)).await.unwrap();
        assert_eq!(seqs[0], Seq(1));
        let rows = backend.read_from(stream, Seq::ZERO, 10).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].payload_ciphertext, vec![1]);
    }
}
