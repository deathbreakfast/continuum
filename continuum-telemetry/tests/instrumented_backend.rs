//! Integration tests for [`InstrumentedLogBackend`](continuum_telemetry::InstrumentedLogBackend).

use std::sync::{Arc, Mutex};

use continuum_backend_mem::InMemoryLogBackend;
use continuum_core::backend::LogBackend;
use continuum_core::error::LogError;
use continuum_core::types::{LogStreamId, Seq, SubscriptionId};
use continuum_telemetry::{
    telemetry_from_env, AppendOutcome, AppendTelemetry, InstrumentedLogBackend, NoTelemetry,
    ReadOutcome, ReadTelemetry, TelemetryOp, TelemetrySink,
};
use continuum_test_utils::fixtures::{sample_record, BackendEnv};

const ENV: BackendEnv = BackendEnv::MEMORY;

#[derive(Clone, Debug, Default)]
struct RecordingSink {
    append_calls: Arc<Mutex<Vec<(AppendTelemetry, AppendOutcome)>>>,
    read_calls: Arc<Mutex<Vec<(ReadTelemetry, ReadOutcome)>>>,
    errors: Arc<Mutex<Vec<TelemetryOp>>>,
}

impl TelemetrySink for RecordingSink {
    fn on_append_batch(&self, ctx: &AppendTelemetry, outcome: &AppendOutcome) {
        self.append_calls
            .lock()
            .unwrap()
            .push((ctx.clone(), outcome.clone()));
    }

    fn on_read(&self, ctx: &ReadTelemetry, outcome: &ReadOutcome) {
        self.read_calls
            .lock()
            .unwrap()
            .push((ctx.clone(), outcome.clone()));
    }

    fn on_checkpoint(
        &self,
        _: &continuum_telemetry::CheckpointTelemetry,
        _: &continuum_core::error::Result<()>,
    ) {
    }

    fn on_truncate(
        &self,
        _: &continuum_telemetry::TruncateTelemetry,
        _: &continuum_core::error::Result<u64>,
    ) {
    }

    fn on_error(&self, op: TelemetryOp, _: &LogError) {
        self.errors.lock().unwrap().push(op);
    }
}

#[tokio::test]
async fn append_emits_telemetry() {
    let sink = RecordingSink::default();
    let sink_for_assert = sink.clone();
    let b = InstrumentedLogBackend::new(InMemoryLogBackend::new(), sink);
    let stream = ENV.stream("t");
    let seqs = b.append(stream, &[sample_record()]).await.unwrap();
    assert_eq!(seqs[0], Seq(1));
    let calls = sink_for_assert.append_calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0.batch_len, 1);
    assert_eq!(calls[0].1.assigned_count, 1);
}

#[tokio::test]
async fn read_from_topic_emits_read_telemetry() {
    let sink = RecordingSink::default();
    let sink_for_assert = sink.clone();
    let b = InstrumentedLogBackend::new(InMemoryLogBackend::new(), sink);
    let s_a = ENV.stream_with_key("t", "a");
    let s_b = ENV.stream_with_key("t", "b");
    b.append(s_a, &[sample_record()]).await.unwrap();
    b.append(s_b, &[sample_record()]).await.unwrap();
    let rows = b
        .read_from_topic(ENV.stream("t"), None, Seq::ZERO, 10)
        .await
        .unwrap();
    assert_eq!(rows.len(), 2);
    let calls = sink_for_assert.read_calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].1.row_count, 2);
}

#[tokio::test]
async fn error_emits_on_error() {
    let sink = RecordingSink::default();
    let sink_for_assert = sink.clone();
    let b = InstrumentedLogBackend::new(InMemoryLogBackend::new(), sink);
    let stream = LogStreamId::new(ENV.destination(), "", None);
    assert!(b.append(stream, &[sample_record()]).await.is_err());
    let errors = sink_for_assert.errors.lock().unwrap();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0], TelemetryOp::Append);
}

async fn exercise_with_env(value: Option<&str>) {
    match value {
        Some(v) => unsafe { std::env::set_var("CONTINUUM_TELEMETRY", v) },
        None => unsafe { std::env::remove_var("CONTINUUM_TELEMETRY") },
    }
    let sink = DynTelemetrySink(telemetry_from_env());
    let b = InstrumentedLogBackend::new(InMemoryLogBackend::new(), sink);
    let seqs = b.append(ENV.stream("t"), &[sample_record()]).await.unwrap();
    assert_eq!(seqs[0], Seq(1));
    unsafe {
        std::env::remove_var("CONTINUUM_TELEMETRY");
    }
}

struct DynTelemetrySink(Arc<dyn TelemetrySink>);

impl std::fmt::Debug for DynTelemetrySink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DynTelemetrySink").finish_non_exhaustive()
    }
}

impl TelemetrySink for DynTelemetrySink {
    fn on_append_batch(&self, ctx: &AppendTelemetry, outcome: &AppendOutcome) {
        self.0.on_append_batch(ctx, outcome);
    }

    fn on_read(&self, ctx: &ReadTelemetry, outcome: &ReadOutcome) {
        self.0.on_read(ctx, outcome);
    }

    fn on_checkpoint(
        &self,
        ctx: &continuum_telemetry::CheckpointTelemetry,
        outcome: &continuum_core::error::Result<()>,
    ) {
        self.0.on_checkpoint(ctx, outcome);
    }

    fn on_truncate(
        &self,
        ctx: &continuum_telemetry::TruncateTelemetry,
        outcome: &continuum_core::error::Result<u64>,
    ) {
        self.0.on_truncate(ctx, outcome);
    }

    fn on_error(&self, op: TelemetryOp, err: &LogError) {
        self.0.on_error(op, err);
    }
}

#[tokio::test]
async fn telemetry_from_env_off() {
    exercise_with_env(Some("off")).await;
}

#[tokio::test]
async fn telemetry_from_env_default() {
    exercise_with_env(None).await;
}

#[tokio::test]
async fn telemetry_from_env_console() {
    exercise_with_env(Some("console")).await;
}

#[tokio::test]
async fn no_telemetry_wrapper_forwards() {
    let b = InstrumentedLogBackend::new(InMemoryLogBackend::new(), NoTelemetry);
    let sub = SubscriptionId::new("sub");
    let stream = ENV.stream("t");
    let seqs = b.append(stream.clone(), &[sample_record()]).await.unwrap();
    b.commit_checkpoint(&sub, stream.clone(), seqs[0])
        .await
        .unwrap();
    assert_eq!(
        b.load_checkpoint(&sub, stream).await.unwrap(),
        Some(seqs[0])
    );
}
