#!/usr/bin/env bash
# Start detached Scylla raw bench on EC2 (survives WSL disconnect).
# Usage: run-scylla.sh
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
# shellcheck disable=SC1091
source "$ROOT/lib/common.sh"

HOST="$(manifest_host scylla)"
ssh_cmd "$HOST" "bash -s" <<EOF
set -euo pipefail
ROOT="\${HOME}/${REMOTE_DIR}"
mkdir -p "\$ROOT/logs" "\$ROOT/done"
if [[ -f "\$ROOT/run.pid" ]] && kill -0 "\$(cat "\$ROOT/run.pid")" 2>/dev/null; then
  echo "scylla run-all already running pid=\$(cat "\$ROOT/run.pid")"
  exit 0
fi
nohup bash "\$ROOT/remote/scylla-run-all.sh" >"\$ROOT/run.log" 2>&1 &
echo \$! >"\$ROOT/run.pid"
echo "started scylla-run-all pid=\$(cat "\$ROOT/run.pid")"
EOF

echo "Scylla bench detached on $HOST — check: bash infra/raw-engine-bench/scripts/status.sh"
