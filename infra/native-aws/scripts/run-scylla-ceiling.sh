#!/usr/bin/env bash
# Track ceiling driver: detached BM-M4 Z1 on/off sweep.
# Usage: run-scylla-ceiling.sh <topology> <hardware> "<c-list>"
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
REPO_ROOT="$(cd "$ROOT/../.." && pwd)"
TOPO="${1:?topology}"
HARDWARE="${2:?hardware}"
C_LIST="${3:-256}"

wait_campaign() {
  local elapsed=0
  while [[ $elapsed -lt 3600 ]]; do
    if bash "$ROOT/scripts/campaign-status-topology.sh" "$TOPO" ceiling 2>/dev/null | grep -q "STATUS: DONE"; then
      return 0
    fi
    sleep 15
    elapsed=$((elapsed + 15))
  done
  echo "timeout waiting for ceiling on $TOPO" >&2
  return 1
}

bash "$ROOT/scripts/run-topology-campaign-detached.sh" "$TOPO" \
  "$REPO_ROOT/continuum-bench/scripts/run-scylla-ceiling-campaign.sh" \
  ceiling "$HARDWARE" "$C_LIST"
wait_campaign
echo "Ceiling complete: $TOPO hardware=$HARDWARE C=$C_LIST"
