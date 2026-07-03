//! Scylla backend builder for benchmark dimensions.

use std::sync::Arc;

use anyhow::{Context, Result};
use continuum::backends::{ScyllaLogBackend, ScyllaLogConfig};

use super::{wrap_scylla, BackendHandle, SharedHandle};
use crate::harness::dimensions::{Telemetry, Topology};

fn scylla_config() -> Result<ScyllaLogConfig> {
    let contact_points = std::env::var("CONTINUUM_BENCH_SCYLLA_CONTACT_POINTS")
        .or_else(|_| std::env::var("CONTINUUM_BENCH_SCYLLA_URL"))
        .context("CONTINUUM_BENCH_SCYLLA_CONTACT_POINTS not set for scylla storage")?;
    Ok(ScyllaLogConfig {
        contact_points: contact_points
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect(),
        keyspace: std::env::var("CONTINUUM_BENCH_SCYLLA_KEYSPACE")
            .unwrap_or_else(|_| "continuum".into()),
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

    let backend =
        ScyllaLogBackend::from_session(Arc::clone(shared_backend.session()), shared_backend.keyspace())
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
