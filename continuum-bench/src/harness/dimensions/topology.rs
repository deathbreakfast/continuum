//! Topology and telemetry dimensions from EXPERIMENTS.md.

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

/// Topology dimension from EXPERIMENTS.md.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum Topology {
    #[value(name = "isolated-lab")]
    IsolatedLab,
    #[value(name = "shared-handle")]
    SharedHandle,
    #[value(name = "remote-surreal")]
    RemoteSurreal,
}

impl Topology {
    /// Short slug for report filenames.
    pub fn slug(self) -> &'static str {
        match self {
            Topology::IsolatedLab => "isolated-lab",
            Topology::SharedHandle => "shared-handle",
            Topology::RemoteSurreal => "remote-surreal",
        }
    }
}

/// Telemetry dimension from EXPERIMENTS.md.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum Telemetry {
    Off,
    Console,
    Stub,
}

impl Telemetry {
    /// Whether this telemetry mode is implemented.
    pub fn is_supported(self) -> bool {
        matches!(self, Telemetry::Off | Telemetry::Console)
    }

    /// Short slug for report filenames.
    pub fn slug(self) -> &'static str {
        match self {
            Telemetry::Off => "off",
            Telemetry::Console => "console",
            Telemetry::Stub => "stub",
        }
    }
}
