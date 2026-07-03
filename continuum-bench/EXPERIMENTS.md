# Continuum benchmark experiment registry

Pre-registered experiment IDs, dimension matrix, Results log, and runner commands.

**Methodology, scale analysis, and interpretation:** see [`PERFORMANCE_STUDY.md`](PERFORMANCE_STUDY.md) (standalone research paper). This file is the operational record of what was run and what passed.

---

## Dimensions

| Dimension | Values |
|-----------|--------|
| Storage | `mem`, `surreal-rocksdb`, `surreal-mem`, `surreal-tikv`, `postgres`, `sqlite`, **`scylla`**, **`tikv-raw`** |
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
| **BM-P1** | Multi-partition append (round-robin) | aggregate ops/s | error rate &lt;0.1% | *(native-scale campaign — see §Native adapters)* |
| **BM-P2** | Multi-partition tail read | poll p95 ms | error rate &lt;0.1% | *(native-scale campaign)* |
| **BM-M1** | Multi-client append (C workers) | aggregate ops/s | error rate &lt;0.1% | *(native-scale campaign)* |
| **BM-M2** | Multi-client ceiling (default C=64) | aggregate ops/s + p99 | error rate &lt;0.1%; feeds fleet projection | *(native-scale campaign)* |
| **BM-M3** | Multi-client hot stream (`key=None`) | aggregate ops/s + p99 | error rate &lt;0.1%; Track M concurrency ladder | *(native-concurrency campaign)* |
| **BM-M4** | Multi-client spread partitions (C×K) | aggregate ops/s + p99 | error rate &lt;0.1%; Track P concurrent spread | *(partition-campaign)* |
---

## Native adapters (ScyllaDB + raw TiKV)

**Infra:** [`infra/scylla/`](../infra/scylla/) (profiles `scylla-1`, `scylla-3`), [`infra/tikv-raw/`](../infra/tikv-raw/) (profile `tikv-raw-minimal`, host networking on Linux/WSL).

| Env | Purpose |
|-----|---------|
| `CONTINUUM_BENCH_SCYLLA_CONTACT_POINTS` | Comma-separated CQL contact points |
| `CONTINUUM_BENCH_SCYLLA_KEYSPACE` | Keyspace (default `continuum`) |
| `CONTINUUM_BENCH_TIKV_PD_ENDPOINT` | PD URL (default `127.0.0.1:2379`) |
| `CONTINUUM_BENCH_PARTITION_COUNT` | BM-P* partition sweep (default 10) |
| `CONTINUUM_BENCH_CLIENT_COUNT` | BM-M* client sweep (default 8 / 64) |
| `CONTINUUM_BENCH_LOAD_PARTITION_COUNT` | BM-L* partitioned load (default 1 = hot stream) |
| `CONTINUUM_BENCH_SCYLLA_TOPOLOGY` | `scylla-1`, `scylla-2n`, `scylla-3n`, `scylla-4n` (report dimension) |
| `CONTINUUM_BENCH_TIKV_TOPOLOGY` | `tikv-minimal`, `tikv-ha-2`, `tikv-ha-3`, `tikv-scale-4`, `tikv-scale-5` |

**Matrix subsets:** `native-lab` (BM-C0–C4, BM-L0–L3), **`native-lab-partitioned`** (BM-L0–L3 with load partition env), `native-scale` (BM-P1/P2/M1/M2/M4), `native-concurrency` (BM-M3), `native-projection-inputs` (BM-L0–L3 + BM-M2), **`native-topology`** (projection + scale for Phase B).

**Canonical hardware (July 2026 campaign):** **`aws-t3-medium` only** for native + partitioning + scale-out. Reuse June 2026 sqlite/surreal baselines on the same profile. `dev-wsl` / `aws-t4g-*` / `aws-t3-small` are out of scope for this study.

**AWS infra:** [`infra/native-aws/`](../infra/native-aws/) — Phase A colocated (2× t3.medium), Phase B topologies (`native-scylla-2n`, `native-scylla-4n`, `native-tikv-ha-2`, `native-tikv-scale-4`; optional `native-scylla-3n`, `native-tikv-ha-3`, `native-tikv-scale-5`).

**Container images** (pinned in [`infra/native-aws/config/defaults.env`](../infra/native-aws/config/defaults.env)): `scylladb/scylla:6.2`, `pingcap/pd:v8.5.0`, `pingcap/tikv:v8.5.0`.

```bash
# Phase A
infra/native-aws/scripts/provision-colocated.sh both
infra/native-aws/scripts/bootstrap.sh native-colocated scylla   # parallel
infra/native-aws/scripts/bootstrap.sh native-colocated tikv
infra/native-aws/scripts/build-al2023.sh
infra/native-aws/scripts/deploy-bench.sh native-colocated target/al2023/continuum-bench
infra/native-aws/scripts/run-campaign.sh native-lab
infra/native-aws/scripts/run-campaign.sh partition-campaign
infra/native-aws/scripts/fetch-reports.sh

# Phase B — Track T distributed scaling (one topology at a time)
infra/native-aws/scripts/provision-topology.sh native-scylla-2n
infra/native-aws/scripts/bootstrap-topology.sh native-scylla-2n
infra/native-aws/scripts/preflight-topology.sh native-scylla-2n
infra/native-aws/scripts/deploy-bench.sh native-scylla-2n target/al2023/continuum-bench bench
infra/native-aws/scripts/run-topology-campaign.sh native-scylla-2n distributed-scale aws-t3-medium
infra/native-aws/scripts/fetch-reports.sh native-scylla-2n
infra/native-aws/scripts/teardown.sh native-scylla-2n

# All four topologies sequentially
infra/native-aws/scripts/run-distributed-scale-all.sh

# Scaling curve projection (peak BM-M4 per topology)
cargo run -p continuum-bench -- project-scaling-curve --hardware aws-t3-medium --storage scylla
cargo run -p continuum-bench -- project-scaling-curve --hardware aws-t3-medium --storage tikv-raw
```

**Sharding (three layers):** logical partition (`LogStreamId.key` → `storage_key()`), optional multi-cell routing (`KeyHashEvaluator` in `continuum-core`), physical shard placement inside Scylla/TiKV clusters (driver/PD — not Continuum). One 8-node Scylla cluster = one `ScyllaLogBackend`; spread load with partition keys, not per-node backends.

**dev-wsl native-lab (July 2026, partial):** native adapters exceed the **>10× surreal-tikv** gate (~40/s single-stream). Sample results:

| ID | sqlite | scylla | tikv-raw |
|----|--------|--------|----------|
| BM-C0 p50/p95 | 34/80 ms | 75/100 ms | 41/58 ms |
| BM-C1 batch 1→1000 | 26→51/s | 13→374/s | 21→138/s |
| BM-L3 achieved | — | 15/s | 19/s |

**dev-wsl native-scale (July 2026):**

| ID | scylla | tikv-raw |
|----|--------|----------|
| BM-P1 (K=10) | 13/s | 24/s |
| BM-M1 (C=8) | 26/s | 68/s |
| BM-M2 (C=64) | 25/s (55% err FAIL) | 184/s PASS |

Fleet projections: `projection-dev-wsl-scylla-any.json`, `projection-dev-wsl-tikv-raw-any.json`.

### `aws-t3-medium` native-lab Phase A (July 2026)

**Campaign:** Phase A colocated via [`infra/native-aws/`](../infra/native-aws/) — **2× t3.medium** (us-west-2, AL2023): Scylla `scylla-1` on host A, TiKV `tikv-minimal` on host B. Binary built with `build-al2023.sh`, deployed with `deploy-bench.sh`. Subset: `native-lab` (BM-C0–C4, BM-L0–L3).

**Reports:** **18** JSON (`9` scylla + `9` tikv-raw). **17/18** PASS — tikv-raw **BM-C3** checkpoint p95 FAIL (6.2 ms vs gate).

| ID | sqlite (June baseline) | scylla (`scylla-1` colocated) | tikv-raw (`tikv-minimal` colocated) |
|----|------------------------|-------------------------------|--------------------------------------|
| **BM-C0** p50 / p95 (ms) | 0.500 / 0.562 PASS | 15.3 / 16.4 PASS | 10.2 / 13.0 PASS |
| **BM-C1** batch 1 → 1000 (events/s) | 1951 → 4102 PASS | 64 → **1766** PASS | 60 → **1577** PASS |
| **BM-C2** p95 @100k (ms) | 0.089 PASS | 0.228 PASS | 0.683 PASS |
| **BM-C3** p95 checkpoint (ms) | 0.315 PASS | 0.397 PASS | **6.211 FAIL** |
| **BM-C4** post/pre read ratio | 1.35× PASS | 0.79× PASS | 0.96× PASS |
| **BM-L0** achieved / p99 | 100 / 6.5 ms | 63 / 23.8 ms | 74 / 29.2 ms |
| **BM-L1** achieved / p99 | 1000 / 0.9 ms | 64 / 23.8 ms | 58 / 30.0 ms |
| **BM-L2** achieved / p99 | 1909 / 0.8 ms | 64 / 23.4 ms | 51 / 29.4 ms |
| **BM-L3** achieved / p99 | **1898** / 0.8 ms | **64** / 23.2 ms | **45** / 38.9 ms |

**Interpretation:**

- **Batch path (BM-C1 @1000):** native scylla/tikv-raw reach **~90% of sqlite** batch throughput on the same instance class — the native adapters remove SQL/Surreal query overhead.
- **Hot stream (BM-L3, `key=None`):** single-partition ceiling **~64/s scylla**, **~45/s tikv-raw** vs sqlite **~1900/s** — multi-node clusters do not help until callers set partition keys (`CONTINUUM_BENCH_LOAD_PARTITION_COUNT` / Track P).
- **vs dev-wsl native-lab:** Scylla L3 **64/s** here vs **~15/s** on dev-wsl — colocated AWS + AL2023 binary is the canonical native baseline.
- **vs surreal-tikv (Appendix E):** tikv-raw L3 **~45/s** ≈ surreal-tikv **~43/s**, but C1 batch **1577/s** vs surreal-tikv **~40/s** single-stream.

**Fleet projections** (BM-L3 ceiling, compute only):

| Storage | Topology | Per-shard ceiling | Partitions for 1B/s | $/M ops |
|---------|----------|-------------------|---------------------|---------|
| scylla | scylla-1 | 64/s | 15,706,538 | $0.18 |
| tikv-raw | tikv-minimal | 45/s | 22,417,736 | $0.26 |

JSON: `projection-aws-t3-medium-scylla-any.json`, `projection-aws-t3-medium-tikv-raw-tikv-minimal.json`.

### Raw engine max throughput (July 2026)

**Tooling:** `cassandra-stress` (Scylla host) and `go-ycsb` raw mode (TiKV host). Same Docker config as Phase A native-lab (`--smp 1 --memory 750M` Scylla). Test A = spread keys + auto threads (≤512); Test B = single partition/key + same thread ramp. Runs detached on EC2 via [`infra/raw-engine-bench/`](../infra/raw-engine-bench/).

| Engine | Test | Max ops/s | Threads at peak | p95 ms | vs Continuum hot stream |
| ------ | ---- | --------- | --------------- | ------ | ----------------------- |
| Scylla | A spread | 14,872 | 316 | 35.10 | — |
| Scylla | B single-key | 903 | 16 | 26.00 | vs 64/s |
| TiKV | A spread | 7,290 | 1024 | 236.03 | — |
| TiKV | B single-key | 4,577 | 64 | 24.73 | vs 45/s |


---

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
cargo run -p continuum-bench -- project-fleet --storage scylla
cargo run -p continuum-bench -- project-fleet --storage tikv-raw

# Native adapter campaigns (scylla + tikv-raw; require infra env)
source infra/scylla/scripts/export-env.sh scylla-1
source infra/tikv-raw/scripts/export-env.sh
continuum-bench/scripts/run-native-preset.sh dev-wsl native-lab
continuum-bench/scripts/run-native-preset.sh dev-wsl native-scale
# Scale sweeps: CONTINUUM_BENCH_PARTITION_COUNT=10,100,1000  CONTINUUM_BENCH_CLIENT_COUNT=8,64,128

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

## Track M — Concurrency (BM-M3)

Multi-client append to a **single hot stream** (`key=None`). Sweeps `CONTINUUM_BENCH_CLIENT_COUNT` ∈ {8, 64, 128} on `aws-t3-medium`.

| C | Storage | Result |
| --- | --- | --- |
| 8 | scylla/scylla-1 | 21.3/s p99=395.5ms err=0.0000% PASS |
| 64 | scylla/scylla-1 | 3.60/s p99=22661.6ms err=0.0000% PASS |
| 64 | scylla/scylla-1 | 68.1/s p99=14230.8ms err=0.0000% PASS |
| 128 | scylla/scylla-1 | 2.18/s p99=42874.0ms err=41.2088% FAIL |
| 8 | tikv-raw/tikv-minimal | 47.5/s p99=489.5ms err=7.9049% FAIL |
| 64 | tikv-raw/tikv-minimal | 11.1/s p99=4270.7ms err=53.3865% FAIL |
| 64 | tikv-raw/tikv-minimal | 3.91/s p99=50008.8ms err=12.1076% FAIL |
| 128 | tikv-raw/tikv-minimal | 7.62/s p99=8885.1ms err=64.2061% FAIL |

**Interpretation:** Hot-stream throughput stays near single-client ceiling (~64/s scylla, ~45/s tikv-raw) regardless of client count — the backend serializes on one partition.


## Track P — Partitioning (BM-P*, BM-L* partitioned, BM-M4)

Spread writes across partition keys to use multiple shards.

### BM-P1 partition sweep (`CONTINUUM_BENCH_PARTITION_COUNT`)

| K | Storage | Result |
| --- | --- | --- |
| 10 | scylla/scylla-1 | 62.3/s p99=27.4ms err=0.0000% PASS |
| 64 | scylla/scylla-1 | 63.0/s p99=26.9ms err=0.0000% PASS |
| 128 | scylla/scylla-1 | 63.4/s p99=26.2ms err=0.0000% PASS |
| 10 | tikv-raw/tikv-minimal | 139/s p99=13.2ms err=0.0000% PASS |
| 64 | tikv-raw/tikv-minimal | 131/s p99=18.0ms err=0.0000% PASS |
| 128 | tikv-raw/tikv-minimal | 133/s p99=17.0ms err=0.0000% PASS |

### BM-L3 partitioned load (`CONTINUUM_BENCH_LOAD_PARTITION_COUNT`)

| K | Storage | Result |
| --- | --- | --- |
| 10 | scylla/scylla-1 | 63.4/s p99=22.5ms err=0.0000% PASS |
| 64 | scylla/scylla-1 | 62.3/s p99=26.8ms err=0.0000% PASS |
| 64 | scylla/scylla-1 | 184/s p99=11.1ms err=0.0000% PASS |
| 256 | scylla/scylla-1 | 63.4/s p99=23.1ms err=0.0000% PASS |
| 10 | tikv-raw/tikv-minimal | 133/s p99=16.5ms err=0.0000% PASS |
| 64 | tikv-raw/tikv-minimal | 134/s p99=16.7ms err=0.0000% PASS |
| 64 | tikv-raw/tikv-minimal | 138/s p99=15.4ms err=0.0000% PASS |
| 256 | tikv-raw/tikv-minimal | 136/s p99=16.0ms err=0.0000% PASS |

### BM-M4 concurrent + partitioned (C=K sweep)

| K | C | Storage | Result |
| --- | --- | --- | --- |
| 8 | 8 | scylla/scylla-1 | 115/s p99=89.8ms err=0.0000% PASS |
| 64 | 64 | scylla/scylla-1 | 2803/s p99=50.5ms err=0.0000% PASS |
| 128 | 128 | scylla/scylla-1 | 3241/s p99=89.6ms err=0.0000% PASS |
| 256 | 256 | scylla/scylla-1 | 3318/s p99=150.4ms err=0.0000% PASS |
| 8 | 8 | tikv-raw/tikv-minimal | 97.9/s p99=157.9ms err=0.0000% PASS |
| 64 | 64 | tikv-raw/tikv-minimal | 873/s p99=135.7ms err=0.0000% PASS |
| 128 | 128 | tikv-raw/tikv-minimal | 1091/s p99=202.0ms err=0.0000% PASS |
| 256 | 256 | tikv-raw/tikv-minimal | 1045/s p99=391.2ms err=0.0000% PASS |
| 512 | 512 | tikv-raw/tikv-minimal | 1134/s p99=747.6ms err=0.0000% PASS |
| 1024 | 1024 | tikv-raw/tikv-minimal | 1201/s p99=1606.5ms err=0.0000% PASS |

**Interpretation:** Post-opt spread-key throughput scales with C=K on Scylla (115/s → **3,318/s** at C=256, 0% errors). Pre-opt C=128 failed at 8.6% errors (~115/s); adapter changes fixed that entirely. TiKV plateaus around **~1.1k ops/s** from C=128 through C=1024 (873 → 1,201/s) while p50 latency grows linearly — more in-process tasks add queueing, not aggregate throughput. Compare raw Test A spread-key ceilings: Scylla **14,872/s** @ 316 threads; TiKV **7,290/s** @ 1024 threads. Continuum BM-M4 reaches **~22%** (Scylla C=256) and **~16%** (TiKV C=1024) of those raw peaks on one t3.medium host with a single driver client.


## Append optimization (July 2026, Phase 1 + 2)

Adapter-only changes in [`continuum-backend-scylla`](../continuum-backend-scylla/src/lib.rs) and [`continuum-backend-tikv-raw`](../continuum-backend-tikv-raw/src/lib.rs). No `continuum-core` changes. Enable round-trip counting with `CONTINUUM_APPEND_DEBUG_OPS=1`.

**Per-append round trips (steady state, single new record):**

| Stage | Scylla (before) | Scylla (after P1+P2) | TiKV (before) | TiKV (after P1+P2) |
| ----- | --------------- | -------------------- | ------------- | ------------------- |
| Idempotency | SELECT | — (INSERT IF NOT EXISTS) | txn + commit | merged read txn |
| Stream init | LWT IF NOT EXISTS | lazy on CAS miss | — | — |
| Seq allocation | SELECT + LWT CAS | LWT block / 64 | txn meta CAS | block reserve / 64 |
| Topic index | LWT IF NOT EXISTS | plain INSERT | conditional put | idempotent put |
| Event writes | 2× INSERT | idem LWT + INSERT | puts in txn | write txn |

### Post-optimization results (`aws-t3-medium`, 2026-07-01)

| ID | Scylla before → after | TiKV before → after | Raw Test A (spread) |
| --- | --------------------- | ------------------- | ------------------- |
| **BM-C0** p50 | 15.3ms → **5.1ms** | 10.2ms → **7.0ms** | — |
| **BM-L3** hot | 64/s → **184/s** | 45/s → **138/s** | 14,872 / 7,290 ops/s |
| **BM-M3** C=64 hot | 4/s → **68/s** | 45/s → 4/s (conflicts) | Test B: 903 / 4,577 ops/s |
| **BM-M4** C=K=64 | 112/s → **2,803/s** | 84/s → **873/s** | — |

### BM-M4 concurrency scaling (post-opt, 2026-07-02)

| C=K | Scylla ops/s | Scylla p99 | TiKV ops/s | TiKV p99 |
| --- | --- | --- | --- | --- |
| 64 | 2,803 | 50.5ms | 873 | 135.7ms |
| 128 | 3,241 | 89.6ms | 1,091 | 202.0ms |
| 256 | 3,318 | 150.4ms | 1,045 | 391.2ms |
| 512 | — | — | 1,134 | 747.6ms |
| 1024 | — | — | 1,201 | 1,606.5ms |

All runs: 0% error rate, PASS. Reports: `profiling/continuum-bench/reports/bm-m4-*-pk*-c*.json`.

**Interpretation:** The gap vs raw spread-key tools is adapter round-trips and per-append consensus, not generic Continuum overhead (sqlite ~1900/s on the same trait). Phase 1 removed redundant reads and merged TiKV transactions; Phase 2 client-side seq blocks amortize the remaining Scylla LWT / TiKV meta updates. Scylla continues to gain throughput through C=256; TiKV saturates near ~1.1k/s regardless of task count up to 1024. Hot-stream TiKV M3 still contends under 64 concurrent writers — partition keys (Track P) remain required for aggregate scale.


## Track T — Topology scaling (Phase B, 1 → 2 → 4 storage nodes)

Dedicated bench EC2 + N storage nodes on AWS (`infra/native-aws/topologies/`). N=1 baseline remains Phase A colocated (`scylla-1` / `tikv-minimal`); N≥2 uses private-IP driver contact points from `export-env-topology.sh`.

| Storage nodes | Topology slug | AWS manifest |
| --- | --- | --- |
| 1 | scylla-1 / tikv-minimal | `native-colocated` (Phase A) |
| 2 | scylla-2n / tikv-ha-2 | `native-scylla-2n` / `native-tikv-ha-2` |
| 4 | scylla-4n / tikv-scale-4 | `native-scylla-4n` / `native-tikv-scale-4` |

**Primary metric:** peak **BM-M4** `achieved_ops_per_sec` from adaptive C=K sweep (`run-distributed-scale-campaign.sh`):

- Scylla ladder: C=K ∈ {128, 256, 512, 1024, …} — escalate while gain ≥10%, err &lt;1%, bench CPU &lt;85%
- TiKV ladder: C=K ∈ {64, 128, 256, 512, 1024, …} — same stop rules
- Fixed-load reference: Scylla C=K=128, TiKV C=K=64 at every N
- Control: BM-L3 hot stream (expect flat vs N)

**Supporting:** BM-L3 partitioned (K=64 load), BM-P1 (K=128).

### Track T results (`aws-t3-medium`)

| Storage nodes | Topology | Peak BM-M4 ops/s | C=K @ peak | vs N=1 | ops/s per node | Hot BM-L3 |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | scylla-1 (colocated) | 3,318 | 256 | 1.00× | 3,318 | ~64/s hot |
| 2 | scylla-2n | 3,444 | 256 | 1.04× | 1,722 | ~155/s hot |
| 4 | scylla-4n | 3,519 | 128 | 1.06× | 880 | ~169/s hot |
| 1 | tikv-minimal (colocated) | 1,201 | 1024 | 1.00× | 1,201 | ~45/s hot |
| 2 | tikv-ha-2 | 1,608 | 128 | 1.34× | 804 | ~90/s hot |
| 4 | tikv-scale-4 | 1,620 | 64 | 1.35× | 405 | ~100/s hot |

**Scylla interpretation (July 2026):** Sub-linear scaling (+4% @ 2n, +6% @ 4n vs colocated N=1). Per-node efficiency falls sharply (1,722 and 880 ops/s/node) — spread-key append remains **bench- and coordination-bound** on `t3.medium`, not storage-saturated. Hot-stream BM-L3 rose with node count (~155/s, ~169/s vs ~64/s colocated); treat as layout/control artifact, not hot-partition relief.

**TiKV interpretation (July 2026):** Modest cluster scaling (+34–35% vs colocated N=1 at 1,201/s) from 2→4 nodes with peak essentially flat (1,608/s @ 2n C=128 vs 1,620/s @ 4n C=64). Per-node efficiency drops (804 → 405 ops/s/node) — **PD/meta coordination and bench RTT** dominate before TiKV exhausts. Hot-stream ~90–100/s (vs ~45/s colocated).

### Track T bench resource profile @ peak BM-M4

From `resource_profile` on the **bench EC2** (`aws-t3-medium`: 2 vCPU, ~3.75 GiB RAM). CPU % is summed across cores (200% ≈ both cores saturated). **System mem peak** is the reliable RAM signal on these runs; `process_rss_bytes_*` is inflated by a known sysinfo quirk on AL2023 — do not use RSS for sizing.

| Topology | Peak ops/s | C=K @ peak | Bench CPU peak | Bench CPU mean | Sys mem peak | Bench-bound? |
| --- | --- | --- | --- | --- | --- | --- |
| scylla-1 (colocated) | 3,318 | 256 | 26% | 21% | 1.26 GiB | No |
| scylla-2n | 3,444 | 256 | 31% | 27% | 0.57 GiB | No |
| scylla-4n | 3,519 | 128 | 30% | 25% | 0.36 GiB | No |
| tikv-minimal (colocated) | 1,201 | 1024 | 86% | 79% | 2.97 GiB | Borderline |
| tikv-ha-2 | 1,608 | 128 | 161%† | 123%† | 0.40 GiB† | Yes |
| tikv-scale-4 | 1,620 | 64 | 180% | 154% | 0.40 GiB | Yes |

† `tikv-ha-2` peak throughput is at C=128; CPU/mem row is from the fetched C=64 report (161% / 0.40 GiB) — same bench-bound regime.

**Read:** Scylla distributed runs left **~70% CPU headroom** on the 2-vCPU bench — not CPU- or RAM-capped; the plateau is coordination/network. TiKV distributed runs **saturated 1.6–1.8 cores** while system RAM stayed ~0.4 GiB — **bench CPU-bound**, not memory-bound. Phase 5 (larger bench) is primarily a TiKV lever; Scylla needs coordination/network tuning or larger storage nodes before a bigger bench helps.

**Methodology footnote:** N=1 is colocated (bench + storage same EC2); N≥2 is dedicated bench over VPC private IP. Compare scaling **trends**, not absolute parity.

```bash
cargo run -p continuum-bench -- project-scaling-curve --hardware aws-t3-medium --storage scylla
cargo run -p continuum-bench -- project-fleet --hardware aws-t3-medium --storage scylla --scylla-topology scylla-4n
```

### Phase 5 — larger instance class (gated on manual verification)

Re-run Track T on **`aws-c7i-4xlarge`** (16 vCPU) or **`aws-i4i-xlarge`** (NVMe) using manifests `native-scylla-4n-c7i.yaml` / `native-tikv-scale-4-c7i.yaml`. Per-role `instance_type:` in topology YAML is supported by `provision-topology.sh`. **Do not start until Phase B results are manually verified.**

```bash
CONTINUUM_NATIVE_AWS_INSTANCE_TYPE=c7i.4xlarge \
  infra/native-aws/scripts/run-topology-campaign.sh native-scylla-4n-c7i distributed-scale aws-c7i-4xlarge
```

See **Appendix G.2** in [`PERFORMANCE_STUDY.md`](PERFORMANCE_STUDY.md) for the Phase 5 scaling table template.

## Tracks U–Y — Scylla bottleneck diagnosis (July 2026)

Post–Track T diagnosis: identify whether the BM-M4 plateau is storage-saturated, adapter/coordination-bound, or bench/client-limited. **Scylla-only execution** this round; TiKV tracks use the same structure but are **deferred**.

**Orchestrator** (WSL-resilient, sequential topologies, guaranteed teardown):

```bash
nohup infra/native-aws/scripts/run-scylla-diagnosis.sh \
  > /tmp/scylla-diagnosis.log 2>&1 &
tail -f /tmp/scylla-diagnosis.log
# Poll detached campaign: infra/native-aws/scripts/campaign-status-topology.sh native-scylla-2n track-u
```

State file: `infra/native-aws/state/scylla-diagnosis.json`. S3 artifact cache: `setup-artifact-bucket.sh` + `artifact-fetch.sh` (key `al2023/<git-sha>-<lock-hash>/continuum-bench`). **All `Project=continuum-bench` instances terminated** via `teardown-all.sh` on completion or failure (override: `--no-teardown-on-fail`).

**Run status (2026-07-02):** `SCYLLA_DIAGNOSIS_COMPLETE` — all orchestrator steps finished. Tables U.1–Y.1 filled below; gaps noted where report fetch/overwrite lost data.

Fixed load from Track T peaks (no full adaptive re-sweep):

| Topology | C=K | Tracks |
| --- | --- | --- |
| colocated (`native-colocated` scylla) | 256 | U, V, W, Y |
| scylla-2n | 256 | U, V, W, X |
| scylla-4n | 128 | U, W |

### Track U — Storage-side saturation

**Question:** Are Scylla nodes loaded, or is the bench failing to feed them?

```bash
infra/native-aws/scripts/run-scylla-track-u.sh native-scylla-2n 256
infra/native-aws/scripts/collect-scylla-storage-metrics.sh native-scylla-2n end
```

Metrics JSON: `infra/native-aws/state/<topology>-storage-{mid,end}.json` — per-node `nodetool status`, `tablestats`, `docker stats`, loadavg.

#### Table U.1 — Storage metrics @ BM-M4 peak load (July 2026)

Metrics from `collect-scylla-storage-metrics.sh` **mid** sample (during detached BM-M4). Source: `infra/native-aws/state/<topology>-storage-mid.json`.

| Topology | Node | CPU / load | UN tokens | Write rate | Verdict |
| --- | --- | --- | --- | --- | --- |
| colocated | scylla-0 | ~109% docker, load 1.46 | 256 UN | 1,536 local writes @ 0.003 ms | Hot CPU, μs latency → not IO-bound |
| scylla-2n | scylla-0 | ~101% docker, load 0.39 | 256 UN | 77,543 local writes @ 0.005 ms | Busy @ peak; μs latency |
| scylla-2n | scylla-1 | — | — | — | Not sampled (collector only persisted seed node) |
| scylla-4n | scylla-0 | ~99% docker, load 0.50 | 256 UN | 1,280 local writes @ 0.004 ms | Hot CPU; μs latency; seed only |

**Pass/fail:** all nodes low CPU + low write rate → **not storage-bound**; even load + moderate CPU → sharding OK, look upstream; one node hot → routing/replication issue.

**Result:** Scylla nodes show CPU activity at peak but **microsecond local write latency** and **0% iowait** → **not storage-saturated**. Throughput plateau is upstream of Scylla (adapter/coordination or bench client).

### Track V — Adapter round-trip budget

**Question:** How many CQL round trips per append?

Env: `CONTINUUM_APPEND_DEBUG_OPS=1`. Report `notes` include `round_trips=N ops=M per_append=… rt_per_append=…`.

```bash
infra/native-aws/scripts/run-scylla-track-v.sh native-colocated
```

#### Table V.1 — Round trips per append (July 2026)

`CONTINUUM_APPEND_DEBUG_OPS=1`. Reports: `profiling/continuum-bench/reports/bm-m4-scylla-*-pk{C}-c{C}.json` (`notes` field).

| Topology | C=K | ops/s | p99 ms | round_trips | ops/append | rt/append |
| --- | --- | --- | --- | --- | --- | --- |
| colocated | 64 | 2,605 | 52.5 | 237,124 | 3.03 | 3.03 |
| colocated | 256 | 1,141 | 404.5 | 103,688 | 3.01 | 3.01 |
| scylla-2n | 64 | 2,836 | 42.4 | 258,048 | 3.03 | 3.03 |
| scylla-2n | 256 | 2,225 | 186.5 | — | — | — |

Compare to theoretical minimum (~2 RT steady state, Appendix F.6).

**Result:** **~3.0–3.03 RT/append** at C=64 on colocated and scylla-2n — above the ~2 RT floor → **adapter/coordination overhead** (LWT / multi-step append). C=256 rows run with debug enabled (lower ops/s, higher p99). scylla-2n @ C=256 report lacks debug `notes` (overwrite timing).

### Track W — Raw engine comparison

**Question:** Does `cassandra-stress` spread-key scale with N when Continuum does not?

```bash
infra/native-aws/scripts/run-scylla-track-w.sh native-scylla-2n
# Results: infra/native-aws/state/<topology>-track-w/scylla-a.json
```

#### Table W.1 — Raw spread vs Continuum BM-M4 (July 2026)

Track W: `cassandra-stress` spread-key peak (`infra/native-aws/state/<topology>-track-w/scylla-a.json`). Track U BM-M4 @ fixed C=K for Continuum column.

| Topology | N | cassandra-stress ops/s | Continuum BM-M4 ops/s | Raw scales? |
| --- | --- | --- | --- | --- |
| colocated | 1 | 14,627 | ~3,318† | Yes (raw ≫ Continuum) |
| scylla-2n | 2 | 29,524 | 2,225 | Yes (2× raw vs colocated; Continuum ↓) |
| scylla-4n | 4 | **failed**‡ | 3,226 | — |

† Colocated Track U report overwritten before fetch; Continuum value from Track T peak (July 2026).  
‡ 4n stress run hit CQL connection errors from bench host; log incomplete, parse returned null.

**Interpretation:** raw scales + Continuum flat → adapter/coordination; both flat → network/topology/bench; raw flat → Scylla config or instance too small.

**Result:** **Raw stress scales with N; Continuum does not** (2n Continuum 2,225 ops/s vs raw 29.5k) → **adapter/coordination bound**, not Scylla write throughput.

### Track X — Client parallelism (dual process)

**Question:** Is a single bench process the limiter?

Two `continuum-bench` processes on the bench host with disjoint partition ranges via `CONTINUUM_BENCH_PARTITION_OFFSET` + `CONTINUUM_BENCH_PARTITION_COUNT=K/2`.

```bash
infra/native-aws/scripts/run-scylla-track-x.sh native-scylla-2n 256
```

#### Table X.1 — Single vs dual process (July 2026)

Track X re-run on `native-scylla-2n` @ C=K=256 via `run-scylla-levers.sh` (tagged reports `x-single`, `x-dual-a`, `x-dual-b`).

| Topology | Processes | Aggregate ops/s | vs single |
| --- | --- | --- | --- |
| scylla-2n | 1 | 3,561 | 1.00× |
| scylla-2n | 2 | 3,345 | 0.94× |

Dual aggregate = sum of dual-a (1,673) + dual-b (1,672) @ C=128 each.

~2× → bench/client bound; flat → coordination/storage.

**Result:** **Not client-bound** — dual process does not ~2× throughput; adapter/coordination remains the limiter (consistent with Tracks V/W).

### Track Y — Coordination tuning (optional)

**Question:** Can larger seq blocks reduce LWT frequency?

Env: `CONTINUUM_SCYLLA_SEQ_BLOCK_SIZE` (default 64). Colocated A/B @ C=256, block sizes 64 vs 256.

```bash
infra/native-aws/scripts/run-scylla-track-y.sh native-colocated
```

#### Table Y.1 — Seq block size (July 2026)

Colocated A/B @ C=256, `CONTINUUM_SCYLLA_SEQ_BLOCK_SIZE` 64 vs 256 (re-run via `run-scylla-levers.sh`, tags `y-blk64` / `y-blk256`).

| Block size | ops/s | round_trips/append | p99 ms |
| --- | --- | --- | --- |
| 64 | 3,115 | 3.03 | 149.6 |
| 256 | 2,997 | 3.01 | 169.6 |

**Result:** **No meaningful effect** — block size 64 vs 256 does not materially change rt/append or throughput at C=256.

### TiKV tracks U–Y (deferred)

Same track definitions apply to `tikv-ha-2` / `tikv-scale-4` with TiKV-specific storage metrics (PD store status, TiKV `docker stats`). Not executed in this Scylla-focused run.

See **Appendix H** in [`PERFORMANCE_STUDY.md`](PERFORMANCE_STUDY.md) for the consolidated bottleneck verdict table.

---

## Tracks Z1–Z5 — Scylla append-path optimization levers (July 2026)

Follow-on to Tracks U–Y: each lever is behind a **default-off** env flag; A/B campaigns compare baseline vs treatment at fixed C=K. Executed on AWS via `infra/native-aws/scripts/run-scylla-levers.sh` (`aws-t3-medium`, colocated, July 2 2026).

```bash
infra/native-aws/scripts/run-scylla-levers.sh
# or per-lever:
continuum-bench/scripts/run-scylla-lever-campaign.sh z2 aws-t3-medium 256
```

Reports use `CONTINUUM_BENCH_REPORT_TAG` (`z2-baseline`, `z2-treatment`, …) to prevent overwrite.

### Track Z1 — Idempotency mode (L1)

**Question:** Throughput cost of per-append idempotency LWT?

**Flag:** `CONTINUUM_SCYLLA_IDEMPOTENCY=lwt|none` (default `lwt`).

**Risk:** **High** — `none` → at-least-once; duplicates on retry.

#### Table Z1.1 — Idempotency (July 2026)

| Variant | ops/s | rt/append | p99 ms |
| --- | --- | --- | --- |
| baseline (`lwt`) | 3,112 | 3.03 | 165.5 |
| treatment (`none`) | 14,164 | 2.03 | 87.4 |

**Result:** **LWT dominates cost** — removing idempotency LWT yields ~4.5× throughput and drops rt/append toward the ~2 RT floor. **Do not enable `none` in production** without an exactly-once policy decision.

### Track Z2 — Topic-index cache (L2)

**Question:** Does skipping repeat `stream_index` writes remove the hot-partition RT?

**Flag:** `CONTINUUM_SCYLLA_TOPIC_INDEX_CACHE=0|1` (**default on** after Track AA; Z2 A/B used explicit off/on).

**Risk:** **Low** — stale cache if index rows deleted out-of-band.

#### Table Z2.1 — Topic-index cache (July 2026)

| Variant | ops/s | rt/append | p99 ms |
| --- | --- | --- | --- |
| baseline (L2 off) | 3,500 | 3.04 | 173.9 |
| treatment (L2 on) | 3,504 | 2.04 | 153.5 |

**Result:** **RT hypothesis confirmed** under Z1 on — topic-index cache removes ~1 RT/append (3.04→2.04) but throughput stays ~3.5k ops/s (LWT-bound). Track AA later showed L2 **does** raise multi-node throughput when Z1 is off (~24k→33k on 2n); L2 is now **default on**.

### Track Z3 — Pipelined writes (L3)

**Question:** Overlap event + index INSERT wall-clock?

**Flag:** `CONTINUUM_SCYLLA_PIPELINE_WRITES=1` (default off; single-record BM-M4 path).

**Risk:** **Low** — same CQL semantics; LWT remains gate.

#### Table Z3.1 — Pipelined writes (July 2026)

| Variant | ops/s | rt/append | p99 ms |
| --- | --- | --- | --- |
| baseline | 3,308 | 3.04 | 148.3 |
| treatment | 3,089 | 3.03 | 186.8 |

**Result:** **No benefit** — overlapping event + index INSERT does not improve throughput; slight regression within run variance.

### Track Z4 — Write consistency (L4)

**Question:** Lower CL when RF>1?

**Flag:** `CONTINUUM_SCYLLA_WRITE_CONSISTENCY=quorum|one|local_one` (default unset).

**Risk:** **Inert at RF=1**; durability tradeoff at RF>1.

#### Table Z4.1 — Write consistency (July 2026)

| Variant | ops/s | rt/append | p99 ms |
| --- | --- | --- | --- |
| baseline | 2,325 | 3.04 | 195.7 |
| treatment (`one`) | 1,691 | 3.04 | 545.7 |

**Result:** **Inert at RF=1 in theory; noisy in practice** — CL knob shows run-to-run variance at colocated RF=1, not a reliable lever here. Re-test only when RF>1.

### Track Z5 — Driver pool per shard (L5)

**Question:** More connections per shard → more in-flight CQL?

**Flag:** `CONTINUUM_SCYLLA_POOL_PER_SHARD=<n>` (default unset).

**Risk:** **Low** — operational overload if unbounded.

#### Table Z5.1 — Pool per shard (July 2026)

| Variant | ops/s | rt/append | p99 ms |
| --- | --- | --- | --- |
| baseline | 3,292 | 3.04 | 355.2 |
| treatment (`=4`) | 2,973 | 3.03 | 170.3 |

**Result:** **No benefit** — `POOL_PER_SHARD=4` does not raise throughput; driver pool was not the limiter (consistent with Track X not being client-bound).

---

## Ceilings — Z1 on/off × topology (`aws-t3-medium`, July 2026)

Full ceiling matrix: Z1 LWT on vs off at C=K=256 on dedicated bench + N Scylla nodes. Executed via `run-scylla-ceilings.sh` Phase A (t3). Phase B (`aws-c7i-4xlarge`) **deferred** — account vCPU limit (16) insufficient for 2× `c7i.4xlarge` per topology.

```bash
infra/native-aws/scripts/run-scylla-ceilings.sh --skip-artifact
continuum-bench/scripts/analyze-scylla-ceilings.py --reports-dir profiling/continuum-bench/reports
```

#### Table Ceiling.1 — Peak BM-M4 by topology and Z1 mode (July 2026)

| Topology | Z1 | ops/s | p99 ms |
| --- | --- | --- | --- |
| scylla-1 (colocated) | on (`lwt`) | 3,112 | 165.5 |
| scylla-1 (colocated) | off (`none`) | 14,164 | 87.4 |
| scylla-2n | on | 3,636 | 184.5 |
| scylla-2n | off | 18,517 | 67.2 |
| scylla-4n | on | 3,987 | 194.8 |
| scylla-4n | off | 20,118 | 57.4 |

**Result:** Z1-off raises ceiling ~4.5–5× at every N; multi-node Z1-on peaks stay flat (~3.1–4.0k ops/s). Z1-off 4n (20.1k) is only ~8% above 2n (18.5k) — storage/coordination still sub-linear. **4n Z1-on regresses** vs 2n (3,987 vs 3,636) — extra coordination cost without LWT removal. **Note:** Ceiling runs used L2 off (pre–Track AA); with L2 default on, Z1-off multi-node peaks are ~31k ops/s (Table AA.3).

#### Table Ceiling.2 — Monthly compute cost projection (`aws-t3-medium`, Z1-off peaks)

Model: `clusters = ceil(target_ops / peak_ops)`; `monthly_usd = clusters × nodes × $0.0416/hr × 730`. Storage nodes only (bench amortized).

| Topology | Peak ops/s | 10k/s | 50k/s | 100k/s | 1M/s |
| --- | --- | --- | --- | --- | --- |
| scylla-1 | 14,164 | $30 | $121 | $243 | $2,156 |
| scylla-2n | 18,517 | $61 | $182 | $364 | $3,340 |
| scylla-4n | 20,118 | $121 | $364 | $607 | $6,074 |

With Z1-on (production default), 4n is the worst $/ops tier at scale — prefer 1n–2n until LWT cost is addressed.

---

## Track AA — Index scaling (BM-M4/M5 × L2 × Z1 off, July 2026)

Validates whether `continuum_stream_index` hot-partition writes limit multi-node throughput when Z1 (LWT) is off. Executed via `run-scylla-index-scaling-master.sh` on `native-scylla-{1n,2n,4n}` (`aws-t3-medium`, C=K=256, Z1 off).

```bash
infra/native-aws/scripts/run-scylla-index-scaling-master.sh --skip-artifact
continuum-bench/scripts/analyze-scylla-index-scaling.py --reports-dir profiling/continuum-bench/reports
```

#### Table AA.1 — Index scaling matrix (Z1 off, peak ops/s)

| Topology | m4 L2 off | m4 L2 on | t64 L2 off | t64 L2 on | rt/append (m4 off→on) |
| --- | --- | --- | --- | --- | --- |
| scylla-1 | 20,054 | 24,279 | 17,517 | 23,412 | 2.03 → 1.03 |
| scylla-2n | 24,399 | **33,137** | 21,468 | 31,257 | 2.03 → 1.03 |
| scylla-4n | 23,979 | **33,811** | 21,475 | 34,133 | 2.03 → 1.03 |

Prior ceiling (m4 z1off, L2 off): 2n 18,517; 4n 20,118. Raw cassandra-stress 2n: 29,524.

#### Table AA.2 — Pre-registered verdict

| Criterion | Outcome |
| --- | --- |
| T=64 spreads index load vs single topic (2n, L2 off) | **No** — t64 21.5k ≤ m4 24.4k |
| L2 on vs off at Z1 off (2n m4) | **Yes** — L2 on +36% (33.1k vs 24.4k); removes ~1 RT/append |
| 2n approaches raw with L2 on | **Yes** — 33.1k ≥ raw 29.5k on 2n |

**Result:** Per-append `stream_index` INSERT (L2 off) is the multi-node bottleneck when Z1 is off — not fixed by topic fan-out alone. **L2 (topic-index cache) default-on** removes repeat index writes and raises 2n/4n Z1-off throughput ~33–34k ops/s (~1.8× prior ceiling). Topic sharding remains operational guidance for single-topic deployments until schema re-partition (Phase 3).

**Fix applied:** `ScyllaLogConfig.topic_index_cache` default `true`; bench harness uses `env_flag_default(..., true)` so unset env keeps L2 on; campaigns set `CONTINUUM_SCYLLA_TOPIC_INDEX_CACHE=0` for L2-off baselines.

#### Table AA.3 — Phase 2 validation (default config, Z1 off, BM-M4)

Re-run after fix via `run-scylla-index-scaling-phase2-master.sh` (env unset → L2 on).

| Topology | ops/s | rt/append | vs prior ceiling (L2 off) | vs Phase 1 L2 on | vs raw 2n |
| --- | --- | --- | --- | --- | --- |
| scylla-2n | **30,682** | 1.03 | 1.66× (18,517) | 0.93× (33,137) | **1.04×** (29,524) |
| scylla-4n | **29,583** | 1.03 | 1.47× (20,118) | 0.87× (33,811) | — |

**Result:** Default L2 on is live and effective — 2n meets/exceeds raw cassandra-stress; both topologies drop to ~1 RT/append. 4n does not gain further over 2n (bench/driver bound at ~30k on t3.medium). Track W 4n raw baseline still failed (incomplete stress log).

---

## Parallelism — topic fan-out + multi-publisher (BM-M5, July 2026)

Executed via `run-scylla-parallelism.sh` on `native-scylla-1n` and `native-scylla-2n` (`aws-t3-medium`, Z1 on, C=K=256). All instances torn down; `SCYLLA_PARALLELISM_COMPLETE`.

```bash
infra/native-aws/scripts/run-scylla-parallelism.sh --skip-artifact
```

#### Table P5.1 — Topic fan-out (T topics × K keys, L2 on/off)

| Topology | T | L2 off | L2 on |
| --- | --- | --- | --- |
| scylla-1 | 1 | 3,663 | 3,690 |
| scylla-1 | 8 | 3,832 | 3,605 |
| scylla-1 | 64 | 3,376 | 3,699 |
| scylla-2n | 1 | 3,797 | 4,073 |
| scylla-2n | 8 | 3,759 | 3,856 |
| scylla-2n | 64 | 3,412 | 3,831 |

**Result:** Topic count does not materially change throughput (~3.4–4.1k ops/s) — fan-out is not the bottleneck at T≤64.

#### Table P5.2 — Per-topic idempotency (mixed LWT/none @ T=64)

| Topology | Policy | ops/s | p99 ms |
| --- | --- | --- | --- |
| scylla-1 | Global LWT + 50% topics `none` | 11,036 | 102.2 |
| scylla-2n | Global LWT + 50% topics `none` | 10,625 | 103.7 |

**Result:** Partial LWT removal raises throughput ~3× vs all-LWT baseline — confirms LWT is per-topic cost, not global partition hot-spot alone.

#### Table P5.3 — Multi-publisher aggregate (N processes, sum ops/s)

| Topology | N publishers | Aggregate ops/s | vs N=1 |
| --- | --- | --- | --- |
| scylla-1 | 1 | 3,200 | 1.00× |
| scylla-1 | 2 | 2,829 | 0.88× |
| scylla-1 | 4 | 2,916 | 0.91× |
| scylla-2n | 1 | 3,314 | 1.00× |
| scylla-2n | 2 | 2,890 | 0.87× |
| scylla-2n | 4 | 2,857 | 0.86× |

**Result:** **Not publisher-bound** — more processes reduce aggregate throughput (contention on shared Scylla/coordinator path). Consistent with Track X (dual process 0.94×).

