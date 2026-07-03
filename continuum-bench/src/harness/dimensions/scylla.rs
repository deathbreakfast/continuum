//! Scylla cluster topology presets for native adapter campaigns.

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

/// Scylla cluster topology preset from `infra/scylla/` or `infra/native-aws/`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum ScyllaTopology {
    #[value(name = "scylla-1")]
    One,
    #[value(name = "scylla-3n")]
    ThreeNode,
    #[value(name = "custom")]
    Custom,
}

impl ScyllaTopology {
    /// Short slug for report filenames and env export.
    pub fn slug(self) -> &'static str {
        match self {
            ScyllaTopology::One => "scylla-1",
            ScyllaTopology::ThreeNode => "scylla-3n",
            ScyllaTopology::Custom => "custom",
        }
    }

    /// Native campaign presets.
    pub fn native_presets() -> &'static [ScyllaTopology] {
        &[ScyllaTopology::One, ScyllaTopology::ThreeNode]
    }

    /// Parse a preset slug from env or CLI.
    pub fn parse(s: &str) -> Option<ScyllaTopology> {
        match s.to_ascii_lowercase().as_str() {
            "scylla-1" | "1" | "one" => Some(ScyllaTopology::One),
            "scylla-3n" | "3n" | "scylla-3" => Some(ScyllaTopology::ThreeNode),
            "custom" => Some(ScyllaTopology::Custom),
            _ => None,
        }
    }

    /// Read topology from `CONTINUUM_BENCH_SCYLLA_TOPOLOGY` when set.
    pub fn from_env() -> Option<ScyllaTopology> {
        std::env::var("CONTINUUM_BENCH_SCYLLA_TOPOLOGY")
            .ok()
            .and_then(|s| ScyllaTopology::parse(&s))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scylla_topology_parse_roundtrip() {
        for topo in ScyllaTopology::native_presets() {
            assert_eq!(ScyllaTopology::parse(topo.slug()), Some(*topo));
        }
    }
}
