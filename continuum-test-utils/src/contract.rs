//! Shared [`LogBackend`] contract assertions.

use continuum_core::backend::LogBackend;
use continuum_core::types::{AppendRecord, LogStreamId, Seq, SubscriptionId};
use continuum_core::validation::MAX_READ_LIMIT;

use crate::fixtures::{sample_record, BackendEnv};

/// Asserts append assigns sequence 1 for a single record.
///
/// # Panics
///
/// Panics if the backend operation fails or assertions do not hold.
pub async fn append_single(b: &dyn LogBackend, env: &BackendEnv) {
    let stream = env.stream("t");
    let seqs = b.append(stream, &[sample_record()]).await.unwrap();
    assert_eq!(seqs[0], Seq(1));
}

/// Asserts append assigns contiguous sequences for a batch.
///
/// # Panics
///
/// Panics if the backend operation fails or assertions do not hold.
pub async fn append_batch(b: &dyn LogBackend, env: &BackendEnv) {
    let stream = env.stream("t");
    let recs = [sample_record(), sample_record(), sample_record()];
    let seqs = b.append(stream, &recs).await.unwrap();
    assert_eq!(seqs, vec![Seq(1), Seq(2), Seq(3)]);
}

/// Asserts append with no records returns an empty sequence list.
///
/// # Panics
///
/// Panics if the backend operation fails or assertions do not hold.
pub async fn append_empty(b: &dyn LogBackend, env: &BackendEnv) {
    let seqs = b.append(env.stream("t"), &[]).await.unwrap();
    assert!(seqs.is_empty());
}

/// Asserts duplicate event ids are idempotent and do not create extra rows.
///
/// # Panics
///
/// Panics if the backend operation fails or assertions do not hold.
pub async fn duplicate_event_id(b: &dyn LogBackend, env: &BackendEnv) {
    let stream = env.stream("t");
    let rec = sample_record();
    let id = rec.event_id;
    let s1 = b.append(stream.clone(), &[rec]).await.unwrap();
    let s2 = b
        .append(stream, &[AppendRecord::new(id, vec![0])])
        .await
        .unwrap();
    assert_eq!(s1[0], s2[0]);
    let rows = b.read_from(env.stream("t"), Seq::ZERO, 10).await.unwrap();
    assert_eq!(rows.len(), 1);
}

/// Asserts read-from returns all records after sequence zero.
///
/// # Panics
///
/// Panics if the backend operation fails or assertions do not hold.
pub async fn read_from_start(b: &dyn LogBackend, env: &BackendEnv) {
    let stream = env.stream("t");
    b.append(stream.clone(), &[sample_record(), sample_record()])
        .await
        .unwrap();
    let rows = b.read_from(stream, Seq::ZERO, 10).await.unwrap();
    assert_eq!(rows.len(), 2);
    assert!(rows[0].seq < rows[1].seq);
}

/// Asserts read-from after the first sequence skips earlier records.
///
/// # Panics
///
/// Panics if the backend operation fails or assertions do not hold.
pub async fn read_from_mid(b: &dyn LogBackend, env: &BackendEnv) {
    let stream = env.stream("t");
    let seqs = b
        .append(stream.clone(), &[sample_record(), sample_record()])
        .await
        .unwrap();
    let rows = b.read_from(stream, seqs[0], 10).await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].seq, seqs[1]);
}

/// Asserts read-from with limit zero returns no rows.
///
/// # Panics
///
/// Panics if the backend operation fails or assertions do not hold.
pub async fn read_limit_zero(b: &dyn LogBackend, env: &BackendEnv) {
    let rows = b.read_from(env.stream("t"), Seq::ZERO, 0).await.unwrap();
    assert!(rows.is_empty());
}

/// Asserts reads from an empty stream return no rows.
///
/// # Panics
///
/// Panics if the backend operation fails or assertions do not hold.
pub async fn read_wrong_stream(b: &dyn LogBackend, env: &BackendEnv) {
    b.append(env.stream("a"), &[sample_record()])
        .await
        .unwrap();
    let rows = b.read_from(env.stream("b"), Seq::ZERO, 10).await.unwrap();
    assert!(rows.is_empty());
}

/// Asserts load-checkpoint returns none when no checkpoint exists.
///
/// # Panics
///
/// Panics if the backend operation fails or assertions do not hold.
pub async fn checkpoint_none(b: &dyn LogBackend, env: &BackendEnv) {
    let sub = SubscriptionId::new("sub");
    let got = b.load_checkpoint(&sub, env.stream("t")).await.unwrap();
    assert!(got.is_none());
}

/// Asserts commit-checkpoint persists and load-checkpoint round-trips the value.
///
/// # Panics
///
/// Panics if the backend operation fails or assertions do not hold.
pub async fn checkpoint_roundtrip(b: &dyn LogBackend, env: &BackendEnv) {
    let sub = SubscriptionId::new("sub");
    let stream = env.stream("t");
    b.commit_checkpoint(&sub, stream.clone(), Seq(5))
        .await
        .unwrap();
    assert_eq!(
        b.load_checkpoint(&sub, stream).await.unwrap(),
        Some(Seq(5))
    );
}

/// Asserts checkpoints only advance monotonically.
///
/// # Panics
///
/// Panics if the backend operation fails or assertions do not hold.
pub async fn checkpoint_monotonic(b: &dyn LogBackend, env: &BackendEnv) {
    let sub = SubscriptionId::new("sub");
    let stream = env.stream("t");
    b.commit_checkpoint(&sub, stream.clone(), Seq(5))
        .await
        .unwrap();
    b.commit_checkpoint(&sub, stream.clone(), Seq(3))
        .await
        .unwrap();
    assert_eq!(
        b.load_checkpoint(&sub, stream).await.unwrap(),
        Some(Seq(5))
    );
}

/// Asserts truncate-before removes records strictly before the given sequence.
///
/// # Panics
///
/// Panics if the backend operation fails or assertions do not hold.
pub async fn truncate(b: &dyn LogBackend, env: &BackendEnv) {
    let stream = env.stream("t");
    let seqs = b
        .append(stream.clone(), &[sample_record(), sample_record()])
        .await
        .unwrap();
    b.truncate_before(stream.clone(), seqs[1]).await.unwrap();
    let rows = b.read_from(stream, Seq::ZERO, 10).await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].seq, seqs[1]);
}

/// Asserts truncate-before with no matching records removes zero rows.
///
/// # Panics
///
/// Panics if the backend operation fails or assertions do not hold.
pub async fn truncate_before_min(b: &dyn LogBackend, env: &BackendEnv) {
    let stream = env.stream("t");
    let removed = b.truncate_before(stream, Seq(1)).await.unwrap();
    assert_eq!(removed, 0);
}

/// Asserts append, checkpoint, truncate, and read work together end-to-end.
///
/// # Panics
///
/// Panics if the backend operation fails or assertions do not hold.
pub async fn e2e_lifecycle(b: &dyn LogBackend, env: &BackendEnv) {
    let sub = SubscriptionId::new("sub");
    let stream = env.stream("t");
    let seqs = b.append(stream.clone(), &[sample_record()]).await.unwrap();
    b.commit_checkpoint(&sub, stream.clone(), seqs[0])
        .await
        .unwrap();
    b.truncate_before(stream.clone(), seqs[0]).await.unwrap();
    let rows = b.read_from(stream.clone(), Seq::ZERO, 10).await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(
        b.load_checkpoint(&sub, stream).await.unwrap(),
        Some(seqs[0])
    );
}

/// Asserts sequence counters are independent per destination.
///
/// # Panics
///
/// Panics if the backend operation fails or assertions do not hold.
pub async fn independent_destinations(b: &dyn LogBackend, env: &BackendEnv) {
    let d1 = env.destination_named(&format!("{}-a", env.logical_dest));
    let d2 = env.destination_named(&format!("{}-b", env.logical_dest));
    let s1 = LogStreamId::new(d1, "topic", Some("k".into()));
    let s2 = LogStreamId::new(d2, "topic", Some("k".into()));
    let a = b.append(s1, &[sample_record()]).await.unwrap();
    let c = b.append(s2, &[sample_record()]).await.unwrap();
    assert_eq!(a[0], Seq(1));
    assert_eq!(c[0], Seq(1));
}

/// Asserts append rejects an empty topic.
///
/// # Panics
///
/// Panics if the backend accepts an empty topic or assertions do not hold.
pub async fn empty_topic_rejected(b: &dyn LogBackend, env: &BackendEnv) {
    let stream = LogStreamId::new(env.destination(), "", None);
    assert!(b.append(stream, &[sample_record()]).await.is_err());
}

/// Asserts read-from rejects limits above the configured maximum.
///
/// # Panics
///
/// Panics if the backend accepts an oversized limit or assertions do not hold.
pub async fn read_limit_validation(b: &dyn LogBackend, env: &BackendEnv) {
    assert!(
        b.read_from(env.stream("t"), Seq::ZERO, MAX_READ_LIMIT + 1)
            .await
            .is_err()
    );
}

/// Asserts partition keys maintain independent sequence counters.
///
/// # Panics
///
/// Panics if the backend operation fails or assertions do not hold.
pub async fn distinct_partition_keys(b: &dyn LogBackend, env: &BackendEnv) {
    let s1 = env.stream_with_key("t", "a");
    let s2 = env.stream_with_key("t", "b");
    let a = b.append(s1.clone(), &[sample_record()]).await.unwrap();
    let c = b.append(s2.clone(), &[sample_record()]).await.unwrap();
    assert_eq!(a[0], Seq(1));
    assert_eq!(c[0], Seq(1));
}

/// Asserts read-from-topic without a key returns records from all partition keys.
///
/// # Panics
///
/// Panics if the backend operation fails or assertions do not hold.
pub async fn read_from_topic_all_keys(b: &dyn LogBackend, env: &BackendEnv) {
    let s_a = env.stream_with_key("t", "a");
    let s_b = env.stream_with_key("t", "b");
    b.append(s_a, &[sample_record()]).await.unwrap();
    b.append(s_b, &[sample_record()]).await.unwrap();
    let topic_stream = env.stream("t");
    let rows = b
        .read_from_topic(topic_stream, None, Seq::ZERO, 10)
        .await
        .unwrap();
    assert_eq!(rows.len(), 2);
    let keys: Vec<_> = rows.iter().map(|r| r.key.as_deref()).collect();
    assert!(keys.contains(&Some("a")));
    assert!(keys.contains(&Some("b")));
}

/// Asserts read-from-topic with a key returns only that partition's records.
///
/// # Panics
///
/// Panics if the backend operation fails or assertions do not hold.
pub async fn read_from_topic_single_key(b: &dyn LogBackend, env: &BackendEnv) {
    let s_a = env.stream_with_key("t", "a");
    let s_b = env.stream_with_key("t", "b");
    b.append(s_a, &[sample_record()]).await.unwrap();
    b.append(s_b, &[sample_record()]).await.unwrap();
    let topic_stream = env.stream("t");
    let rows = b
        .read_from_topic(topic_stream, Some("a"), Seq::ZERO, 10)
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].key.as_deref(), Some("a"));
}

/// Asserts read-from-topic honors after-sequence and limit parameters.
///
/// # Panics
///
/// Panics if the backend operation fails or assertions do not hold.
pub async fn read_from_topic_after_and_limit(b: &dyn LogBackend, env: &BackendEnv) {
    let s_a = env.stream_with_key("t", "a");
    let s_b = env.stream_with_key("t", "b");
    b.append(s_a.clone(), &[sample_record()]).await.unwrap();
    b.append(s_a, &[sample_record()]).await.unwrap();
    b.append(s_b, &[sample_record()]).await.unwrap();
    let topic_stream = env.stream("t");
    let rows = b
        .read_from_topic(topic_stream, None, Seq(1), 1)
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].seq, Seq(2));
    assert_eq!(rows[0].key.as_deref(), Some("a"));
}
