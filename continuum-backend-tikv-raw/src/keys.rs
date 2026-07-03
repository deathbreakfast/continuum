//! TiKV key encoding for the continuum transport log.

use continuum_core::error::{LogError, Result};
use continuum_core::types::Seq;
use tikv_client::Key;

const PREFIX: &str = "continuum/";

pub fn meta_key(stream_key: &str) -> Key {
    Key::from(format!("{PREFIX}s/{stream_key}/meta"))
}

pub fn event_prefix(stream_key: &str) -> String {
    format!("{PREFIX}s/{stream_key}/e/")
}

pub fn event_key(stream_key: &str, seq: Seq) -> Key {
    Key::from(format!("{}{:020}", event_prefix(stream_key), seq.as_i64()))
}

pub fn idempotency_key(stream_key: &str, event_id: &uuid::Uuid) -> Key {
    Key::from(format!("{PREFIX}s/{stream_key}/id/{event_id}"))
}

pub fn checkpoint_key(
    subscription: &continuum_core::types::SubscriptionId,
    stream_key: &str,
) -> Key {
    Key::from(format!("{PREFIX}cp/{}/{}", subscription.0, stream_key))
}

pub fn topic_index_prefix(topic_prefix: &str) -> String {
    format!("{PREFIX}topic/{topic_prefix}/streams/")
}

pub fn topic_stream_key(topic_prefix: &str, stream_key: &str) -> Key {
    Key::from(format!(
        "{}{stream_key}",
        topic_index_prefix(topic_prefix)
    ))
}

pub fn scan_end(prefix: &str) -> Key {
    let mut end = prefix.as_bytes().to_vec();
    end.push(0xff);
    Key::from(end)
}

pub fn seq_from_event_key(key: &[u8]) -> Result<Seq> {
    let text = std::str::from_utf8(key).map_err(|e| LogError::Backend(e.to_string()))?;
    let seq_part = text
        .rsplit('/')
        .next()
        .ok_or_else(|| LogError::Backend("missing seq in event key".into()))?;
    let seq = seq_part
        .parse::<i64>()
        .map_err(|e| LogError::Backend(e.to_string()))?;
    Ok(Seq(seq))
}

pub fn stream_key_from_topic_index(key: &[u8], topic_prefix: &str) -> Result<String> {
    let text = std::str::from_utf8(key).map_err(|e| LogError::Backend(e.to_string()))?;
    let prefix = topic_index_prefix(topic_prefix);
    text.strip_prefix(&prefix)
        .map(str::to_string)
        .ok_or_else(|| LogError::Backend("topic index key prefix mismatch".into()))
}
