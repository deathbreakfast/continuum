//! Destination resolution from topics.
//!
//! Use [`LogFromDestination`] when all topics share one backend, or [`LogTopicRouter`] for
//! prefix-based routing. [`resolve_stream`] combines destination resolution with
//! [`super::LogRouter`] backend lookup.

use std::fmt::Debug;

use async_trait::async_trait;

use crate::error::Result;
use crate::router::LogRouter;
use crate::types::LogDestination;

/// Context for destination evaluators (populated by the host).
#[derive(Debug, Clone, Default)]
pub struct LogResolverContext {
    /// Deployment environment label.
    pub environment: Option<String>,
    /// Cell id when running split workloads.
    pub cell_id: Option<String>,
}

/// Resolves a [`LogDestination`] from a topic (and optional key).
#[async_trait]
pub trait LogEvaluator: Send + Sync + Debug + 'static {
    /// Evaluator name for logs and diagnostics.
    fn name(&self) -> &'static str;

    /// Resolve destination for a topic (and optional partition key).
    async fn resolve_for_topic(
        &self,
        ctx: &LogResolverContext,
        topic: &str,
        key: Option<&str>,
    ) -> Result<LogDestination>;
}

/// Static destination for all topics (single-backend default).
///
/// Wraps a [`LogDestination`] and returns it unchanged for every topic. Pair with
/// [`super::LogRouter::with_default`] at boot for single-backend setups.
///
/// # Examples
///
/// ```rust
/// # use continuum_core::{
/// #     LogBackendKind, LogDestination, LogEvaluator, LogFromDestination, LogResolverContext,
/// # };
/// # #[tokio::main]
/// # async fn main() -> continuum_core::Result<()> {
/// let dest = LogDestination::new("default", LogBackendKind::Memory);
/// let evaluator = LogFromDestination(dest.clone());
/// let got = evaluator
///     .resolve_for_topic(&LogResolverContext::default(), "events", None)
///     .await?;
/// assert_eq!(got, dest);
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct LogFromDestination(
    /// Destination returned for every topic.
    pub LogDestination,
);

#[async_trait]
impl LogEvaluator for LogFromDestination {
    fn name(&self) -> &'static str {
        "LogFromDestination"
    }

    async fn resolve_for_topic(
        &self,
        _ctx: &LogResolverContext,
        _topic: &str,
        _key: Option<&str>,
    ) -> Result<LogDestination> {
        Ok(self.0.clone())
    }
}

/// Topic-prefix routing rules with fallback (longest prefix wins).
///
/// # Examples
///
/// ```rust
/// # use continuum_core::{LogBackendKind, LogDestination, LogEvaluator, LogResolverContext, LogTopicRouter};
/// # #[tokio::main]
/// # async fn main() {
/// let fallback = LogDestination::new("default", LogBackendKind::Memory);
/// let metrics = LogDestination::new("metrics", LogBackendKind::Memory);
/// let router = LogTopicRouter::new(fallback.clone())
///     .prefix("metrics.", metrics.clone());
/// let ctx = LogResolverContext::default();
/// let dest = router
///     .resolve_for_topic(&ctx, "metrics.cache_hits", None)
///     .await
///     .expect("resolve");
/// assert_eq!(dest, metrics);
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct LogTopicRouter {
    rules: Vec<(String, LogDestination)>,
    fallback: LogDestination,
}

impl LogTopicRouter {
    /// Empty router — use [`Self::with_fallback`] or [`Self::prefix`].
    #[must_use]
    pub const fn new(fallback: LogDestination) -> Self {
        Self {
            rules: Vec::new(),
            fallback,
        }
    }

    /// Add a prefix rule (longest match wins).
    #[must_use]
    pub fn prefix(mut self, prefix: impl Into<String>, destination: LogDestination) -> Self {
        self.rules.push((prefix.into(), destination));
        self
    }

    /// Set fallback destination for unmatched topics.
    #[must_use]
    pub fn with_fallback(mut self, fallback: LogDestination) -> Self {
        self.fallback = fallback;
        self
    }
}

#[async_trait]
impl LogEvaluator for LogTopicRouter {
    fn name(&self) -> &'static str {
        "LogTopicRouter"
    }

    async fn resolve_for_topic(
        &self,
        _ctx: &LogResolverContext,
        topic: &str,
        _key: Option<&str>,
    ) -> Result<LogDestination> {
        let mut best: Option<&(String, LogDestination)> = None;
        for rule in &self.rules {
            if topic.starts_with(&rule.0)
                && best.is_none_or(|b| rule.0.len() > b.0.len())
            {
                best = Some(rule);
            }
        }
        Ok(best.map_or_else(|| self.fallback.clone(), |(_, d)| d.clone()))
    }
}

/// Resolve destination then backend in one step.
///
/// # Examples
///
/// ```rust
/// # use std::sync::Arc;
/// # use continuum_core::{
/// #     LogBackendKind, LogDestination, LogEvaluator, LogFromDestination, LogResolverContext,
/// #     LogRouter,
/// # };
/// # use continuum_core::router::resolve_stream;
/// # use continuum_backend_mem::InMemoryLogBackend;
/// # #[tokio::main]
/// # async fn main() -> continuum_core::Result<()> {
/// let dest = LogDestination::new("default", LogBackendKind::Memory);
/// let backend = Arc::new(InMemoryLogBackend::new());
/// let router = LogRouter::with_default(&dest, backend);
/// let evaluator = LogFromDestination(dest);
/// let (resolved_dest, resolved_backend) = resolve_stream(
///     &evaluator,
///     &router,
///     &LogResolverContext::default(),
///     "events",
///     None,
/// )
/// .await?;
/// assert_eq!(resolved_dest.logical, "default");
/// let _ = resolved_backend;
/// # Ok(())
/// # }
/// ```
///
/// # Errors
///
/// Returns an error when destination resolution or backend lookup fails.
pub async fn resolve_stream(
    evaluator: &dyn LogEvaluator,
    router: &LogRouter,
    ctx: &LogResolverContext,
    topic: &str,
    key: Option<String>,
) -> Result<(LogDestination, std::sync::Arc<dyn crate::backend::LogBackend>)> {
    let dest = evaluator.resolve_for_topic(ctx, topic, key.as_deref()).await?;
    let backend = router.resolve_backend(&dest)?;
    Ok((dest, backend))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::LogBackendKind;

    #[tokio::test]
    async fn from_destination_returns_static() {
        let dest = LogDestination::new("default", LogBackendKind::Memory);
        let eval = LogFromDestination(dest.clone());
        let got = eval
            .resolve_for_topic(&LogResolverContext::default(), "any.topic", None)
            .await
            .expect("resolve");
        assert_eq!(got, dest);
    }

    #[tokio::test]
    async fn topic_router_longest_prefix() {
        let fallback = LogDestination::new("default", LogBackendKind::SurrealLocal);
        let metrics = LogDestination::new("metrics", LogBackendKind::SurrealLocal);
        let router = LogTopicRouter::new(fallback.clone())
            .prefix("metrics.", metrics.clone());
        let ctx = LogResolverContext::default();
        assert_eq!(
            router
                .resolve_for_topic(&ctx, "metrics.cache_hits", None)
                .await
                .expect("resolve"),
            metrics
        );
        assert_eq!(
            router
                .resolve_for_topic(&ctx, "other", None)
                .await
                .expect("resolve"),
            fallback
        );
    }
}
