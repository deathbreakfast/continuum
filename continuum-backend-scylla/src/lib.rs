//! ScyllaDB [`LogBackend`](continuum_core::backend::LogBackend) for the continuum transport log.

mod append_ops;
mod error_map;
mod schema;

use std::fmt;
use std::sync::Arc;

use dashmap::DashMap;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use scylla::client::session::Session;
use scylla::client::session_builder::SessionBuilder;
use scylla::serialize::row::SerializeRow;
use scylla::DeserializeRow;
use uuid::Uuid;

use continuum_core::backend::LogBackend;
use continuum_core::error::{LogError, Result};
use continuum_core::types::{AppendRecord, EventRecord, LogStreamId, Seq, SubscriptionId};
use continuum_core::validation::{validate_read_limit, validate_topic};

use error_map::map_err;

/// Reserved seq numbers per stream (client-side block after one LWT).
const SEQ_BLOCK_SIZE: i64 = 64;

#[derive(Debug, Clone, Copy)]
struct SeqBlock {
    next: i64,
    end: i64,
}

/// Connection settings for [`ScyllaLogBackend`].
#[derive(Debug, Clone)]
pub struct ScyllaLogConfig {
    /// Contact points (host:port). Driver discovers full topology.
    pub contact_points: Vec<String>,
    /// CQL keyspace for continuum tables.
    pub keyspace: String,
    /// Optional datacenter for DC-aware routing.
    pub datacenter: Option<String>,
    /// Optional username.
    pub username: Option<String>,
    /// Optional password.
    pub password: Option<String>,
}

impl Default for ScyllaLogConfig {
    fn default() -> Self {
        Self {
            contact_points: vec!["127.0.0.1:9042".into()],
            keyspace: "continuum".into(),
            datacenter: None,
            username: None,
            password: None,
        }
    }
}

/// Scylla-backed transport log.
pub struct ScyllaLogBackend {
    session: Arc<Session>,
    keyspace: String,
    select_event_id: scylla::statement::prepared::PreparedStatement,
    insert_event: scylla::statement::prepared::PreparedStatement,
    insert_event_id_lwt: scylla::statement::prepared::PreparedStatement,
    select_stream_seq: scylla::statement::prepared::PreparedStatement,
    insert_stream: scylla::statement::prepared::PreparedStatement,
    update_stream_lwt: scylla::statement::prepared::PreparedStatement,
    select_events: scylla::statement::prepared::PreparedStatement,
    upsert_checkpoint: scylla::statement::prepared::PreparedStatement,
    select_checkpoint: scylla::statement::prepared::PreparedStatement,
    count_truncate: scylla::statement::prepared::PreparedStatement,
    delete_truncate: scylla::statement::prepared::PreparedStatement,
    insert_stream_index: scylla::statement::prepared::PreparedStatement,
    select_stream_keys: scylla::statement::prepared::PreparedStatement,
    seq_blocks: DashMap<String, SeqBlock>,
}

#[derive(DeserializeRow)]
struct SeqRow {
    seq: i64,
}

#[derive(DeserializeRow)]
struct CountRow {
    cnt: i64,
}

#[derive(DeserializeRow)]
struct AppliedRow {
    #[scylla(rename = "[applied]")]
    applied: bool,
    next_seq: Option<i64>,
}

#[derive(DeserializeRow)]
struct EventRow {
    seq: i64,
    event_id: Uuid,
    ts_millis: i64,
    attempt: i32,
    actor_ref: Option<String>,
    payload_ciphertext: Vec<u8>,
}

#[derive(DeserializeRow)]
struct StreamKeyRow {
    stream_key: String,
}

impl ScyllaLogBackend {
    /// Connect to a Scylla cluster and bootstrap schema.
    pub async fn connect(config: ScyllaLogConfig) -> Result<Self> {
        let mut builder = SessionBuilder::new();
        for cp in &config.contact_points {
            builder = builder.known_node(cp.as_str());
        }
        if let Some(dc) = &config.datacenter {
            builder = builder.prefer_datacenter(dc.clone());
        }
        if let (Some(user), Some(pass)) = (&config.username, &config.password) {
            builder = builder.user(user.clone(), pass.clone());
        }
        let session = builder.build().await.map_err(map_err)?;
        Self::from_session(Arc::new(session), &config.keyspace).await
    }

    /// Wrap an existing session (schema bootstrap runs).
    pub async fn from_session(session: Arc<Session>, keyspace: &str) -> Result<Self> {
        schema::ensure_schema(&session, keyspace).await?;
        let ks = keyspace.to_string();
        let q = |sql: &str| sql.replace("continuum.", &format!("{ks}."));

        let backend = Self {
            session: Arc::clone(&session),
            keyspace: ks.clone(),
            select_event_id: session
                .prepare(q(
                    "SELECT seq FROM continuum.continuum_event_id WHERE stream_key = ? AND event_id = ?",
                ))
                .await
                .map_err(map_err)?,
            insert_event: session
                .prepare(q(
                    "INSERT INTO continuum.continuum_event (stream_key, seq, event_id, ts_millis, attempt, actor_ref, payload_ciphertext) VALUES (?, ?, ?, ?, ?, ?, ?)",
                ))
                .await
                .map_err(map_err)?,
            insert_event_id_lwt: session
                .prepare(q(
                    "INSERT INTO continuum.continuum_event_id (stream_key, event_id, seq) VALUES (?, ?, ?) IF NOT EXISTS",
                ))
                .await
                .map_err(map_err)?,
            select_stream_seq: session
                .prepare(q("SELECT next_seq FROM continuum.continuum_stream WHERE stream_key = ?"))
                .await
                .map_err(map_err)?,
            insert_stream: session
                .prepare(q(
                    "INSERT INTO continuum.continuum_stream (stream_key, next_seq) VALUES (?, ?) IF NOT EXISTS",
                ))
                .await
                .map_err(map_err)?,
            update_stream_lwt: session
                .prepare(q(
                    "UPDATE continuum.continuum_stream SET next_seq = ? WHERE stream_key = ? IF next_seq = ?",
                ))
                .await
                .map_err(map_err)?,
            select_events: session
                .prepare(q(
                    "SELECT seq, event_id, ts_millis, attempt, actor_ref, payload_ciphertext FROM continuum.continuum_event WHERE stream_key = ? AND seq > ? LIMIT ?",
                ))
                .await
                .map_err(map_err)?,
            upsert_checkpoint: session
                .prepare(q(
                    "INSERT INTO continuum.continuum_checkpoint (subscription, stream_key, seq) VALUES (?, ?, ?)",
                ))
                .await
                .map_err(map_err)?,
            select_checkpoint: session
                .prepare(q(
                    "SELECT seq FROM continuum.continuum_checkpoint WHERE subscription = ? AND stream_key = ?",
                ))
                .await
                .map_err(map_err)?,
            count_truncate: session
                .prepare(q(
                    "SELECT COUNT(*) AS cnt FROM continuum.continuum_event WHERE stream_key = ? AND seq < ?",
                ))
                .await
                .map_err(map_err)?,
            delete_truncate: session
                .prepare(q(
                    "DELETE FROM continuum.continuum_event WHERE stream_key = ? AND seq < ?",
                ))
                .await
                .map_err(map_err)?,
            insert_stream_index: session
                .prepare(q(
                    "INSERT INTO continuum.continuum_stream_index (topic_prefix, stream_key) VALUES (?, ?)",
                ))
                .await
                .map_err(map_err)?,
            select_stream_keys: session
                .prepare(q(
                    "SELECT stream_key FROM continuum.continuum_stream_index WHERE topic_prefix = ?",
                ))
                .await
                .map_err(map_err)?,
            seq_blocks: DashMap::new(),
        };
        Ok(backend)
    }

    /// Underlying driver session.
    #[must_use]
    pub fn session(&self) -> &Arc<Session> {
        &self.session
    }

    /// Configured keyspace name.
    #[must_use]
    pub fn keyspace(&self) -> &str {
        &self.keyspace
    }

    async fn execute_unpaged(
        &self,
        stmt: &scylla::statement::prepared::PreparedStatement,
        values: impl SerializeRow,
    ) -> Result<scylla::response::query_result::QueryResult> {
        append_ops::record_round_trip(1);
        self.session
            .execute_unpaged(stmt, values)
            .await
            .map_err(map_err)
    }

    async fn write_event_new(
        &self,
        stream_key: &str,
        seq: Seq,
        rec: &AppendRecord,
    ) -> Result<bool> {
        let id_result = self
            .execute_unpaged(
                &self.insert_event_id_lwt,
                (stream_key, rec.event_id, seq.as_i64()),
            )
            .await?;
        if !lwt_applied(&id_result) {
            return Ok(false);
        }
        self.execute_unpaged(
            &self.insert_event,
            (
                stream_key,
                seq.as_i64(),
                rec.event_id,
                rec.ts.timestamp_millis(),
                i32::try_from(rec.attempt).unwrap_or(i32::MAX),
                rec.actor_ref.as_deref(),
                rec.payload_ciphertext.as_slice(),
            ),
        )
        .await?;
        Ok(true)
    }

    async fn existing_seq(&self, stream_key: &str, event_id: &Uuid) -> Result<Option<Seq>> {
        let rows = self
            .execute_unpaged(&self.select_event_id, (stream_key, *event_id))
            .await?;
        Ok(maybe_first_row::<SeqRow>(rows).map(|r| Seq(r.seq)))
    }

    async fn ensure_stream_row(&self, stream_key: &str) -> Result<()> {
        self.execute_unpaged(&self.insert_stream, (stream_key, 0_i64))
            .await?;
        Ok(())
    }

    async fn reserve_seq_block_lwt(&self, stream_key: &str, block_size: i64) -> Result<SeqBlock> {
        for _ in 0..64 {
            let current = self
                .execute_unpaged(&self.select_stream_seq, (stream_key,))
                .await?;
            let current = maybe_first_row::<NextSeqRow>(current)
                .map(|r| r.next_seq)
                .unwrap_or(0);
            let end = current + block_size;
            let applied = self
                .execute_unpaged(&self.update_stream_lwt, (end, stream_key, current))
                .await?;
            if lwt_applied(&applied) {
                return Ok(SeqBlock {
                    next: current,
                    end,
                });
            }
            if lwt_missing_row(&applied) {
                self.ensure_stream_row(stream_key).await?;
            }
        }
        Err(LogError::Conflict(
            "scylla seq block reservation CAS exhausted retries".into(),
        ))
    }

    async fn allocate_seq_batch(&self, stream_key: &str, count: usize) -> Result<Vec<Seq>> {
        if count == 0 {
            return Ok(vec![]);
        }
        let mut out = Vec::with_capacity(count);
        while out.len() < count {
            let remaining = count - out.len();
            if let Some(mut cached) = self.seq_blocks.get_mut(stream_key) {
                let available = (cached.end - cached.next) as usize;
                if available > 0 {
                    let take = remaining.min(available);
                    for offset in 0..take {
                        out.push(Seq(cached.next + i64::try_from(offset + 1).unwrap_or(0)));
                    }
                    cached.next += i64::try_from(take).unwrap_or(0);
                    continue;
                }
            }
            let block = self
                .reserve_seq_block_lwt(stream_key, SEQ_BLOCK_SIZE)
                .await?;
            let available = (block.end - block.next) as usize;
            let take = remaining.min(available);
            for offset in 0..take {
                out.push(Seq(block.next + i64::try_from(offset + 1).unwrap_or(0)));
            }
            let consumed = block.next + i64::try_from(take).unwrap_or(0);
            if consumed < block.end {
                self.seq_blocks.insert(
                    stream_key.to_string(),
                    SeqBlock {
                        next: consumed,
                        end: block.end,
                    },
                );
            } else {
                self.seq_blocks.remove(stream_key);
            }
        }
        Ok(out)
    }

    async fn register_topic_stream(&self, topic_prefix: &str, stream_key: &str) -> Result<()> {
        self.execute_unpaged(&self.insert_stream_index, (topic_prefix, stream_key))
            .await?;
        Ok(())
    }

    fn topic_prefix(stream: &LogStreamId) -> String {
        format!(
            "{}{}{}{}",
            stream.destination.router_key(),
            continuum_core::types::STORAGE_KEY_SEP,
            stream.topic,
            continuum_core::types::STORAGE_KEY_SEP,
        )
    }
}

#[derive(DeserializeRow)]
struct NextSeqRow {
    next_seq: i64,
}

#[async_trait]
impl LogBackend for ScyllaLogBackend {
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
        let topic_prefix = Self::topic_prefix(&stream);
        let seqs = self
            .allocate_seq_batch(&stream_key, records.len())
            .await?;
        let mut out = Vec::with_capacity(records.len());

        for (rec, seq) in records.iter().zip(seqs) {
            if self.write_event_new(&stream_key, seq, rec).await? {
                out.push(seq);
            } else {
                let existing = self
                    .existing_seq(&stream_key, &rec.event_id)
                    .await?
                    .ok_or_else(|| {
                        LogError::Conflict(
                            "idempotency insert not applied but seq row missing".into(),
                        )
                    })?;
                out.push(existing);
            }
        }

        self.register_topic_stream(&topic_prefix, &stream_key)
            .await?;

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
        let rows = self
            .session
            .execute_unpaged(
                &self.select_events,
                (
                    stream_key.as_str(),
                    after.as_i64(),
                    i32::try_from(limit).unwrap_or(i32::MAX),
                ),
            )
            .await
            .map_err(map_err)?;
        rows_to_events(&stream, rows, stream.key.clone())
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

        let topic_prefix = Self::topic_prefix(&stream);
        let index_rows = self
            .session
            .execute_unpaged(&self.select_stream_keys, (topic_prefix.as_str(),))
            .await
            .map_err(map_err)?;
        let stream_keys: Vec<String> = collect_rows::<StreamKeyRow>(index_rows)
            .into_iter()
            .map(|r| r.stream_key)
            .collect();

        let mut rows = Vec::new();
        for sk in stream_keys {
            let key = sk.strip_prefix(&topic_prefix).and_then(|rest| {
                if rest.is_empty() {
                    None
                } else {
                    Some(rest.to_string())
                }
            });
            let partial = self
                .read_from(
                    LogStreamId::new(
                        stream.destination.clone(),
                        stream.topic.clone(),
                        key,
                    ),
                    after,
                    limit,
                )
                .await?;
            rows.extend(partial);
            if rows.len() >= limit {
                break;
            }
        }
        rows.sort_by_key(|r| r.seq);
        rows.truncate(limit);
        Ok(rows)
    }

    async fn commit_checkpoint(
        &self,
        subscription: &SubscriptionId,
        stream: LogStreamId,
        seq: Seq,
    ) -> Result<()> {
        let stream_key = stream.storage_key();
        if let Some(current) = self.load_checkpoint(subscription, stream.clone()).await? {
            if seq <= current {
                return Ok(());
            }
        }
        self.session
            .execute_unpaged(
                &self.upsert_checkpoint,
                (subscription.0.as_str(), stream_key.as_str(), seq.as_i64()),
            )
            .await
            .map_err(map_err)?;
        Ok(())
    }

    async fn load_checkpoint(
        &self,
        subscription: &SubscriptionId,
        stream: LogStreamId,
    ) -> Result<Option<Seq>> {
        let stream_key = stream.storage_key();
        let rows = self
            .session
            .execute_unpaged(
                &self.select_checkpoint,
                (subscription.0.as_str(), stream_key.as_str()),
            )
            .await
            .map_err(map_err)?;
        Ok(maybe_first_row::<SeqRow>(rows).map(|r| Seq(r.seq)))
    }

    async fn truncate_before(&self, stream: LogStreamId, seq: Seq) -> Result<u64> {
        let stream_key = stream.storage_key();
        let count_rows = self
            .session
            .execute_unpaged(
                &self.count_truncate,
                (stream_key.as_str(), seq.as_i64()),
            )
            .await
            .map_err(map_err)?;
        let removed = maybe_first_row::<CountRow>(count_rows)
            .map(|r| r.cnt)
            .unwrap_or(0);
        self.session
            .execute_unpaged(
                &self.delete_truncate,
                (stream_key.as_str(), seq.as_i64()),
            )
            .await
            .map_err(map_err)?;
        Ok(removed.cast_unsigned())
    }
}

impl fmt::Debug for ScyllaLogBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ScyllaLogBackend")
            .field("keyspace", &self.keyspace)
            .finish_non_exhaustive()
    }
}

fn lwt_applied(result: &scylla::response::query_result::QueryResult) -> bool {
    result
        .clone()
        .into_rows_result()
        .ok()
        .and_then(|rows| rows.maybe_first_row::<AppliedRow>().ok().flatten())
        .map(|r| r.applied)
        .unwrap_or(true)
}

fn lwt_missing_row(result: &scylla::response::query_result::QueryResult) -> bool {
    result
        .clone()
        .into_rows_result()
        .ok()
        .and_then(|rows| rows.maybe_first_row::<AppliedRow>().ok().flatten())
        .is_some_and(|r| !r.applied && r.next_seq.is_none())
}

fn maybe_first_row<R>(result: scylla::response::query_result::QueryResult) -> Option<R>
where
    R: for<'frame> scylla::deserialize::row::DeserializeRow<'frame, 'frame>,
{
    result
        .into_rows_result()
        .ok()
        .and_then(|rows| rows.maybe_first_row::<R>().ok().flatten())
}

fn collect_rows<R>(result: scylla::response::query_result::QueryResult) -> Vec<R>
where
    R: for<'frame> scylla::deserialize::row::DeserializeRow<'frame, 'frame>,
{
    result
        .into_rows_result()
        .ok()
        .and_then(|rows| {
            rows.rows::<R>()
                .ok()
                .map(|iter| iter.filter_map(|r| r.ok()).collect())
        })
        .unwrap_or_default()
}

fn rows_to_events(
    stream: &LogStreamId,
    rows: scylla::response::query_result::QueryResult,
    key: Option<String>,
) -> Result<Vec<EventRecord>> {
    let event_rows = collect_rows::<EventRow>(rows);
    Ok(event_rows
        .into_iter()
        .map(|row| EventRecord {
            destination: stream.destination.clone(),
            event_id: row.event_id,
            topic: stream.topic.clone(),
            key: key.clone(),
            seq: Seq(row.seq),
            ts: DateTime::from_timestamp_millis(row.ts_millis).unwrap_or_else(Utc::now),
            attempt: row.attempt.cast_unsigned(),
            actor_ref: row.actor_ref,
            payload_ciphertext: row.payload_ciphertext,
        })
        .collect())
}
