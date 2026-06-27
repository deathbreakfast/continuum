//! Smoke tests for stub backends that return [`LogError::Unsupported`](continuum_core::LogError::Unsupported).

use continuum_core::backend::LogBackend;
use continuum_core::error::LogError;
use continuum_core::types::SubscriptionId;

use crate::fixtures::{sample_record, BackendEnv};

fn assert_unsupported(result: continuum_core::error::Result<impl std::fmt::Debug>, msg: &str) {
    match result {
        Err(LogError::Unsupported(actual)) => assert_eq!(actual, msg),
        other => panic!("expected Unsupported({msg:?}), got {other:?}"),
    }
}

/// Every port method on a stub backend returns `Unsupported` with the expected message.
pub async fn assert_all_unsupported(b: &dyn LogBackend, env: &BackendEnv, msg: &str) {
    let stream = env.stream("t");
    let sub = SubscriptionId::new("sub");

    assert_unsupported(b.append(stream.clone(), &[sample_record()]).await, msg);
    assert_unsupported(
        b.read_from(stream.clone(), continuum_core::types::Seq::ZERO, 10).await,
        msg,
    );
    assert_unsupported(
        b.commit_checkpoint(&sub, stream.clone(), continuum_core::types::Seq(1))
            .await,
        msg,
    );
    assert_unsupported(b.load_checkpoint(&sub, stream.clone()).await, msg);
    assert_unsupported(
        b.read_from_topic(stream.clone(), None, continuum_core::types::Seq::ZERO, 10)
            .await,
        msg,
    );
    assert_unsupported(
        b.truncate_before(stream, continuum_core::types::Seq(1)).await,
        msg,
    );
}
