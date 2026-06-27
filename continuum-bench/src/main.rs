//! Synthetic continuum benchmarks and experiment matrix runner.

mod cli;
mod experiments;
mod fill_results;
mod harness;
mod matrix;
mod metrics;
mod report;
mod util;

use anyhow::Result;
use clap::Parser;

use cli::{Cli, Command};
use experiments::{print_catalog, run_and_report};
use harness::capture_hardware;
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
        } => {
            let id = harness::ExperimentId::parse(&experiment)
                .ok_or_else(|| anyhow::anyhow!("unknown experiment: {experiment}"))?;
            let dims = harness::RunDimensions {
                storage,
                topology,
                telemetry,
                hardware,
            };
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
        } => {
            run_matrix(matrix::MatrixOptions {
                hardware,
                subset,
                from,
                skip_existing,
                storages,
                skip_experiments,
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
    }
    Ok(())
}
