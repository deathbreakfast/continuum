# Continuum benchmark reports

JSON reports from `continuum-bench` runs. Each file captures one experiment × dimension combination.

**Research paper:** lab results in [`continuum-bench/PERFORMANCE_STUDY.md`](../../continuum-bench/PERFORMANCE_STUDY.md) Appendix A (39 dev-wsl runs, June 2025). **Cloud baselines** (throughput, replay/checkpoint/truncate, cost) in Appendix D (partial, June 2026: `aws-t3-medium`, `aws-t3-small`, `aws-t4g-medium`). **Distributed surreal-tikv** (budget cloud campaign) → Appendix E.

## Path

**Embedded adapters:**
`profiling/continuum-bench/reports/{experiment_id}-{storage}-{topology}-{telemetry}-{hardware}.json`

**surreal-tikv** (distributed path):
`profiling/continuum-bench/reports/{experiment_id}-surreal-tikv-{tikv_topology}-{telemetry}-{hardware}.json`

Example: `bm-l2-surreal-tikv-tikv-minimal-off-aws-t4g-medium.json`

**Fleet projections:**
`projection-{hardware}-surreal-tikv-{tikv_topology}.json`

## Budget surreal-tikv campaign (2026)

- **Scope:** colocated `tikv-minimal` on `aws-t4g-medium` / `aws-t3-medium` only
- **Phase 4 deferred:** ha-3/scale-5/surreal-* require multi-EC2 — not colocated on 4 GiB
- **`component_hardware`** in surreal-tikv reports: `runtime`, optional `surreal`, optional `tikv` slugs (same slug when colocated)
- **Do not run `fill-results`** for this campaign — update EXPERIMENTS.md and Appendix E manually

## Invalid data

**Do not use for analysis:** `postgres` reports on `aws-t3-medium` and `aws-t3-small` from 2026-06-26 — pre-fix adapter (`PostgresLogBackend::from_pool`). Valid postgres baseline: `aws-t4g-medium` (2026-06-27) until t3 re-run.

**Do not cite** `resource_profile.process_rss_bytes_*` on cloud runs until RSS measurement is fixed (implausible values vs instance RAM).

## Schema

| Field | Description |
|-------|-------------|
| `experiment_id` | e.g. `bm-c0` |
| `dimensions` | `storage`, `topology`, `telemetry`, `hardware`; surreal-tikv adds `tikv_topology`, `surreal_instances`, `surreal_deployment` |
| `component_hardware` | Optional — `runtime`, `surreal`, `tikv` slugs (surreal-tikv runs) |
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
# surreal-tikv budget campaign:
continuum-bench/scripts/run-tikv-preset.sh tikv-minimal aws-t4g-medium --skip-c6
```

Experiment registry and CLI reference: [`continuum-bench/EXPERIMENTS.md`](../../continuum-bench/EXPERIMENTS.md).
