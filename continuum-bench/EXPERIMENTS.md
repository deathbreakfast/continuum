# Continuum benchmark experiment registry

Pre-registered experiment IDs, dimension matrix, Results log, and runner commands.

**Methodology, scale analysis, and interpretation:** see [`PERFORMANCE_STUDY.md`](PERFORMANCE_STUDY.md) (standalone research paper). This file is the operational record of what was run and what passed.

---

## Dimensions

| Dimension | Values |
|-----------|--------|
| Storage | `mem`, `surreal-rocksdb`, `surreal-mem`, `surreal-tikv`, `postgres`, `sqlite` |
| Topology | `isolated-lab`, `shared-handle`, `remote-surreal` |
| Telemetry | `off`, `console`, `stub` |
| Hardware | `dev-wsl`, `ci-small`, `bare-metal-{small,medium,large}`, `aws-t3-medium`, `aws-t3-small`, `aws-t4g-{small,medium,large}`, `aws-i4i-xlarge`, `aws-c7i-4xlarge` |
| TiKV topology | `tikv-minimal`, `tikv-ha-3`, `tikv-scale-5`, `custom` (via env) |
| Surreal deployment | `colocated`, `remote`, `multi-node` |
| Surreal instances | `1`, `2`, `4` (multi-node presets; env `CONTINUUM_BENCH_SURREAL_INSTANCES`) |
| Component hardware | `runtime`, optional `surreal`, optional `tikv` profiles in report JSON |

**Planned hardware matrix (tiered):**

| Tier | Profiles | Question |
|------|----------|----------|
| Cost-effective | `ci-small`, `aws-t3.medium`, `aws-t4g.small`, `bare-metal-small` | Can a modest machine handle my load? |
| Scale | `aws-c7i-4xlarge`, `aws-i4i.xlarge`, `bare-metal-large` | What is the upper envelope? |

Extend [`dimensions/mod.rs`](src/harness/dimensions/mod.rs) when instances are provisioned; results → paper Appendix D/E.

Each run JSON records CPU, RAM, root mount, host drive, and `engine_path` in `hardware_detail`.

**Cloud sizing profiles** (`aws-t3-medium`, `aws-t3-small`, future `aws-*`): each completed run also includes `resource_profile` — process RSS (start/end/peak), mean/peak CPU % (1 s samples), and system used RAM (start/end/peak). **`dev-wsl`** omits `resource_profile` (shared dev machine; lab sanity only).

---

## Experiment log

| ID | Workload | Primary metric | Pass criteria | Results (dev-wsl) |
|----|----------|----------------|---------------|-------------------|
| BM-C0 | Raw append | p50/p95 append ms | Flat vs op count at 5k ops | p50=0.003ms p95=0.012ms PASS (mem/console); p50=0.001ms p95=0.002ms PASS (mem/off); p50=1.073ms p95=1.844ms PASS (surreal-mem/console); p50=0.906ms p95=1.331ms PASS (surreal-mem/off); p50=8.424ms p95=14.507ms PASS (surreal-rocksdb/console); p50=8.322ms p95=11.321ms PASS (surreal-rocksdb/off) |
| BM-C1 | Batched append 1/10/100/1000 | events/s | Throughput scales with batch | 1=986698/s 1000=1818299/s PASS (mem/off); 1=935/s 1000=4531/s PASS (surreal-mem/off); 1=115/s 1000=3401/s PASS (surreal-rocksdb/off) |
| BM-C2 | Read-tail / fanout | poll ms vs table size | Flat at 100k rows | p95@100k=0.612ms FAIL (mem/off); p95@100k=0.283ms PASS (surreal-mem/off); p95@100k=0.330ms PASS (surreal-rocksdb/off) |
| BM-C3 | Checkpoint churn | checkpoint upsert ms | Flat over 10k commits | p95=0.001ms PASS (mem/off); p95=0.742ms PASS (surreal-mem/off); p95=6.591ms FAIL (surreal-rocksdb/off) |
| BM-C4 | Truncate reclaim | space + read ms post-truncate | Read stable after truncate | post/pre=0.25x PASS (mem/off); post/pre=1.15x PASS (surreal-mem/off); post/pre=0.93x PASS (surreal-rocksdb/off) |
| BM-C5 | Co-tenancy | same vs isolated handle growth | Growth only on same handle | ratio=0.43 FAIL (mem/off); ratio=21.27 PASS (surreal-mem/off); ratio=0.20 FAIL (surreal-rocksdb/off) |
| BM-C6 | Paced 1h soak | growth ratio | &lt;2× isolated at 1 op/s | growth=1.19x PASS (mem/off); growth=1.03x PASS (surreal-mem/off); growth=101.49x FAIL (surreal-rocksdb/off) |
| BM-L0 | Load 100 ops/s | sustained p99 | error rate &lt;0.1% | p99=0.073ms 100/s err=0.0000% PASS (mem/off); p99=7.109ms 100/s err=0.0000% PASS (surreal-mem/off); p99=57.764ms 95/s err=0.0000% PASS (surreal-rocksdb/off) |
| BM-L1 | Load 1k ops/s | sustained p99 | error rate &lt;0.1% | p99=0.143ms 1000/s err=0.0000% PASS (mem/console); p99=0.054ms 1000/s err=0.0000% PASS (mem/off); p99=1.826ms 832/s err=0.0000% PASS (surreal-mem/console); p99=1.633ms 955/s err=0.0000% PASS (surreal-mem/off); p99=27.319ms 93/s err=0.0000% PASS (surreal-rocksdb/console); p99=14.675ms 110/s err=0.0000% PASS (surreal-rocksdb/off) |
| BM-L2 | Load 10k ops/s | sustained p99 | error rate &lt;0.1% | p99=0.032ms 10000/s err=0.0000% PASS (mem/off); p99=2.723ms 783/s err=0.0000% PASS (surreal-mem/off); p99=519.498ms 25/s err=0.0000% PASS (surreal-rocksdb/off) |
| BM-L3 | Load 100k ops/s | sustained p99 | error rate &lt;0.1% | p99=0.014ms 99998/s err=0.0000% PASS (mem/off); p99=2.107ms 849/s err=0.0000% PASS (surreal-mem/off); p99=18.601ms 101/s err=0.0000% PASS (surreal-rocksdb/off) |
---

## Cloud results

Research-question coverage and interpretation: [`PERFORMANCE_STUDY.md`](PERFORMANCE_STUDY.md) §1.3, §5.5, Appendix D.

**Postgres on x86 (t3):** reports through 2026-06-26 used a pre-fix adapter (`PostgresLogBackend::from_pool` — `BYTEA` / `$n` placeholder bugs). Those JSONs are **invalid for analysis**; t4g postgres (post-fix) is the valid postgres baseline until t3 re-run.

### `aws-t3-medium` (2026-06-26) — full matrix + SQL subset

**Instance:** `t3.medium`, us-west-2, Amazon Linux 2023, 2 vCPU, ~3.7 GiB RAM, gp3 EBS. **59** reports; **37/48** isolated-lab/off PASS (postgres 10 invalid).

| ID | mem | sqlite | postgres | surreal-mem | surreal-rocksdb |
|----|-----|--------|----------|-------------|-----------------|
| **BM-C0** p50 / p95 (ms) | 0.002 / 0.005 PASS | 0.500 / 0.562 PASS | **invalid** | 1.402 / 1.959 PASS | 2.146 / 2.278 PASS |
| **BM-C1** batch 1 → 1000 (events/s) | 213k → 690k PASS | 1951 → 4102 PASS | **invalid** | 673 → 1265 PASS | 464 → 983 PASS |
| **BM-C2** p95 poll @100k (ms) | 0.503 FAIL | 0.089 PASS | **invalid** | 0.270 PASS | 0.410 PASS |
| **BM-C3** p95 checkpoint (ms) | 0.003 PASS | 0.315 PASS | **invalid** | 0.594 PASS | 1.061 PASS |
| **BM-C4** post/pre read ratio | 0.48× PASS | 1.35× PASS | **invalid** | 0.99× PASS | 1.13× PASS |
| **BM-C5** growth ratio (monolith) | 1.00 FAIL | 0.00 FAIL | **invalid** | 1.00 FAIL | 0.20 FAIL |
| **BM-C6** growth ratio (1 h) | 0.00× PASS | — | — | 0.00× PASS | 109.77× FAIL |
| **BM-L0** achieved / p99 | 100 / 0.045 ms | 100 / 6.463 ms | **invalid** | — | — |
| **BM-L1** achieved / p99 | 1000 / 0.021 ms | 1000 / 0.919 ms | **invalid** | — | — |
| **BM-L2** achieved / p99 | 10000 / 0.014 ms | 1909 / 0.786 ms | **invalid** | — | — |
| **BM-L3** achieved / p99 | 99999 / 0.009 ms | 1898 / 0.776 ms | **invalid** | — | — |

**SQL subset (`--subset sql`, 20 runs):** sqlite 9/10 pass (BM-C4 PASS); postgres 0/10 (adapter invalid).

### `aws-t3-small` (2026-06-26) — partial full matrix + SQL subset

**Instance:** `t3.small`, 2 vCPU, ~1.9 GiB RAM. Full 39-run matrix **not viable** (stalled BM-C1 `surreal-rocksdb`). **47** reports; lite `mem`/`surreal-mem` + **20** SQL subset runs synced.

| ID | mem | sqlite | postgres | surreal-mem | surreal-rocksdb |
|----|-----|--------|----------|-------------|-----------------|
| **BM-C0** p50 / p95 (ms) | 0.003 / 0.006 PASS | 0.516 / 1.054 PASS | **invalid** | 1.418 / 1.506 PASS | 2.109 / 2.208 PASS |
| **BM-C1** batch 1 → 1000 | 306k → 717k PASS | 1412 → 3842 PASS | **invalid** | 700 → 1335 PASS | **stalled** |
| **BM-C2** p95 @100k | 0.341 FAIL | 0.092 PASS | **invalid** | 0.265 PASS | — |
| **BM-C3** p95 checkpoint | 0.001 PASS | 0.236 PASS | **invalid** | 0.578 PASS | — |
| **BM-C4** post/pre | 0.46× PASS | 0.87× PASS | **invalid** | 1.02× PASS | — |
| **BM-C5** growth ratio | 8.22 PASS | 0.00 FAIL | **invalid** | 1.70 PASS | — |
| **BM-C6** growth | 1.06× PASS | — | — | 1.07× PASS | — |
| **BM-L0–L3** sqlite | — | 100→1880/s PASS | **invalid** | lite mem PASS | — |

**Viability:** not recommended for full surreal-rocksdb matrix on 2 GiB; sqlite SQL subset and lite `mem`/`surreal-mem` OK. See Appendix D.1.1.

### `aws-t4g-medium` (2026-06-27) — full matrix + SQL subset

**Instance:** `t4g.medium`, us-west-2, ARM, 2 vCPU, ~3.7 GiB RAM, gp3 EBS. **59** reports; **45/48** isolated-lab/off PASS.

| ID | mem | sqlite | postgres | surreal-mem | surreal-rocksdb |
|----|-----|--------|----------|-------------|-----------------|
| **BM-C0** p50 / p95 (ms) | 0.002 / 0.003 PASS | 0.480 / 0.547 PASS | 3.970 / 4.305 PASS | 1.781 / 2.043 PASS | 2.560 / 3.041 PASS |
| **BM-C1** batch 1 → 1000 | 341k → 884k PASS | 1995 → 4056 PASS | 248 → 516 PASS | 518 → 1340 PASS | 364 → 921 PASS |
| **BM-C2** p95 @100k | 0.480 FAIL | 0.091 PASS | 0.610 PASS | 0.318 PASS | 0.668 PASS |
| **BM-C3** p95 checkpoint | 0.001 PASS | 0.266 PASS | 2.172 PASS | 0.769 PASS | 1.495 PASS |
| **BM-C4** post/pre | 0.40× PASS | **8.49× FAIL** | 0.93× PASS | 1.11× PASS | 1.18× PASS |
| **BM-C5** growth ratio | 1.00 FAIL | 28.18 PASS | 0.00 FAIL | 4194304× PASS* | 0.20 FAIL |
| **BM-C6** growth | 0.00× PASS | — | — | 0.00× PASS | 105.65× FAIL |
| **BM-L0** achieved / p99 | 100 / 0.029 ms | 100 / 1.022 ms | 100 / 7.168 ms | — | — |
| **BM-L1** achieved / p99 | 1000 / 0.022 ms | 1000 / 0.859 ms | 246 / 5.749 ms | — | — |
| **BM-L2** achieved / p99 | 10000 / 0.016 ms | 1928 / 0.752 ms | 246 / 5.628 ms | — | — |
| **BM-L3** achieved / p99 | 99998 / 0.007 ms | 1874 / 0.914 ms | 242 / 5.827 ms | — | — |

\*surreal-mem BM-C5 ratio suspect (measurement edge case). **SQL subset:** 18/20 pass (BM-C4 sqlite FAIL, BM-C5 postgres FAIL).

**Postgres truncate (BM-C4):** PASS on t4g (0.93×, 24,999 rows removed)—not OOM. t3 postgres never reached truncate (adapter init failure).

---

## Distributed Surreal/TiKV campaign (budget cloud — completed 2026-06-30)

Existing **BM-C\*** and **BM-L\*** experiment IDs apply to `surreal-tikv`. **Phase 4** (multi-TiKV / multi-Surreal topology sweeps) is **deferred** until `infra/surreal-tikv-aws/` multi-EC2 infra is built and merged.

**Scope:** colocated `tikv-minimal` on `aws-t4g-medium` and `aws-t3-medium` (us-west-2). **Out of scope:** dev-wsl, high-end instances, postgres, `aws-t3-small` (not attempted), colocated ha-3/scale-5/surreal-* on 4 GiB.

**Do not run `fill-results`** for this campaign — update this file and [`PERFORMANCE_STUDY.md`](PERFORMANCE_STUDY.md) Appendix E manually.

| Phase | Goal | Status |
|-------|------|--------|
| 0 | Harness fixes (`--tikv-topology`, `run-tikv-preset.sh`, infra ulimits, Surreal v3.1.5) | **Done** |
| 1 | Feasibility gate (bm-c0 + BM-L0–L3) | **Pass** (both profiles) |
| 2 | Full operational + ceiling path | **Done** (9 reports × 2 hardware) |
| 3 | Fleet projection + cost | **Done** |
| 4 | Topology/count sweeps (multi-EC2) | **Deferred** |

**Infra fixes applied:** TiKV `ulimits.nofile` in compose; SurrealDB image `v3.1.5` (matches bench client); remote WS auth (`protocol-ws` + `CONTINUUM_BENCH_SURREAL_USER/PASS`).

### Feasibility (colocated budget)

| Preset | RAM hint | Colocated on t3/t4g.medium (4 GiB) | Result |
|--------|----------|-------------------------------------|--------|
| `tikv-minimal` | ~8 GiB | MAYBE (4 GiB swap recommended) | **Pass** — ~37–43 ops/s ceiling |
| `tikv-ha-3` | ~16 GiB | **NO** | Not tested — requires multi-EC2 |
| `tikv-scale-5` | ~16+ GiB | **NO** | Not tested — requires multi-EC2 |
| `surreal-2n` | ~16 GiB | **NO** | Not tested — requires multi-EC2 |
| `surreal-4n` | ~20+ GiB | **NO** | Not tested — requires multi-EC2 |

### Results — `tikv-minimal` colocated (2026-06-30)

#### `aws-t4g-medium` (ARM, 2 vCPU, ~3.7 GiB + 4 GiB swap)

| ID | Result | Notes |
|----|--------|-------|
| BM-C0 | PASS | p50=26.3ms p95=29.9ms |
| BM-C1 | PASS | 37/s → 124/s (batch 1→1000) |
| BM-C2 | PASS | p95@100k=5.6ms |
| BM-C3 | PASS | p95 checkpoint=15.3ms |
| BM-C4 | PASS | post/pre=0.96× |
| BM-L0–L3 | PASS | ceiling **~37.7 ops/s** (L3), p99 ~34.5ms |

#### `aws-t3-medium` (x86, 2 vCPU, ~3.7 GiB + 4 GiB swap)

| ID | Result | Notes |
|----|--------|-------|
| BM-C0 | PASS | p50=18.3ms p95=25.0ms |
| BM-C1 | PASS | 43/s → 142/s (batch 1→1000) |
| BM-C2 | PASS | p95@100k=7.9ms |
| BM-C3 | **FAIL** | p95 checkpoint=15.3ms (decile slope criterion) |
| BM-C4 | PASS | post/pre=1.03× |
| BM-L0–L3 | PASS | ceiling **~43.5 ops/s** (L3), p99 ~34.6ms |

**Compare Appendix D (same hardware):** sqlite ~1,928/s L2; surreal-rocksdb ~340–440/s L2; postgres ~246/s L2 (t4g). surreal-tikv colocated minimal is **~50–100× slower** than sqlite on burstable cloud.

### Fleet projection (`tikv-minimal`, compute-only us-west-2 on-demand)

| Hardware | Per-node ceiling (L3) | $/M ops | Nodes for 1B/s | Compute $/hr @ 1B/s |
|----------|----------------------|---------|----------------|---------------------|
| `aws-t4g-medium` | 37.7 ops/s | $0.248 | 26,558,591 | ~$892k/hr |
| `aws-t3-medium` | 43.5 ops/s | $0.266 | 22,992,285 | ~$956k/hr |

Projection JSON: `profiling/continuum-bench/reports/projection-aws-*-surreal-tikv-tikv-minimal.json`

### Runbooks

Region: **us-west-2**. SSH key: `~/.ssh/continuum-bench.pem`. Pre-build binary off-instance; SCP to worker (do not compile on burstable worker).

```bash
# On EC2 worker (after Docker + optional 4 GiB swap)
infra/surreal-tikv/scripts/up.sh tikv-minimal
eval "$(infra/surreal-tikv/scripts/export-env.sh tikv-minimal)"
export CONTINUUM_BENCH_SURREAL_HARDWARE=aws-t4g-medium
export CONTINUUM_BENCH_TIKV_HARDWARE=aws-t4g-medium

continuum-bench run bm-c0 --storage surreal-tikv \
  --tikv-topology tikv-minimal --hardware aws-t4g-medium --telemetry off

continuum-bench matrix --subset tikv-projection-inputs \
  --hardware aws-t4g-medium --tikv-topology tikv-minimal \
  --skip-experiments bm-c6 --skip-existing
```

Pass: bm-c0 completes; BM-L0–L3 JSON with non-zero `achieved_ops_per_sec`; no OOM/hang >30 min. Repeat on `aws-t3-medium` if t4g passes.

### Phase 2 runbook (full campaign)

```bash
continuum-bench/scripts/run-tikv-preset.sh tikv-minimal aws-t4g-medium --skip-c6
# repeat for aws-t3-medium
```

Runs `tikv-lab-colocated` filtered to `tikv-minimal` (BM-C1/C2/C3/C4 + BM-L0–L3; skips BM-C5/C6). Run BM-C6 separately after minimal path is stable.

### Phase 3 — Fleet projection

```bash
cargo run -p continuum-bench -- project-fleet \
  --hardware aws-t4g-medium --storage surreal-tikv --tikv-topology tikv-minimal
```

### Phase 4 — Multi-EC2 (deferred)

Topology/count sweeps (`tikv-ha-3`, `tikv-scale-5`, `surreal-2n`) require separate EC2 instances — see future `infra/surreal-tikv-aws/`. Planned layout: bench on 1× t3.medium; PD+TiKV on 1–3× t3.small; Surreal on 1–2× t3.small.

**Reports (surreal-tikv):** `{id}-surreal-tikv-{tikv_topology}-{telemetry}-{hardware}.json`

**Compare against Appendix D baselines** on same hardware: sqlite ~1.9k/s L2; surreal-rocksdb ~340–440/s L2; postgres ~246/s L2 (t4g.medium).

---

## Run

```bash
# Single experiment
cargo run -p continuum-bench -- run bm-c0 --storage mem --telemetry off

# Full dev-wsl matrix
cargo run --release -p continuum-bench -- matrix --hardware dev-wsl

# Resume / skip completed reports
cargo run --release -p continuum-bench -- matrix --from bm-c4 --skip-existing

# SQL adapter subset (sqlite + postgres, telemetry off, skips BM-C6 soak)
export CONTINUUM_BENCH_POSTGRES_URL=postgres://postgres:bench@localhost:5432/bench
cargo run --release -p continuum-bench -- matrix --hardware dev-wsl --subset sql --skip-existing
# or: continuum-bench/scripts/run-sql-matrix.sh dev-wsl

# Sync Results column from JSON (optional)
cargo run -p continuum-bench -- fill-results

# Hardware profile (CPU, RAM, root mount, host drive)
cargo run -p continuum-bench -- hardware

# Fleet projection from BM-L* reports
cargo run -p continuum-bench -- project-fleet --storage surreal-tikv --tikv-topology tikv-ha-3

# TiKV campaign slices (require CONTINUUM_BENCH_SURREAL_URL; budget cloud default)
continuum-bench/scripts/run-tikv-preset.sh tikv-minimal aws-t4g-medium --skip-c6
cargo run --release -p continuum-bench -- matrix --subset tikv-projection-inputs \
  --hardware aws-t4g-medium --tikv-topology tikv-minimal --skip-experiments bm-c6
```

**PostgreSQL:** set `CONTINUUM_BENCH_POSTGRES_URL` to include postgres in the matrix and run `--storage postgres`. **SQLite** is included in the default matrix (temp file per run).

**PostgreSQL for benchmarks (Docker):**

```bash
docker run -d --name continuum-bench-pg \
  -e POSTGRES_PASSWORD=bench -e POSTGRES_DB=bench \
  -p 5432:5432 postgres:16-alpine
export CONTINUUM_BENCH_POSTGRES_URL=postgres://postgres:bench@localhost:5432/bench
```

Use the same pattern on EC2 (one container per instance). SQL subset: `--subset sql` (11 experiments × sqlite/postgres, no BM-C6).

**Reports:** `profiling/continuum-bench/reports/{experiment}-{storage}-{topology}-{telemetry}-{hardware}.json`

**Paper appendix:** lab tables in [`PERFORMANCE_STUDY.md`](PERFORMANCE_STUDY.md) Appendix A; cloud baselines in Appendix D (partial, June 2026).
