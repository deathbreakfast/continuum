//! Map TiKV client errors to [`LogError`](continuum_core::error::LogError).

use continuum_core::error::LogError;

pub fn map_err(err: impl std::fmt::Display) -> LogError {
    LogError::Backend(err.to_string())
}
