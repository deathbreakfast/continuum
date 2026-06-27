# Continuum benchmark reports

JSON reports from `continuum-bench` runs. Each file captures one experiment × dimension combination.

**Research paper:** lab results in [`continuum-bench/PERFORMANCE_STUDY.md`](../../continuum-bench/PERFORMANCE_STUDY.md) Appendix A (39 dev-wsl runs, June 2025). **Cloud baselines** (throughput, replay/checkpoint/truncate, cost) in Appendix D (partial, June 2026: `aws-t3-medium`, `aws-t3-small`, `aws-t4g-medium`).

## Path

`profiling/continuum-bench/reports/{experiment_id}-{storage}-{topology}-{telemetry}-{hardware}.json`

## Invalid data

**Do not use for analysis:** `postgres` reports on `aws-t3-medium` and `aws-t3-small` from 2026-06-26 — pre-fix adapter (`PostgresLogBackend::from_pool`). Valid postgres baseline: `aws-t4g-medium` (2026-06-27) until t3 re-run.

**Do not cite** `resource_profile.process_rss_bytes_*` on cloud runs until RSS measurement is fixed (implausible values vs instance RAM).

## Schema

| Field | Description |
|-------|-------------|
| `experiment_id` | e.g. `bm-c0` |
| `dimensions` | `storage`, `topology`, `telemetry`, `hardware` |
| `hardware_detail` | CPU, RAM, OS, `root_mount`, `host_drive`, per-run `engine_path` |
| `started_at` | UTC timestamp |
| `elapsed_secs` | Wall time |
| `metrics` | Experiment-specific (p50/p95, events/s, growth ratio, etc.) |
| `resource_profile` | Optional — process RSS + CPU + system RAM (cloud sizing hardware only; see below) |
| `pass_criteria` | Pre-registered criterion (see performance study §4.3) |
| `pass` | Boolean pass/fail |
| `status` | `completed`, `skipped_unsupported`, `skipped_no_remote`, `failed` |
| `notes` | Human-readable summary |

### `resource_profile` (cloud sizing runs)

Present when `dimensions.hardware` is a cloud sizing profile (e.g. `aws-t3-medium`). Omitted for `dev-wsl` lab sanity runs.

| Field | Description |
|-------|-------------|
| `process_rss_bytes_start` / `end` / `peak` | Benchmark process RSS |
| `process_cpu_percent_mean` / `peak` | Process CPU utilization during run |
| `system_mem_used_bytes_start` / `end` / `peak` | Host used memory |
| `sample_count` | Background samples (1 s interval) |
| `sample_interval_ms` | Sample interval |

## Generate

```bash
cargo run --release -p continuum-bench -- matrix --hardware dev-wsl
cargo run -p continuum-bench -- fill-results
```

Experiment registry and CLI reference: [`continuum-bench/EXPERIMENTS.md`](../../continuum-bench/EXPERIMENTS.md).
