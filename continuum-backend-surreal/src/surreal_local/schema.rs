//! Bootstrap Surreal tables and indexes for the transport log.
//!
//! Defines `continuum_event`, `continuum_stream`, and `continuum_checkpoint` with indexes
//! for idempotent append (`stream_key`, `event_id`) and ordered read (`stream_key`, `seq`).

use continuum_core::error::Result;

use super::db_conn::DbConn;

const SCHEMA: &str = r"
DEFINE TABLE IF NOT EXISTS continuum_event SCHEMAFULL;
DEFINE FIELD stream_key ON TABLE continuum_event TYPE string;
DEFINE FIELD seq ON TABLE continuum_event TYPE int;
DEFINE FIELD event_id ON TABLE continuum_event TYPE string;
DEFINE FIELD ts_millis ON TABLE continuum_event TYPE int;
DEFINE FIELD attempt ON TABLE continuum_event TYPE int;
DEFINE FIELD actor_ref ON TABLE continuum_event TYPE option<string>;
DEFINE FIELD payload_ciphertext ON TABLE continuum_event TYPE array<int>;
DEFINE INDEX continuum_event_id ON TABLE continuum_event FIELDS stream_key, event_id UNIQUE;
DEFINE INDEX continuum_stream_seq ON TABLE continuum_event FIELDS stream_key, seq;

DEFINE TABLE IF NOT EXISTS continuum_stream SCHEMAFULL;
DEFINE FIELD stream_key ON TABLE continuum_stream TYPE string;
DEFINE FIELD next_seq ON TABLE continuum_stream TYPE int DEFAULT 0;
DEFINE INDEX continuum_stream_key ON TABLE continuum_stream FIELDS stream_key UNIQUE;

DEFINE TABLE IF NOT EXISTS continuum_checkpoint SCHEMAFULL;
DEFINE FIELD subscription ON TABLE continuum_checkpoint TYPE string;
DEFINE FIELD stream_key ON TABLE continuum_checkpoint TYPE string;
DEFINE FIELD seq ON TABLE continuum_checkpoint TYPE int;
DEFINE INDEX continuum_checkpoint_key ON TABLE continuum_checkpoint FIELDS subscription, stream_key UNIQUE;
";

/// Create tables and indexes if missing.
pub async fn ensure_schema(db: &DbConn) -> Result<()> {
    db.query(SCHEMA).await?;
    Ok(())
}
