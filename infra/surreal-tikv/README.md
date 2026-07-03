# SurrealDB + TiKV lab stack

Docker Compose presets for continuum-bench distributed campaigns (`surreal-tikv` storage dimension).

## Prerequisites

- Docker Engine with Compose v2
- ~8 GiB RAM for `tikv-minimal`; ~16+ GiB for `tikv-ha-3` and larger presets
- `curl` for health checks in `scripts/up.sh`

## Presets

| Preset | PD | TiKV | Surreal | Notes |
|--------|----|------|---------|-------|
| `tikv-minimal` | 1 | 1 | 1 | Phase 1 method validation |
| `tikv-ha-3` | 1 | 3 | 1 | HA storage on single host |
| `tikv-scale-5` | 1 | 5 | 1 | Scale envelope (RAM-heavy) |
| `surreal-2n` | 1 | 3 | 2 + nginx LB | Phase 3 scale-out |
| `surreal-4n` | 1 | 3 | 4 + nginx LB | Phase 3 scale-out |

Pinned images: `pingcap/pd:v8.5.0`, `pingcap/tikv:v8.5.0`, `surrealdb/surrealdb:v3.1.5`.

**Budget cloud notes:** TiKV containers require `ulimits.nofile` ≥ 262144 (set in `compose.yaml`). On 4 GiB instances use **4 GiB swap** before `tikv-minimal`. Bench client needs `protocol-ws` + `CONTINUUM_BENCH_SURREAL_USER/PASS` (exported by `export-env.sh`).

## Quick start

```bash
# From repo root
infra/surreal-tikv/scripts/up.sh tikv-minimal
eval "$(infra/surreal-tikv/scripts/export-env.sh tikv-minimal)"

cargo run --release -p continuum-bench -- matrix \
  --hardware dev-wsl --subset tikv-lab-colocated --skip-existing

infra/surreal-tikv/scripts/down.sh
```

## Bench env vars

| Variable | Purpose |
|----------|---------|
| `CONTINUUM_BENCH_SURREAL_URL` | WebSocket URL to Surreal (required for TiKV matrices) |
| `CONTINUUM_BENCH_TIKV_TOPOLOGY` | Preset slug recorded in report JSON |
| `CONTINUUM_BENCH_TIKV_PD_ENDPOINT` | PD URL stored in report metadata |
| `CONTINUUM_BENCH_SURREAL_INSTANCES` | Surreal node count (multi-node presets) |
| `CONTINUUM_BENCH_SURREAL_HARDWARE` | Optional separate Surreal hardware profile |
| `CONTINUUM_BENCH_TIKV_HARDWARE` | Optional separate TiKV hardware profile |

## Matrix slices

See [`continuum-bench/EXPERIMENTS.md`](../../continuum-bench/EXPERIMENTS.md) — Distributed Surreal/TiKV campaign.

```bash
cargo run -p continuum-bench -- matrix --subset tikv-topology --hardware dev-wsl
cargo run -p continuum-bench -- matrix --subset surreal-scale --hardware dev-wsl
cargo run -p continuum-bench -- project-fleet --storage surreal-tikv --tikv-topology tikv-ha-3
```

## Resource limits (optional)

To simulate smaller cloud profiles on a single host, add `deploy.resources.limits` to services in `compose.yaml` or use `docker update` after start.
