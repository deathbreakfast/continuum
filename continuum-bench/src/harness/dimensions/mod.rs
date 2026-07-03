//! Experiment dimension types and matrix enumeration.
//!
//! **Internal — performance engineers.** See [`EXPERIMENTS.md`](../../../EXPERIMENTS.md)
//! for the pre-registered dimension matrix and `TiKV` campaign runbooks.

mod hardware;
mod scylla;
mod storage;
pub mod tikv;
mod topology;

pub use hardware::Hardware;
pub use scylla::ScyllaTopology;
pub use storage::Storage;
pub use tikv::{ComponentHardware, SurrealDeployment, TikvTopology, surreal_instances_from_env};
pub use topology::{Telemetry, Topology};

use clap::ValueEnum;

/// Full dimension set for one benchmark run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct RunDimensions {
    pub storage: Storage,
    pub topology: Topology,
    pub telemetry: Telemetry,
    pub hardware: Hardware,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tikv_topology: Option<TikvTopology>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scylla_topology: Option<ScyllaTopology>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub surreal_deployment: Option<SurrealDeployment>,
    pub surreal_instances: u8,
}

impl RunDimensions {
    /// Build isolated-lab dimensions with default `TiKV` fields unset.
    pub const fn isolated(
        storage: Storage,
        telemetry: Telemetry,
        hardware: Hardware,
    ) -> Self {
        Self {
            storage,
            topology: Topology::IsolatedLab,
            telemetry,
            hardware,
            tikv_topology: None,
            scylla_topology: None,
            surreal_deployment: None,
            surreal_instances: 1,
        }
    }

    /// Build remote-surreal / surreal-tikv dimensions for distributed campaigns.
    pub fn remote_surreal_tikv(
        hardware: Hardware,
        tikv_topology: TikvTopology,
        surreal_deployment: SurrealDeployment,
        surreal_instances: u8,
    ) -> Self {
        Self {
            storage: Storage::SurrealTikv,
            topology: Topology::RemoteSurreal,
            telemetry: Telemetry::Off,
            hardware,
            tikv_topology: Some(tikv_topology),
            scylla_topology: None,
            surreal_deployment: Some(surreal_deployment),
            surreal_instances,
        }
    }

    /// Whether this run needs a live remote Surreal stack.
    pub fn needs_remote_surreal(self) -> bool {
        self.storage.needs_remote_surreal() || self.topology == Topology::RemoteSurreal
    }

    /// Whether this run needs a live Scylla cluster.
    pub fn needs_remote_scylla(self) -> bool {
        self.storage.needs_remote_scylla()
    }

    /// Whether this run needs a live TiKV PD endpoint (raw client).
    pub fn needs_remote_tikv_raw(self) -> bool {
        self.storage.needs_remote_tikv_raw()
    }
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
    BmP1,
    BmP2,
    BmM1,
    BmM2,
    BmM3,
    BmM4,
    BmM5,
}

impl ExperimentId {
    /// All registered experiment ids in run order.
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
            ExperimentId::BmP1,
            ExperimentId::BmP2,
            ExperimentId::BmM1,
            ExperimentId::BmM2,
            ExperimentId::BmM3,
            ExperimentId::BmM4,
            ExperimentId::BmM5,
        ]
    }

    /// Short slug used in CLI and report filenames.
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
            ExperimentId::BmP1 => "bm-p1",
            ExperimentId::BmP2 => "bm-p2",
            ExperimentId::BmM1 => "bm-m1",
            ExperimentId::BmM2 => "bm-m2",
            ExperimentId::BmM3 => "bm-m3",
            ExperimentId::BmM4 => "bm-m4",
            ExperimentId::BmM5 => "bm-m5",
        }
    }

    /// Parse an experiment slug from CLI input.
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
            "bm-p1" => Some(ExperimentId::BmP1),
            "bm-p2" => Some(ExperimentId::BmP2),
            "bm-m1" => Some(ExperimentId::BmM1),
            "bm-m2" => Some(ExperimentId::BmM2),
            "bm-m3" => Some(ExperimentId::BmM3),
            "bm-m4" => Some(ExperimentId::BmM4),
            "bm-m5" => Some(ExperimentId::BmM5),
            _ => None,
        }
    }

    /// Pre-registered pass criteria from EXPERIMENTS.md.
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
            ExperimentId::BmP1 => "aggregate ops scales with partition count",
            ExperimentId::BmP2 => "read completes for all partitions",
            ExperimentId::BmM1 | ExperimentId::BmM2 | ExperimentId::BmM3 | ExperimentId::BmM4
            | ExperimentId::BmM5 => {
                "error rate <0.1%"
            }
        }
    }

    /// Primary metric recorded in reports.
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
            ExperimentId::BmP1 => "aggregate ops/s",
            ExperimentId::BmP2 => "read ops/s",
            ExperimentId::BmM1
            | ExperimentId::BmM2
            | ExperimentId::BmM3
            | ExperimentId::BmM4
            | ExperimentId::BmM5 => "aggregate ops/s",
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
                        tikv_topology: None,
                        scylla_topology: None,
                        surreal_deployment: None,
                        surreal_instances: 1,
                    },
                ));
            }
        }
    }

    append_remote_surreal_runs(&mut runs, Hardware::DevWsl);
    append_postgres_runs(&mut runs, Hardware::DevWsl);
    runs
}

fn append_remote_surreal_runs(runs: &mut Vec<(ExperimentId, RunDimensions)>, hardware: Hardware) {
    if std::env::var("CONTINUUM_BENCH_SURREAL_URL").is_err() {
        return;
    }
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
                hardware,
                tikv_topology: None,
                scylla_topology: None,
                surreal_deployment: None,
                surreal_instances: 1,
            },
        ));
    }
}

fn append_postgres_runs(runs: &mut Vec<(ExperimentId, RunDimensions)>, hardware: Hardware) {
    if std::env::var("CONTINUUM_BENCH_POSTGRES_URL").is_err() {
        return;
    }
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
                    hardware,
                    tikv_topology: None,
                    scylla_topology: None,
                    surreal_deployment: None,
                    surreal_instances: 1,
                },
            ));
        }
    }
}

/// Matrix subset selector for [`run_matrix`](crate::matrix::run_matrix).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum)]
pub enum MatrixSubset {
    /// Full pre-registered matrix ([`dev_wsl_matrix`]).
    #[default]
    Full,
    /// `SQLite` + Postgres only; telemetry off; excludes BM-C6 soak.
    Sql,
    /// All `TiKV` presets on single host — requires `CONTINUUM_BENCH_SURREAL_URL`.
    #[value(name = "tikv-lab-colocated")]
    TikvLabColocated,
    /// BM-L1/L2 ceiling probes sweeping `TiKV` topology.
    #[value(name = "tikv-topology")]
    TikvTopology,
    /// BM-L1/L2 with varying Surreal instance count.
    #[value(name = "surreal-scale")]
    SurrealScale,
    /// BM-L0–L3 on active `TiKV` topology for fleet projection inputs.
    #[value(name = "tikv-projection-inputs")]
    TikvProjectionInputs,
    /// BM-L0–L3 on native adapters for fleet projection inputs.
    #[value(name = "native-projection-inputs")]
    NativeProjectionInputs,
    /// BM-C*/BM-L* parity on scylla + tikv-raw (+ sqlite baseline).
    #[value(name = "native-lab")]
    NativeLab,
    /// BM-M3 concurrency ladder on native adapters (hot stream).
    #[value(name = "native-concurrency")]
    NativeConcurrency,
    /// BM-P*/BM-M* partition and client sweeps on native adapters.
    #[value(name = "native-scale")]
    NativeScale,
    /// BM-L0–L3 with `CONTINUUM_BENCH_LOAD_PARTITION_COUNT` > 1.
    #[value(name = "native-lab-partitioned")]
    NativeLabPartitioned,
    /// Native projection inputs + scale for active topology env (Phase B).
    #[value(name = "native-topology")]
    NativeTopology,
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
                    tikv_topology: None,
                    scylla_topology: None,
                    surreal_deployment: None,
                    surreal_instances: 1,
                },
            ));
        }
    }
    runs
}

/// `TiKV` lab matrix: all BM-* experiments × each lab `TiKV` preset (Phase 1).
pub fn tikv_lab_colocated_matrix(hardware: Hardware) -> Vec<(ExperimentId, RunDimensions)> {
    let mut runs = Vec::new();
    for &topo in TikvTopology::lab_presets() {
        for &exp in ExperimentId::all() {
            if exp == ExperimentId::BmC5 {
                continue;
            }
            runs.push((
                exp,
                RunDimensions::remote_surreal_tikv(
                    hardware,
                    topo,
                    SurrealDeployment::Colocated,
                    tikv::surreal_instances_from_env(),
                ),
            ));
        }
    }
    runs
}

/// `TiKV` topology sweep: BM-L1/L2 ceiling probes (Phase 2).
pub fn tikv_topology_matrix(hardware: Hardware) -> Vec<(ExperimentId, RunDimensions)> {
    let exps = [ExperimentId::BmL1, ExperimentId::BmL2];
    let mut runs = Vec::new();
    for &topo in TikvTopology::lab_presets() {
        for exp in exps {
            runs.push((
                exp,
                RunDimensions::remote_surreal_tikv(
                    hardware,
                    topo,
                    SurrealDeployment::Colocated,
                    1,
                ),
            ));
        }
    }
    runs
}

/// Surreal scale-out sweep: BM-L1/L2 with 1, 2, 4 instances (Phase 3).
pub fn surreal_scale_matrix(hardware: Hardware) -> Vec<(ExperimentId, RunDimensions)> {
    let topo = TikvTopology::from_env().unwrap_or(TikvTopology::Ha3);
    let exps = [ExperimentId::BmL1, ExperimentId::BmL2];
    let instances = [1_u8, 2, 4];
    let mut runs = Vec::new();
    for &n in &instances {
        for exp in exps {
            let deployment = if n > 1 {
                SurrealDeployment::MultiNode
            } else {
                SurrealDeployment::Colocated
            };
            runs.push((
                exp,
                RunDimensions::remote_surreal_tikv(hardware, topo, deployment, n),
            ));
        }
    }
    runs
}

/// Load-tier inputs for fleet projection (Phase 2/3).
pub fn tikv_projection_inputs_matrix(hardware: Hardware) -> Vec<(ExperimentId, RunDimensions)> {
    let topo = TikvTopology::from_env().unwrap_or(TikvTopology::Ha3);
    let exps = [
        ExperimentId::BmL0,
        ExperimentId::BmL1,
        ExperimentId::BmL2,
        ExperimentId::BmL3,
    ];
    exps.into_iter()
        .map(|exp| {
            (
                exp,
                RunDimensions::remote_surreal_tikv(
                    hardware,
                    topo,
                    SurrealDeployment::Colocated,
                    tikv::surreal_instances_from_env(),
                ),
            )
        })
        .collect()
}

fn native_storages() -> Vec<Storage> {
    let mut storages = vec![Storage::Sqlite];
    if std::env::var("CONTINUUM_BENCH_SCYLLA_CONTACT_POINTS")
        .or_else(|_| std::env::var("CONTINUUM_BENCH_SCYLLA_URL"))
        .is_ok()
    {
        storages.push(Storage::Scylla);
    }
    if std::env::var("CONTINUUM_BENCH_TIKV_PD_ENDPOINT").is_ok() {
        storages.push(Storage::TikvRaw);
    }
    storages
}

fn native_dims(storage: Storage, hardware: Hardware) -> RunDimensions {
    let mut dims = RunDimensions::isolated(storage, Telemetry::Off, hardware);
    if storage == Storage::Scylla {
        dims.scylla_topology = ScyllaTopology::from_env().or(Some(ScyllaTopology::One));
    }
    if storage == Storage::TikvRaw {
        dims.tikv_topology = TikvTopology::from_env().or(Some(TikvTopology::Minimal));
    }
    dims
}

fn native_lab_experiments() -> &'static [ExperimentId] {
    &[
        ExperimentId::BmC0,
        ExperimentId::BmC1,
        ExperimentId::BmC2,
        ExperimentId::BmC3,
        ExperimentId::BmC4,
        ExperimentId::BmL0,
        ExperimentId::BmL1,
        ExperimentId::BmL2,
        ExperimentId::BmL3,
    ]
}

/// Native adapter parity matrix: BM-C*/BM-L* on scylla/tikv-raw when env set (+ sqlite).
pub fn native_lab_matrix(hardware: Hardware) -> Vec<(ExperimentId, RunDimensions)> {
    let storages = native_storages();
    let mut runs = Vec::new();
    for &exp in native_lab_experiments() {
        for &storage in &storages {
            runs.push((exp, native_dims(storage, hardware)));
        }
    }
    runs
}

/// BM-L0–L3 with partitioned keys (`CONTINUUM_BENCH_LOAD_PARTITION_COUNT` > 1).
pub fn native_lab_partitioned_matrix(hardware: Hardware) -> Vec<(ExperimentId, RunDimensions)> {
    let storages = native_storages();
    let exps = [
        ExperimentId::BmL0,
        ExperimentId::BmL1,
        ExperimentId::BmL2,
        ExperimentId::BmL3,
    ];
    let mut runs = Vec::new();
    for &exp in &exps {
        for &storage in &storages {
            runs.push((exp, native_dims(storage, hardware)));
        }
    }
    runs
}

/// Native scale matrix: BM-P1/P2/M1/M2/M4 on scylla + tikv-raw when configured.
pub fn native_scale_matrix(hardware: Hardware) -> Vec<(ExperimentId, RunDimensions)> {
    let storages: Vec<Storage> = native_storages()
        .into_iter()
        .filter(|s| matches!(s, Storage::Scylla | Storage::TikvRaw))
        .collect();
    let exps = [
        ExperimentId::BmP1,
        ExperimentId::BmP2,
        ExperimentId::BmM1,
        ExperimentId::BmM2,
        ExperimentId::BmM4,
    ];
    let mut runs = Vec::new();
    for &storage in &storages {
        for &exp in &exps {
            runs.push((exp, native_dims(storage, hardware)));
        }
    }
    runs
}

/// BM-M3 hot-stream concurrency sweep on configured native storages.
pub fn native_concurrency_matrix(hardware: Hardware) -> Vec<(ExperimentId, RunDimensions)> {
    let storages: Vec<Storage> = native_storages()
        .into_iter()
        .filter(|s| matches!(s, Storage::Scylla | Storage::TikvRaw))
        .collect();
    let mut runs = Vec::new();
    for &storage in &storages {
        runs.push((ExperimentId::BmM3, native_dims(storage, hardware)));
    }
    runs
}

/// Native projection inputs: BM-L0–L3 + BM-M2 on configured native storages.
pub fn native_projection_inputs_matrix(hardware: Hardware) -> Vec<(ExperimentId, RunDimensions)> {
    let storages: Vec<Storage> = native_storages()
        .into_iter()
        .filter(|s| matches!(s, Storage::Scylla | Storage::TikvRaw))
        .collect();
    let exps = [
        ExperimentId::BmL0,
        ExperimentId::BmL1,
        ExperimentId::BmL2,
        ExperimentId::BmL3,
        ExperimentId::BmM2,
    ];
    let mut runs = Vec::new();
    for &storage in &storages {
        for &exp in &exps {
            runs.push((exp, native_dims(storage, hardware)));
        }
    }
    runs
}

/// Phase B: projection inputs + partition/client scale experiments with topology env.
pub fn native_topology_matrix(hardware: Hardware) -> Vec<(ExperimentId, RunDimensions)> {
    let mut runs = native_projection_inputs_matrix(hardware);
    runs.extend(native_scale_matrix(hardware));
    runs
}

/// Resolve matrix runs for the given subset and hardware profile.
pub fn matrix_for_subset(subset: MatrixSubset, hardware: Hardware) -> Vec<(ExperimentId, RunDimensions)> {
    match subset {
        MatrixSubset::Full => match hardware {
            Hardware::DevWsl => dev_wsl_matrix(),
            hw => dev_wsl_matrix()
                .into_iter()
                .map(|(id, mut dims)| {
                    dims.hardware = hw;
                    (id, dims)
                })
                .collect(),
        },
        MatrixSubset::Sql => sql_adapter_matrix(hardware),
        MatrixSubset::TikvLabColocated => tikv_lab_colocated_matrix(hardware),
        MatrixSubset::TikvTopology => tikv_topology_matrix(hardware),
        MatrixSubset::SurrealScale => surreal_scale_matrix(hardware),
        MatrixSubset::TikvProjectionInputs => tikv_projection_inputs_matrix(hardware),
        MatrixSubset::NativeLab => native_lab_matrix(hardware),
        MatrixSubset::NativeLabPartitioned => native_lab_partitioned_matrix(hardware),
        MatrixSubset::NativeScale => native_scale_matrix(hardware),
        MatrixSubset::NativeConcurrency => native_concurrency_matrix(hardware),
        MatrixSubset::NativeProjectionInputs => native_projection_inputs_matrix(hardware),
        MatrixSubset::NativeTopology => native_topology_matrix(hardware),
    }
}

/// Whether a matrix subset requires a remote Scylla cluster.
pub fn subset_needs_remote_scylla(subset: MatrixSubset) -> bool {
    matches!(
        subset,
        MatrixSubset::NativeLab
            | MatrixSubset::NativeLabPartitioned
            | MatrixSubset::NativeScale
            | MatrixSubset::NativeConcurrency
            | MatrixSubset::NativeProjectionInputs
            | MatrixSubset::NativeTopology
    )
}

/// Whether a matrix subset requires a raw TiKV PD endpoint.
pub fn subset_needs_remote_tikv_raw(subset: MatrixSubset) -> bool {
    matches!(
        subset,
        MatrixSubset::NativeLab
            | MatrixSubset::NativeLabPartitioned
            | MatrixSubset::NativeScale
            | MatrixSubset::NativeConcurrency
            | MatrixSubset::NativeProjectionInputs
            | MatrixSubset::NativeTopology
    )
}

/// Whether a matrix subset requires a remote Surreal stack.
pub fn subset_needs_remote_surreal(subset: MatrixSubset) -> bool {
    matches!(
        subset,
        MatrixSubset::TikvLabColocated
            | MatrixSubset::TikvTopology
            | MatrixSubset::SurrealScale
            | MatrixSubset::TikvProjectionInputs
    )
}
