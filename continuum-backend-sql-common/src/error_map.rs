//! Map sqlx errors to [`LogError`](continuum_core::error::LogError).

use continuum_core::error::LogError;

pub fn map_err(e: sqlx::Error) -> LogError {
    match e {
        sqlx::Error::Database(db) if db.is_unique_violation() => {
            LogError::Conflict(db.message().to_string())
        }
        other => LogError::Backend(other.to_string()),
    }
}
