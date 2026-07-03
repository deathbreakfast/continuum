#!/usr/bin/env bash
# Track W driver: raw cassandra-stress spread vs Continuum on topology.
# Usage: run-scylla-track-w.sh <topology>
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
REPO_ROOT="$(cd "$ROOT/../.." && pwd)"
RAW_ROOT="$REPO_ROOT/infra/raw-engine-bench"
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
BENCH_HOST="$(topology_bench_host "$TOPO")"
SCYLLA_NODES="$(topology_scylla_nodes_csv "$TOPO")"
if [[ "$TOPO" == native-colocated ]]; then
  SCYLLA_NODES="127.0.0.1"
fi
REMOTE_DIR="${RAW_ENGINE_BENCH_REMOTE_DIR:-raw-engine-bench}"

ssh_cmd "$BENCH_HOST" "mkdir -p ~/${REMOTE_DIR}/{logs,done,remote}"
scp_to "$BENCH_HOST" "$RAW_ROOT/remote/scylla-run-all.sh" "~/${REMOTE_DIR}/remote/"
ssh_cmd "$BENCH_HOST" "chmod +x ~/${REMOTE_DIR}/remote/scylla-run-all.sh"

ssh_cmd "$BENCH_HOST" "bash -lc '
  set -euo pipefail
  ROOT=\"\${HOME}/${REMOTE_DIR}\"
  rm -f \"\$ROOT/done/scylla-a\" \"\$ROOT/run.pid\"
  export SCYLLA_NODES=\"${SCYLLA_NODES}\"
  nohup env SCYLLA_NODES=\"${SCYLLA_NODES}\" bash \"\$ROOT/remote/scylla-run-all.sh\" >\"\$ROOT/run.log\" 2>&1 &
  echo \$! >\"\$ROOT/run.pid\"
  echo \"started scylla-run-all pid=\$(cat \"\$ROOT/run.pid\") SCYLLA_NODES=${SCYLLA_NODES}\"
'"

elapsed=0
while [[ $elapsed -lt 3600 ]]; do
  if ssh_cmd "$BENCH_HOST" "test -f ~/${REMOTE_DIR}/done/scylla-a"; then
    break
  fi
  sleep 30
  elapsed=$((elapsed + 30))
done

DEST="$ROOT/state/${TOPO}-track-w"
mkdir -p "$DEST"
scp_from "$BENCH_HOST" "~/${REMOTE_DIR}/logs/scylla-a.log" "$DEST/scylla-a.log" 2>/dev/null || true
if [[ -f "$DEST/scylla-a.log" ]]; then
  bash "$RAW_ROOT/scripts/parse-stress-log.sh" cassandra-stress "$DEST/scylla-a.log" >"$DEST/scylla-a.json"
fi
echo "Track W complete: $TOPO results in $DEST"
