//! Map Scylla driver errors to [`LogError`](continuum_core::error::LogError).

use continuum_core::error::{LogError, Result};

pub fn map_err(err: impl std::fmt::Display) -> LogError {
    LogError::Backend(err.to_string())
}

pub fn into_result<T>(value: std::result::Result<T, impl std::fmt::Display>) -> Result<T> {
    value.map_err(map_err)
}
