//! CLI definitions for continuum-bench.

use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::harness::{Hardware, MatrixSubset, ScyllaTopology, Storage, Telemetry, Topology, TikvTopology};

/// Synthetic continuum benchmarks and experiment matrix runner.
#[derive(Parser)]
#[command(name = "continuum-bench")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

/// Benchmark subcommands.
#[derive(Subcommand)]
pub enum Command {
    /// Run a single experiment.
    Run {
        /// Experiment id (e.g. bm-c0).
        experiment: String,
        #[arg(long, default_value = "mem")]
        storage: Storage,
        #[arg(long, default_value = "isolated-lab")]
        topology: Topology,
        #[arg(long, default_value = "off")]
        telemetry: Telemetry,
        #[arg(long, default_value = "dev-wsl")]
        hardware: Hardware,
        #[arg(long)]
        tikv_topology: Option<TikvTopology>,
    },
    /// Run the dev-wsl benchmark matrix.
    Matrix {
        #[arg(long, default_value = "dev-wsl")]
        hardware: Hardware,
        /// Matrix preset: `full`, `sql`, or `TiKV` campaign slices.
        #[arg(long, value_enum, default_value_t = MatrixSubset::Full)]
        subset: MatrixSubset,
        /// Skip experiments before this id (e.g. bm-c4).
        #[arg(long)]
        from: Option<String>,
        /// Skip runs whose report JSON already exists.
        #[arg(long, default_value_t = false)]
        skip_existing: bool,
        /// Restrict to storage backends (e.g. sqlite,postgres).
        #[arg(long, value_delimiter = ',')]
        storages: Option<Vec<Storage>>,
        /// Skip experiment ids (e.g. bm-c6).
        #[arg(long, value_delimiter = ',')]
        skip_experiments: Option<Vec<String>>,
        /// Retain only runs whose `tikv_topology` matches (e.g. tikv-minimal).
        #[arg(long)]
        tikv_topology: Option<TikvTopology>,
        /// Retain only runs whose `scylla_topology` matches (e.g. scylla-3n).
        #[arg(long)]
        scylla_topology: Option<ScyllaTopology>,
    },
    /// Fill EXPERIMENTS.md Results columns from latest JSON reports.
    FillResults,
    /// List registered experiment IDs.
    Experiments,
    /// Print captured hardware profile as JSON.
    Hardware {
        #[arg(long, default_value = "dev-wsl")]
        profile: Hardware,
    },
    /// Build 1B/s fleet projection from collected report JSON files.
    ProjectFleet {
        #[arg(long, default_value = "dev-wsl")]
        hardware: Hardware,
        #[arg(long, default_value = "surreal-tikv")]
        storage: Storage,
        #[arg(long)]
        tikv_topology: Option<TikvTopology>,
        #[arg(long)]
        scylla_topology: Option<ScyllaTopology>,
        #[arg(long, default_value = "profiling/continuum-bench/reports")]
        reports_dir: PathBuf,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Build storage-node scaling curve from peak BM-M4 reports across topologies.
    ProjectScalingCurve {
        #[arg(long, default_value = "aws-t3-medium")]
        hardware: Hardware,
        #[arg(long, default_value = "scylla")]
        storage: Storage,
        #[arg(long, default_value = "profiling/continuum-bench/reports")]
        reports_dir: PathBuf,
        #[arg(long)]
        out: Option<PathBuf>,
    },
}
