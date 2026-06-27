//! Input validation helpers.

use crate::error::{LogError, Result};

/// Maximum `read_from` batch size.
pub const MAX_READ_LIMIT: usize = 10_000;

/// Reject empty topic names.
///
/// # Errors
///
/// Returns [`LogError::Validation`] when `topic` is empty.
pub fn validate_topic(topic: &str) -> Result<()> {
    if topic.is_empty() {
        return Err(LogError::Validation("topic must not be empty".into()));
    }
    Ok(())
}

/// Reject excessive read limits.
///
/// # Errors
///
/// Returns [`LogError::Validation`] when `limit` exceeds [`MAX_READ_LIMIT`].
pub fn validate_read_limit(limit: usize) -> Result<()> {
    if limit > MAX_READ_LIMIT {
        return Err(LogError::Validation(format!(
            "read limit {limit} exceeds maximum {MAX_READ_LIMIT}"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_topic_rejected() {
        assert!(validate_topic("").is_err());
    }

    #[test]
    fn excessive_limit_rejected() {
        assert!(validate_read_limit(MAX_READ_LIMIT + 1).is_err());
    }
}
