//! Benchmark metrics: latency percentiles, growth, pass evaluation, resource profiling.

pub mod growth;
pub mod latency;
pub mod pass_eval;
pub mod resource_profile;

pub use growth::{growth_ratio, process_rss_bytes};
pub use latency::LatencySamples;
pub use pass_eval::{evaluate_pass, latency_to_json, results_summary};
pub use resource_profile::{ResourceProfiler, RunResourceProfile};
