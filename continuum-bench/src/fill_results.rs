//! Update EXPERIMENTS.md Results column from JSON reports.

use std::collections::HashMap;
use std::fs;

use anyhow::{Context, Result};

use crate::harness::ExperimentId;
use crate::metrics::results_summary;
use crate::report::{load_all_reports, ReportStatus};

const EXPERIMENTS_MD: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/EXPERIMENTS.md");

fn row_key(id: ExperimentId) -> &'static str {
    match id {
        ExperimentId::BmC0 => "BM-C0",
        ExperimentId::BmC1 => "BM-C1",
        ExperimentId::BmC2 => "BM-C2",
        ExperimentId::BmC3 => "BM-C3",
        ExperimentId::BmC4 => "BM-C4",
        ExperimentId::BmC5 => "BM-C5",
        ExperimentId::BmC6 => "BM-C6",
        ExperimentId::BmL0 => "BM-L0",
        ExperimentId::BmL1 => "BM-L1",
        ExperimentId::BmL2 => "BM-L2",
        ExperimentId::BmL3 => "BM-L3",
    }
}

/// Aggregate completed reports per experiment and patch EXPERIMENTS.md.
pub fn fill_experiments_md() -> Result<()> {
    let reports = load_all_reports()?;
    let mut summaries: HashMap<String, String> = HashMap::new();

    for (_, report) in reports {
        if report.status != ReportStatus::Completed {
            continue;
        }
        let id = report.experiment_id.clone();
        let Some(exp) = ExperimentId::parse(&id) else {
            continue;
        };
        let summary = format!(
            "{} ({}/{})",
            results_summary(exp, &report.metrics, report.pass),
            report.dimensions.storage,
            report.dimensions.telemetry
        );
        summaries
            .entry(id)
            .and_modify(|existing| {
                existing.push_str("; ");
                existing.push_str(&summary);
            })
            .or_insert(summary);
    }

    let text = fs::read_to_string(EXPERIMENTS_MD).context("read EXPERIMENTS.md")?;
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();

    for id in ExperimentId::all() {
        let slug = id.slug();
        let key = row_key(*id);
        let result = summaries.get(slug).cloned().unwrap_or_else(|| "—".into());
        for line in &mut lines {
            if line.starts_with('|') && line.contains(key) {
                let cols: Vec<&str> = line.split('|').collect();
                if cols.len() >= 6 {
                    *line = format!(
                        "|{}|{}|{}|{}| {result} |",
                        cols[1], cols[2], cols[3], cols[4]
                    );
                }
            }
        }
    }

    fs::write(EXPERIMENTS_MD, lines.join("\n") + "\n").context("write EXPERIMENTS.md")?;
    Ok(())
}
