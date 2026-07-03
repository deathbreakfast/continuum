//! CQL schema bootstrap for the continuum transport log.

use scylla::client::session::Session;

use crate::error_map::{into_result, map_err};

pub async fn ensure_schema(
    session: &Session,
    keyspace: &str,
    replication_factor: u32,
) -> continuum_core::Result<()> {
    let rf = replication_factor.max(1);
    into_result(
        session
            .query_unpaged(
                format!(
                    "CREATE KEYSPACE IF NOT EXISTS {keyspace} WITH replication = {{'class': 'SimpleStrategy', 'replication_factor': {rf}}}"
                ),
                &[],
            )
            .await,
    )?;
    session.use_keyspace(keyspace, false).await.map_err(map_err)?;

    for ddl in [
        format!(
            "CREATE TABLE IF NOT EXISTS {keyspace}.continuum_event (
                stream_key text,
                seq bigint,
                event_id uuid,
                ts_millis bigint,
                attempt int,
                actor_ref text,
                payload_ciphertext blob,
                PRIMARY KEY ((stream_key), seq)
            ) WITH CLUSTERING ORDER BY (seq ASC)"
        ),
        format!(
            "CREATE TABLE IF NOT EXISTS {keyspace}.continuum_event_id (
                stream_key text,
                event_id uuid,
                seq bigint,
                PRIMARY KEY ((stream_key), event_id)
            )"
        ),
        format!(
            "CREATE TABLE IF NOT EXISTS {keyspace}.continuum_stream (
                stream_key text PRIMARY KEY,
                next_seq bigint
            )"
        ),
        format!(
            "CREATE TABLE IF NOT EXISTS {keyspace}.continuum_checkpoint (
                subscription text,
                stream_key text,
                seq bigint,
                PRIMARY KEY ((subscription), stream_key)
            )"
        ),
        format!(
            "CREATE TABLE IF NOT EXISTS {keyspace}.continuum_stream_index (
                topic_prefix text,
                stream_key text,
                PRIMARY KEY ((topic_prefix), stream_key)
            )"
        ),
    ] {
        into_result(session.query_unpaged(ddl, &[]).await)?;
    }
    Ok(())
}
