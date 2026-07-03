//! `ScyllaDB` [`LogBackend`] for the continuum transport log.
//!
//! Enable via the `scylla` feature on the [`continuum`](https://docs.rs/continuum) facade.
//! See [Getting started](https://docs.rs/continuum/latest/continuum/index.html#getting-started)
//! and the [documentation map](https://docs.rs/continuum/latest/continuum/index.html#documentation-map).

mod append_ops;
mod config;
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
use scylla::statement::Consistency;
use scylla::DeserializeRow;
use uuid::Uuid;

use continuum_core::backend::LogBackend;
use continuum_core::error::{LogError, Result};
use continuum_core::types::{AppendRecord, EventRecord, LogStreamId, Seq, SubscriptionId};
use continuum_core::validation::{validate_read_limit, validate_topic};

use error_map::map_err;

pub use config::{consistency_from_str, IdempotencyMode, IdempotencyPolicy};

/// Snapshot append round-trip counters when `CONTINUUM_APPEND_DEBUG_OPS` is enabled.
#[must_use]
pub fn append_debug_snapshot() -> (u64, u64) {
    append_ops::snapshot()
}

/// Reset append round-trip counters (tests / benchmarks).
pub fn append_debug_reset() {
    append_ops::reset();
}

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
    /// Idempotency policy (default: exactly-once via lightweight transactions).
    ///
    /// See [`IdempotencyPolicy`] and [`IdempotencyMode`]. Disabling LWT trades
    /// exactly-once for higher throughput (at-least-once delivery).
    pub idempotency: IdempotencyPolicy,
    /// When `true` (default), skip repeat `stream_index` writes after the first
    /// sighting of each topic+stream pair in this process.
    pub topic_index_cache: bool,
    /// Optional write consistency override on event and index inserts.
    ///
    /// When `None`, the driver default applies.
    pub write_consistency: Option<Consistency>,
    /// Keyspace replication factor for schema bootstrap.
    pub replication_factor: u32,
    /// Sequence numbers reserved per stream per lightweight-transaction block.
    pub seq_block_size: i64,
}

impl Default for ScyllaLogConfig {
    fn default() -> Self {
        Self {
            contact_points: vec!["127.0.0.1:9042".into()],
            keyspace: "continuum".into(),
            datacenter: None,
            username: None,
            password: None,
            idempotency: IdempotencyPolicy::default(),
            topic_index_cache: true,
            write_consistency: None,
            replication_factor: 1,
            seq_block_size: 64,
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
    stream_index_seen: DashMap<String, ()>,
    idempotency: IdempotencyPolicy,
    topic_index_cache: bool,
    seq_block_size: i64,
}

#[derive(DeserializeRow)]
struct SeqRow {
    seq: i64,
}

#[derive(DeserializeRow)]
struct CountRow {
    cnt: i64,
}

/// LWT result when only `[applied]` is present.
#[derive(DeserializeRow)]
struct AppliedFlag {
    #[scylla(rename = "[applied]")]
    applied: bool,
}

/// Stream seq CAS row (`[applied]`, `next_seq`).
///
/// Scylla may include `next_seq` as null on success; the driver requires every
/// result column on the struct.
#[derive(DeserializeRow)]
struct StreamLwtRow {
    #[scylla(rename = "[applied]")]
    applied: bool,
    next_seq: Option<i64>,
}

/// Event-id LWT row (`[applied]`, `stream_key`, `event_id`, `seq`).
///
/// On success Scylla returns nulls for the non-applied columns; on conflict it
/// returns the existing values. All non-flag fields are therefore optional.
#[derive(DeserializeRow)]
#[allow(dead_code)]
struct EventIdLwtRow {
    #[scylla(rename = "[applied]")]
    applied: bool,
    stream_key: Option<String>,
    event_id: Option<Uuid>,
    seq: Option<i64>,
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
    ///
    /// # Errors
    ///
    /// Returns an error if the session cannot be established or schema bootstrap fails.
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
        Self::from_session(Arc::new(session), &config).await
    }

    /// Wrap an existing session (schema bootstrap runs).
    ///
    /// # Errors
    ///
    /// Returns an error if schema bootstrap or statement preparation fails.
    pub async fn from_session(session: Arc<Session>, config: &ScyllaLogConfig) -> Result<Self> {
        schema::ensure_schema(&session, &config.keyspace, config.replication_factor).await?;
        let ks = config.keyspace.clone();
        let q = |sql: &str| sql.replace("continuum.", &format!("{ks}."));

        let mut insert_event = session
            .prepare(q(
                "INSERT INTO continuum.continuum_event (stream_key, seq, event_id, ts_millis, attempt, actor_ref, payload_ciphertext) VALUES (?, ?, ?, ?, ?, ?, ?)",
            ))
            .await
            .map_err(map_err)?;
        let mut insert_stream_index = session
            .prepare(q(
                "INSERT INTO continuum.continuum_stream_index (topic_prefix, stream_key) VALUES (?, ?)",
            ))
            .await
            .map_err(map_err)?;
        if let Some(c) = config.write_consistency {
            insert_event.set_consistency(c);
            insert_stream_index.set_consistency(c);
        }

        let backend = Self {
            session: Arc::clone(&session),
            keyspace: ks.clone(),
            select_event_id: session
                .prepare(q(
                    "SELECT seq FROM continuum.continuum_event_id WHERE stream_key = ? AND event_id = ?",
                ))
                .await
                .map_err(map_err)?,
            insert_event,
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
            insert_stream_index,
            select_stream_keys: session
                .prepare(q(
                    "SELECT stream_key FROM continuum.continuum_stream_index WHERE topic_prefix = ?",
                ))
                .await
                .map_err(map_err)?,
            seq_blocks: DashMap::new(),
            stream_index_seen: DashMap::new(),
            idempotency: config.idempotency.clone(),
            topic_index_cache: config.topic_index_cache,
            seq_block_size: config.seq_block_size.max(1),
        };
        Ok(backend)
    }

    /// Underlying driver session.
    #[must_use]
    pub const fn session(&self) -> &Arc<Session> {
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

    async fn insert_event_row(
        &self,
        stream_key: &str,
        seq: Seq,
        rec: &AppendRecord,
    ) -> Result<()> {
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
        Ok(())
    }

    async fn reserve_event_id_lwt(
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
        Ok(lwt_applied(&id_result))
    }

    async fn write_event_new(
        &self,
        stream_key: &str,
        seq: Seq,
        rec: &AppendRecord,
        idempotency: IdempotencyMode,
    ) -> Result<bool> {
        match idempotency {
            IdempotencyMode::Lwt => {
                if !self.reserve_event_id_lwt(stream_key, seq, rec).await? {
                    return Ok(false);
                }
            }
            IdempotencyMode::None => {}
        }
        self.insert_event_row(stream_key, seq, rec).await?;
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
                .map_or(0, |r| r.next_seq);
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
                let available =
                    usize::try_from(cached.end.saturating_sub(cached.next)).unwrap_or(0);
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
                .reserve_seq_block_lwt(stream_key, self.seq_block_size)
                .await?;
            let available = usize::try_from(block.end.saturating_sub(block.next)).unwrap_or(0);
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
        if self.topic_index_cache {
            let key = config::stream_index_cache_key(topic_prefix, stream_key);
            if self.stream_index_seen.contains_key(&key) {
                return Ok(());
            }
            self.execute_unpaged(&self.insert_stream_index, (topic_prefix, stream_key))
                .await?;
            self.stream_index_seen.insert(key, ());
            return Ok(());
        }
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
        let idempotency = self.idempotency.mode_for(&stream.topic);
        let seqs = self
            .allocate_seq_batch(&stream_key, records.len())
            .await?;
        let mut out = Vec::with_capacity(records.len());

        for (rec, seq) in records.iter().zip(seqs) {
            if self
                .write_event_new(&stream_key, seq, rec, idempotency)
                .await?
            {
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

        if !self.topic_index_cache
            || !self.stream_index_seen.contains_key(&config::stream_index_cache_key(
                &topic_prefix,
                &stream_key,
            ))
        {
            self.register_topic_stream(&topic_prefix, &stream_key)
                .await?;
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
        Ok(rows_to_events(&stream, rows, stream.key.as_deref()))
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
            .map_or(0, |r| r.cnt);
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

fn try_lwt_row<R>(result: &scylla::response::query_result::QueryResult) -> Option<R>
where
    R: for<'frame> scylla::deserialize::row::DeserializeRow<'frame, 'frame>,
{
    result
        .clone()
        .into_rows_result()
        .ok()
        .and_then(|rows| rows.maybe_first_row::<R>().ok().flatten())
}

/// Read `[applied]` from an LWT result.
///
/// The scylla driver type-checks that **every result column** maps to a struct
/// field, so the row shape depends on the statement:
/// - stream CAS: `[applied]`, `next_seq`
/// - event-id miss: `[applied]`, `stream_key`, `event_id`, `seq`
/// - successful insert: `[applied]` only
fn lwt_applied(result: &scylla::response::query_result::QueryResult) -> bool {
    if let Some(r) = try_lwt_row::<StreamLwtRow>(result) {
        return r.applied;
    }
    if let Some(r) = try_lwt_row::<EventIdLwtRow>(result) {
        return r.applied;
    }
    try_lwt_row::<AppliedFlag>(result).is_some_and(|r| r.applied)
}

fn lwt_missing_row(result: &scylla::response::query_result::QueryResult) -> bool {
    if lwt_applied(result) {
        return false;
    }
    // Not applied: conflict includes existing `next_seq`; missing row has null/absent.
    try_lwt_row::<StreamLwtRow>(result).is_none_or(|r| r.next_seq.is_none())
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
                .map(|iter| iter.filter_map(std::result::Result::ok).collect())
        })
        .unwrap_or_default()
}

fn rows_to_events(
    stream: &LogStreamId,
    rows: scylla::response::query_result::QueryResult,
    key: Option<&str>,
) -> Vec<EventRecord> {
    let event_rows = collect_rows::<EventRow>(rows);
    event_rows
        .into_iter()
        .map(|row| EventRecord {
            destination: stream.destination.clone(),
            event_id: row.event_id,
            topic: stream.topic.clone(),
            key: key.map(str::to_owned),
            seq: Seq(row.seq),
            ts: DateTime::from_timestamp_millis(row.ts_millis).unwrap_or_else(Utc::now),
            attempt: row.attempt.cast_unsigned(),
            actor_ref: row.actor_ref,
            payload_ciphertext: row.payload_ciphertext,
        })
        .collect()
}
