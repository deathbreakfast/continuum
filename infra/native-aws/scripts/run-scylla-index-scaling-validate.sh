#!/usr/bin/env bash
# Phase 2 validation driver.
# Usage: run-scylla-index-scaling-validate.sh <topology> [hardware]
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
REPO_ROOT="$(cd "$ROOT/../.." && pwd)"
TOPO="${1:?topology}"
HARDWARE="${2:-aws-t3-medium}"

wait_campaign() {
  local elapsed=0
  while [[ $elapsed -lt 3600 ]]; do
    if bash "$ROOT/scripts/campaign-status-topology.sh" "$TOPO" index-validate 2>/dev/null | grep -q "STATUS: DONE"; then
      return 0
    fi
    sleep 15
    elapsed=$((elapsed + 15))
  done
  echo "timeout waiting for index-validate on $TOPO" >&2
  return 1
}

bash "$ROOT/scripts/run-topology-campaign-detached.sh" "$TOPO" \
  "$REPO_ROOT/continuum-bench/scripts/run-scylla-index-scaling-validate-campaign.sh" \
  index-validate "$HARDWARE"
wait_campaign
echo "Index validate complete: $TOPO hardware=$HARDWARE"
