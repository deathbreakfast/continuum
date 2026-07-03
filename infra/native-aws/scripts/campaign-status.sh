#!/usr/bin/env bash
# Check status of detached native-aws campaigns (nohup + DONE marker).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export CONTINUUM_NATIVE_AWS_ROOT="$ROOT"
# shellcheck disable=SC1091
source "$ROOT/lib/manifest.sh"
# shellcheck disable=SC1091
source "$ROOT/lib/ssh.sh"

CAMPAIGN="${1:-}"
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

check_role() {
  local role="$1"
  local campaign="$2"
  local host
  host="$(manifest_host "$role")"
  echo "=== $role ($host) campaign=$campaign ==="
  ssh_cmd "$host" "bash -lc '
    set +e
    marker=~/continuum-bench/campaign-${campaign}.done
    log=~/continuum-bench/campaign-${campaign}.log
    if [[ -f \"\$marker\" ]]; then
      echo \"STATUS: DONE\"
      cat \"\$marker\"
    elif pgrep -af \"run-${campaign}-campaign\" >/dev/null 2>&1; then
      echo \"STATUS: RUNNING\"
      pgrep -af \"run-${campaign}-campaign\" || true
    else
      echo \"STATUS: NOT_RUNNING\"
    fi
    if [[ -f \"\$log\" ]]; then
      echo \"--- tail \$log ---\"
      tail -20 \"\$log\"
    fi
  '"
}

for role in scylla tikv; do
  [[ -n "$ROLE" && "$ROLE" != "$role" ]] && continue
  if [[ -n "$CAMPAIGN" ]]; then
    check_role "$role" "$CAMPAIGN"
  else
    for c in concurrency partition; do
      check_role "$role" "$c"
    done
  fi
done
