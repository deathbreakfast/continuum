//! In-memory [`LogBackend`] for tests and local dev.

use std::collections::HashMap;
use std::sync::RwLock;

use async_trait::async_trait;

use continuum_core::backend::LogBackend;
use continuum_core::error::{LogError, Result};
use continuum_core::types::{
    AppendRecord, CheckpointKey, EventRecord, LogStreamId, Seq, SubscriptionId,
};
use continuum_core::validation::{validate_read_limit, validate_topic};

#[derive(Debug, Default)]
struct StreamState {
    next_seq: i64,
    records: Vec<EventRecord>,
    event_id_index: HashMap<uuid::Uuid, Seq>,
}

#[derive(Debug, Default)]
struct Inner {
    streams: HashMap<String, StreamState>,
    checkpoints: HashMap<String, Seq>,
    truncate_floor: HashMap<String, Seq>,
}

/// Process-local log backend (not durable across restarts).
///
/// # Examples
///
/// ```rust
/// use continuum_backend_mem::InMemoryLogBackend;
/// use continuum_core::{AppendRecord, LogBackend, LogBackendKind, LogDestination, LogStreamId, Seq};
/// use uuid::Uuid;
///
/// # #[tokio::main]
/// # async fn main() -> continuum_core::Result<()> {
/// let backend = InMemoryLogBackend::new();
/// let stream = LogStreamId::new(
///     LogDestination::new("default", LogBackendKind::Memory),
///     "events",
///     None,
/// );
/// let seqs = backend
///     .append(stream.clone(), &[AppendRecord::new(Uuid::new_v4(), vec![1])])
///     .await?;
/// assert_eq!(seqs.len(), 1);
/// assert_eq!(backend.read_from(stream, Seq::ZERO, 10).await?.len(), 1);
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Default)]
pub struct InMemoryLogBackend {
    inner: RwLock<Inner>,
}

impl InMemoryLogBackend {
    /// New empty backend.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use continuum_backend_mem::InMemoryLogBackend;
    ///
    /// let backend = InMemoryLogBackend::new();
    /// let _ = backend;
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn stream_key(stream: &LogStreamId) -> String {
        stream.storage_key()
    }
}

#[async_trait]
impl LogBackend for InMemoryLogBackend {
    async fn append(
        &self,
        stream: LogStreamId,
        records: &[AppendRecord],
    ) -> Result<Vec<Seq>> {
        if records.is_empty() {
            return Ok(vec![]);
        }
        validate_topic(&stream.topic)?;

        let key = Self::stream_key(&stream);
        let mut inner = self
            .inner
            .write()
            .map_err(|_| LogError::Internal("memory backend lock poisoned".into()))?;
        let state = inner.streams.entry(key).or_default();
        let mut out = Vec::with_capacity(records.len());

        for rec in records {
            if let Some(seq) = state.event_id_index.get(&rec.event_id) {
                out.push(*seq);
                continue;
            }
            state.next_seq += 1;
            let seq = Seq(state.next_seq);
            state.event_id_index.insert(rec.event_id, seq);
            state.records.push(EventRecord {
                destination: stream.destination.clone(),
                event_id: rec.event_id,
                topic: stream.topic.clone(),
                key: stream.key.clone(),
                seq,
                ts: rec.ts,
                attempt: rec.attempt,
                actor_ref: rec.actor_ref.clone(),
                payload_ciphertext: rec.payload_ciphertext.clone(),
            });
            out.push(seq);
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

        let key = Self::stream_key(&stream);
        let inner = self
            .inner
            .read()
            .map_err(|_| LogError::Internal("memory backend lock poisoned".into()))?;

        let Some(state) = inner.streams.get(&key) else {
            return Ok(vec![]);
        };

        let mut rows: Vec<_> = state
            .records
            .iter()
            .filter(|r| r.seq > after)
            .cloned()
            .collect();
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
        let ck = CheckpointKey::new(subscription, stream.storage_key());
        let mut inner = self
            .inner
            .write()
            .map_err(|_| LogError::Internal("memory backend lock poisoned".into()))?;
        inner
            .checkpoints
            .entry(ck.wire_key())
            .and_modify(|existing| {
                if seq > *existing {
                    *existing = seq;
                }
            })
            .or_insert(seq);
        Ok(())
    }

    async fn load_checkpoint(
        &self,
        subscription: &SubscriptionId,
        stream: LogStreamId,
    ) -> Result<Option<Seq>> {
        let ck = CheckpointKey::new(subscription, stream.storage_key());
        let inner = self
            .inner
            .read()
            .map_err(|_| LogError::Internal("memory backend lock poisoned".into()))?;
        Ok(inner.checkpoints.get(&ck.wire_key()).copied())
    }

    async fn truncate_before(&self, stream: LogStreamId, seq: Seq) -> Result<u64> {
        let key = Self::stream_key(&stream);
        let mut inner = self
            .inner
            .write()
            .map_err(|_| LogError::Internal("memory backend lock poisoned".into()))?;

        let current_floor = inner
            .truncate_floor
            .get(&key)
            .copied()
            .unwrap_or(Seq::ZERO);
        if seq <= current_floor {
            return Ok(0);
        }

        let before = inner
            .streams
            .get(&key)
            .map_or(0, |s| s.records.len() as u64);

        if let Some(state) = inner.streams.get_mut(&key) {
            state.records.retain(|r| r.seq >= seq);
        }

        inner.truncate_floor.insert(key.clone(), seq);
        let after = inner
            .streams
            .get(&key)
            .map_or(0, |s| s.records.len() as u64);
        Ok(before.saturating_sub(after))
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

        let destination = stream.destination.clone();
        let topic = stream.topic.clone();
        let topic_prefix =
            LogStreamId::new(destination.clone(), topic.clone(), None).storage_key();

        let inner = self
            .inner
            .read()
            .map_err(|_| LogError::Internal("memory backend lock poisoned".into()))?;

        let mut rows: Vec<EventRecord> = inner
            .streams
            .values()
            .flat_map(|state| state.records.iter())
            .filter(|r| {
                r.destination == destination
                    && r.topic == topic
                    && r.seq > after
                    && match topic_key {
                        None => true,
                        Some(k) => r.key.as_deref() == Some(k),
                    }
                    && format!(
                        "{}{}{}{}",
                        r.destination.router_key(),
                        continuum_core::types::STORAGE_KEY_SEP,
                        r.topic,
                        continuum_core::types::STORAGE_KEY_SEP,
                    )
                    .starts_with(&topic_prefix)
            })
            .cloned()
            .collect();
        rows.sort_by_key(|r| r.seq);
        rows.truncate(limit);
        Ok(rows)
    }
}
