//! CLI definitions for continuum-bench.

use clap::{Parser, Subcommand};

use crate::harness::{Hardware, MatrixSubset, Storage, Telemetry, Topology};

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
    },
    /// Run the dev-wsl benchmark matrix.
    Matrix {
        #[arg(long, default_value = "dev-wsl")]
        hardware: Hardware,
        /// Matrix preset: `full` (default) or `sql` (sqlite/postgres subset).
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
}
