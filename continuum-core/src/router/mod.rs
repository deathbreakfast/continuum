//! Log destination routing and backend selection.
//!
//! At boot the host registers concrete [`crate::backend::LogBackend`] implementations on a
//! [`LogRouter`] keyed by [`crate::LogDestination`]. Topic-level routing uses a
//! [`LogEvaluator`] — typically [`LogFromDestination`] (single static destination) or
//! [`LogTopicRouter`] (longest-prefix rules with fallback).
//!
//! See also: [`crate::types::LogDestination`], [`crate::types::LogStreamId`].

mod evaluator;
mod key_hash_evaluator;
mod log_router;
mod router_key;

pub use evaluator::{
    resolve_stream, LogEvaluator, LogFromDestination, LogResolverContext, LogTopicRouter,
};
pub use key_hash_evaluator::KeyHashEvaluator;
pub use log_router::LogRouter;
pub use router_key::log_router_key;
