//! Synthetic continuum benchmarks and experiment matrix runner.

mod cli;
mod experiments;
mod fill_results;
mod harness;
mod matrix;
mod metrics;
mod projection;
mod report;
mod util;

use anyhow::Result;
use clap::Parser;

use cli::{Cli, Command};
use experiments::{print_catalog, run_and_report};
use harness::{capture_hardware, surreal_instances_from_env, Hardware, RunDimensions, Storage, SurrealDeployment, Topology, TikvTopology};
use matrix::run_matrix;

#[tokio::main]
async fn main() -> Result<()> {
    match Cli::parse().command {
        Command::Run {
            experiment,
            storage,
            topology,
            telemetry,
            hardware,
            tikv_topology,
        } => {
            let id = harness::ExperimentId::parse(&experiment)
                .ok_or_else(|| anyhow::anyhow!("unknown experiment: {experiment}"))?;
            let dims = build_run_dimensions(storage, topology, telemetry, hardware, tikv_topology);
            let report = run_and_report(id, dims).await?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Command::Matrix {
            hardware,
            subset,
            from,
            skip_existing,
            storages,
            skip_experiments,
            tikv_topology,
        } => {
            run_matrix(matrix::MatrixOptions {
                hardware,
                subset,
                from,
                skip_existing,
                storages,
                skip_experiments,
                tikv_topology,
            })
            .await?;
        }
        Command::FillResults => {
            fill_results::fill_experiments_md()?;
            println!("Updated continuum-bench/EXPERIMENTS.md");
        }
        Command::Experiments => print_catalog(),
        Command::Hardware { profile } => {
            let detail = capture_hardware(profile)?;
            println!("{}", serde_json::to_string_pretty(&detail)?);
        }
        Command::ProjectFleet {
            hardware,
            storage,
            tikv_topology,
            reports_dir,
            out,
        } => {
            let reports_path = if reports_dir.is_absolute() {
                reports_dir
            } else {
                std::env::current_dir()?.join(reports_dir)
            };
            projection::project_fleet(
                hardware.slug(),
                storage.slug(),
                tikv_topology.map(TikvTopology::slug),
                &reports_path,
                out,
            )?;
        }
    }
    Ok(())
}

fn build_run_dimensions(
    storage: Storage,
    topology: Topology,
    telemetry: harness::Telemetry,
    hardware: Hardware,
    tikv_topology: Option<TikvTopology>,
) -> RunDimensions {
    if storage == Storage::SurrealTikv {
        let topo = tikv_topology
            .or_else(TikvTopology::from_env)
            .unwrap_or(TikvTopology::Minimal);
        return RunDimensions::remote_surreal_tikv(
            hardware,
            topo,
            SurrealDeployment::Colocated,
            surreal_instances_from_env(),
        );
    }
    let mut dims = RunDimensions::isolated(storage, telemetry, hardware);
    dims.topology = topology;
    dims
}