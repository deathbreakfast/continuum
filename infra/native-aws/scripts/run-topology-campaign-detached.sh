#!/usr/bin/env bash
# Start a detached campaign script on a topology bench host (survives WSL disconnect).
# Usage: run-topology-campaign-detached.sh <topology> <local-script-path> <done-marker-basename> [extra-args...]
# Example: run-topology-campaign-detached.sh native-scylla-2n continuum-bench/scripts/run-scylla-track-u-campaign.sh track-u 256
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
REPO_ROOT="$(cd "$ROOT/../.." && pwd)"
export CONTINUUM_NATIVE_AWS_ROOT="$ROOT"
# shellcheck disable=SC1091
source "$ROOT/config/defaults.env"
# shellcheck disable=SC1091
source "$ROOT/lib/manifest.sh"
# shellcheck disable=SC1091
source "$ROOT/lib/ssh.sh"
# shellcheck disable=SC1091
source "$ROOT/lib/topology.sh"

TOPO="${1:?topology}"
LOCAL_SCRIPT="${2:?script path}"
MARKER="${3:?done marker basename without .done}"
shift 3
EXTRA_ARGS=("$@")

[[ -f "$LOCAL_SCRIPT" ]] || { echo "script not found: $LOCAL_SCRIPT" >&2; exit 1; }

MANIFEST="$(manifest_read "$TOPO")"
BENCH_HOST="$(topology_bench_host "$TOPO")"
ENV_EXPORTS="$(bash "$ROOT/scripts/export-env-topology.sh" "$TOPO" 2>/dev/null || true)"

SCRIPT_NAME="$(basename "$LOCAL_SCRIPT")"
REMOTE_SCRIPT="~/continuum-bench/$SCRIPT_NAME"

scp_to "$BENCH_HOST" "$LOCAL_SCRIPT" "$REMOTE_SCRIPT"
ssh_cmd "$BENCH_HOST" "chmod +x $REMOTE_SCRIPT"

# Colocated topology uses export-env.sh on scylla role — bench may not exist.
if [[ -z "$ENV_EXPORTS" ]]; then
  BENCH_TOPO="$(python3 -c "import json,sys; print(json.load(sys.stdin).get('bench_topology',''))" <<< "$MANIFEST")"
  if [[ "$TOPO" == native-colocated ]]; then
    ENV_EXPORTS="$(bash "$ROOT/scripts/export-env.sh" scylla)"
  fi
fi

ssh_cmd "$BENCH_HOST" "bash -lc '
  set -euo pipefail
  $ENV_EXPORTS
  export PATH=\"\$HOME/continuum-bench:\$PATH\"
  export CONTINUUM_BENCH_BIN=\"\$HOME/continuum-bench/continuum-bench\"
  mkdir -p ~/continuum-bench/metrics
  rm -f ~/continuum-bench/campaign-${MARKER}.done ~/continuum-bench/campaign-${MARKER}.log ~/continuum-bench/campaign-${MARKER}.pid
  nohup bash $REMOTE_SCRIPT ${EXTRA_ARGS[*]:-} \
    > ~/continuum-bench/campaign-${MARKER}.log 2>&1 &
  echo \$! > ~/continuum-bench/campaign-${MARKER}.pid
  echo \"started $MARKER pid=\$(cat ~/continuum-bench/campaign-${MARKER}.pid)\"
'"

echo "Detached campaign $MARKER on $TOPO bench ($BENCH_HOST)"
