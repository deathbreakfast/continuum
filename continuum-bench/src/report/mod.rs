//! Benchmark report schema and persistence.

pub mod schema;
pub mod write;

pub use schema::{ReportStatus, RunReport};
pub use write::{load_all_reports, report_filename, reports_dir, write_report};
