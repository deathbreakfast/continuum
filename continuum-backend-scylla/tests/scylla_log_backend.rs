//! Scylla contract tests (requires `CONTINUUM_TEST_SCYLLA_CONTACT_POINTS`).

use continuum_backend_scylla::{ScyllaLogBackend, ScyllaLogConfig};
use continuum_test_utils::contract;
use continuum_test_utils::fixtures::BackendEnv;
use uuid::Uuid;

fn contact_points() -> Option<Vec<String>> {
    std::env::var("CONTINUUM_TEST_SCYLLA_CONTACT_POINTS")
        .ok()
        .map(|s| s.split(',').map(str::trim).map(str::to_string).collect())
}

async fn backend() -> Option<ScyllaLogBackend> {
    let points = contact_points()?;
    Some(
        ScyllaLogBackend::connect(ScyllaLogConfig {
            contact_points: points,
            keyspace: std::env::var("CONTINUUM_TEST_SCYLLA_KEYSPACE")
                .unwrap_or_else(|_| "continuum_test".into()),
            ..Default::default()
        })
        .await
        .expect("scylla connect"),
    )
}

macro_rules! scylla_test {
    ($name:ident, $contract:ident) => {
        #[tokio::test]
        #[ignore = "requires CONTINUUM_TEST_SCYLLA_CONTACT_POINTS"]
        async fn $name() {
            let Some(b) = backend().await else {
                eprintln!("CONTINUUM_TEST_SCYLLA_CONTACT_POINTS not set — skipping");
                return;
            };
            let scope = Box::leak(
                format!("{}-{}", stringify!($name), Uuid::new_v4()).into_boxed_str(),
            );
            let env = BackendEnv {
                kind: BackendEnv::SCYLLA.kind,
                logical_dest: scope,
            };
            contract::$contract(&b, &env).await;
        }
    };
}

scylla_test!(append_single, append_single);
scylla_test!(append_batch, append_batch);
scylla_test!(append_empty, append_empty);
scylla_test!(duplicate_event_id, duplicate_event_id);
scylla_test!(read_from_start, read_from_start);
scylla_test!(read_from_mid, read_from_mid);
scylla_test!(read_limit_zero, read_limit_zero);
scylla_test!(read_wrong_stream, read_wrong_stream);
scylla_test!(checkpoint_none, checkpoint_none);
scylla_test!(checkpoint_roundtrip, checkpoint_roundtrip);
scylla_test!(checkpoint_monotonic, checkpoint_monotonic);
scylla_test!(truncate, truncate);
scylla_test!(truncate_before_min, truncate_before_min);
scylla_test!(e2e_lifecycle, e2e_lifecycle);
scylla_test!(independent_destinations, independent_destinations);
scylla_test!(empty_topic_rejected, empty_topic_rejected);
scylla_test!(read_limit_validation, read_limit_validation);
scylla_test!(distinct_partition_keys, distinct_partition_keys);
scylla_test!(read_from_topic_all_keys, read_from_topic_all_keys);
scylla_test!(read_from_topic_single_key, read_from_topic_single_key);
    scylla_test!(read_from_topic_after_and_limit, read_from_topic_after_and_limit);

#[tokio::test]
#[ignore = "requires CONTINUUM_TEST_SCYLLA_CONTACT_POINTS"]
async fn topic_index_cache_skips_repeat_index_write() {
    use continuum_core::backend::LogBackend;

    let Some(points) = contact_points() else {
        return;
    };
    std::env::set_var("CONTINUUM_APPEND_DEBUG_OPS", "1");
    continuum_backend_scylla::append_debug_reset();

    let b = ScyllaLogBackend::connect(ScyllaLogConfig {
        contact_points: points,
        keyspace: std::env::var("CONTINUUM_TEST_SCYLLA_KEYSPACE")
            .unwrap_or_else(|_| "continuum_test".into()),
        topic_index_cache: true,
        ..Default::default()
    })
    .await
    .expect("scylla connect");

    let scope = Box::leak(format!("topic-cache-{}", Uuid::new_v4()).into_boxed_str());
    let env = BackendEnv {
        kind: BackendEnv::SCYLLA.kind,
        logical_dest: scope,
    };
    let stream = env.stream_with_key("bench", "k1");
    let rec = continuum_test_utils::fixtures::sample_record();

    b.append(stream.clone(), std::slice::from_ref(&rec))
        .await
        .expect("first append");
    let (rt1, ops1) = continuum_backend_scylla::append_debug_snapshot();
    continuum_backend_scylla::append_debug_reset();

    let rec2 = continuum_test_utils::fixtures::sample_record();
    b.append(stream, std::slice::from_ref(&rec2))
        .await
        .expect("second append");
    let (rt2, _ops2) = continuum_backend_scylla::append_debug_snapshot();

    std::env::remove_var("CONTINUUM_APPEND_DEBUG_OPS");

    assert!(rt1 >= 3, "first append should include index write (rt1={rt1}, ops1={ops1})");
    assert!(
        rt2 <= rt1.saturating_sub(1),
        "cached index should reduce round trips (rt1={rt1}, rt2={rt2})"
    );
}

#[tokio::test]
#[ignore = "requires CONTINUUM_TEST_SCYLLA_CONTACT_POINTS"]
async fn topic_index_legacy_writes_each_append_when_cache_off() {
    use continuum_core::backend::LogBackend;

    let Some(points) = contact_points() else {
        return;
    };
    std::env::set_var("CONTINUUM_APPEND_DEBUG_OPS", "1");
    continuum_backend_scylla::append_debug_reset();

    let b = ScyllaLogBackend::connect(ScyllaLogConfig {
        contact_points: points,
        keyspace: std::env::var("CONTINUUM_TEST_SCYLLA_KEYSPACE")
            .unwrap_or_else(|_| "continuum_test".into()),
        topic_index_cache: false,
        ..Default::default()
    })
    .await
    .expect("scylla connect");

    let scope = Box::leak(format!("topic-legacy-{}", Uuid::new_v4()).into_boxed_str());
    let env = BackendEnv {
        kind: BackendEnv::SCYLLA.kind,
        logical_dest: scope,
    };
    let stream = env.stream_with_key("bench", "k1");
    let rec = continuum_test_utils::fixtures::sample_record();

    b.append(stream.clone(), std::slice::from_ref(&rec))
        .await
        .expect("first append");
    let (rt1, _) = continuum_backend_scylla::append_debug_snapshot();
    continuum_backend_scylla::append_debug_reset();

    let rec2 = continuum_test_utils::fixtures::sample_record();
    b.append(stream, std::slice::from_ref(&rec2))
        .await
        .expect("second append");
    let (rt2, _) = continuum_backend_scylla::append_debug_snapshot();

    std::env::remove_var("CONTINUUM_APPEND_DEBUG_OPS");

    assert!(rt1 >= 3, "first append should include index write (rt1={rt1})");
    // First append also pays stream-row bootstrap RTs; compare warm appends only.
    assert!(
        rt2 >= 3,
        "legacy mode should retain index write on repeat append (rt1={rt1}, rt2={rt2})"
    );
}

#[tokio::test]
async fn schema_idempotent_in_memory_skipped_without_env() {
    if contact_points().is_none() {
        return;
    }
    let points = contact_points().unwrap();
    ScyllaLogBackend::connect(ScyllaLogConfig {
        contact_points: points.clone(),
        keyspace: "continuum_test".into(),
        ..Default::default()
    })
    .await
    .expect("first connect");
    ScyllaLogBackend::connect(ScyllaLogConfig {
        contact_points: points,
        keyspace: "continuum_test".into(),
        ..Default::default()
    })
    .await
    .expect("second connect");
}
