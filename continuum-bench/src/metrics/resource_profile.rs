//! Process and system resource sampling during benchmark runs (cloud sizing profiles).

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use sysinfo::{MemoryRefreshKind, ProcessRefreshKind, RefreshKind, System};
use tokio::sync::watch;
use tokio::task::JoinHandle;

use crate::util::u64_to_f64;

/// RSS and CPU samples aggregated over a single experiment run.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RunResourceProfile {
    pub process_rss_bytes_start: u64,
    pub process_rss_bytes_end: u64,
    pub process_rss_bytes_peak: u64,
    pub process_cpu_percent_mean: f64,
    pub process_cpu_percent_peak: f64,
    pub system_mem_used_bytes_start: u64,
    pub system_mem_used_bytes_peak: u64,
    pub system_mem_used_bytes_end: u64,
    pub sample_count: u32,
    pub sample_interval_ms: u64,
}

struct SharedSamples {
    process_rss_peak: AtomicU64,
    system_used_peak: AtomicU64,
    cpu_peak_micro: AtomicU64,
    cpu_sum_micro: AtomicU64,
    sample_count: AtomicU32,
}

fn snapshot_process(sys: &mut System, pid: sysinfo::Pid) -> (u64, f64) {
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
    let proc = sys.process(pid);
    let rss = proc.map_or(0, |p| p.memory() * 1024);
    let cpu = proc.map_or(0.0, |p| f64::from(p.cpu_usage()));
    (rss, cpu)
}

fn snapshot_system_used(sys: &mut System) -> u64 {
    sys.refresh_memory();
    sys.used_memory()
}

#[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn cpu_percent_to_micro(cpu: f64) -> u64 {
    (cpu * 10_000.0).round() as u64
}

/// Background sampler; call [`ResourceProfiler::finish`] to stop and build the profile.
pub struct ResourceProfiler {
    stop_tx: watch::Sender<bool>,
    task: JoinHandle<(u64, u64, u64, u64)>,
    shared: Arc<SharedSamples>,
    sample_interval_ms: u64,
}

impl ResourceProfiler {
    pub fn start(sample_interval_ms: u64) -> Self {
        let (stop_tx, stop_rx) = watch::channel(false);
        let shared = Arc::new(SharedSamples {
            process_rss_peak: AtomicU64::new(0),
            system_used_peak: AtomicU64::new(0),
            cpu_peak_micro: AtomicU64::new(0),
            cpu_sum_micro: AtomicU64::new(0),
            sample_count: AtomicU32::new(0),
        });

        let shared_task = Arc::clone(&shared);
        let mut stop_rx_task = stop_rx.clone();
        let interval = Duration::from_millis(sample_interval_ms);

        let task = tokio::spawn(async move {
            let pid = sysinfo::get_current_pid().ok();
            let mut sys = System::new_with_specifics(
                RefreshKind::nothing()
                    .with_memory(MemoryRefreshKind::everything())
                    .with_processes(ProcessRefreshKind::everything()),
            );

            let (start_rss, _) = pid.map_or((0, 0.0), |p| snapshot_process(&mut sys, p));
            let start_sys = snapshot_system_used(&mut sys);

            if let Some(pid) = pid {
                if start_rss > 0 {
                    shared_task
                        .process_rss_peak
                        .fetch_max(start_rss, Ordering::Relaxed);
                }
                if start_sys > 0 {
                    shared_task
                        .system_used_peak
                        .fetch_max(start_sys, Ordering::Relaxed);
                }
                snapshot_process(&mut sys, pid);
                tokio::time::sleep(Duration::from_millis(200)).await;
            }

            while !*stop_rx_task.borrow_and_update() {
                if let Some(pid) = pid {
                    let (rss, cpu) = snapshot_process(&mut sys, pid);
                    if rss > 0 {
                        shared_task
                            .process_rss_peak
                            .fetch_max(rss, Ordering::Relaxed);
                    }
                    let cpu_micro = cpu_percent_to_micro(cpu);
                    shared_task.sample_count.fetch_add(1, Ordering::Relaxed);
                    shared_task.cpu_sum_micro.fetch_add(cpu_micro, Ordering::Relaxed);
                    let peak = shared_task.cpu_peak_micro.load(Ordering::Relaxed);
                    if cpu_micro > peak {
                        shared_task.cpu_peak_micro.store(cpu_micro, Ordering::Relaxed);
                    }
                }
                let used = snapshot_system_used(&mut sys);
                if used > 0 {
                    shared_task
                        .system_used_peak
                        .fetch_max(used, Ordering::Relaxed);
                }
                tokio::time::sleep(interval).await;
            }

            let (end_rss, _) = pid.map_or((0, 0.0), |p| snapshot_process(&mut sys, p));
            let end_sys = snapshot_system_used(&mut sys);
            (start_rss, end_rss, start_sys, end_sys)
        });

        Self {
            stop_tx,
            task,
            shared,
            sample_interval_ms,
        }
    }

    pub async fn finish(self) -> RunResourceProfile {
        let _ = self.stop_tx.send(true);
        let (start_rss, end_rss, start_sys, end_sys) = self.task.await.unwrap_or((0, 0, 0, 0));

        let count = self.shared.sample_count.load(Ordering::Relaxed);
        let cpu_sum_micro = self.shared.cpu_sum_micro.load(Ordering::Relaxed);
        let cpu_peak_micro = self.shared.cpu_peak_micro.load(Ordering::Relaxed);
        let cpu_mean = if count > 0 {
            u64_to_f64(cpu_sum_micro) / 10_000.0 / f64::from(count)
        } else {
            0.0
        };
        let cpu_peak = u64_to_f64(cpu_peak_micro) / 10_000.0;

        let process_rss_peak = self
            .shared
            .process_rss_peak
            .load(Ordering::Relaxed)
            .max(start_rss)
            .max(end_rss);

        let system_peak = self
            .shared
            .system_used_peak
            .load(Ordering::Relaxed)
            .max(start_sys)
            .max(end_sys);

        RunResourceProfile {
            process_rss_bytes_start: start_rss,
            process_rss_bytes_end: end_rss,
            process_rss_bytes_peak: process_rss_peak,
            process_cpu_percent_mean: cpu_mean,
            process_cpu_percent_peak: cpu_peak,
            system_mem_used_bytes_start: start_sys,
            system_mem_used_bytes_peak: system_peak,
            system_mem_used_bytes_end: end_sys,
            sample_count: count,
            sample_interval_ms: self.sample_interval_ms,
        }
    }
}
