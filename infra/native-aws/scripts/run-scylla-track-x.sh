#!/usr/bin/env bash
# Track X driver: dual-process bench on scylla-2n.
# Usage: run-scylla-track-x.sh <topology> [C=K]
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
REPO_ROOT="$(cd "$ROOT/../.." && pwd)"
TOPO="${1:?topology}"
CK="${2:-256}"

wait_campaign() {
  local elapsed=0
  while [[ $elapsed -lt 1200 ]]; do
    if bash "$ROOT/scripts/campaign-status-topology.sh" "$TOPO" track-x 2>/dev/null | grep -q "STATUS: DONE"; then
      return 0
    fi
    sleep 15
    elapsed=$((elapsed + 15))
  done
  echo "timeout waiting for track-x on $TOPO" >&2
  return 1
}

bash "$ROOT/scripts/run-topology-campaign-detached.sh" "$TOPO" \
  "$REPO_ROOT/continuum-bench/scripts/run-scylla-track-x-campaign.sh" track-x aws-t3-medium "$CK"
wait_campaign
echo "Track X complete: $TOPO C=K=$CK"
