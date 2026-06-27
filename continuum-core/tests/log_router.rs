//! Integration tests for log routing and destination resolution.

use std::sync::Arc;

use continuum_backend_mem::InMemoryLogBackend;
use continuum_core::backend::LogBackend;
use continuum_core::router::{
    resolve_stream, LogEvaluator, LogFromDestination, LogResolverContext, LogRouter,
    LogTopicRouter,
};
use continuum_core::types::{LogBackendKind, LogDestination, LogStreamId, Seq, STORAGE_KEY_SEP};
use continuum_test_utils::fixtures::{sample_record, BackendEnv};

const ENV: BackendEnv = BackendEnv::MEMORY;

#[test]
fn r1_router_resolve() {
    let mem = Arc::new(InMemoryLogBackend::new()) as Arc<dyn LogBackend>;
    let dest = LogDestination::new("default", LogBackendKind::Memory);
    let router = LogRouter::with_default(&dest, Arc::clone(&mem));
    let resolved = router.resolve_backend(&dest).unwrap();
    assert!(Arc::ptr_eq(&mem, &resolved));
}

#[test]
fn r2_unknown_destination() {
    let router = LogRouter::new();
    let dest = LogDestination::new("missing", LogBackendKind::Memory);
    assert!(router.resolve_backend(&dest).is_err());
}

#[tokio::test]
async fn r3_from_destination_evaluator() {
    let dest = ENV.destination();
    let eval = LogFromDestination(dest.clone());
    let got = eval
        .resolve_for_topic(&LogResolverContext::default(), "any", None)
        .await
        .unwrap();
    assert_eq!(got, dest);
}

#[tokio::test]
async fn r4_topic_prefix() {
    let fallback = LogDestination::new("default", LogBackendKind::SurrealLocal);
    let prefixed = LogDestination::new("metrics", LogBackendKind::SurrealLocal);
    let eval = LogTopicRouter::new(fallback).prefix("metrics.", prefixed.clone());
    let ctx = LogResolverContext::default();
    assert_eq!(
        eval.resolve_for_topic(&ctx, "metrics.hits", None)
            .await
            .unwrap(),
        prefixed
    );
}

#[tokio::test]
async fn r5_topic_fallback() {
    let fallback = LogDestination::new("default", LogBackendKind::SurrealLocal);
    let eval = LogTopicRouter::new(fallback.clone())
        .prefix("other.", LogDestination::new("x", LogBackendKind::SurrealLocal));
    let ctx = LogResolverContext::default();
    assert_eq!(
        eval.resolve_for_topic(&ctx, "unmatched", None)
            .await
            .unwrap(),
        fallback
    );
}

#[tokio::test]
async fn r6_independent_destinations_via_backend() {
    let b = InMemoryLogBackend::new();
    let d1 = LogDestination::new("a", LogBackendKind::Memory);
    let d2 = LogDestination::new("b", LogBackendKind::Memory);
    let s1 = LogStreamId::new(d1, "topic", Some("k".into()));
    let s2 = LogStreamId::new(d2, "topic", Some("k".into()));
    let a = b.append(s1, &[sample_record()]).await.unwrap();
    let c = b.append(s2, &[sample_record()]).await.unwrap();
    assert_eq!(a[0], Seq(1));
    assert_eq!(c[0], Seq(1));
}

#[tokio::test]
async fn resolve_stream_happy_path() {
    let dest = ENV.destination();
    let backend = Arc::new(InMemoryLogBackend::new()) as Arc<dyn LogBackend>;
    let router = LogRouter::with_default(&dest, backend);
    let evaluator = LogFromDestination(dest.clone());
    let (resolved_dest, resolved_backend) = resolve_stream(
        &evaluator,
        &router,
        &LogResolverContext::default(),
        "events",
        None,
    )
    .await
    .unwrap();
    assert_eq!(resolved_dest, dest);
    let _ = resolved_backend;
}

#[tokio::test]
async fn resolve_stream_unknown_backend() {
    let dest = LogDestination::new("missing", LogBackendKind::Memory);
    let evaluator = LogFromDestination(dest);
    let router = LogRouter::new();
    assert!(
        resolve_stream(
            &evaluator,
            &router,
            &LogResolverContext::default(),
            "events",
            None,
        )
        .await
        .is_err()
    );
}

#[test]
fn register_runtime_adds_backend() {
    let router = LogRouter::new();
    let mem = Arc::new(InMemoryLogBackend::new()) as Arc<dyn LogBackend>;
    let dest = LogDestination::new("runtime", LogBackendKind::Memory);
    router.register_runtime(&dest, mem).unwrap();
    assert!(router.resolve_backend(&dest).is_ok());
}

#[test]
fn set_global_and_resolve() {
    if LogRouter::try_global().is_none() {
        let mem = Arc::new(InMemoryLogBackend::new()) as Arc<dyn LogBackend>;
        let dest = LogDestination::new("global_test", LogBackendKind::Memory);
        let router = LogRouter::with_default(&dest, mem);
        LogRouter::set_global(router);
        assert!(LogRouter::try_global().is_some());
        let stream = LogStreamId::new(dest, "t", None);
        assert!(stream.resolve_backend(&LogRouter::global()).is_ok());
    }
}

#[test]
fn storage_key_format() {
    let dest = LogDestination::new("default", LogBackendKind::Memory);
    let stream = LogStreamId::new(dest, "events", Some("k".into()));
    assert_eq!(
        stream.storage_key(),
        format!(
            "memory:default{STORAGE_KEY_SEP}events{STORAGE_KEY_SEP}k"
        )
    );
}
