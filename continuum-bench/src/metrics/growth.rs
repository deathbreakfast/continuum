//! Process memory and on-disk growth tracking.

use sysinfo::{ProcessRefreshKind, RefreshKind, System};

use crate::util::u64_to_f64;

/// Snapshot of process RSS in bytes.
pub fn process_rss_bytes() -> u64 {
    let pid = sysinfo::get_current_pid().ok();
    let Some(pid) = pid else {
        return 0;
    };
    let mut sys = System::new_with_specifics(
        RefreshKind::nothing().with_processes(ProcessRefreshKind::everything()),
    );
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
    sys.process(pid)
        .map_or(0, |p| p.memory() * 1024)
}

/// Ratio of after/before with guard for zero before.
pub fn growth_ratio(before: u64, after: u64) -> f64 {
    if before == 0 {
        if after == 0 {
            1.0
        } else {
            f64::INFINITY
        }
    } else {
        u64_to_f64(after) / u64_to_f64(before)
    }
}
