//! Write JSON reports under `profiling/continuum-bench/reports/`.

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};

use super::schema::RunReport;
use crate::harness::{RunDimensions, ScyllaTopology, Storage, TikvTopology};

/// Reports directory: `CONTINUUM_BENCH_REPORTS_DIR` or workspace default at build time.
pub fn reports_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("CONTINUUM_BENCH_REPORTS_DIR") {
        if !dir.is_empty() {
            return PathBuf::from(dir);
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../profiling/continuum-bench/reports")
}

fn report_env_suffix() -> String {
    let mut tags = Vec::new();
    if let Ok(v) = std::env::var("CONTINUUM_BENCH_LOAD_PARTITION_COUNT") {
        if v.parse::<usize>().ok().as_ref().is_some_and(|&n| n > 1) {
            tags.push(format!("k{v}"));
        }
    }
    if let Ok(v) = std::env::var("CONTINUUM_BENCH_PARTITION_COUNT") {
        tags.push(format!("pk{v}"));
    }
    if let Ok(v) = std::env::var("CONTINUUM_BENCH_CLIENT_COUNT") {
        tags.push(format!("c{v}"));
    }
    if let Ok(v) = std::env::var("CONTINUUM_BENCH_PARTITION_OFFSET") {
        if !v.is_empty() {
            tags.push(format!("off{v}"));
        }
    }
    if let Ok(v) = std::env::var("CONTINUUM_BENCH_REPORT_TAG") {
        let tag = v.trim();
        if !tag.is_empty() {
            tags.push(tag.to_string());
        }
    }
    if tags.is_empty() {
        String::new()
    } else {
        format!("-{}", tags.join("-"))
    }
}

/// Filename for a completed run.
pub fn report_filename(experiment_id: &str, dims: RunDimensions) -> String {
    let env_suffix = report_env_suffix();
    if dims.storage == Storage::SurrealTikv {
        let topo = dims
            .tikv_topology
            .map_or("tikv-unknown", TikvTopology::slug);
        return format!(
            "{experiment_id}-surreal-tikv-{topo}-{}-{}{env_suffix}.json",
            dims.telemetry.slug(),
            dims.hardware.slug()
        );
    }
    if dims.storage == Storage::Scylla {
        let topo = dims
            .scylla_topology
            .map_or("scylla-1", ScyllaTopology::slug);
        return format!(
            "{experiment_id}-scylla-{topo}-{}-{}-{}{env_suffix}.json",
            dims.topology.slug(),
            dims.telemetry.slug(),
            dims.hardware.slug()
        );
    }
    if dims.storage == Storage::TikvRaw {
        let topo = dims
            .tikv_topology
            .map_or("tikv-minimal", TikvTopology::slug);
        return format!(
            "{experiment_id}-tikv-raw-{topo}-{}-{}-{}{env_suffix}.json",
            dims.topology.slug(),
            dims.telemetry.slug(),
            dims.hardware.slug()
        );
    }
    format!(
        "{}-{}-{}-{}-{}{}.json",
        experiment_id,
        dims.storage.slug(),
        dims.topology.slug(),
        dims.telemetry.slug(),
        dims.hardware.slug(),
        env_suffix
    )
}

/// Write report JSON and return the path written.
pub fn write_report(report: &RunReport, dims: RunDimensions) -> Result<PathBuf> {
    let dir = reports_dir();
    fs::create_dir_all(&dir).context("create reports dir")?;
    let path = dir.join(report_filename(&report.experiment_id, dims));
    let json = serde_json::to_string_pretty(report).context("serialize report")?;
    fs::write(&path, json).context("write report")?;
    Ok(path)
}

/// Load all JSON reports from the reports directory.
pub fn load_all_reports() -> Result<Vec<(PathBuf, RunReport)>> {
    let dir = reports_dir();
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let text = fs::read_to_string(&path)?;
        let report: RunReport = serde_json::from_str(&text)?;
        out.push((path, report));
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::harness::{Hardware, RunDimensions, Storage, SurrealDeployment, Telemetry, TikvTopology, Topology};

    #[test]
    fn surreal_tikv_filename_includes_topology() {
        let dims = RunDimensions {
            storage: Storage::SurrealTikv,
            topology: Topology::RemoteSurreal,
            telemetry: Telemetry::Off,
            hardware: Hardware::DevWsl,
            tikv_topology: Some(TikvTopology::Ha3),
            scylla_topology: None,
            surreal_deployment: Some(SurrealDeployment::Colocated),
            surreal_instances: 1,
        };
        let name = report_filename("bm-l1", dims);
        assert!(name.contains("surreal-tikv-tikv-ha-3"));
    }

    #[test]
    fn scylla_filename_includes_topology() {
        let dims = RunDimensions {
            storage: Storage::Scylla,
            topology: Topology::IsolatedLab,
            telemetry: Telemetry::Off,
            hardware: Hardware::AwsT3Medium,
            tikv_topology: None,
            scylla_topology: Some(crate::harness::ScyllaTopology::ThreeNode),
            surreal_deployment: None,
            surreal_instances: 1,
        };
        let name = report_filename("bm-l3", dims);
        assert!(name.contains("scylla-scylla-3n"));
    }

    #[test]
    fn report_tag_and_partition_offset_in_filename() {
        std::env::set_var("CONTINUUM_BENCH_PARTITION_COUNT", "256");
        std::env::set_var("CONTINUUM_BENCH_CLIENT_COUNT", "256");
        std::env::set_var("CONTINUUM_BENCH_PARTITION_OFFSET", "128");
        std::env::set_var("CONTINUUM_BENCH_REPORT_TAG", "x-dual-b");

        let dims = RunDimensions {
            storage: Storage::Scylla,
            topology: Topology::IsolatedLab,
            telemetry: Telemetry::Off,
            hardware: Hardware::AwsT3Medium,
            tikv_topology: None,
            scylla_topology: Some(crate::harness::ScyllaTopology::TwoNode),
            surreal_deployment: None,
            surreal_instances: 1,
        };
        let name = report_filename("bm-m4", dims);
        assert!(name.contains("-pk256-c256-off128-x-dual-b"));

        std::env::remove_var("CONTINUUM_BENCH_PARTITION_COUNT");
        std::env::remove_var("CONTINUUM_BENCH_CLIENT_COUNT");
        std::env::remove_var("CONTINUUM_BENCH_PARTITION_OFFSET");
        std::env::remove_var("CONTINUUM_BENCH_REPORT_TAG");
    }
}
