//! Scylla cluster topology presets for native adapter campaigns.

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

/// Scylla cluster topology preset from `infra/scylla/` or `infra/native-aws/`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum ScyllaTopology {
    #[value(name = "scylla-1")]
    One,
    #[value(name = "scylla-2n")]
    TwoNode,
    #[value(name = "scylla-3n")]
    ThreeNode,
    #[value(name = "scylla-4n")]
    FourNode,
    #[value(name = "custom")]
    Custom,
}

impl ScyllaTopology {
    /// Short slug for report filenames and env export.
    pub fn slug(self) -> &'static str {
        match self {
            ScyllaTopology::One => "scylla-1",
            ScyllaTopology::TwoNode => "scylla-2n",
            ScyllaTopology::ThreeNode => "scylla-3n",
            ScyllaTopology::FourNode => "scylla-4n",
            ScyllaTopology::Custom => "custom",
        }
    }

    /// Storage node count for this preset (excluding bench host).
    pub fn storage_node_count(self) -> u8 {
        match self {
            ScyllaTopology::One => 1,
            ScyllaTopology::TwoNode => 2,
            ScyllaTopology::ThreeNode => 3,
            ScyllaTopology::FourNode => 4,
            ScyllaTopology::Custom => 0,
        }
    }

    /// Native campaign presets (1 → 2 → 4 node series).
    pub fn native_presets() -> &'static [ScyllaTopology] {
        &[
            ScyllaTopology::One,
            ScyllaTopology::TwoNode,
            ScyllaTopology::FourNode,
        ]
    }

    /// Parse a preset slug from env or CLI.
    pub fn parse(s: &str) -> Option<ScyllaTopology> {
        match s.to_ascii_lowercase().as_str() {
            "scylla-1" | "1" | "one" => Some(ScyllaTopology::One),
            "scylla-2n" | "2n" | "scylla-2" => Some(ScyllaTopology::TwoNode),
            "scylla-3n" | "3n" | "scylla-3" => Some(ScyllaTopology::ThreeNode),
            "scylla-4n" | "4n" | "scylla-4" => Some(ScyllaTopology::FourNode),
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
        for topo in [
            ScyllaTopology::One,
            ScyllaTopology::TwoNode,
            ScyllaTopology::ThreeNode,
            ScyllaTopology::FourNode,
        ] {
            assert_eq!(ScyllaTopology::parse(topo.slug()), Some(topo));
            assert!(topo.storage_node_count() > 0 || topo == ScyllaTopology::Custom);
        }
    }
}
