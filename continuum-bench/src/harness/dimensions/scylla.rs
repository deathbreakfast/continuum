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
    pub const fn slug(self) -> &'static str {
        match self {
            Self::One => "scylla-1",
            Self::TwoNode => "scylla-2n",
            Self::ThreeNode => "scylla-3n",
            Self::FourNode => "scylla-4n",
            Self::Custom => "custom",
        }
    }

    /// Storage node count for this preset (excluding bench host).
    pub const fn storage_node_count(self) -> u8 {
        match self {
            Self::One => 1,
            Self::TwoNode => 2,
            Self::ThreeNode => 3,
            Self::FourNode => 4,
            Self::Custom => 0,
        }
    }

    /// Native campaign presets (1 → 2 → 4 node series).
    pub const fn native_presets() -> &'static [Self] {
        &[
            Self::One,
            Self::TwoNode,
            Self::FourNode,
        ]
    }

    /// Parse a preset slug from env or CLI.
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "scylla-1" | "1" | "one" => Some(Self::One),
            "scylla-2n" | "2n" | "scylla-2" => Some(Self::TwoNode),
            "scylla-3n" | "3n" | "scylla-3" => Some(Self::ThreeNode),
            "scylla-4n" | "4n" | "scylla-4" => Some(Self::FourNode),
            "custom" => Some(Self::Custom),
            _ => None,
        }
    }

    /// Read topology from `CONTINUUM_BENCH_SCYLLA_TOPOLOGY` when set.
    pub fn from_env() -> Option<Self> {
        std::env::var("CONTINUUM_BENCH_SCYLLA_TOPOLOGY")
            .ok()
            .and_then(|s| Self::parse(&s))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scylla_topology_parse_roundtrip() {
        for topo in ScyllaTopology::native_presets() {
            assert_eq!(ScyllaTopology::parse(topo.slug()), Some(*topo));
            assert!(topo.storage_node_count() > 0);
        }
        for topo in [
            ScyllaTopology::ThreeNode,
            ScyllaTopology::Custom,
        ] {
            assert_eq!(ScyllaTopology::parse(topo.slug()), Some(topo));
        }
    }
}
