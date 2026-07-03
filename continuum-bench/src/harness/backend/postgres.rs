//! `PostgreSQL` backend builder for benchmark dimensions.

use std::sync::Arc;

use anyhow::{Context, Result};

use continuum::backends::PostgresLogBackend;

use super::{wrap_postgres, BackendHandle, SharedHandle};
use crate::harness::dimensions::{Telemetry, Topology};

/// Build a Postgres backend for the given topology.
pub async fn build_postgres(
    topology: Topology,
    telemetry: Telemetry,
    shared: Option<SharedHandle>,
) -> Result<BackendHandle> {
    let url = std::env::var("CONTINUUM_BENCH_POSTGRES_URL")
        .context("CONTINUUM_BENCH_POSTGRES_URL not set for postgres storage")?;

    let pool = if topology == Topology::SharedHandle {
        if let Some(SharedHandle::Postgres(pool)) = shared {
            pool
        } else {
            open_postgres_pool(&url).await?
        }
    } else {
        open_postgres_pool(&url).await?
    };

    let inner = PostgresLogBackend::from_pool((*pool).clone())
        .await
        .context("PostgresLogBackend::from_pool")?;

    Ok(BackendHandle {
        backend: Arc::new(wrap_postgres(inner, telemetry)),
        engine_path: url,
        _temp_dir: None,
        _shared: Some(SharedHandle::Postgres(pool)),
    })
}

async fn open_postgres_pool(url: &str) -> Result<Arc<sqlx::PgPool>> {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(url)
        .await
        .context("postgres connect")?;
    Ok(Arc::new(pool))
}

/// Open a fresh shared `PostgreSQL` pool for co-tenancy experiments.
pub async fn open_shared_postgres() -> Result<Arc<sqlx::PgPool>> {
    let url = std::env::var("CONTINUUM_BENCH_POSTGRES_URL")
        .context("CONTINUUM_BENCH_POSTGRES_URL not set")?;
    open_postgres_pool(&url).await
}
