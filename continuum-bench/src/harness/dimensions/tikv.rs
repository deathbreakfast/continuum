//! `TiKV` topology and Surreal deployment dimensions for distributed campaigns.
//!
//! **Internal — performance engineers.** Env vars:
//! - `CONTINUUM_BENCH_TIKV_TOPOLOGY` — preset slug (e.g. `tikv-ha-3`)
//! - `CONTINUUM_BENCH_SURREAL_INSTANCES` — Surreal node count behind LB
//! - `CONTINUUM_BENCH_SURREAL_HARDWARE` / `CONTINUUM_BENCH_TIKV_HARDWARE` — optional component profiles

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

use super::hardware::Hardware;

/// `TiKV` cluster topology preset from `infra/surreal-tikv/` compose profiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum TikvTopology {
    #[value(name = "tikv-minimal")]
    Minimal,
    #[value(name = "tikv-ha-2")]
    Ha2,
    #[value(name = "tikv-ha-3")]
    Ha3,
    #[value(name = "tikv-scale-4")]
    Scale4,
    #[value(name = "tikv-scale-5")]
    Scale5,
    #[value(name = "custom")]
    Custom,
}

impl TikvTopology {
    /// Short slug for report filenames and env export.
    pub const fn slug(self) -> &'static str {
        match self {
            Self::Minimal => "tikv-minimal",
            Self::Ha2 => "tikv-ha-2",
            Self::Ha3 => "tikv-ha-3",
            Self::Scale4 => "tikv-scale-4",
            Self::Scale5 => "tikv-scale-5",
            Self::Custom => "custom",
        }
    }

    /// `TiKV` store count for this preset (excluding PD and bench).
    pub const fn storage_node_count(self) -> u8 {
        match self {
            Self::Minimal => 1,
            Self::Ha2 => 2,
            Self::Ha3 => 3,
            Self::Scale4 => 4,
            Self::Scale5 => 5,
            Self::Custom => 0,
        }
    }

    /// All lab presets swept by the `tikv-lab-colocated` matrix slice.
    pub const fn lab_presets() -> &'static [Self] {
        &[
            Self::Minimal,
            Self::Ha3,
            Self::Scale5,
        ]
    }

    /// Native campaign presets (1 → 2 → 4 node series).
    pub const fn native_presets() -> &'static [Self] {
        &[
            Self::Minimal,
            Self::Ha2,
            Self::Scale4,
        ]
    }

    /// Parse a preset slug from env or CLI.
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "tikv-minimal" | "minimal" => Some(Self::Minimal),
            "tikv-ha-2" | "ha2" => Some(Self::Ha2),
            "tikv-ha-3" | "ha3" => Some(Self::Ha3),
            "tikv-scale-4" | "scale4" => Some(Self::Scale4),
            "tikv-scale-5" | "scale5" => Some(Self::Scale5),
            "custom" => Some(Self::Custom),
            _ => None,
        }
    }

    /// Read topology from `CONTINUUM_BENCH_TIKV_TOPOLOGY` when set.
    pub fn from_env() -> Option<Self> {
        std::env::var("CONTINUUM_BENCH_TIKV_TOPOLOGY")
            .ok()
            .and_then(|s| Self::parse(&s))
    }
}

/// Where the `SurrealDB` server runs relative to the benchmark process.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum SurrealDeployment {
    Colocated,
    Remote,
    #[value(name = "multi-node")]
    MultiNode,
}

impl SurrealDeployment {
    /// Short slug stored in report JSON.
    pub const fn slug(self) -> &'static str {
        match self {
            Self::Colocated => "colocated",
            Self::Remote => "remote",
            Self::MultiNode => "multi-node",
        }
    }
}

/// Per-component hardware profile labels captured in reports.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentHardware {
    pub runtime: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub surreal: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tikv: Option<String>,
}

impl ComponentHardware {
    /// Build component hardware from the active run dimensions and optional env overrides.
    pub fn from_run(hardware: Hardware) -> Self {
        let runtime = hardware.slug().to_string();
        let surreal = std::env::var("CONTINUUM_BENCH_SURREAL_HARDWARE")
            .ok()
            .or_else(|| Some(runtime.clone()));
        let tikv = std::env::var("CONTINUUM_BENCH_TIKV_HARDWARE")
            .ok()
            .or_else(|| Some(runtime.clone()));
        Self {
            runtime,
            surreal,
            tikv,
        }
    }
}

/// Surreal instance count from env (defaults to 1).
pub fn surreal_instances_from_env() -> u8 {
    std::env::var("CONTINUUM_BENCH_SURREAL_INSTANCES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tikv_topology_parse_roundtrip() {
        for topo in TikvTopology::lab_presets() {
            assert_eq!(TikvTopology::parse(topo.slug()), Some(*topo));
        }
        for topo in TikvTopology::native_presets() {
            assert_eq!(TikvTopology::parse(topo.slug()), Some(*topo));
            assert!(topo.storage_node_count() > 0);
        }
    }
}
