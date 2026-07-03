//! Surreal client handle for embedded local or dynamic `Any` engines.
//!
//! Unified query interface over either an injected remote/dynamic client or an embedded
//! local `RocksDB` handle.

use std::sync::Arc;

use surrealdb::Surreal;
use surrealdb::engine::any::Any;
use surrealdb::engine::local::Db;

use continuum_core::error::Result;

use super::error_map::map_err;

/// Either an injected remote/dynamic client or an embedded local `RocksDB` handle.
#[derive(Clone)]
pub enum DbConn {
    /// Remote or in-memory `Any` engine.
    Any(Arc<Surreal<Any>>),
    /// Embedded local `RocksDB` handle.
    Local(Arc<Surreal<Db>>),
}

impl DbConn {
    /// Wrap a dynamic Surreal client.
    pub const fn any(db: Arc<Surreal<Any>>) -> Self {
        Self::Any(db)
    }

    /// Wrap an embedded local `RocksDB` Surreal client.
    pub const fn local(db: Arc<Surreal<Db>>) -> Self {
        Self::Local(db)
    }

    pub(crate) async fn query(&self, sql: &str) -> Result<surrealdb::IndexedResults> {
        match self {
            Self::Any(db) => db.query(sql).await.map_err(|e| map_err(&e)),
            Self::Local(db) => db.query(sql).await.map_err(|e| map_err(&e)),
        }
    }

    pub(crate) async fn query_sk_id(
        &self,
        sql: &str,
        sk: String,
        id: String,
    ) -> Result<surrealdb::IndexedResults> {
        match self {
            Self::Any(db) => db
                .query(sql)
                .bind(("sk", sk))
                .bind(("id", id))
                .await
                .map_err(|e| map_err(&e)),
            Self::Local(db) => db
                .query(sql)
                .bind(("sk", sk))
                .bind(("id", id))
                .await
                .map_err(|e| map_err(&e)),
        }
    }

    pub(crate) async fn query_sk(&self, sql: &str, sk: String) -> Result<surrealdb::IndexedResults> {
        match self {
            Self::Any(db) => db.query(sql).bind(("sk", sk)).await.map_err(|e| map_err(&e)),
            Self::Local(db) => db.query(sql).bind(("sk", sk)).await.map_err(|e| map_err(&e)),
        }
    }

    pub(crate) async fn upsert_stream_seq(
        &self,
        sk: String,
        next: i64,
    ) -> Result<surrealdb::IndexedResults> {
        let sql =
            "UPSERT continuum_stream SET stream_key = $sk, next_seq = $next WHERE stream_key = $sk";
        match self {
            Self::Any(db) => db
                .query(sql)
                .bind(("sk", sk))
                .bind(("next", next))
                .await
                .map_err(|e| map_err(&e)),
            Self::Local(db) => db
                .query(sql)
                .bind(("sk", sk))
                .bind(("next", next))
                .await
                .map_err(|e| map_err(&e)),
        }
    }

    pub(crate) async fn query_stream_read(
        &self,
        stream_key: String,
        after: i64,
        limit: i64,
    ) -> Result<surrealdb::IndexedResults> {
        let sql = r"
            SELECT seq, event_id, ts_millis, attempt, actor_ref, payload_ciphertext
            FROM continuum_event
            WHERE stream_key = $stream_key AND seq > $after
            ORDER BY seq ASC
            LIMIT $limit
        ";
        match self {
            Self::Any(db) => db
                .query(sql)
                .bind(("stream_key", stream_key))
                .bind(("after", after))
                .bind(("limit", limit))
                .await
                .map_err(|e| map_err(&e)),
            Self::Local(db) => db
                .query(sql)
                .bind(("stream_key", stream_key))
                .bind(("after", after))
                .bind(("limit", limit))
                .await
                .map_err(|e| map_err(&e)),
        }
    }

    pub(crate) async fn query_topic_read(
        &self,
        topic_prefix: String,
        after: i64,
        limit: i64,
    ) -> Result<surrealdb::IndexedResults> {
        let sql = r"
            SELECT seq, event_id, ts_millis, attempt, actor_ref, payload_ciphertext, stream_key
            FROM continuum_event
            WHERE string::starts_with(stream_key, $topic_prefix) AND seq > $after
            ORDER BY seq ASC
            LIMIT $limit
        ";
        match self {
            Self::Any(db) => db
                .query(sql)
                .bind(("topic_prefix", topic_prefix))
                .bind(("after", after))
                .bind(("limit", limit))
                .await
                .map_err(|e| map_err(&e)),
            Self::Local(db) => db
                .query(sql)
                .bind(("topic_prefix", topic_prefix))
                .bind(("after", after))
                .bind(("limit", limit))
                .await
                .map_err(|e| map_err(&e)),
        }
    }

    pub(crate) async fn upsert_checkpoint(
        &self,
        sub: String,
        stream_key: String,
        seq: i64,
    ) -> Result<surrealdb::IndexedResults> {
        let sql = r"
            UPSERT continuum_checkpoint SET
                subscription = $sub,
                stream_key = $stream_key,
                seq = $seq
            WHERE subscription = $sub AND stream_key = $stream_key
        ";
        match self {
            Self::Any(db) => db
                .query(sql)
                .bind(("sub", sub))
                .bind(("stream_key", stream_key))
                .bind(("seq", seq))
                .await
                .map_err(|e| map_err(&e)),
            Self::Local(db) => db
                .query(sql)
                .bind(("sub", sub))
                .bind(("stream_key", stream_key))
                .bind(("seq", seq))
                .await
                .map_err(|e| map_err(&e)),
        }
    }

    pub(crate) async fn query_checkpoint(
        &self,
        sub: String,
        stream_key: String,
    ) -> Result<surrealdb::IndexedResults> {
        let sql = r"
            SELECT seq FROM continuum_checkpoint
            WHERE subscription = $sub AND stream_key = $stream_key
            LIMIT 1
        ";
        match self {
            Self::Any(db) => db
                .query(sql)
                .bind(("sub", sub))
                .bind(("stream_key", stream_key))
                .await
                .map_err(|e| map_err(&e)),
            Self::Local(db) => db
                .query(sql)
                .bind(("sub", sub))
                .bind(("stream_key", stream_key))
                .await
                .map_err(|e| map_err(&e)),
        }
    }

    pub(crate) async fn count_truncate(
        &self,
        stream_key: String,
        bound: i64,
    ) -> Result<surrealdb::IndexedResults> {
        let sql = "SELECT count() AS count FROM continuum_event WHERE stream_key = $stream_key AND seq < $bound GROUP ALL";
        match self {
            Self::Any(db) => db
                .query(sql)
                .bind(("stream_key", stream_key.clone()))
                .bind(("bound", bound))
                .await
                .map_err(|e| map_err(&e)),
            Self::Local(db) => db
                .query(sql)
                .bind(("stream_key", stream_key))
                .bind(("bound", bound))
                .await
                .map_err(|e| map_err(&e)),
        }
    }

    pub(crate) async fn delete_truncate(
        &self,
        stream_key: String,
        bound: i64,
    ) -> Result<surrealdb::IndexedResults> {
        let sql = "DELETE continuum_event WHERE stream_key = $stream_key AND seq < $bound";
        match self {
            Self::Any(db) => db
                .query(sql)
                .bind(("stream_key", stream_key))
                .bind(("bound", bound))
                .await
                .map_err(|e| map_err(&e)),
            Self::Local(db) => db
                .query(sql)
                .bind(("stream_key", stream_key))
                .bind(("bound", bound))
                .await
                .map_err(|e| map_err(&e)),
        }
    }
}
