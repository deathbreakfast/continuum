#!/usr/bin/env bash
# Multi-publisher driver for one N value.
# Usage: run-scylla-publishers.sh <topology> <N> [hardware]
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
REPO_ROOT="$(cd "$ROOT/../.." && pwd)"
TOPO="${1:?topology}"
N="${2:?N}"
HARDWARE="${3:-aws-t3-medium}"
MARKER="publishers-N${N}"

wait_campaign() {
  local elapsed=0
  while [[ $elapsed -lt 1800 ]]; do
    if bash "$ROOT/scripts/campaign-status-topology.sh" "$TOPO" "$MARKER" 2>/dev/null | grep -q "STATUS: DONE"; then
      return 0
    fi
    sleep 15
    elapsed=$((elapsed + 15))
  done
  echo "timeout waiting for $MARKER on $TOPO" >&2
  return 1
}

bash "$ROOT/scripts/run-topology-campaign-detached.sh" "$TOPO" \
  "$REPO_ROOT/continuum-bench/scripts/run-scylla-publishers-campaign.sh" \
  "$MARKER" "$N" 64 256 "$HARDWARE"
wait_campaign
echo "Publishers N=$N complete: $TOPO"
