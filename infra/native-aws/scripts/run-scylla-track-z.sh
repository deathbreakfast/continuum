#!/usr/bin/env bash
# Track Z lever driver: A/B BM-M4 @ C=K for one optimization lever.
# Usage: run-scylla-track-z.sh <topology> <z1|z2|z3|z4|z5> [C=K]
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
REPO_ROOT="$(cd "$ROOT/../.." && pwd)"
TOPO="${1:?topology}"
LEVER="${2:?z1|z2|z3|z4|z5}"
CK="${3:-256}"

wait_campaign() {
  local elapsed=0
  while [[ $elapsed -lt 900 ]]; do
    if bash "$ROOT/scripts/campaign-status-topology.sh" "$TOPO" "$LEVER" 2>/dev/null | grep -q "STATUS: DONE"; then
      return 0
    fi
    sleep 15
    elapsed=$((elapsed + 15))
  done
  echo "timeout waiting for $LEVER on $TOPO" >&2
  return 1
}

bash "$ROOT/scripts/run-topology-campaign-detached.sh" "$TOPO" \
  "$REPO_ROOT/continuum-bench/scripts/run-scylla-lever-campaign.sh" \
  "$LEVER" "$LEVER" aws-t3-medium "$CK"
wait_campaign
echo "Track Z ($LEVER) complete: $TOPO C=K=$CK"
