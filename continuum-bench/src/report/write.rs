//! Write JSON reports under `profiling/continuum-bench/reports/`.

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};

use super::schema::RunReport;
use crate::harness::RunDimensions;

/// Reports directory: `CONTINUUM_BENCH_REPORTS_DIR` or workspace default at build time.
pub fn reports_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("CONTINUUM_BENCH_REPORTS_DIR") {
        if !dir.is_empty() {
            return PathBuf::from(dir);
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../profiling/continuum-bench/reports")
}

/// Filename for a completed run.
pub fn report_filename(experiment_id: &str, dims: RunDimensions) -> String {
    format!(
        "{}-{}-{}-{}-{}.json",
        experiment_id,
        dims.storage.slug(),
        dims.topology.slug(),
        dims.telemetry.slug(),
        dims.hardware.slug()
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
