//! Experiment dimension types and matrix enumeration.

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

/// Storage backend dimension from EXPERIMENTS.md.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum Storage {
    Mem,
    #[value(name = "surreal-mem")]
    SurrealMem,
    #[value(name = "surreal-rocksdb")]
    SurrealRocksdb,
    Postgres,
    Sqlite,
}

impl Storage {
    /// Whether this storage backend is implemented in v0.1.
    pub fn is_supported(self) -> bool {
        matches!(
            self,
            Storage::Mem
                | Storage::SurrealMem
                | Storage::SurrealRocksdb
                | Storage::Sqlite
                | Storage::Postgres
        )
    }

    /// Short slug for report filenames.
    pub fn slug(self) -> &'static str {
        match self {
            Storage::Mem => "mem",
            Storage::SurrealMem => "surreal-mem",
            Storage::SurrealRocksdb => "surreal-rocksdb",
            Storage::Postgres => "postgres",
            Storage::Sqlite => "sqlite",
        }
    }
}

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
    pub fn is_supported(self) -> bool {
        matches!(self, Telemetry::Off | Telemetry::Console)
    }

    pub fn slug(self) -> &'static str {
        match self {
            Telemetry::Off => "off",
            Telemetry::Console => "console",
            Telemetry::Stub => "stub",
        }
    }
}

/// Hardware profile label from EXPERIMENTS.md.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum Hardware {
    #[value(name = "dev-wsl")]
    DevWsl,
    #[value(name = "ci-small")]
    CiSmall,
    #[value(name = "bare-metal-small")]
    BareMetalSmall,
    #[value(name = "bare-metal-medium")]
    BareMetalMedium,
    #[value(name = "bare-metal-large")]
    BareMetalLarge,
    #[value(name = "aws-t3-medium")]
    AwsT3Medium,
    #[value(name = "aws-t3-small")]
    AwsT3Small,
    #[value(name = "aws-t4g-small")]
    AwsT4gSmall,
    #[value(name = "aws-t4g-medium")]
    AwsT4gMedium,
    #[value(name = "aws-t4g-large")]
    AwsT4gLarge,
    #[value(name = "aws-c7i-4xlarge")]
    AwsC7i4xlarge,
    #[value(name = "aws-i4i-xlarge")]
    AwsI4iXlarge,
}

impl Hardware {
    pub fn slug(self) -> &'static str {
        match self {
            Hardware::DevWsl => "dev-wsl",
            Hardware::CiSmall => "ci-small",
            Hardware::BareMetalSmall => "bare-metal-small",
            Hardware::BareMetalMedium => "bare-metal-medium",
            Hardware::BareMetalLarge => "bare-metal-large",
            Hardware::AwsT3Medium => "aws-t3-medium",
            Hardware::AwsT3Small => "aws-t3-small",
            Hardware::AwsT4gSmall => "aws-t4g-small",
            Hardware::AwsT4gMedium => "aws-t4g-medium",
            Hardware::AwsT4gLarge => "aws-t4g-large",
            Hardware::AwsC7i4xlarge => "aws-c7i-4xlarge",
            Hardware::AwsI4iXlarge => "aws-i4i-xlarge",
        }
    }

    /// Cloud / isolated-VM sizing profiles capture per-run CPU/RSS; lab `dev-wsl` is sanity-only.
    pub fn captures_run_resource_profile(self) -> bool {
        matches!(
            self,
            Hardware::CiSmall
                | Hardware::AwsT3Medium
                | Hardware::AwsT3Small
                | Hardware::AwsT4gSmall
                | Hardware::AwsT4gMedium
                | Hardware::AwsT4gLarge
                | Hardware::AwsC7i4xlarge
                | Hardware::AwsI4iXlarge
        )
    }
}

/// Full dimension set for one benchmark run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RunDimensions {
    pub storage: Storage,
    pub topology: Topology,
    pub telemetry: Telemetry,
    pub hardware: Hardware,
}

/// Registered experiment identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExperimentId {
    BmC0,
    BmC1,
    BmC2,
    BmC3,
    BmC4,
    BmC5,
    BmC6,
    BmL0,
    BmL1,
    BmL2,
    BmL3,
}

impl ExperimentId {
    pub fn all() -> &'static [ExperimentId] {
        &[
            ExperimentId::BmC0,
            ExperimentId::BmC1,
            ExperimentId::BmC2,
            ExperimentId::BmC3,
            ExperimentId::BmC4,
            ExperimentId::BmC5,
            ExperimentId::BmC6,
            ExperimentId::BmL0,
            ExperimentId::BmL1,
            ExperimentId::BmL2,
            ExperimentId::BmL3,
        ]
    }

    pub fn slug(self) -> &'static str {
        match self {
            ExperimentId::BmC0 => "bm-c0",
            ExperimentId::BmC1 => "bm-c1",
            ExperimentId::BmC2 => "bm-c2",
            ExperimentId::BmC3 => "bm-c3",
            ExperimentId::BmC4 => "bm-c4",
            ExperimentId::BmC5 => "bm-c5",
            ExperimentId::BmC6 => "bm-c6",
            ExperimentId::BmL0 => "bm-l0",
            ExperimentId::BmL1 => "bm-l1",
            ExperimentId::BmL2 => "bm-l2",
            ExperimentId::BmL3 => "bm-l3",
        }
    }

    pub fn parse(s: &str) -> Option<ExperimentId> {
        match s.to_ascii_lowercase().as_str() {
            "bm-c0" => Some(ExperimentId::BmC0),
            "bm-c1" => Some(ExperimentId::BmC1),
            "bm-c2" => Some(ExperimentId::BmC2),
            "bm-c3" => Some(ExperimentId::BmC3),
            "bm-c4" => Some(ExperimentId::BmC4),
            "bm-c5" => Some(ExperimentId::BmC5),
            "bm-c6" => Some(ExperimentId::BmC6),
            "bm-l0" => Some(ExperimentId::BmL0),
            "bm-l1" => Some(ExperimentId::BmL1),
            "bm-l2" => Some(ExperimentId::BmL2),
            "bm-l3" => Some(ExperimentId::BmL3),
            _ => None,
        }
    }

    pub fn pass_criteria(self) -> &'static str {
        match self {
            ExperimentId::BmC0 => "Flat vs op count at 5k ops",
            ExperimentId::BmC1 => "Throughput scales with batch",
            ExperimentId::BmC2 => "Flat at 100k rows",
            ExperimentId::BmC3 => "Flat over 10k commits",
            ExperimentId::BmC4 => "Read stable after truncate",
            ExperimentId::BmC5 => "Growth only on same handle",
            ExperimentId::BmC6 => "<2× isolated at 1 op/s",
            ExperimentId::BmL0 | ExperimentId::BmL1 | ExperimentId::BmL2 | ExperimentId::BmL3 => {
                "error rate <0.1%"
            }
        }
    }

    pub fn primary_metric(self) -> &'static str {
        match self {
            ExperimentId::BmC0 => "p50/p95 append ms",
            ExperimentId::BmC1 => "events/s",
            ExperimentId::BmC2 => "poll ms vs table size",
            ExperimentId::BmC3 => "checkpoint upsert ms",
            ExperimentId::BmC4 => "space + read ms post-truncate",
            ExperimentId::BmC5 => "same vs isolated handle growth",
            ExperimentId::BmC6 => "growth ratio",
            ExperimentId::BmL0
            | ExperimentId::BmL1
            | ExperimentId::BmL2
            | ExperimentId::BmL3 => "sustained p99",
        }
    }
}

/// Default matrix runs for `--hardware dev-wsl`.
pub fn dev_wsl_matrix() -> Vec<(ExperimentId, RunDimensions)> {
    let storages = [
        Storage::Mem,
        Storage::SurrealMem,
        Storage::SurrealRocksdb,
        Storage::Sqlite,
    ];
    let mut runs = Vec::new();

    for &exp in ExperimentId::all() {
        let topology = if exp == ExperimentId::BmC5 {
            Topology::SharedHandle
        } else {
            Topology::IsolatedLab
        };

        for &storage in &storages {
            let telemetries: Vec<Telemetry> = if exp == ExperimentId::BmC0 || exp == ExperimentId::BmL1 {
                vec![Telemetry::Off, Telemetry::Console]
            } else {
                vec![Telemetry::Off]
            };

            for &tel in &telemetries {
                runs.push((
                    exp,
                    RunDimensions {
                        storage,
                        topology,
                        telemetry: tel,
                        hardware: Hardware::DevWsl,
                    },
                ));
            }
        }
    }

    if std::env::var("CONTINUUM_BENCH_SURREAL_URL").is_ok() {
        for &exp in ExperimentId::all() {
            if exp == ExperimentId::BmC5 {
                continue;
            }
            runs.push((
                exp,
                RunDimensions {
                    storage: Storage::SurrealMem,
                    topology: Topology::RemoteSurreal,
                    telemetry: Telemetry::Off,
                    hardware: Hardware::DevWsl,
                },
            ));
        }
    }

    if std::env::var("CONTINUUM_BENCH_POSTGRES_URL").is_ok() {
        for &exp in ExperimentId::all() {
            let topology = if exp == ExperimentId::BmC5 {
                Topology::SharedHandle
            } else {
                Topology::IsolatedLab
            };

            let telemetries: Vec<Telemetry> = if exp == ExperimentId::BmC0 || exp == ExperimentId::BmL1 {
                vec![Telemetry::Off, Telemetry::Console]
            } else {
                vec![Telemetry::Off]
            };

            for &tel in &telemetries {
                runs.push((
                    exp,
                    RunDimensions {
                        storage: Storage::Postgres,
                        topology,
                        telemetry: tel,
                        hardware: Hardware::DevWsl,
                    },
                ));
            }
        }
    }

    runs
}

/// Matrix subset selector for [`run_matrix`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum)]
pub enum MatrixSubset {
    /// Full pre-registered matrix ([`dev_wsl_matrix`]).
    #[default]
    Full,
    /// `SQLite` + Postgres only; telemetry off; excludes BM-C6 soak.
    Sql,
}

/// SQL adapter benchmark matrix: sqlite + postgres (when URL set), telemetry off, no BM-C6.
pub fn sql_adapter_matrix(hardware: Hardware) -> Vec<(ExperimentId, RunDimensions)> {
    let mut storages = vec![Storage::Sqlite];
    if std::env::var("CONTINUUM_BENCH_POSTGRES_URL").is_ok() {
        storages.push(Storage::Postgres);
    }

    let mut runs = Vec::new();
    for &exp in ExperimentId::all() {
        if exp == ExperimentId::BmC6 {
            continue;
        }
        let topology = if exp == ExperimentId::BmC5 {
            Topology::SharedHandle
        } else {
            Topology::IsolatedLab
        };

        for &storage in &storages {
            runs.push((
                exp,
                RunDimensions {
                    storage,
                    topology,
                    telemetry: Telemetry::Off,
                    hardware,
                },
            ));
        }
    }
    runs
}
