#!/usr/bin/env bash
# Poll detached campaign on a topology bench host.
# Usage: campaign-status-topology.sh <topology> [marker-basename]
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
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
MARKER="${2:-}"

BENCH_HOST="$(topology_bench_host "$TOPO")"

check_marker() {
  local m="$1"
  echo "=== $TOPO bench ($BENCH_HOST) campaign=$m ==="
  ssh_cmd "$BENCH_HOST" "bash -lc '
    set +e
    marker=~/continuum-bench/campaign-${m}.done
    log=~/continuum-bench/campaign-${m}.log
    pidfile=~/continuum-bench/campaign-${m}.pid
    if [[ -f \"\$marker\" ]]; then
      echo \"STATUS: DONE\"
      cat \"\$marker\" 2>/dev/null || true
    elif [[ -f \"\$pidfile\" ]] && kill -0 \"\$(cat \"\$pidfile\")\" 2>/dev/null; then
      echo \"STATUS: RUNNING pid=\$(cat \"\$pidfile\")\"
    else
      echo \"STATUS: NOT_RUNNING\"
    fi
    if [[ -f \"\$log\" ]]; then
      echo \"--- tail \$log ---\"
      tail -25 \"\$log\"
    fi
  '"
}

if [[ -n "$MARKER" ]]; then
  check_marker "$MARKER"
else
  for m in track-u track-v track-w track-x track-y z1 z2 z3 z4 z5; do
    check_marker "$m" || true
  done
fi
