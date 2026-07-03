#!/usr/bin/env bash
# Track Y driver: seq block size A/B on colocated.
# Usage: run-scylla-track-y.sh <topology>
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
REPO_ROOT="$(cd "$ROOT/../.." && pwd)"
TOPO="${1:?topology}"

wait_campaign() {
  local elapsed=0
  while [[ $elapsed -lt 900 ]]; do
    if bash "$ROOT/scripts/campaign-status-topology.sh" "$TOPO" track-y 2>/dev/null | grep -q "STATUS: DONE"; then
      return 0
    fi
    sleep 15
    elapsed=$((elapsed + 15))
  done
  echo "timeout waiting for track-y on $TOPO" >&2
  return 1
}

bash "$ROOT/scripts/run-topology-campaign-detached.sh" "$TOPO" \
  "$REPO_ROOT/continuum-bench/scripts/run-scylla-track-y-campaign.sh" track-y
wait_campaign
echo "Track Y complete: $TOPO"
