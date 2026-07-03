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
    pub const fn slug(self) -> &'static str {
        match self {
            Self::IsolatedLab => "isolated-lab",
            Self::SharedHandle => "shared-handle",
            Self::RemoteSurreal => "remote-surreal",
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
    pub const fn is_supported(self) -> bool {
        matches!(self, Self::Off | Self::Console)
    }

    /// Short slug for report filenames.
    pub const fn slug(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Console => "console",
            Self::Stub => "stub",
        }
    }
}
