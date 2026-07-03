#!/usr/bin/env bash
# Run Phase B campaign on topology bench node.
# Usage: run-topology-campaign.sh <topology-name> [subset] [hardware]
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export CONTINUUM_NATIVE_AWS_ROOT="$ROOT"
# shellcheck disable=SC1091
source "$ROOT/config/defaults.env"
# shellcheck disable=SC1091
source "$ROOT/lib/manifest.sh"
# shellcheck disable=SC1091
source "$ROOT/lib/ssh.sh"

TOPO="${1:?topology}"
SUBSET="${2:-native-topology}"
HARDWARE="${3:-aws-t3-medium}"

MANIFEST="$(manifest_read "$TOPO")"
BENCH_HOST="$(python3 -c "import json,sys; m=json.load(sys.stdin); print(next(i['public_ip'] for i in m['instances'] if i['role']=='bench'))" <<< "$MANIFEST")"
ENV_EXPORTS="$(bash "$ROOT/scripts/export-env-topology.sh" "$TOPO")"

# Rsync campaign script so bench host has latest without full repo sync.
scp_to "$BENCH_HOST" "$ROOT/../../continuum-bench/scripts/run-distributed-scale-campaign.sh" "~/continuum-bench/run-distributed-scale-campaign.sh"
ssh_cmd "$BENCH_HOST" "chmod +x ~/continuum-bench/run-distributed-scale-campaign.sh"

ssh_cmd "$BENCH_HOST" "bash -lc '
  set -euo pipefail
  $ENV_EXPORTS
  export PATH=\"\$HOME/continuum-bench:\$PATH\"
  export CONTINUUM_BENCH_BIN=\"\$HOME/continuum-bench/continuum-bench\"
  if [[ \"$SUBSET\" == partition-campaign ]]; then
    bash ~/continuum/continuum-bench/scripts/run-partition-campaign.sh $HARDWARE
  elif [[ \"$SUBSET\" == distributed-scale ]]; then
    bash ~/continuum-bench/run-distributed-scale-campaign.sh $HARDWARE auto
  else
    ~/continuum-bench/continuum-bench matrix --hardware $HARDWARE --subset $SUBSET --skip-existing
  fi
'"

echo "topology campaign done: $TOPO $SUBSET $HARDWARE"
