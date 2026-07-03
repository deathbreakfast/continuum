//! `SQLite` backend builder for benchmark dimensions.

use std::sync::Arc;

use anyhow::{Context, Result};
use continuum::backends::SqliteLogBackend;
use tempfile::TempDir;

use super::{wrap_sqlite, BackendHandle, SharedHandle};
use crate::harness::dimensions::{Telemetry, Topology};

/// Build a `SQLite` backend for the given topology.
pub async fn build_sqlite(
    topology: Topology,
    telemetry: Telemetry,
    shared: Option<SharedHandle>,
) -> Result<BackendHandle> {
    let (pool, engine_path, temp_dir) = if topology == Topology::SharedHandle {
        if let Some(SharedHandle::Sqlite(pool)) = shared {
            (pool, "shared-sqlite-pool".into(), None)
        } else {
            open_sqlite_pool().await?
        }
    } else {
        open_sqlite_pool().await?
    };

    let inner = SqliteLogBackend::from_pool((*pool).clone())
        .await
        .context("SqliteLogBackend::from_pool")?;

    Ok(BackendHandle {
        backend: Arc::new(wrap_sqlite(inner, telemetry)),
        engine_path,
        _temp_dir: temp_dir,
        _shared: Some(SharedHandle::Sqlite(pool)),
    })
}

async fn open_sqlite_pool() -> Result<(Arc<sqlx::SqlitePool>, String, Option<TempDir>)> {
    let dir = TempDir::new().context("tempdir for sqlite")?;
    let path = dir.path().join("bench.db");
    let url = format!("sqlite://{}?mode=rwc", path.display());
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&url)
        .await
        .context("sqlite connect")?;
    Ok((Arc::new(pool), url, Some(dir)))
}

/// Open a fresh shared `SQLite` pool for co-tenancy experiments.
pub async fn open_shared_sqlite() -> Result<(Arc<sqlx::SqlitePool>, TempDir)> {
    let (pool, _, temp) = open_sqlite_pool().await?;
    let temp = temp.context("sqlite tempdir")?;
    Ok((pool, temp))
}
