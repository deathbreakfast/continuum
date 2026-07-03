//! Hash partition keys to one of N registered destinations (multi-cell routing).

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use async_trait::async_trait;

use crate::error::Result;
use crate::types::LogDestination;

use super::{LogEvaluator, LogResolverContext};

/// Routes topics to a destination by `hash(key) % N` for multi-cluster / cell layouts.
///
/// Use when a single topic is sharded across several storage cells: the same
/// partition key always resolves to the same [`LogDestination`], while different
/// keys spread across the registered destinations. A missing key (`None`) always
/// maps to the first destination.
///
/// # Examples
///
/// ```rust
/// # use continuum_core::{
/// #     KeyHashEvaluator, LogBackendKind, LogDestination, LogEvaluator, LogResolverContext,
/// # };
/// # #[tokio::main]
/// # async fn main() -> continuum_core::Result<()> {
/// let destinations = vec![
///     LogDestination::new("cell-0", LogBackendKind::Memory),
///     LogDestination::new("cell-1", LogBackendKind::Memory),
/// ];
/// let eval = KeyHashEvaluator::new(destinations);
/// let ctx = LogResolverContext::default();
/// let a = eval.resolve_for_topic(&ctx, "events", Some("user-42")).await?;
/// let b = eval.resolve_for_topic(&ctx, "events", Some("user-42")).await?;
/// assert_eq!(a, b);
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct KeyHashEvaluator {
    destinations: Vec<LogDestination>,
}

impl KeyHashEvaluator {
    /// Build an evaluator over `destinations` (must be non-empty).
    ///
    /// # Panics
    ///
    /// Panics when `destinations` is empty.
    #[must_use]
    pub fn new(destinations: Vec<LogDestination>) -> Self {
        assert!(
            !destinations.is_empty(),
            "KeyHashEvaluator requires at least one destination"
        );
        Self { destinations }
    }

    fn index_for_key(key: Option<&str>, len: usize) -> usize {
        match key {
            Some(k) => {
                let mut hasher = DefaultHasher::new();
                k.hash(&mut hasher);
                (hasher.finish() as usize) % len
            }
            None => 0,
        }
    }
}

#[async_trait]
impl LogEvaluator for KeyHashEvaluator {
    fn name(&self) -> &'static str {
        "KeyHashEvaluator"
    }

    async fn resolve_for_topic(
        &self,
        _ctx: &LogResolverContext,
        _topic: &str,
        key: Option<&str>,
    ) -> Result<LogDestination> {
        let idx = Self::index_for_key(key, self.destinations.len());
        Ok(self.destinations[idx].clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::LogBackendKind;

    #[tokio::test]
    async fn same_key_same_destination() {
        let dests: Vec<_> = (0..4)
            .map(|i| LogDestination::new(format!("cell-{i}"), LogBackendKind::Scylla))
            .collect();
        let eval = KeyHashEvaluator::new(dests.clone());
        let ctx = LogResolverContext::default();
        let a = eval
            .resolve_for_topic(&ctx, "events", Some("user-42"))
            .await
            .expect("resolve");
        let b = eval
            .resolve_for_topic(&ctx, "events", Some("user-42"))
            .await
            .expect("resolve");
        assert_eq!(a, b);
        assert!(dests.contains(&a));
    }

    #[tokio::test]
    async fn none_key_uses_cell_zero() {
        let dests = vec![
            LogDestination::new("cell-0", LogBackendKind::Scylla),
            LogDestination::new("cell-1", LogBackendKind::Scylla),
        ];
        let eval = KeyHashEvaluator::new(dests);
        let got = eval
            .resolve_for_topic(&LogResolverContext::default(), "events", None)
            .await
            .expect("resolve");
        assert_eq!(got.logical, "cell-0");
    }
}
