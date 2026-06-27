//! Contract tests for [`SurrealLocalLogBackend`](continuum_backend_surreal::SurrealLocalLogBackend).

use std::sync::Arc;

use continuum_backend_surreal::SurrealLocalLogBackend;
use continuum_core::backend::LogBackend;
use continuum_core::types::{Seq, SubscriptionId};
use continuum_test_utils::contract;
use continuum_test_utils::fixtures::{sample_record, BackendEnv};
use surrealdb::engine::any::Any;
use surrealdb::Surreal;

const ENV: BackendEnv = BackendEnv::SURREAL;

async fn test_db() -> Arc<Surreal<Any>> {
    let db: Surreal<Any> = Surreal::init();
    db.connect("mem://").await.expect("connect mem");
    db.use_ns("continuum")
        .use_db("test")
        .await
        .expect("ns/db");
    Arc::new(db)
}

async fn backend() -> SurrealLocalLogBackend {
    SurrealLocalLogBackend::new(test_db().await)
        .await
        .expect("backend")
}

#[tokio::test]
async fn s1_create_on_append() {
    contract::append_single(&backend().await, &ENV).await;
}

#[tokio::test]
async fn s2_durable_after_reopen() {
    let db = test_db().await;
    let stream = ENV.stream("t");
    {
        let b = SurrealLocalLogBackend::new(Arc::clone(&db)).await.unwrap();
        b.append(stream.clone(), &[sample_record()]).await.unwrap();
    }
    let b2 = SurrealLocalLogBackend::new(db).await.unwrap();
    let rows = b2.read_from(stream, Seq::ZERO, 10).await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].seq, Seq(1));
}

#[tokio::test]
async fn s3_batch_100() {
    let b = backend().await;
    let stream = ENV.stream("t");
    let recs: Vec<_> = (0..100).map(|_| sample_record()).collect();
    let seqs = b.append(stream, &recs).await.unwrap();
    assert_eq!(seqs.len(), 100);
    assert_eq!(seqs[0], Seq(1));
    assert_eq!(seqs[99], Seq(100));
}

#[tokio::test]
async fn s4_duplicate_event_id() {
    contract::duplicate_event_id(&backend().await, &ENV).await;
}

#[tokio::test]
async fn s5_read_semantics() {
    let b = backend().await;
    contract::read_from_start(&b, &ENV).await;
    contract::read_from_mid(&b, &ENV).await;
    contract::read_limit_zero(&b, &ENV).await;
    contract::read_wrong_stream(&b, &ENV).await;
}

#[tokio::test]
async fn s6_checkpoint_reopen() {
    let db = test_db().await;
    let stream = ENV.stream("t");
    let sub = SubscriptionId::new("sub");
    {
        let b = SurrealLocalLogBackend::new(Arc::clone(&db)).await.unwrap();
        let seqs = b.append(stream.clone(), &[sample_record()]).await.unwrap();
        b.commit_checkpoint(&sub, stream.clone(), seqs[0])
            .await
            .unwrap();
    }
    let b2 = SurrealLocalLogBackend::new(db).await.unwrap();
    assert_eq!(
        b2.load_checkpoint(&sub, stream).await.unwrap(),
        Some(Seq(1))
    );
}

#[tokio::test]
async fn s7_truncate_logical() {
    contract::truncate(&backend().await, &ENV).await;
}

#[tokio::test]
async fn s8_schema_idempotent() {
    let db = test_db().await;
    SurrealLocalLogBackend::new(Arc::clone(&db)).await.unwrap();
    SurrealLocalLogBackend::new(db).await.unwrap();
}

#[tokio::test]
async fn s9_partition_keys_independent() {
    contract::distinct_partition_keys(&backend().await, &ENV).await;
}

#[tokio::test]
async fn s10_independent_destinations() {
    contract::independent_destinations(&backend().await, &ENV).await;
}

#[tokio::test]
async fn read_from_topic_all_keys() {
    contract::read_from_topic_all_keys(&backend().await, &ENV).await;
}

#[tokio::test]
async fn read_from_topic_single_key() {
    contract::read_from_topic_single_key(&backend().await, &ENV).await;
}

#[tokio::test]
async fn read_from_topic_after_and_limit() {
    contract::read_from_topic_after_and_limit(&backend().await, &ENV).await;
}
