//! `SQLite` contract tests.

use continuum_backend_sqlite::SqliteLogBackend;
use continuum_core::backend::LogBackend;
use continuum_core::types::{Seq, SubscriptionId};
use continuum_test_utils::contract;
use continuum_test_utils::fixtures::{sample_record, BackendEnv};
use tempfile::TempDir;

const ENV: BackendEnv = BackendEnv::SQLITE;

async fn backend() -> (SqliteLogBackend, TempDir) {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("test.db");
    let backend = SqliteLogBackend::new(&path).await.expect("backend");
    (backend, dir)
}

#[tokio::test]
async fn s1_create_on_append() {
    let (b, _dir) = backend().await;
    contract::append_single(&b, &ENV).await;
}

#[tokio::test]
async fn s2_durable_after_reopen() {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("test.db");
    let stream = ENV.stream("t");
    {
        let b = SqliteLogBackend::new(&path).await.unwrap();
        b.append(stream.clone(), &[sample_record()]).await.unwrap();
    }
    let b2 = SqliteLogBackend::new(&path).await.unwrap();
    let rows = b2.read_from(stream, Seq::ZERO, 10).await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].seq, Seq(1));
}

#[tokio::test]
async fn s3_batch_100() {
    let (b, _dir) = backend().await;
    let stream = ENV.stream("t");
    let recs: Vec<_> = (0..100).map(|_| sample_record()).collect();
    let seqs = b.append(stream, &recs).await.unwrap();
    assert_eq!(seqs.len(), 100);
    assert_eq!(seqs[0], Seq(1));
    assert_eq!(seqs[99], Seq(100));
}

#[tokio::test]
async fn s4_duplicate_event_id() {
    let (b, _dir) = backend().await;
    contract::duplicate_event_id(&b, &ENV).await;
}

#[tokio::test]
async fn s5_read_semantics() {
    let (b, _dir) = backend().await;
    contract::read_from_start(&b, &ENV).await;
    contract::read_from_mid(&b, &ENV).await;
    contract::read_limit_zero(&b, &ENV).await;
    contract::read_wrong_stream(&b, &ENV).await;
}

#[tokio::test]
async fn s6_checkpoint_reopen() {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("test.db");
    let stream = ENV.stream("t");
    let sub = SubscriptionId::new("sub");
    {
        let b = SqliteLogBackend::new(&path).await.unwrap();
        let seqs = b.append(stream.clone(), &[sample_record()]).await.unwrap();
        b.commit_checkpoint(&sub, stream.clone(), seqs[0])
            .await
            .unwrap();
    }
    let b2 = SqliteLogBackend::new(&path).await.unwrap();
    assert_eq!(
        b2.load_checkpoint(&sub, stream).await.unwrap(),
        Some(Seq(1))
    );
}

#[tokio::test]
async fn s7_truncate_logical() {
    let (b, _dir) = backend().await;
    contract::truncate(&b, &ENV).await;
}

#[tokio::test]
async fn s8_schema_idempotent() {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("test.db");
    SqliteLogBackend::new(&path).await.unwrap();
    SqliteLogBackend::new(&path).await.unwrap();
}

#[tokio::test]
async fn s9_partition_keys_independent() {
    let (b, _dir) = backend().await;
    contract::distinct_partition_keys(&b, &ENV).await;
}

#[tokio::test]
async fn s10_independent_destinations() {
    let (b, _dir) = backend().await;
    contract::independent_destinations(&b, &ENV).await;
}

#[tokio::test]
async fn read_from_topic_all_keys() {
    let (b, _dir) = backend().await;
    contract::read_from_topic_all_keys(&b, &ENV).await;
}

#[tokio::test]
async fn read_from_topic_single_key() {
    let (b, _dir) = backend().await;
    contract::read_from_topic_single_key(&b, &ENV).await;
}

#[tokio::test]
async fn read_from_topic_after_and_limit() {
    let (b, _dir) = backend().await;
    contract::read_from_topic_after_and_limit(&b, &ENV).await;
}

#[tokio::test]
async fn empty_topic_rejected() {
    let (b, _dir) = backend().await;
    contract::empty_topic_rejected(&b, &ENV).await;
}

#[tokio::test]
async fn read_limit_validation() {
    let (b, _dir) = backend().await;
    contract::read_limit_validation(&b, &ENV).await;
}

#[tokio::test]
async fn append_batch() {
    let (b, _dir) = backend().await;
    contract::append_batch(&b, &ENV).await;
}

#[tokio::test]
async fn append_empty() {
    let (b, _dir) = backend().await;
    contract::append_empty(&b, &ENV).await;
}

#[tokio::test]
async fn checkpoint_none() {
    let (b, _dir) = backend().await;
    contract::checkpoint_none(&b, &ENV).await;
}

#[tokio::test]
async fn checkpoint_roundtrip() {
    let (b, _dir) = backend().await;
    contract::checkpoint_roundtrip(&b, &ENV).await;
}

#[tokio::test]
async fn checkpoint_monotonic() {
    let (b, _dir) = backend().await;
    contract::checkpoint_monotonic(&b, &ENV).await;
}

#[tokio::test]
async fn truncate_before_min() {
    let (b, _dir) = backend().await;
    contract::truncate_before_min(&b, &ENV).await;
}

#[tokio::test]
async fn e2e_lifecycle() {
    let (b, _dir) = backend().await;
    contract::e2e_lifecycle(&b, &ENV).await;
}
