//! Scylla backend builder for benchmark dimensions.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Context, Result};
use continuum::backends::{
    consistency_from_str, IdempotencyMode, IdempotencyPolicy, ScyllaLogBackend, ScyllaLogConfig,
};

use super::{wrap_scylla, BackendHandle, SharedHandle};
use crate::harness::dimensions::{Telemetry, Topology};

/// Env bool with default when unset; explicit `0`/`false` disables.
fn env_flag_default(name: &str, default: bool) -> bool {
    match std::env::var(name).ok() {
        Some(v) if v == "1" || v.eq_ignore_ascii_case("true") => true,
        Some(v) if v == "0" || v.eq_ignore_ascii_case("false") => false,
        _ => default,
    }
}

fn idempotency_policy_from_env() -> IdempotencyPolicy {
    if let Ok(none_topics) = std::env::var("CONTINUUM_SCYLLA_IDEMPOTENCY_NONE_TOPICS") {
        let overrides: HashMap<String, IdempotencyMode> = none_topics
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|t| (t.to_string(), IdempotencyMode::None))
            .collect();
        if !overrides.is_empty() {
            return IdempotencyPolicy::PerTopic {
                default: IdempotencyMode::Lwt,
                overrides,
            };
        }
    }

    let mode = match std::env::var("CONTINUUM_SCYLLA_IDEMPOTENCY")
        .ok()
        .as_deref()
        .map(str::trim)
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("none") => IdempotencyMode::None,
        _ => IdempotencyMode::Lwt,
    };
    IdempotencyPolicy::Global(mode)
}

fn scylla_config() -> Result<ScyllaLogConfig> {
    let contact_points = std::env::var("CONTINUUM_BENCH_SCYLLA_CONTACT_POINTS")
        .or_else(|_| std::env::var("CONTINUUM_BENCH_SCYLLA_URL"))
        .context("CONTINUUM_BENCH_SCYLLA_CONTACT_POINTS not set for scylla storage")?;

    let write_consistency = std::env::var("CONTINUUM_SCYLLA_WRITE_CONSISTENCY")
        .ok()
        .and_then(|s| consistency_from_str(&s));

    let seq_block_size = std::env::var("CONTINUUM_SCYLLA_SEQ_BLOCK_SIZE")
        .ok()
        .and_then(|s| s.parse().ok())
        .filter(|&n| n > 0)
        .unwrap_or(64);

    Ok(ScyllaLogConfig {
        contact_points: contact_points
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect(),
        keyspace: std::env::var("CONTINUUM_BENCH_SCYLLA_KEYSPACE")
            .unwrap_or_else(|_| "continuum".into()),
        idempotency: idempotency_policy_from_env(),
        topic_index_cache: env_flag_default("CONTINUUM_SCYLLA_TOPIC_INDEX_CACHE", true),
        write_consistency,
        replication_factor: 1,
        seq_block_size,
        ..Default::default()
    })
}

async fn connect_shared() -> Result<Arc<ScyllaLogBackend>> {
    Ok(Arc::new(ScyllaLogBackend::connect(scylla_config()?).await?))
}

/// Build a Scylla backend for the given topology.
pub async fn build_scylla(
    topology: Topology,
    telemetry: Telemetry,
    shared: Option<SharedHandle>,
) -> Result<BackendHandle> {
    let config = scylla_config()?;
    let engine_path = format!(
        "scylla://{}/{}",
        config.contact_points.join(","),
        config.keyspace
    );

    let shared_backend = if topology == Topology::SharedHandle {
        if let Some(SharedHandle::Scylla(existing)) = shared {
            existing
        } else {
            connect_shared().await?
        }
    } else {
        connect_shared().await?
    };

    let backend = ScyllaLogBackend::from_session(
        Arc::clone(shared_backend.session()),
        &config,
    )
    .await?;

    Ok(BackendHandle {
        backend: Arc::new(wrap_scylla(backend, telemetry)),
        engine_path,
        _temp_dir: None,
        _shared: Some(SharedHandle::Scylla(shared_backend)),
    })
}

/// Open a fresh shared Scylla backend for co-tenancy experiments.
pub async fn open_shared_scylla() -> Result<Arc<ScyllaLogBackend>> {
    connect_shared().await
}
