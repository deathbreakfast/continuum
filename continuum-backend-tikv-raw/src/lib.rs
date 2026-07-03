//! Raw `TiKV` [`LogBackend`] (Placement Driver client, no Surreal).
//!
//! Enable via the `tikv-raw` feature on the [`continuum`](https://docs.rs/continuum) facade.
//! See [Getting started](https://docs.rs/continuum/latest/continuum/index.html#getting-started)
//! and the [documentation map](https://docs.rs/continuum/latest/continuum/index.html#documentation-map).

pub mod append_ops;
mod error_map;
mod keys;

use std::fmt;
use std::sync::Arc;

use dashmap::DashMap;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tikv_client::{KvPair, TransactionClient, Value};
use uuid::Uuid;

use continuum_core::backend::LogBackend;
use continuum_core::error::{LogError, Result};
use continuum_core::types::{AppendRecord, EventRecord, LogStreamId, Seq, SubscriptionId};
use continuum_core::validation::{validate_read_limit, validate_topic};

use error_map::map_err;
use keys::{checkpoint_key, event_key, idempotency_key, meta_key, scan_end, topic_stream_key};

const SEQ_BLOCK_SIZE: i64 = 64;

#[derive(Debug, Clone, Copy)]
struct SeqBlock {
    next: i64,
    end: i64,
}

/// Connection settings for [`TikvRawLogBackend`].
#[derive(Debug, Clone, Default)]
pub struct TikvRawLogConfig {
    /// PD endpoint(s), e.g. `127.0.0.1:2379`.
    pub pd_endpoints: Vec<String>,
}

/// TiKV-backed transport log using transactional RawKV-style keys.
pub struct TikvRawLogBackend {
    client: Arc<TransactionClient>,
    seq_blocks: DashMap<String, SeqBlock>,
}

#[derive(Debug, Serialize, Deserialize)]
struct StreamMeta {
    next_seq: i64,
}

#[derive(Debug, Serialize, Deserialize)]
struct StoredEvent {
    event_id: Uuid,
    ts_millis: i64,
    attempt: u32,
    actor_ref: Option<String>,
    payload_ciphertext: Vec<u8>,
}

impl TikvRawLogBackend {
    /// Connect to `TiKV` via PD and return a backend.
    ///
    /// # Errors
    ///
    /// Returns an error if the PD client cannot be created.
    pub async fn connect(config: TikvRawLogConfig) -> Result<Self> {
        let endpoints = if config.pd_endpoints.is_empty() {
            vec!["127.0.0.1:2379".into()]
        } else {
            config.pd_endpoints
        };
        let client = TransactionClient::new(endpoints)
            .await
            .map_err(map_err)?;
        Ok(Self {
            client: Arc::new(client),
            seq_blocks: DashMap::new(),
        })
    }

    /// Wrap an existing `TiKV` transaction client.
    #[must_use]
    pub fn from_client(client: Arc<TransactionClient>) -> Self {
        Self {
            client,
            seq_blocks: DashMap::new(),
        }
    }

    /// Underlying `TiKV` client.
    #[must_use]
    pub const fn client(&self) -> &Arc<TransactionClient> {
        &self.client
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

    fn block_has_seq(&self, stream_key: &str) -> bool {
        self.seq_blocks
            .get(stream_key)
            .is_some_and(|entry| entry.next < entry.end)
    }

    fn take_seq_from_block(&self, stream_key: &str) -> Option<Seq> {
        let mut entry = self.seq_blocks.get_mut(stream_key)?;
        if entry.next >= entry.end {
            return None;
        }
        entry.next += 1;
        Some(Seq(entry.next))
    }

    async fn reserve_seq_block(&self, stream_key: &str) -> Result<()> {
        for attempt in 0..16 {
            let mut txn = self.client.begin_optimistic().await.map_err(map_err)?;
            append_ops::record_round_trip(1);
            let meta_k = meta_key(stream_key);
            let current = txn
                .get(meta_k.clone())
                .await
                .map_err(map_err)?
                .and_then(|v| decode_meta(&v).map(|m| m.next_seq))
                .unwrap_or(0);
            let end = current + SEQ_BLOCK_SIZE;
            txn.put(meta_k, encode_meta(&StreamMeta { next_seq: end }))
                .await
                .map_err(map_err)?;
            match txn.commit().await.map_err(map_err) {
                Ok(_) => {
                    drop(txn);
                    self.seq_blocks.insert(
                        stream_key.to_string(),
                        SeqBlock {
                            next: current,
                            end,
                        },
                    );
                    return Ok(());
                }
                Err(e) if attempt + 1 < 16 => {
                    let _ = txn.rollback().await;
                    drop(txn);
                    let msg = e.to_string();
                    if msg.contains("write conflict") || msg.contains("Conflict") {
                        continue;
                    }
                    return Err(e);
                }
                Err(e) => {
                    let _ = txn.rollback().await;
                    drop(txn);
                    return Err(e);
                }
            }
        }
        Err(LogError::Conflict(
            "tikv seq block reservation exhausted retries".into(),
        ))
    }

    async fn allocate_seq(&self, stream_key: &str) -> Result<Seq> {
        if !self.block_has_seq(stream_key) {
            self.reserve_seq_block(stream_key).await?;
        }
        self.take_seq_from_block(stream_key).ok_or_else(|| {
            LogError::Backend("tikv seq block empty after reservation".into())
        })
    }
}

#[async_trait]
impl LogBackend for TikvRawLogBackend {
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
        let mut out = vec![Seq(0); records.len()];

        // Pass 1: idempotency read txn.
        {
            let mut txn = self.client.begin_optimistic().await.map_err(map_err)?;
            append_ops::record_round_trip(1);
            for (idx, rec) in records.iter().enumerate() {
                let id_key = idempotency_key(&stream_key, &rec.event_id);
                if let Some(existing) = txn.get(id_key).await.map_err(map_err)? {
                    out[idx] = decode_seq(&existing).map_err(|()| {
                        LogError::Backend("invalid idempotency seq bytes".into())
                    })?;
                }
            }
            txn.commit().await.map_err(map_err)?;
            drop(txn);
        }

        let new_records: Vec<(usize, &AppendRecord)> = records
            .iter()
            .enumerate()
            .filter(|(idx, _)| out[*idx] == Seq(0))
            .collect();

        if new_records.is_empty() {
            return Ok(out);
        }

        let mut seqs = Vec::with_capacity(new_records.len());
        for _ in &new_records {
            seqs.push(self.allocate_seq(&stream_key).await?);
        }

        // Pass 2: write txn (no meta CAS — seq blocks reserved out-of-band).
        for attempt in 0..16 {
            let mut txn = self.client.begin_optimistic().await.map_err(map_err)?;
            append_ops::record_round_trip(1);

            let attempt_result: Result<()> = async {
                for ((idx, rec), seq) in new_records.iter().zip(&seqs) {
                    let id_key = idempotency_key(&stream_key, &rec.event_id);
                    if txn.get(id_key.clone()).await.map_err(map_err)?.is_some() {
                        return Err(LogError::Conflict(
                            "tikv idempotency race during write".into(),
                        ));
                    }
                    out[*idx] = *seq;
                    let stored = StoredEvent {
                        event_id: rec.event_id,
                        ts_millis: rec.ts.timestamp_millis(),
                        attempt: rec.attempt,
                        actor_ref: rec.actor_ref.clone(),
                        payload_ciphertext: rec.payload_ciphertext.clone(),
                    };
                    txn.put(event_key(&stream_key, *seq), encode_event(&stored))
                        .await
                        .map_err(map_err)?;
                    txn.put(id_key, encode_seq(*seq)).await.map_err(map_err)?;
                }
                txn.put(
                    topic_stream_key(&topic_prefix, &stream_key),
                    Value::from([] as [u8; 0]),
                )
                .await
                .map_err(map_err)?;
                txn.commit().await.map_err(map_err)?;
                Ok(())
            }
            .await;

            match attempt_result {
                Ok(()) => {
                    drop(txn);
                    return Ok(out);
                }
                Err(LogError::Conflict(_)) if attempt + 1 < 16 => {
                    let _ = txn.rollback().await;
                    drop(txn);
                }
                Err(LogError::Backend(msg))
                    if attempt + 1 < 16
                        && (msg.contains("write conflict") || msg.contains("Conflict")) =>
                {
                    let _ = txn.rollback().await;
                    drop(txn);
                }
                Err(e) => {
                    let _ = txn.rollback().await;
                    drop(txn);
                    return Err(e);
                }
            }
        }

        Err(LogError::Conflict(
            "tikv append write exhausted retries".into(),
        ))
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
        let start = event_key(&stream_key, Seq(after.as_i64() + 1));
        let end = scan_end(&keys::event_prefix(&stream_key));
        let pairs: Vec<KvPair> = {
            let mut txn = self.client.begin_optimistic().await.map_err(map_err)?;
            let pairs = txn
                .scan((start, end), u32::try_from(limit).unwrap_or(u32::MAX))
                .await
                .map_err(map_err)?
                .collect();
            txn.commit().await.map_err(map_err)?;
            drop(txn);
            pairs
        };

        let mut out = Vec::new();
        for pair in pairs {
            let key_bytes: Vec<u8> = pair.0.clone().into();
            let seq = keys::seq_from_event_key(&key_bytes)?;
            if let Ok(stored) = decode_event(&pair.1) {
                out.push(EventRecord {
                    destination: stream.destination.clone(),
                    event_id: stored.event_id,
                    topic: stream.topic.clone(),
                    key: stream.key.clone(),
                    seq,
                    ts: DateTime::from_timestamp_millis(stored.ts_millis)
                        .unwrap_or_else(Utc::now),
                    attempt: stored.attempt,
                    actor_ref: stored.actor_ref,
                    payload_ciphertext: stored.payload_ciphertext,
                });
            }
        }
        out.sort_by_key(|r| r.seq);
        Ok(out)
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
        let start = topic_stream_key(&topic_prefix, "");
        let end = scan_end(&keys::topic_index_prefix(&topic_prefix));
        let index_pairs: Vec<KvPair> = {
            let mut txn = self.client.begin_optimistic().await.map_err(map_err)?;
            let pairs = txn
                .scan((start, end), 10_000)
                .await
                .map_err(map_err)?
                .collect();
            txn.commit().await.map_err(map_err)?;
            drop(txn);
            pairs
        };

        let mut rows = Vec::new();
        for pair in index_pairs {
            let key_bytes: Vec<u8> = pair.0.clone().into();
            let stream_key = keys::stream_key_from_topic_index(&key_bytes, &topic_prefix)?;
            let key_suffix = stream_key.strip_prefix(&topic_prefix).and_then(|rest| {
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
                        key_suffix,
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
        if let Some(current) = self.load_checkpoint(subscription, stream.clone()).await? {
            if seq <= current {
                return Ok(());
            }
        }
        let key = checkpoint_key(subscription, &stream.storage_key());
        let mut txn = self.client.begin_optimistic().await.map_err(map_err)?;
        txn.put(key, encode_seq(seq)).await.map_err(map_err)?;
        txn.commit().await.map_err(map_err)?;
        drop(txn);
        Ok(())
    }

    async fn load_checkpoint(
        &self,
        subscription: &SubscriptionId,
        stream: LogStreamId,
    ) -> Result<Option<Seq>> {
        let key = checkpoint_key(subscription, &stream.storage_key());
        let mut txn = self.client.begin_optimistic().await.map_err(map_err)?;
        let value = txn.get(key).await.map_err(map_err)?;
        txn.commit().await.map_err(map_err)?;
        drop(txn);
        Ok(value.and_then(|v| decode_seq(&v).ok()))
    }

    async fn truncate_before(&self, stream: LogStreamId, seq: Seq) -> Result<u64> {
        let stream_key = stream.storage_key();
        let start = event_key(&stream_key, Seq(1));
        let end = event_key(&stream_key, seq);
        let mut txn = self.client.begin_optimistic().await.map_err(map_err)?;
        let pairs: Vec<KvPair> = txn
            .scan((start, end), 10_000)
            .await
            .map_err(map_err)?
            .collect();
        let removed = u64::try_from(pairs.len()).unwrap_or(0);
        for pair in pairs {
            txn.delete(pair.0).await.map_err(map_err)?;
        }
        txn.commit().await.map_err(map_err)?;
        drop(txn);
        Ok(removed)
    }
}

impl fmt::Debug for TikvRawLogBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TikvRawLogBackend").finish_non_exhaustive()
    }
}

fn encode_meta(meta: &StreamMeta) -> Value {
    Value::from(bincode::serialize(meta).unwrap_or_default())
}

fn decode_meta(value: &Value) -> Option<StreamMeta> {
    bincode::deserialize(value.as_ref()).ok()
}

fn encode_event(event: &StoredEvent) -> Value {
    Value::from(bincode::serialize(event).unwrap_or_default())
}

fn decode_event(value: &Value) -> std::result::Result<StoredEvent, bincode::Error> {
    bincode::deserialize(value.as_ref())
}

fn encode_seq(seq: Seq) -> Value {
    Value::from(seq.as_i64().to_le_bytes().to_vec())
}

fn decode_seq(value: &Value) -> std::result::Result<Seq, ()> {
    let bytes: &[u8] = value.as_ref();
    if bytes.len() != 8 {
        return Err(());
    }
    let mut arr = [0u8; 8];
    arr.copy_from_slice(bytes);
    Ok(Seq(i64::from_le_bytes(arr)))
}
