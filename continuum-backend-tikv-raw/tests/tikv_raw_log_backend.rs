//! Raw TiKV contract tests (requires `CONTINUUM_TEST_TIKV_PD_ENDPOINT`).

use continuum_backend_tikv_raw::{TikvRawLogBackend, TikvRawLogConfig};
use continuum_test_utils::contract;
use continuum_test_utils::fixtures::BackendEnv;
use uuid::Uuid;

fn pd_endpoints() -> Option<Vec<String>> {
    std::env::var("CONTINUUM_TEST_TIKV_PD_ENDPOINT")
        .ok()
        .map(|s| vec![s])
}

async fn backend() -> Option<TikvRawLogBackend> {
    let endpoints = pd_endpoints()?;
    Some(
        TikvRawLogBackend::connect(TikvRawLogConfig {
            pd_endpoints: endpoints,
        })
        .await
        .expect("tikv connect"),
    )
}

macro_rules! tikv_test {
    ($name:ident, $contract:ident) => {
        #[tokio::test]
        #[ignore = "requires CONTINUUM_TEST_TIKV_PD_ENDPOINT"]
        async fn $name() {
            let Some(b) = backend().await else {
                eprintln!("CONTINUUM_TEST_TIKV_PD_ENDPOINT not set — skipping");
                return;
            };
            let scope = Box::leak(
                format!("{}-{}", stringify!($name), Uuid::new_v4()).into_boxed_str(),
            );
            let env = BackendEnv {
                kind: BackendEnv::TIKV_RAW.kind,
                logical_dest: scope,
            };
            contract::$contract(&b, &env).await;
        }
    };
}

tikv_test!(append_single, append_single);
tikv_test!(append_batch, append_batch);
tikv_test!(append_empty, append_empty);
tikv_test!(duplicate_event_id, duplicate_event_id);
tikv_test!(read_from_start, read_from_start);
tikv_test!(read_from_mid, read_from_mid);
tikv_test!(read_limit_zero, read_limit_zero);
tikv_test!(read_wrong_stream, read_wrong_stream);
tikv_test!(checkpoint_none, checkpoint_none);
tikv_test!(checkpoint_roundtrip, checkpoint_roundtrip);
tikv_test!(checkpoint_monotonic, checkpoint_monotonic);
tikv_test!(truncate, truncate);
tikv_test!(truncate_before_min, truncate_before_min);
tikv_test!(e2e_lifecycle, e2e_lifecycle);
tikv_test!(independent_destinations, independent_destinations);
tikv_test!(empty_topic_rejected, empty_topic_rejected);
tikv_test!(read_limit_validation, read_limit_validation);
tikv_test!(distinct_partition_keys, distinct_partition_keys);
tikv_test!(read_from_topic_all_keys, read_from_topic_all_keys);
tikv_test!(read_from_topic_single_key, read_from_topic_single_key);
tikv_test!(read_from_topic_after_and_limit, read_from_topic_after_and_limit);
