//! Continuum-native telemetry port (append/read/checkpoint/truncate diagnostics).
//!
//! Wrap any [`LogBackend`] with [`InstrumentedLogBackend`]
//! to emit structured timing and outcome hooks via a [`TelemetrySink`]. This layer logs port
//! diagnostics only — not transport payload contents.
//!
//! Enable via the `telemetry-console` feature on the [`continuum`](https://docs.rs/continuum) facade.
//! See [Getting started](https://docs.rs/continuum/latest/continuum/index.html#getting-started)
//! and the [documentation map](https://docs.rs/continuum/latest/continuum/index.html#documentation-map).
//!
//! # Examples
//!
//! ```rust
//! use continuum_backend_mem::InMemoryLogBackend;
//! use continuum_telemetry::{InstrumentedLogBackend, NoTelemetry};
//!
//! let backend = InstrumentedLogBackend::new(InMemoryLogBackend::new(), NoTelemetry);
//! let _ = backend;
//! ```

use std::fmt::Debug;
use std::sync::Arc;
use std::time::Duration;

use continuum_core::backend::LogBackend;
use continuum_core::error::{LogError, Result};
use continuum_core::types::{AppendRecord, EventRecord, LogStreamId, Seq, SubscriptionId};

/// Operation kind for error telemetry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TelemetryOp {
    /// Batch append.
    Append,
    /// Forward read.
    Read,
    /// Checkpoint commit.
    Checkpoint,
    /// Truncate reclaim.
    Truncate,
}

/// Context for append telemetry.
#[derive(Debug, Clone)]
pub struct AppendTelemetry {
    /// Serialized stream key.
    pub stream_key: String,
    /// Records in batch.
    pub batch_len: usize,
}

/// Outcome for append telemetry.
#[derive(Debug, Clone)]
pub struct AppendOutcome {
    /// Wall time for the append call.
    pub duration: Duration,
    /// Sequences assigned (0 on error).
    pub assigned_count: usize,
}

/// Context for read telemetry.
#[derive(Debug, Clone)]
pub struct ReadTelemetry {
    /// Serialized stream key.
    pub stream_key: String,
    /// Requested limit.
    pub limit: usize,
}

/// Outcome for read telemetry.
#[derive(Debug, Clone)]
pub struct ReadOutcome {
    /// Wall time for the read call.
    pub duration: Duration,
    /// Rows returned.
    pub row_count: usize,
}

/// Context for checkpoint telemetry.
#[derive(Debug, Clone)]
pub struct CheckpointTelemetry {
    /// Subscription id string.
    pub subscription: String,
    /// Stream key.
    pub stream_key: String,
    /// Committed sequence.
    pub seq: i64,
}

/// Context for truncate telemetry.
#[derive(Debug, Clone)]
pub struct TruncateTelemetry {
    /// Stream key.
    pub stream_key: String,
    /// Truncate bound sequence.
    pub bound: i64,
}

/// Hooks for continuum port diagnostics (not transport payload logging).
pub trait TelemetrySink: Send + Sync {
    /// After append batch completes or fails.
    fn on_append_batch(&self, ctx: &AppendTelemetry, outcome: &AppendOutcome);
    /// After read completes or fails.
    fn on_read(&self, ctx: &ReadTelemetry, outcome: &ReadOutcome);
    /// After checkpoint commit completes or fails.
    fn on_checkpoint(&self, ctx: &CheckpointTelemetry, outcome: &Result<()>);
    /// After truncate completes or fails.
    fn on_truncate(&self, ctx: &TruncateTelemetry, outcome: &Result<u64>);
    /// On operation error (optional supplement to outcome hooks).
    fn on_error(&self, op: TelemetryOp, err: &LogError);
}

/// No-op telemetry sink.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoTelemetry;

impl TelemetrySink for NoTelemetry {
    fn on_append_batch(&self, _: &AppendTelemetry, _: &AppendOutcome) {}
    fn on_read(&self, _: &ReadTelemetry, _: &ReadOutcome) {}
    fn on_checkpoint(&self, _: &CheckpointTelemetry, _: &Result<()>) {}
    fn on_truncate(&self, _: &TruncateTelemetry, _: &Result<u64>) {}
    fn on_error(&self, _: TelemetryOp, _: &LogError) {}
}

/// Stderr structured lines (default when enabled).
#[derive(Debug, Default, Clone, Copy)]
pub struct ConsoleTelemetry;

impl TelemetrySink for ConsoleTelemetry {
    fn on_append_batch(&self, ctx: &AppendTelemetry, outcome: &AppendOutcome) {
        eprintln!(
            "[continuum-telemetry] append stream={} batch={} assigned={} ms={:.3}",
            ctx.stream_key,
            ctx.batch_len,
            outcome.assigned_count,
            outcome.duration.as_secs_f64() * 1000.0
        );
    }

    fn on_read(&self, ctx: &ReadTelemetry, outcome: &ReadOutcome) {
        eprintln!(
            "[continuum-telemetry] read stream={} limit={} rows={} ms={:.3}",
            ctx.stream_key,
            ctx.limit,
            outcome.row_count,
            outcome.duration.as_secs_f64() * 1000.0
        );
    }

    fn on_checkpoint(&self, ctx: &CheckpointTelemetry, outcome: &Result<()>) {
        eprintln!(
            "[continuum-telemetry] checkpoint sub={} stream={} seq={} ok={}",
            ctx.subscription,
            ctx.stream_key,
            ctx.seq,
            outcome.is_ok()
        );
    }

    fn on_truncate(&self, ctx: &TruncateTelemetry, outcome: &Result<u64>) {
        eprintln!(
            "[continuum-telemetry] truncate stream={} bound={} ok={}",
            ctx.stream_key,
            ctx.bound,
            outcome.is_ok()
        );
    }

    fn on_error(&self, op: TelemetryOp, err: &LogError) {
        eprintln!("[continuum-telemetry] error op={op:?} err={err}");
    }
}

/// Resolve telemetry sink from `CONTINUUM_TELEMETRY` (`off` | `console`, default `console`).
#[must_use]
pub fn telemetry_from_env() -> Arc<dyn TelemetrySink> {
    let raw = std::env::var("CONTINUUM_TELEMETRY")
        .ok()
        .map_or_else(|| "console".to_string(), |v| v.trim().to_ascii_lowercase());
    match raw.as_str() {
        "off" | "0" | "false" | "none" => Arc::new(NoTelemetry),
        _ => Arc::new(ConsoleTelemetry),
    }
}

/// Decorates a [`LogBackend`] with telemetry hooks.
///
/// Forwards all port operations to `inner` and invokes `telemetry` after each call.
///
/// # Examples
///
/// ```rust
/// use continuum_backend_mem::InMemoryLogBackend;
/// use continuum_telemetry::{InstrumentedLogBackend, NoTelemetry};
///
/// let backend = InstrumentedLogBackend::new(InMemoryLogBackend::new(), NoTelemetry);
/// ```
#[derive(Debug)]
pub struct InstrumentedLogBackend<B, T> {
    inner: B,
    telemetry: T,
}

impl<B, T> InstrumentedLogBackend<B, T> {
    /// Wrap `inner` with `telemetry`.
    pub const fn new(inner: B, telemetry: T) -> Self {
        Self { inner, telemetry }
    }
}

impl<B: LogBackend, T: TelemetrySink> InstrumentedLogBackend<B, T> {
    fn stream_key(stream: &LogStreamId) -> String {
        stream.storage_key()
    }
}

#[async_trait::async_trait]
impl<B: LogBackend + Debug, T: TelemetrySink + Debug> LogBackend for InstrumentedLogBackend<B, T> {
    async fn append(
        &self,
        stream: LogStreamId,
        records: &[AppendRecord],
    ) -> Result<Vec<Seq>> {
        let ctx = AppendTelemetry {
            stream_key: Self::stream_key(&stream),
            batch_len: records.len(),
        };
        let start = std::time::Instant::now();
        let result = self.inner.append(stream, records).await;
        let duration = start.elapsed();
        let outcome = AppendOutcome {
            duration,
            assigned_count: result.as_ref().map_or(0, Vec::len),
        };
        self.telemetry.on_append_batch(&ctx, &outcome);
        if let Err(ref e) = result {
            self.telemetry.on_error(TelemetryOp::Append, e);
        }
        result
    }

    async fn read_from(
        &self,
        stream: LogStreamId,
        after: Seq,
        limit: usize,
    ) -> Result<Vec<EventRecord>> {
        let ctx = ReadTelemetry {
            stream_key: Self::stream_key(&stream),
            limit,
        };
        let start = std::time::Instant::now();
        let result = self.inner.read_from(stream, after, limit).await;
        let duration = start.elapsed();
        let outcome = ReadOutcome {
            duration,
            row_count: result.as_ref().map_or(0, Vec::len),
        };
        self.telemetry.on_read(&ctx, &outcome);
        if let Err(ref e) = result {
            self.telemetry.on_error(TelemetryOp::Read, e);
        }
        result
    }

    async fn commit_checkpoint(
        &self,
        subscription: &SubscriptionId,
        stream: LogStreamId,
        seq: Seq,
    ) -> Result<()> {
        let ctx = CheckpointTelemetry {
            subscription: subscription.0.clone(),
            stream_key: Self::stream_key(&stream),
            seq: seq.as_i64(),
        };
        let result = self
            .inner
            .commit_checkpoint(subscription, stream, seq)
            .await;
        self.telemetry.on_checkpoint(&ctx, &result);
        if let Err(ref e) = result {
            self.telemetry.on_error(TelemetryOp::Checkpoint, e);
        }
        result
    }

    async fn load_checkpoint(
        &self,
        subscription: &SubscriptionId,
        stream: LogStreamId,
    ) -> Result<Option<Seq>> {
        self.inner.load_checkpoint(subscription, stream).await
    }

    async fn read_from_topic(
        &self,
        stream: LogStreamId,
        topic_key: Option<&str>,
        after: Seq,
        limit: usize,
    ) -> Result<Vec<EventRecord>> {
        let ctx = ReadTelemetry {
            stream_key: Self::stream_key(&stream),
            limit,
        };
        let start = std::time::Instant::now();
        let result = self
            .inner
            .read_from_topic(stream, topic_key, after, limit)
            .await;
        let duration = start.elapsed();
        let outcome = ReadOutcome {
            duration,
            row_count: result.as_ref().map_or(0, Vec::len),
        };
        self.telemetry.on_read(&ctx, &outcome);
        if let Err(ref e) = result {
            self.telemetry.on_error(TelemetryOp::Read, e);
        }
        result
    }

    async fn truncate_before(&self, stream: LogStreamId, seq: Seq) -> Result<u64> {
        let ctx = TruncateTelemetry {
            stream_key: Self::stream_key(&stream),
            bound: seq.as_i64(),
        };
        let result = self.inner.truncate_before(stream, seq).await;
        self.telemetry.on_truncate(&ctx, &result);
        if let Err(ref e) = result {
            self.telemetry.on_error(TelemetryOp::Truncate, e);
        }
        result
    }
}
