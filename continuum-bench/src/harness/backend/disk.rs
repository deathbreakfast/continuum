//! Disk path helpers for on-disk storage growth metrics.

use std::path::PathBuf;

use anyhow::Result;

/// Directory or file size in bytes (for on-disk storage growth metrics).
pub fn dir_size_bytes(path: &PathBuf) -> u64 {
    dir_size_recursive(path).unwrap_or(0)
}

fn dir_size_recursive(path: &PathBuf) -> Result<u64> {
    let mut total = 0u64;
    if path.is_file() {
        return Ok(path.metadata()?.len());
    }
    if path.is_dir() {
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            total += dir_size_recursive(&entry.path())?;
        }
    }
    Ok(total)
}

/// Parse an engine path into a local disk path when applicable.
pub fn storage_disk_path(engine_path: &str) -> Option<PathBuf> {
    if let Some(path) = engine_path.strip_prefix("rocksdb://") {
        return Some(PathBuf::from(path));
    }
    if let Some(path) = engine_path.strip_prefix("sqlite://") {
        let path = path.split('?').next()?;
        return Some(PathBuf::from(path));
    }
    None
}
