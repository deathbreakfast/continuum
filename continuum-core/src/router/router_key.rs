//! Compound router keys: `"{kind_prefix}:{logical}"`.

use crate::types::LogBackendKind;

/// Build the compound router key used for registration and resolution.
#[must_use]
pub fn log_router_key(logical: &str, kind: LogBackendKind) -> String {
    let prefix = match kind {
        LogBackendKind::SurrealLocal => "surreal",
        LogBackendKind::Memory => "memory",
        LogBackendKind::Postgres => "postgres",
        LogBackendKind::Sqlite => "sqlite",
        LogBackendKind::Scylla => "scylla",
        LogBackendKind::TikvRaw => "tikv",
    };
    format!("{prefix}:{logical}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::LogBackendKind;

    #[test]
    fn surreal_default_key() {
        assert_eq!(log_router_key("default", LogBackendKind::SurrealLocal), "surreal:default");
    }
}
