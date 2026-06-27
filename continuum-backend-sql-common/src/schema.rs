//! Bootstrap SQL tables and indexes for the transport log.

use continuum_core::error::Result;

use crate::{SqlDialect, SqlLogBackend};

fn event_table(dialect: SqlDialect) -> String {
    let payload_ty = match dialect {
        SqlDialect::Postgres => "BYTEA",
        SqlDialect::Sqlite => "BLOB",
    };
    format!(
        r"
CREATE TABLE IF NOT EXISTS continuum_event (
    stream_key TEXT NOT NULL,
    seq BIGINT NOT NULL,
    event_id TEXT NOT NULL,
    ts_millis BIGINT NOT NULL,
    attempt INTEGER NOT NULL,
    actor_ref TEXT,
    payload_ciphertext {payload_ty} NOT NULL
)"
    )
}

const STREAM_TABLE: &str = r"
CREATE TABLE IF NOT EXISTS continuum_stream (
    stream_key TEXT PRIMARY KEY,
    next_seq BIGINT NOT NULL DEFAULT 0
)";

const CHECKPOINT_TABLE: &str = r"
CREATE TABLE IF NOT EXISTS continuum_checkpoint (
    subscription TEXT NOT NULL,
    stream_key TEXT NOT NULL,
    seq BIGINT NOT NULL,
    PRIMARY KEY (subscription, stream_key)
)";

/// Create tables and indexes if missing.
pub async fn ensure_schema(backend: &SqlLogBackend) -> Result<()> {
    // Concurrent `CREATE TABLE IF NOT EXISTS` on PostgreSQL can race on internal
    // catalog entries (`pg_type_typname_nsp_index`) when multiple pools bootstrap
    // at once (e.g. integration tests). Serialize bootstrap cluster-wide.
    let lock = backend.dialect() == SqlDialect::Postgres;
    if lock {
        backend.run_ddl("SELECT pg_advisory_lock(872349012)").await?;
    }
    let result = ensure_schema_tables(backend).await;
    if lock {
        backend.run_ddl("SELECT pg_advisory_unlock(872349012)").await?;
    }
    result
}

async fn ensure_schema_tables(backend: &SqlLogBackend) -> Result<()> {
    let dialect = backend.dialect();
    for ddl in [
        event_table(dialect).as_str(),
        STREAM_TABLE,
        CHECKPOINT_TABLE,
    ] {
        backend.run_ddl(ddl).await?;
    }

    backend
        .run_ddl(
            "CREATE UNIQUE INDEX IF NOT EXISTS continuum_event_id ON continuum_event (stream_key, event_id)",
        )
        .await?;

    backend
        .run_ddl(
            "CREATE INDEX IF NOT EXISTS continuum_stream_seq ON continuum_event (stream_key, seq)",
        )
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::SqlLogBackend;

    #[tokio::test]
    async fn schema_idempotent_sqlite() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.db");
        let url = format!("sqlite://{}?mode=rwc", path.display());
        let b1 = SqlLogBackend::connect_sqlite(&url).await.unwrap();
        let b2 = SqlLogBackend::connect_sqlite(&url).await.unwrap();
        drop(b1);
        drop(b2);
    }
}
