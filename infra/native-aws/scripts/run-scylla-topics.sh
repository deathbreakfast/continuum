#!/usr/bin/env bash
# Topic fan-out driver.
# Usage: run-scylla-topics.sh <topology> [hardware]
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
REPO_ROOT="$(cd "$ROOT/../.." && pwd)"
TOPO="${1:?topology}"
HARDWARE="${2:-aws-t3-medium}"

wait_campaign() {
  local elapsed=0
  while [[ $elapsed -lt 3600 ]]; do
    if bash "$ROOT/scripts/campaign-status-topology.sh" "$TOPO" topics 2>/dev/null | grep -q "STATUS: DONE"; then
      return 0
    fi
    sleep 15
    elapsed=$((elapsed + 15))
  done
  echo "timeout waiting for topics on $TOPO" >&2
  return 1
}

bash "$ROOT/scripts/run-topology-campaign-detached.sh" "$TOPO" \
  "$REPO_ROOT/continuum-bench/scripts/run-scylla-topic-campaign.sh" \
  topics "$HARDWARE"
wait_campaign
echo "Topics complete: $TOPO"
