//! Matrix runner for pre-registered benchmark dimensions.

use anyhow::{bail, Result};

use crate::experiments::fixtures::load_partition_count;
use crate::experiments::run_and_report;
use crate::harness::{
    matrix_for_subset, subset_needs_remote_surreal, ExperimentId, Hardware, MatrixSubset,
    RunDimensions, ScyllaTopology, TikvTopology,
};
use crate::report::{report_filename, reports_dir};

/// Options for [`run_matrix`].
pub struct MatrixOptions {
    pub hardware: Hardware,
    pub subset: MatrixSubset,
    pub from: Option<String>,
    pub skip_existing: bool,
    pub storages: Option<Vec<crate::harness::Storage>>,
    pub skip_experiments: Option<Vec<String>>,
    pub tikv_topology: Option<TikvTopology>,
    pub scylla_topology: Option<ScyllaTopology>,
}

/// Execute matrix runs sequentially.
pub async fn run_matrix(opts: MatrixOptions) -> Result<Vec<(ExperimentId, RunDimensions, bool)>> {
    if subset_needs_remote_surreal(opts.subset)
        && std::env::var("CONTINUUM_BENCH_SURREAL_URL").is_err()
    {
        bail!(
            "matrix subset {:?} requires CONTINUUM_BENCH_SURREAL_URL — start infra/surreal-tikv stack first",
            opts.subset
        );
    }

    if opts.subset == MatrixSubset::NativeLabPartitioned && load_partition_count() <= 1 {
        bail!(
            "matrix subset native-lab-partitioned requires CONTINUUM_BENCH_LOAD_PARTITION_COUNT > 1"
        );
    }

    let mut runs = matrix_for_subset(opts.subset, opts.hardware);

    if let Some(filter) = &opts.storages {
        runs.retain(|(_, dims)| filter.contains(&dims.storage));
    }

    if let Some(skip) = &opts.skip_experiments {
        let skip_ids: Vec<ExperimentId> = skip
            .iter()
            .filter_map(|s| ExperimentId::parse(s))
            .collect();
        if skip_ids.is_empty() && skip.iter().any(|s| !s.is_empty()) {
            bail!("no valid experiment ids in --skip-experiments");
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

    if let Some(filter) = opts.tikv_topology {
        runs.retain(|(_, dims)| dims.tikv_topology == Some(filter));
    }

    if let Some(filter) = opts.scylla_topology {
        runs.retain(|(_, dims)| dims.scylla_topology == Some(filter));
    }

    let mut outcomes = Vec::new();
    for (id, dims) in runs {
        if opts.skip_existing {
            let path = reports_dir().join(report_filename(id.slug(), dims));
            if path.exists() {
                eprintln!(
                    ">>> {} storage={} tikv={:?} (skip existing)",
                    id.slug(),
                    dims.storage.slug(),
                    dims.tikv_topology
                );
                continue;
            }
        }
        eprintln!(
            ">>> {} storage={} topology={} telemetry={} tikv={:?}",
            id.slug(),
            dims.storage.slug(),
            dims.topology.slug(),
            dims.telemetry.slug(),
            dims.tikv_topology
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::harness::Storage;

    #[test]
    fn tikv_topology_filter_narrows_lab_colocated_matrix() {
        let all = matrix_for_subset(MatrixSubset::TikvLabColocated, Hardware::AwsT4gMedium);
        let topologies: std::collections::HashSet<_> = all
            .iter()
            .filter_map(|(_, d)| d.tikv_topology)
            .collect();
        assert_eq!(topologies.len(), 3);

        let minimal = all
            .iter()
            .filter(|(_, d)| d.tikv_topology == Some(TikvTopology::Minimal))
            .count();
        assert!(minimal > 0);
        assert!(minimal < all.len());

        let filtered: Vec<_> = all
            .into_iter()
            .filter(|(_, d)| d.tikv_topology == Some(TikvTopology::Minimal))
            .collect();
        assert!(filtered.iter().all(|(_, d)| d.storage == Storage::SurrealTikv));
        assert!(filtered
            .iter()
            .all(|(_, d)| d.tikv_topology == Some(TikvTopology::Minimal)));
    }
}
