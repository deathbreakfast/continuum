//! Append input and stored event records.
//!
//! # Payload policy
//!
//! The host envelope-encrypts payload bytes **before** [`crate::LogBackend::append`].
//! Continuum stores and returns **opaque ciphertext** only — it does not interpret schema
//! or log plaintext. The host decrypts above the port on read. Payloads are short-lived:
//! reclaim space with [`crate::LogBackend::truncate_before`] after delivery ack or TTL.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{LogDestination, Seq};

/// Input to append — sequence is assigned by the backend.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppendRecord {
    /// Idempotency key; re-append with the same id returns the existing sequence.
    pub event_id: Uuid,
    /// Publish timestamp.
    pub ts: DateTime<Utc>,
    /// Delivery / retry attempt counter.
    pub attempt: u32,
    /// Opaque actor reference (identifier only — not full identity JSON).
    pub actor_ref: Option<String>,
    /// Envelope-encrypted payload bytes (opaque to continuum).
    pub payload_ciphertext: Vec<u8>,
}

impl AppendRecord {
    /// Build a minimal append record for tests and examples.
    ///
    /// # Examples
    ///
    /// ```
    /// use continuum_core::AppendRecord;
    /// use uuid::Uuid;
    ///
    /// let record = AppendRecord::new(Uuid::new_v4(), vec![1, 2, 3]);
    /// assert_eq!(record.attempt, 0);
    /// assert!(record.actor_ref.is_none());
    /// ```
    #[must_use]
    pub fn new(event_id: Uuid, payload_ciphertext: Vec<u8>) -> Self {
        Self {
            event_id,
            ts: Utc::now(),
            attempt: 0,
            actor_ref: None,
            payload_ciphertext,
        }
    }
}

/// Stored record returned from read or append.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventRecord {
    /// Destination this record was appended under.
    pub destination: LogDestination,
    /// Idempotency key.
    pub event_id: Uuid,
    /// Topic at append time.
    pub topic: String,
    /// Optional partition key.
    pub key: Option<String>,
    /// Monotonic sequence within the stream.
    pub seq: Seq,
    /// Publish timestamp.
    pub ts: DateTime<Utc>,
    /// Delivery / retry attempt counter.
    pub attempt: u32,
    /// Opaque actor reference (id only).
    pub actor_ref: Option<String>,
    /// Envelope-encrypted payload bytes (opaque to continuum).
    pub payload_ciphertext: Vec<u8>,
}
