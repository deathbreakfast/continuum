//! Raw `TiKV` backend builder for benchmark dimensions.

use std::sync::Arc;

use anyhow::{Context, Result};
use continuum::backends::{TikvRawLogBackend, TikvRawLogConfig};

use super::{wrap_tikv_raw, BackendHandle, SharedHandle};
use crate::harness::dimensions::{Telemetry, Topology};

fn pd_endpoints() -> Result<Vec<String>> {
    let endpoint = std::env::var("CONTINUUM_BENCH_TIKV_PD_ENDPOINT")
        .context("CONTINUUM_BENCH_TIKV_PD_ENDPOINT not set for tikv-raw storage")?;
    Ok(vec![endpoint])
}

async fn connect_shared() -> Result<Arc<TikvRawLogBackend>> {
    Ok(Arc::new(
        TikvRawLogBackend::connect(TikvRawLogConfig {
            pd_endpoints: pd_endpoints()?,
        })
        .await?,
    ))
}

/// Build a raw `TiKV` backend.
pub async fn build_tikv_raw(
    _topology: Topology,
    telemetry: Telemetry,
    shared: Option<SharedHandle>,
) -> Result<BackendHandle> {
    let endpoints = pd_endpoints()?;
    let engine_path = format!("tikv://{}", endpoints[0]);

    let shared_backend = if let Some(SharedHandle::TikvRaw(existing)) = shared {
        existing
    } else {
        connect_shared().await?
    };
    let backend = TikvRawLogBackend::from_client(Arc::clone(shared_backend.client()));

    Ok(BackendHandle {
        backend: Arc::new(wrap_tikv_raw(backend, telemetry)),
        engine_path,
        _temp_dir: None,
        _shared: Some(SharedHandle::TikvRaw(shared_backend)),
    })
}

/// Open a fresh shared `TiKV` backend for co-tenancy experiments.
pub async fn open_shared_tikv_raw() -> Result<Arc<TikvRawLogBackend>> {
    connect_shared().await
}
