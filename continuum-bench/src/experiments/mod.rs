//! Experiment registry and dispatch.

// Bench helpers bind `&Arc<dyn LogBackend>` and intentionally hold handles across
// measurement windows; clippy's suggestions are false positives in that context.
#[allow(clippy::significant_drop_tightening)]
mod bm_core;
#[allow(clippy::significant_drop_tightening)]
mod bm_load;
#[allow(clippy::significant_drop_tightening)]
mod bm_multi_client;
#[allow(clippy::significant_drop_tightening)]
mod bm_partition;
pub mod fixtures;

use std::time::Instant;

use anyhow::Result;
use chrono::Utc;
use serde_json::Value;

pub use bm_core::prepare_context;
use bm_core::{
    run_bm_c0, run_bm_c1, run_bm_c2, run_bm_c3, run_bm_c4, run_bm_c5, run_bm_c6,
};
use bm_load::run_load;
use bm_multi_client::run_multi_client;
use bm_partition::run_partition;

use crate::harness::{capture_hardware, ExperimentId, RunDimensions};
use crate::metrics::{append_debug_notes, evaluate_pass, results_summary, ResourceProfiler};
use crate::report::{ReportStatus, RunReport, write_report};

fn skipped_report(
    id: ExperimentId,
    dims: RunDimensions,
    hardware_detail: crate::harness::HardwareDetail,
    status: ReportStatus,
    notes: impl Into<String>,
) -> RunReport {
    RunReport::skipped(id.slug(), dims, hardware_detail, status, notes)
}

async fn run_experiment_metrics(id: ExperimentId, ctx: &bm_core::ExperimentContext) -> Result<Value> {
    match id {
        ExperimentId::BmC0 => run_bm_c0(ctx).await,
        ExperimentId::BmC1 => run_bm_c1(ctx).await,
        ExperimentId::BmC2 => run_bm_c2(ctx).await,
        ExperimentId::BmC3 => run_bm_c3(ctx).await,
        ExperimentId::BmC4 => run_bm_c4(ctx).await,
        ExperimentId::BmC5 => run_bm_c5(ctx).await,
        ExperimentId::BmC6 => run_bm_c6(ctx, 3600).await,
        ExperimentId::BmL0 | ExperimentId::BmL1 | ExperimentId::BmL2 | ExperimentId::BmL3 => {
            run_load(ctx, id).await
        }
        ExperimentId::BmP1 | ExperimentId::BmP2 => run_partition(ctx, id).await,
        ExperimentId::BmM1 | ExperimentId::BmM2 | ExperimentId::BmM3 | ExperimentId::BmM4
        | ExperimentId::BmM5 => {
            run_multi_client(ctx, id).await
        }
    }
}

struct FailedRun {
    id: ExperimentId,
    dims: RunDimensions,
    hardware_detail: crate::harness::HardwareDetail,
    pass_criteria: String,
    started: Instant,
    engine_path: String,
    resource_profile: Option<crate::metrics::RunResourceProfile>,
    notes: String,
}

fn failed_report(run: FailedRun) -> RunReport {
    RunReport {
        experiment_id: run.id.slug().into(),
        dimensions: run.dims.into(),
        hardware_detail: run.hardware_detail,
        engine_path: run.engine_path,
        tikv_pd_endpoint: std::env::var("CONTINUUM_BENCH_TIKV_PD_ENDPOINT").ok(),
        started_at: Utc::now(),
        elapsed_secs: run.started.elapsed().as_secs_f64(),
        metrics: serde_json::json!({}),
        resource_profile: run.resource_profile,
        pass_criteria: run.pass_criteria,
        pass: false,
        status: ReportStatus::Failed,
        notes: run.notes,
    }
}

/// Run a single experiment with the given dimensions.
pub async fn run_experiment(id: ExperimentId, dims: RunDimensions) -> Result<RunReport> {
    let hardware_detail = capture_hardware(dims.hardware)?;
    let pass_criteria = id.pass_criteria().to_string();

    if !dims.storage.is_supported() {
        return Ok(skipped_report(
            id,
            dims,
            hardware_detail,
            ReportStatus::SkippedUnsupported,
            format!("storage {} unsupported in v0.1", dims.storage.slug()),
        ));
    }
    if !dims.telemetry.is_supported() {
        return Ok(skipped_report(
            id,
            dims,
            hardware_detail,
            ReportStatus::SkippedUnsupported,
            format!("telemetry {} not implemented", dims.telemetry.slug()),
        ));
    }
    if dims.needs_remote_surreal()
        && std::env::var("CONTINUUM_BENCH_SURREAL_URL").is_err()
    {
        return Ok(skipped_report(
            id,
            dims,
            hardware_detail,
            ReportStatus::SkippedNoRemote,
            "CONTINUUM_BENCH_SURREAL_URL not set",
        ));
    }
    if dims.needs_remote_scylla()
        && std::env::var("CONTINUUM_BENCH_SCYLLA_CONTACT_POINTS")
            .or_else(|_| std::env::var("CONTINUUM_BENCH_SCYLLA_URL"))
            .is_err()
    {
        return Ok(skipped_report(
            id,
            dims,
            hardware_detail,
            ReportStatus::SkippedNoRemote,
            "CONTINUUM_BENCH_SCYLLA_CONTACT_POINTS not set",
        ));
    }
    if dims.needs_remote_tikv_raw() && std::env::var("CONTINUUM_BENCH_TIKV_PD_ENDPOINT").is_err() {
        return Ok(skipped_report(
            id,
            dims,
            hardware_detail,
            ReportStatus::SkippedNoRemote,
            "CONTINUUM_BENCH_TIKV_PD_ENDPOINT not set",
        ));
    }

    let started = Instant::now();
    let profiler = if dims.hardware.captures_run_resource_profile() {
        Some(ResourceProfiler::start(1000))
    } else {
        None
    };

    let ctx = match prepare_context(dims).await {
        Ok(c) => c,
        Err(e) => {
            let resource_profile = if let Some(p) = profiler {
                Some(p.finish().await)
            } else {
                None
            };
            return Ok(failed_report(FailedRun {
                id,
                dims,
                hardware_detail,
                pass_criteria,
                started,
                engine_path: String::new(),
                resource_profile,
                notes: e.to_string(),
            }));
        }
    };

    let engine_path = ctx.handle.engine_path.clone();
    let metrics = match run_experiment_metrics(id, &ctx).await {
        Ok(m) => m,
        Err(e) => {
            let resource_profile = if let Some(p) = profiler {
                Some(p.finish().await)
            } else {
                None
            };
            return Ok(failed_report(FailedRun {
                id,
                dims,
                hardware_detail,
                pass_criteria,
                started,
                engine_path: ctx.handle.engine_path.clone(),
                resource_profile,
                notes: e.to_string(),
            }));
        }
    };
    drop(ctx);
    let resource_profile = if let Some(p) = profiler {
        Some(p.finish().await)
    } else {
        None
    };
    let pass = evaluate_pass(id, &metrics);

    let mut notes = results_summary(id, &metrics, pass);
    if let Some(extra) = append_debug_notes(dims.storage, &metrics) {
        notes.push(' ');
        notes.push_str(&extra);
    }

    Ok(RunReport {
        experiment_id: id.slug().into(),
        dimensions: dims.into(),
        hardware_detail,
        engine_path,
        tikv_pd_endpoint: std::env::var("CONTINUUM_BENCH_TIKV_PD_ENDPOINT").ok(),
        started_at: Utc::now(),
        elapsed_secs: started.elapsed().as_secs_f64(),
        metrics: metrics.clone(),
        resource_profile,
        pass_criteria,
        pass,
        status: ReportStatus::Completed,
        notes,
    })
}

/// Run experiment and fix notes field with metrics.
pub async fn run_and_report(id: ExperimentId, dims: RunDimensions) -> Result<RunReport> {
    let mut report = run_experiment(id, dims).await?;
    if report.status == ReportStatus::Completed {
        let mut notes = results_summary(id, &report.metrics, report.pass);
        if let Some(extra) = append_debug_notes(dims.storage, &report.metrics) {
            notes.push(' ');
            notes.push_str(&extra);
        }
        report.notes = notes;
    }
    let _path = write_report(&report, dims)?;
    Ok(report)
}

/// Print experiment catalog.
pub fn print_catalog() {
    println!("Registered continuum-bench experiments:\n");
    println!(
        "{:<8} {:<28} Pass criteria",
        "ID", "Primary metric"
    );
    println!("{}", "-".repeat(72));
    for &id in ExperimentId::all() {
        println!(
            "{:<8} {:<28} {}",
            id.slug(),
            id.primary_metric(),
            id.pass_criteria()
        );
    }
    println!("\nDimensions: storage, topology, telemetry, hardware — see EXPERIMENTS.md");
}
