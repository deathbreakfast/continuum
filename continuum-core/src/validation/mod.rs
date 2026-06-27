//! Request validation helpers.
//!
//! Backends call these before append and read operations. Violations return
//! [`crate::LogError::Validation`].
//!
//! See also: [`crate::backend::LogBackend`], [`crate::types::AppendRecord`].

mod record;

pub use record::{validate_read_limit, validate_topic, MAX_READ_LIMIT};
