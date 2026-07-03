#!/usr/bin/env bash
# Show raw-engine-bench status on both EC2 hosts.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
# shellcheck disable=SC1091
source "$ROOT/lib/common.sh"

status_host() {
  local role="$1"
  local host
  host="$(manifest_host "$role")"
  echo "=== $role ($host) ==="
  ssh_cmd "$host" "bash -s" <<EOF
set -euo pipefail
ROOT="\${HOME}/${REMOTE_DIR}"
if [[ -f "\$ROOT/run.pid" ]] && kill -0 "\$(cat "\$ROOT/run.pid")" 2>/dev/null; then
  echo "RUNNING pid=\$(cat "\$ROOT/run.pid")"
else
  echo "not running"
fi
echo "done: \$(ls "\$ROOT/done" 2>/dev/null | tr '\n' ' ' || echo none)"
echo "--- run.log tail ---"
tail -5 "\$ROOT/run.log" 2>/dev/null || echo "(no run.log)"
for f in "\$ROOT/logs"/*.log; do
  [[ -f "\$f" ]] || continue
  echo "--- \$(basename "\$f") tail ---"
  tail -3 "\$f"
done
EOF
}

status_host scylla
echo
status_host tikv
