#!/usr/bin/env bash
# Run all Phase B distributed-scale topologies sequentially (t3.medium).
# Usage: run-distributed-scale-all.sh [--skip-build]
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
REPO_ROOT="$(cd "$ROOT/../.." && pwd)"
BIN="${REPO_ROOT}/target/al2023/continuum-bench"
SKIP_BUILD=false
[[ "${1:-}" == --skip-build ]] && SKIP_BUILD=true

TOPOLOGIES=(
  native-scylla-2n
  native-scylla-4n
  native-tikv-ha-2
  native-tikv-scale-4
)

if ! $SKIP_BUILD; then
  bash "$ROOT/scripts/build-al2023.sh" "$BIN"
fi

for TOPO in "${TOPOLOGIES[@]}"; do
  echo "========== $TOPO =========="
  bash "$ROOT/scripts/provision-topology.sh" "$TOPO"
  bash "$ROOT/scripts/bootstrap-topology.sh" "$TOPO"
  bash "$ROOT/scripts/preflight-topology.sh" "$TOPO"
  bash "$ROOT/scripts/deploy-bench.sh" "$TOPO" "$BIN" bench
  bash "$ROOT/scripts/run-topology-campaign.sh" "$TOPO" distributed-scale aws-t3-medium
  bash "$ROOT/scripts/fetch-reports.sh" "$TOPO"
  bash "$ROOT/scripts/teardown.sh" "$TOPO"
done

echo "ALL_DISTRIBUTED_SCALE_DONE"
