//! Named backend registry.
//!
//! Maps logical [`LogDestination`] names to concrete [`LogBackend`] implementations.
//! The host registers backends during startup; [`LogRouter::resolve_backend`] looks up
//! by destination at runtime.
//!
//! See also: [`crate::LogEvaluator`], [`crate::types::LogDestination`].

use std::collections::HashMap;
use std::sync::{Arc, OnceLock, RwLock};

use crate::backend::LogBackend;
use crate::error::{LogError, Result};
use crate::types::LogDestination;

static GLOBAL_ROUTER: OnceLock<Arc<LogRouter>> = OnceLock::new();

/// Maps logical destinations to concrete [`LogBackend`] implementations.
///
/// # Examples
///
/// ```rust
/// # use std::sync::Arc;
/// # use continuum_core::{LogBackendKind, LogDestination, LogRouter};
/// # use continuum_backend_mem::InMemoryLogBackend;
/// let dest = LogDestination::new("default", LogBackendKind::Memory);
/// let backend = Arc::new(InMemoryLogBackend::new());
/// let router = LogRouter::with_default(&dest, backend);
/// let resolved = router.resolve_backend(&dest).expect("registered");
/// let _ = resolved;
/// ```
#[derive(Debug)]
pub struct LogRouter {
    backends: RwLock<HashMap<String, Arc<dyn LogBackend>>>,
}

impl Default for LogRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl LogRouter {
    /// Empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            backends: RwLock::new(HashMap::new()),
        }
    }

    /// Register a single default destination.
    pub fn with_default(destination: &LogDestination, backend: Arc<dyn LogBackend>) -> Self {
        let mut router = Self::new();
        router.register(destination, backend);
        router
    }

    /// Register during initial setup.
    ///
    /// # Panics
    ///
    /// Panics if the internal lock is poisoned.
    pub fn register(&mut self, destination: &LogDestination, backend: Arc<dyn LogBackend>) {
        self.backends
            .write()
            .expect("router lock not poisoned")
            .insert(destination.router_key(), backend);
    }

    /// Register at runtime after [`Self::set_global`].
    ///
    /// # Errors
    ///
    /// Returns an error if the internal lock is poisoned.
    pub fn register_runtime(
        &self,
        destination: &LogDestination,
        backend: Arc<dyn LogBackend>,
    ) -> Result<()> {
        self.backends
            .write()
            .map_err(|_| LogError::Internal("router lock poisoned".into()))?
            .insert(destination.router_key(), backend);
        Ok(())
    }

    /// Resolve a backend by destination.
    ///
    /// # Errors
    ///
    /// Returns an error if the lock is poisoned or the destination is unknown.
    pub fn resolve_backend(&self, destination: &LogDestination) -> Result<Arc<dyn LogBackend>> {
        let key = destination.router_key();
        self.backends
            .read()
            .map_err(|_| LogError::Internal("router lock poisoned".into()))?
            .get(&key)
            .cloned()
            .ok_or_else(|| LogError::Internal(format!("unknown log backend: {key}")))
    }

    /// Install the process-global router.
    pub fn set_global(router: Self) {
        let _ = GLOBAL_ROUTER.set(Arc::new(router));
    }

    /// Global router (panics if unset).
    ///
    /// # Panics
    ///
    /// Panics if [`Self::set_global`] was not called.
    pub fn global() -> Arc<Self> {
        GLOBAL_ROUTER
            .get()
            .cloned()
            .expect("LogRouter::set_global was not called")
    }

    /// Optional global router.
    pub fn try_global() -> Option<Arc<Self>> {
        GLOBAL_ROUTER.get().cloned()
    }
}
