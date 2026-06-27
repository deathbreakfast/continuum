//! Contract tests for [`InMemoryLogBackend`](continuum_backend_mem::InMemoryLogBackend).

use continuum_backend_mem::InMemoryLogBackend;
use continuum_test_utils::contract;
use continuum_test_utils::fixtures::BackendEnv;

const ENV: BackendEnv = BackendEnv::MEMORY;

fn backend() -> InMemoryLogBackend {
    InMemoryLogBackend::new()
}

#[tokio::test]
async fn m1_append_single() {
    contract::append_single(&backend(), &ENV).await;
}

#[tokio::test]
async fn m2_append_batch() {
    contract::append_batch(&backend(), &ENV).await;
}

#[tokio::test]
async fn m3_append_empty() {
    contract::append_empty(&backend(), &ENV).await;
}

#[tokio::test]
async fn m4_duplicate_event_id() {
    contract::duplicate_event_id(&backend(), &ENV).await;
}

#[tokio::test]
async fn m5_read_from_start() {
    contract::read_from_start(&backend(), &ENV).await;
}

#[tokio::test]
async fn m6_read_from_mid() {
    contract::read_from_mid(&backend(), &ENV).await;
}

#[tokio::test]
async fn m7_read_limit_zero() {
    contract::read_limit_zero(&backend(), &ENV).await;
}

#[tokio::test]
async fn m8_read_wrong_stream() {
    contract::read_wrong_stream(&backend(), &ENV).await;
}

#[tokio::test]
async fn m9_checkpoint_none() {
    contract::checkpoint_none(&backend(), &ENV).await;
}

#[tokio::test]
async fn m10_checkpoint_roundtrip() {
    contract::checkpoint_roundtrip(&backend(), &ENV).await;
}

#[tokio::test]
async fn m11_checkpoint_monotonic() {
    contract::checkpoint_monotonic(&backend(), &ENV).await;
}

#[tokio::test]
async fn m12_truncate() {
    contract::truncate(&backend(), &ENV).await;
}

#[tokio::test]
async fn m13_truncate_before_min() {
    contract::truncate_before_min(&backend(), &ENV).await;
}

#[tokio::test]
async fn m14_e2e_lifecycle() {
    contract::e2e_lifecycle(&backend(), &ENV).await;
}

#[tokio::test]
async fn m15_independent_destinations() {
    contract::independent_destinations(&backend(), &ENV).await;
}

#[tokio::test]
async fn empty_topic_rejected() {
    contract::empty_topic_rejected(&backend(), &ENV).await;
}

#[tokio::test]
async fn read_limit_validation() {
    contract::read_limit_validation(&backend(), &ENV).await;
}

#[tokio::test]
async fn distinct_partition_keys() {
    contract::distinct_partition_keys(&backend(), &ENV).await;
}

#[tokio::test]
async fn read_from_topic_all_keys() {
    contract::read_from_topic_all_keys(&backend(), &ENV).await;
}

#[tokio::test]
async fn read_from_topic_single_key() {
    contract::read_from_topic_single_key(&backend(), &ENV).await;
}

#[tokio::test]
async fn read_from_topic_after_and_limit() {
    contract::read_from_topic_after_and_limit(&backend(), &ENV).await;
}
