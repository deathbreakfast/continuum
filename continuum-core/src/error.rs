//! Error types for the continuum log port and backends.

use thiserror::Error;

/// Errors from [`crate::backend::LogBackend`] and routing.
#[derive(Debug, Error)]
pub enum LogError {
    /// Invalid input (topic, limit, etc.).
    #[error("validation: {0}")]
    Validation(String),
    /// Requested stream or record does not exist.
    #[error("not found: {0}")]
    NotFound(String),
    /// Concurrent write conflict; caller may retry.
    #[error("conflict: {0}")]
    Conflict(String),
    /// Underlying storage engine failure.
    #[error("backend: {0}")]
    Backend(String),
    /// Invariant violation or lock poison.
    #[error("internal: {0}")]
    Internal(String),
    /// Backend or feature not implemented.
    #[error("unsupported: {0}")]
    Unsupported(String),
}

/// Result alias for continuum operations.
pub type Result<T> = std::result::Result<T, LogError>;

impl LogError {
    /// True when the caller may retry (conflict or transient backend fault).
    ///
/// # Examples
///
/// ```rust
/// use continuum_core::LogError;
///
/// assert!(LogError::Conflict("cas".into()).is_retryable());
/// assert!(!LogError::Validation("bad topic".into()).is_retryable());
/// assert!(LogError::Unsupported("stub".into()).is_retryable() == false);
/// ```
    #[must_use]
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::Conflict(_) => true,
            Self::Backend(msg) => {
                let s = msg.to_lowercase();
                s.contains("would block") || s.contains("temporarily unavailable")
            }
            _ => false,
        }
    }
}

impl From<std::io::Error> for LogError {
    fn from(e: std::io::Error) -> Self {
        Self::Backend(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conflict_is_retryable() {
        assert!(LogError::Conflict("cas".into()).is_retryable());
    }

    #[test]
    fn validation_not_retryable() {
        assert!(!LogError::Validation("bad".into()).is_retryable());
    }
}
