#!/usr/bin/env bash
# Run Phase A colocated campaign on colocated fleet via SSH.
# Usage: run-campaign.sh <native-lab|native-scale|native-lab-partitioned|concurrency-campaign|partition-campaign> [role]
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export CONTINUUM_NATIVE_AWS_ROOT="$ROOT"
# shellcheck disable=SC1091
source "$ROOT/config/defaults.env"
# shellcheck disable=SC1091
source "$ROOT/lib/manifest.sh"
# shellcheck disable=SC1091
source "$ROOT/lib/ssh.sh"

CAMPAIGN="${1:-native-lab}"
ROLE="${2:-}"

MANIFEST="$(manifest_read native-colocated)"

manifest_host() {
  local role="$1"
  echo "$MANIFEST" | python3 -c "
import json, sys
m = json.load(sys.stdin)
print(next(i['public_ip'] for i in m['instances'] if i['role'] == sys.argv[1]))
" "$role"
}

run_matrix() {
  local host="$1"
  local export_role="$2"
  local storage="$3"
  local subset="$4"
  ssh_cmd "$host" "bash -lc '
    set -euo pipefail
    eval \"\$(bash ~/continuum/infra/native-aws/scripts/export-env.sh $export_role)\"
    export PATH=\"\$HOME/continuum-bench:\$PATH\"
    ~/continuum-bench/continuum-bench matrix --hardware aws-t3-medium --subset $subset --storages $storage --skip-existing
  '"
}

start_detached_script() {
  local role="$1"
  local script="$2"
  local storage="$3"
  local done_marker="$4"
  local host
  host="$(manifest_host "$role")"
  echo ">>> detached $script on $role ($host)"
  ssh_cmd "$host" "bash -lc '
    set -euo pipefail
    eval \"\$(bash ~/continuum/infra/native-aws/scripts/export-env.sh $role)\"
    export PATH=\"\$HOME/continuum-bench:\$PATH\"
    mkdir -p ~/continuum-bench
    rm -f ~/continuum-bench/$done_marker ~/continuum-bench/campaign-${done_marker%.done}.log
    nohup bash ~/continuum/continuum-bench/scripts/$script aws-t3-medium $storage \
      > ~/continuum-bench/campaign-${done_marker%.done}.log 2>&1 &
    echo \$! > ~/continuum-bench/campaign-${done_marker%.done}.pid
    echo \"started pid=\$(cat ~/continuum-bench/campaign-${done_marker%.done}.pid)\"
  '"
}

if [[ "$CAMPAIGN" == "concurrency-campaign" ]]; then
  for role in scylla tikv; do
    [[ -n "$ROLE" && "$ROLE" != "$role" ]] && continue
    storage="scylla"
    [[ "$role" == "tikv" ]] && storage="tikv-raw"
    start_detached_script "$role" "run-concurrency-campaign.sh" "$storage" "campaign-concurrency.done"
  done
  exit 0
fi

if [[ "$CAMPAIGN" == "partition-campaign" ]]; then
  for role in scylla tikv; do
    [[ -n "$ROLE" && "$ROLE" != "$role" ]] && continue
    storage="scylla"
    [[ "$role" == "tikv" ]] && storage="tikv-raw"
    start_detached_script "$role" "run-partition-campaign.sh" "$storage" "campaign-partition.done"
  done
  exit 0
fi

for role in scylla tikv; do
  [[ -n "$ROLE" && "$ROLE" != "$role" ]] && continue
  host="$(manifest_host "$role")"
  storage="scylla"
  [[ "$role" == "tikv" ]] && storage="tikv-raw"
  echo ">>> $CAMPAIGN on $role ($host)"
  run_matrix "$host" "$role" "$storage" "$CAMPAIGN"
done
