#!/usr/bin/env bash
# Track U driver: detached BM-M4 + storage metrics (mid/end).
# Usage: run-scylla-track-u.sh <topology> <C=K>
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
REPO_ROOT="$(cd "$ROOT/../.." && pwd)"
TOPO="${1:?topology}"
CK="${2:?C=K}"

wait_campaign() {
  local marker="$1" timeout="${2:-600}"
  local elapsed=0
  while [[ $elapsed -lt $timeout ]]; do
    if bash "$ROOT/scripts/campaign-status-topology.sh" "$TOPO" "$marker" 2>/dev/null | grep -q "STATUS: DONE"; then
      return 0
    fi
    sleep 15
    elapsed=$((elapsed + 15))
  done
  echo "timeout waiting for $marker on $TOPO" >&2
  return 1
}

bash "$ROOT/scripts/run-topology-campaign-detached.sh" "$TOPO" \
  "$REPO_ROOT/continuum-bench/scripts/run-scylla-track-u-campaign.sh" track-u "$CK"

sleep 15
bash "$ROOT/scripts/collect-scylla-storage-metrics.sh" "$TOPO" mid

wait_campaign track-u 900
bash "$ROOT/scripts/collect-scylla-storage-metrics.sh" "$TOPO" end
echo "Track U complete: $TOPO C=K=$CK"
