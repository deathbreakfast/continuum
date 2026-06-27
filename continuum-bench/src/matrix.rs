//! Matrix runner for pre-registered benchmark dimensions.

use anyhow::Result;

use crate::experiments::run_and_report;
use crate::harness::{
    dev_wsl_matrix, sql_adapter_matrix, ExperimentId, Hardware, MatrixSubset, RunDimensions,
    Storage,
};
use crate::report::{report_filename, reports_dir};

/// Options for [`run_matrix`].
pub struct MatrixOptions {
    pub hardware: Hardware,
    pub subset: MatrixSubset,
    pub from: Option<String>,
    pub skip_existing: bool,
    pub storages: Option<Vec<Storage>>,
    pub skip_experiments: Option<Vec<String>>,
}

/// Execute matrix runs sequentially.
pub async fn run_matrix(opts: MatrixOptions) -> Result<Vec<(ExperimentId, RunDimensions, bool)>> {
    let mut runs: Vec<_> = match opts.subset {
        MatrixSubset::Full => match opts.hardware {
            Hardware::DevWsl => dev_wsl_matrix(),
            hw => dev_wsl_matrix()
                .into_iter()
                .map(|(id, mut dims)| {
                    dims.hardware = hw;
                    (id, dims)
                })
                .collect(),
        },
        MatrixSubset::Sql => sql_adapter_matrix(opts.hardware),
    };

    if let Some(filter) = &opts.storages {
        runs.retain(|(_, dims)| filter.contains(&dims.storage));
    }

    if let Some(skip) = &opts.skip_experiments {
        let skip_ids: Vec<ExperimentId> = skip
            .iter()
            .filter_map(|s| ExperimentId::parse(s))
            .collect();
        if skip_ids.is_empty() && skip.iter().any(|s| !s.is_empty()) {
            anyhow::bail!("no valid experiment ids in --skip-experiments");
        }
        runs.retain(|(id, _)| !skip_ids.contains(id));
    }

    if let Some(from_slug) = &opts.from {
        let from_id = ExperimentId::parse(from_slug)
            .ok_or_else(|| anyhow::anyhow!("unknown experiment: {from_slug}"))?;
        let all = ExperimentId::all();
        let start = all.iter().position(|&id| id == from_id).unwrap_or(0);
        runs.retain(|(id, _)| {
            all.iter()
                .position(|&x| x == *id)
                .is_some_and(|pos| pos >= start)
        });
    }

    let mut outcomes = Vec::new();
    for (id, dims) in runs {
        if opts.skip_existing {
            let path = reports_dir().join(report_filename(id.slug(), dims));
            if path.exists() {
                eprintln!(
                    ">>> {} storage={} topology={} telemetry={} (skip existing)",
                    id.slug(),
                    dims.storage.slug(),
                    dims.topology.slug(),
                    dims.telemetry.slug()
                );
                continue;
            }
        }
        eprintln!(
            ">>> {} storage={} topology={} telemetry={}",
            id.slug(),
            dims.storage.slug(),
            dims.topology.slug(),
            dims.telemetry.slug()
        );
        let report = match run_and_report(id, dims).await {
            Ok(r) => r,
            Err(e) => {
                eprintln!("    ERROR: {e}");
                continue;
            }
        };
        eprintln!(
            "    status={:?} pass={} elapsed={:.1}s notes={}",
            report.status, report.pass, report.elapsed_secs, report.notes
        );
        outcomes.push((id, dims, report.pass));
    }
    Ok(outcomes)
}
