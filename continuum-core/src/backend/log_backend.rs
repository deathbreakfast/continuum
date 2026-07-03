//! Append-only log storage port.
//!
//! See [`LogBackend`] for the full contract.

use std::fmt::Debug;

use async_trait::async_trait;

use crate::error::Result;
use crate::types::{AppendRecord, EventRecord, LogStreamId, Seq, SubscriptionId};

/// Append-only, sequenced log backend for transport persistence.
///
/// Each [`LogStreamId`] (destination + topic + optional key) has a strictly increasing
/// sequence counter. The host encrypts payloads before [`Self::append`] and decrypts after
/// [`Self::read_from`]; this port stores and returns opaque ciphertext only.
///
/// # Design notes
///
/// - **Partition-scoped sequences** — ordering and dedupe are per stream, not global
/// - **Batched append** — callers should batch records to amortize storage round trips
/// - **At-least-once delivery** — consumers dedupe on [`AppendRecord::event_id`]
/// - **Short-lived log** — call [`Self::truncate_before`] after delivery ack or TTL to reclaim space
/// - **Checkpoints** — monotonic per `(subscription, stream)`; callers may coalesce commits
///
/// # Examples
///
/// ```rust
/// # use continuum_core::{
/// #     AppendRecord, LogBackend, LogBackendKind, LogDestination, LogStreamId, Seq,
/// #     SubscriptionId,
/// # };
/// # use continuum_backend_mem::InMemoryLogBackend;
/// # use uuid::Uuid;
/// # #[tokio::main]
/// # async fn main() -> continuum_core::Result<()> {
/// let backend = InMemoryLogBackend::new();
/// let stream = LogStreamId::new(
///     LogDestination::new("default", LogBackendKind::Memory),
///     "events",
///     None,
/// );
/// let record = AppendRecord::new(Uuid::new_v4(), vec![1, 2, 3]);
/// let seqs = backend.append(stream.clone(), &[record]).await?;
/// let events = backend.read_from(stream.clone(), Seq::ZERO, 10).await?;
/// assert_eq!(events.len(), 1);
/// assert_eq!(events[0].seq, seqs[0]);
///
/// let sub = SubscriptionId("worker-1".into());
/// backend.commit_checkpoint(&sub, stream.clone(), seqs[0]).await?;
/// assert_eq!(backend.load_checkpoint(&sub, stream.clone()).await?, Some(seqs[0]));
///
/// let topic_events = backend
///     .read_from_topic(stream.clone(), None, Seq::ZERO, 10)
///     .await?;
/// assert_eq!(topic_events.len(), 1);
///
/// let removed = backend.truncate_before(stream, seqs[0].next()).await?;
/// assert_eq!(removed, 1);
/// # Ok(())
/// # }
/// ```
// Future (not in v0.1, not rendered): batched tailers, compaction, shared fanout.
#[async_trait]
pub trait LogBackend: Send + Sync + Debug {
    /// Append a batch to one stream. Returns assigned sequences in input order.
    ///
    /// # Contract
    ///
    /// - Empty batch → `Ok(vec![])`
    /// - Idempotent on `event_id`: duplicate returns existing seq without a second row
    /// - Returned sequences strictly increase within the same [`LogStreamId`]
    /// - After a successful ack, records are readable via [`Self::read_from`] after crash
    ///   (backend durability / WAL semantics)
    async fn append(
        &self,
        stream: LogStreamId,
        records: &[AppendRecord],
    ) -> Result<Vec<Seq>>;

    /// Forward read: events with `seq > after`, ascending, at most `limit`.
    ///
    /// # Contract
    ///
    /// - `after = Seq::ZERO` reads from the start
    /// - `limit = 0` → empty vec
    /// - Payload bytes are opaque ciphertext; the host decrypts above the port
    async fn read_from(
        &self,
        stream: LogStreamId,
        after: Seq,
        limit: usize,
    ) -> Result<Vec<EventRecord>>;

    /// Persist durable consumer checkpoint (monotonic per subscription + stream).
    ///
    /// # Contract
    ///
    /// - Checkpoint is keyed by `(subscription, stream)`
    /// - Stored seq is monotonic: commits with `seq <=` existing value are no-ops or coalesce
    /// - Coalescing multiple commits at the caller is allowed
    async fn commit_checkpoint(
        &self,
        subscription: &SubscriptionId,
        stream: LogStreamId,
        seq: Seq,
    ) -> Result<()>;

    /// Load durable consumer checkpoint for subscription + stream.
    ///
    /// # Contract
    ///
    /// - Returns `Ok(None)` when no checkpoint exists yet
    /// - Returned seq is the last committed processed position for that subscription + stream
    async fn load_checkpoint(
        &self,
        subscription: &SubscriptionId,
        stream: LogStreamId,
    ) -> Result<Option<Seq>>;

    /// Forward read across all partition keys for a topic when `topic_key` is `None`.
    ///
    /// When `topic_key` is `Some(k)`, only events with that partition key are returned.
    /// Results are sorted ascending by `seq` across matching keys.
    ///
    /// # Contract
    ///
    /// - Same limit and `after` semantics as [`Self::read_from`]
    /// - Payload bytes are opaque ciphertext
    async fn read_from_topic(
        &self,
        stream: LogStreamId,
        topic_key: Option<&str>,
        after: Seq,
        limit: usize,
    ) -> Result<Vec<EventRecord>>;

    /// Logical truncate: drop entries with `seq < bound`; returns removed count.
    ///
    /// Reclaims space once events are delivered and acked. The transport log is not
    /// long-term retention storage.
    ///
    /// # Contract
    ///
    /// - Only affects the given [`LogStreamId`]
    /// - Truncate floor is monotonic: repeated calls with `bound <=` current floor remove nothing
    /// - Returns the number of records removed by this call
    async fn truncate_before(&self, stream: LogStreamId, seq: Seq) -> Result<u64>;
}
