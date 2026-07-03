#!/usr/bin/env bash
# Run Phase B campaign on topology bench node.
# Usage: run-topology-campaign.sh <topology-name> [subset]
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

MANIFEST="$(manifest_read "$TOPO")"
BENCH_HOST="$(python3 -c "import json,sys; m=json.load(sys.stdin); print(next(i['public_ip'] for i in m['instances'] if i['role']=='bench'))" <<< "$MANIFEST")"
ENV_EXPORTS="$(bash "$ROOT/scripts/export-env-topology.sh" "$TOPO")"

ssh_cmd "$BENCH_HOST" "bash -lc '
  set -euo pipefail
  $ENV_EXPORTS
  export PATH=\"\$HOME/continuum-bench:\$PATH\"
  if [[ \"$SUBSET\" == partition-campaign ]]; then
    bash ~/continuum/continuum-bench/scripts/run-partition-campaign.sh aws-t3-medium
  else
    ~/continuum-bench/continuum-bench matrix --hardware aws-t3-medium --subset $SUBSET --skip-existing
  fi
'"

echo "topology campaign done: $TOPO $SUBSET"
