//! `SurrealDB` backend builders for embedded and remote (`TiKV`-backed) storage.
//!
//! **Internal — performance engineers.** Requires `CONTINUUM_BENCH_SURREAL_URL` for
//! [`Storage::SurrealTikv`] and `Topology::RemoteSurreal` paths.

use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use continuum::backends::SurrealLocalLogBackend;
use surrealdb::engine::any::Any;
use surrealdb::Surreal;
use tempfile::TempDir;

use super::{wrap_surreal, BackendHandle, SharedHandle};
use crate::harness::dimensions::{Storage, Telemetry, Topology};

/// Build an embedded or remote Surreal backend for `surreal-mem` / `surreal-rocksdb`.
pub async fn build_surreal(
    storage: Storage,
    topology: Topology,
    telemetry: Telemetry,
    shared: Option<SharedHandle>,
) -> Result<BackendHandle> {
    let (db, engine_path, temp_dir) = resolve_surreal_connection(storage, topology, shared).await?;

    let backend = SurrealLocalLogBackend::new(Arc::clone(&db))
        .await
        .context("SurrealLocalLogBackend::new")?;

    Ok(BackendHandle {
        backend: wrap_surreal(backend, telemetry),
        engine_path,
        _temp_dir: temp_dir,
        _shared: Some(SharedHandle::Surreal(db)),
    })
}

/// Build a remote TiKV-backed Surreal backend (`surreal-tikv` storage dimension).
pub async fn build_surreal_tikv(
    telemetry: Telemetry,
    shared: Option<SharedHandle>,
) -> Result<BackendHandle> {
    let (db, engine_path, temp_dir) =
        resolve_remote_surreal(shared).await.context("surreal-tikv remote connect")?;

    let backend = SurrealLocalLogBackend::new(Arc::clone(&db))
        .await
        .context("SurrealLocalLogBackend::new")?;

    Ok(BackendHandle {
        backend: wrap_surreal(backend, telemetry),
        engine_path,
        _temp_dir: temp_dir,
        _shared: Some(SharedHandle::Surreal(db)),
    })
}

async fn resolve_surreal_connection(
    storage: Storage,
    topology: Topology,
    shared: Option<SharedHandle>,
) -> Result<(Arc<Surreal<Any>>, String, Option<TempDir>)> {
    if topology == Topology::SharedHandle {
        if let Some(SharedHandle::Surreal(db)) = shared {
            return Ok((db, "shared-surreal-handle".into(), None));
        }
        return open_surreal(storage).await;
    }
    if topology == Topology::RemoteSurreal {
        return resolve_remote_surreal(shared).await;
    }
    open_surreal(storage).await
}

async fn resolve_remote_surreal(
    shared: Option<SharedHandle>,
) -> Result<(Arc<Surreal<Any>>, String, Option<TempDir>)> {
    if let Some(SharedHandle::Surreal(db)) = shared {
        let url = std::env::var("CONTINUUM_BENCH_SURREAL_URL")
            .unwrap_or_else(|_| "remote-surreal".into());
        return Ok((db, url, None));
    }
    let url = std::env::var("CONTINUUM_BENCH_SURREAL_URL")
        .context("CONTINUUM_BENCH_SURREAL_URL not set for remote surreal")?;
    let db = open_surreal_url(&url).await?;
    Ok((db, url, None))
}

async fn open_surreal(
    storage: Storage,
) -> Result<(Arc<Surreal<Any>>, String, Option<TempDir>)> {
    match storage {
        Storage::SurrealMem => {
            let db = open_surreal_url("mem://").await?;
            Ok((db, "mem://".into(), None))
        }
        Storage::SurrealRocksdb => {
            let dir = TempDir::new().context("tempdir for rocksdb")?;
            let path = dir.path().to_path_buf();
            let url = format!("rocksdb://{}", path.display());
            let db = open_surreal_url(&url).await?;
            Ok((db, url, Some(dir)))
        }
        _ => Err(anyhow!("not embedded surreal storage")),
    }
}

/// Connect to a Surreal endpoint and select the bench namespace/database.
pub async fn open_surreal_url(url: &str) -> Result<Arc<Surreal<Any>>> {
    let db: Surreal<Any> = Surreal::init();
    db.connect(url).await.context("surreal connect")?;

    if is_remote_surreal_url(url) {
        let username = std::env::var("CONTINUUM_BENCH_SURREAL_USER").unwrap_or_else(|_| "root".into());
        let password = std::env::var("CONTINUUM_BENCH_SURREAL_PASS").unwrap_or_else(|_| "root".into());
        db.signin(surrealdb::opt::auth::Root { username, password })
            .await
            .context("surreal signin")?;
    }

    db.use_ns("continuum")
        .use_db("bench")
        .await
        .context("surreal use_ns/db")?;
    Ok(Arc::new(db))
}

fn is_remote_surreal_url(url: &str) -> bool {
    url.starts_with("ws://")
        || url.starts_with("wss://")
        || url.starts_with("http://")
        || url.starts_with("https://")
}

/// Open a fresh shared Surreal handle for co-tenancy experiments.
pub async fn open_shared_surreal(storage: Storage) -> Result<(Arc<Surreal<Any>>, Option<TempDir>)> {
    let (db, _, temp) = open_surreal(storage).await?;
    Ok((db, temp))
}
