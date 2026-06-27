//! JSON report schema for benchmark runs.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::harness::{HardwareDetail, RunDimensions};
use crate::metrics::RunResourceProfile;

/// Full benchmark run report written to `profiling/continuum-bench/reports/`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunReport {
    pub experiment_id: String,
    pub dimensions: ReportDimensions,
    pub hardware_detail: HardwareDetail,
    pub engine_path: String,
    pub started_at: DateTime<Utc>,
    pub elapsed_secs: f64,
    pub metrics: Value,
    /// Process RSS and CPU samples (cloud sizing profiles only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_profile: Option<RunResourceProfile>,
    pub pass_criteria: String,
    pub pass: bool,
    pub status: ReportStatus,
    pub notes: String,
}

/// Dimension subset stored in reports.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportDimensions {
    pub storage: String,
    pub topology: String,
    pub telemetry: String,
    pub hardware: String,
}

impl From<RunDimensions> for ReportDimensions {
    fn from(d: RunDimensions) -> Self {
        Self {
            storage: d.storage.slug().into(),
            topology: d.topology.slug().into(),
            telemetry: d.telemetry.slug().into(),
            hardware: d.hardware.slug().into(),
        }
    }
}

/// Outcome status for skipped runs.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReportStatus {
    Completed,
    SkippedUnsupported,
    SkippedNoRemote,
    Failed,
}

impl RunReport {
    pub fn skipped(
        experiment_id: &str,
        dimensions: RunDimensions,
        hardware_detail: HardwareDetail,
        status: ReportStatus,
        notes: impl Into<String>,
    ) -> Self {
        Self {
            experiment_id: experiment_id.into(),
            dimensions: dimensions.into(),
            hardware_detail,
            engine_path: String::new(),
            started_at: Utc::now(),
            elapsed_secs: 0.0,
            metrics: serde_json::json!({}),
            resource_profile: None,
            pass_criteria: String::new(),
            pass: false,
            status,
            notes: notes.into(),
        }
    }
}
