//! Surreal backend configuration.
//!
//! Optional namespace/database selection when the host has not already configured the client.

/// Namespace/database selection for [`super::SurrealLocalLogBackend`].
#[derive(Debug, Clone)]
pub struct SurrealLogConfig {
    /// Surreal namespace (optional — host may already select ns/db on the client).
    pub namespace: Option<String>,
    /// Surreal database name.
    pub database: Option<String>,
}

impl Default for SurrealLogConfig {
    fn default() -> Self {
        Self {
            namespace: Some("continuum".into()),
            database: Some("transport".into()),
        }
    }
}
