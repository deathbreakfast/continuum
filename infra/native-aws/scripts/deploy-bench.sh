#!/usr/bin/env bash
# SCP pre-built continuum-bench binary to colocated host(s).
# Usage: deploy-bench.sh [manifest-name] <path-to-binary> [role]
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export CONTINUUM_NATIVE_AWS_ROOT="$ROOT"
# shellcheck disable=SC1091
source "$ROOT/config/defaults.env"
# shellcheck disable=SC1091
source "$ROOT/lib/manifest.sh"
# shellcheck disable=SC1091
source "$ROOT/lib/ssh.sh"

MANIFEST_NAME="native-colocated"
BINARY=""
ROLE=""

if [[ $# -ge 2 && -f "${2:-}" ]]; then
MANIFEST_NAME="${1:-native-colocated}"
BINARY="$2"
ROLE="${3:-}"
elif [[ $# -ge 1 && -f "${1:-}" ]]; then
  BINARY="$1"
else
  echo "usage: $0 [manifest-name] <path-to-binary> [role]" >&2
  exit 1
fi

if [[ ! -f "$CONTINUUM_NATIVE_AWS_KEY_PATH" ]]; then
  echo "SSH key not found: $CONTINUUM_NATIVE_AWS_KEY_PATH" >&2
  exit 1
fi

MANIFEST="$(manifest_read "$MANIFEST_NAME")"

deploy_to() {
  local host="$1"
  ssh_wait_ready "$host"
  ssh_cmd "$host" "mkdir -p ~/continuum-bench/reports"
  scp_to "$host" "$BINARY" "~/continuum-bench/continuum-bench"
  ssh_cmd "$host" "chmod +x ~/continuum-bench/continuum-bench"
  echo "Deployed to ec2-user@${host}:~/continuum-bench/continuum-bench"
}

if [[ -n "$ROLE" ]]; then
  HOST="$(echo "$MANIFEST" | python3 -c "
import json, sys
m = json.load(sys.stdin)
for i in m['instances']:
    if i['role'] == sys.argv[1]:
        print(i['public_ip'])
        break
" "$ROLE")"
  deploy_to "$HOST"
else
  while IFS= read -r host; do
    [[ -n "$host" ]] || continue
    deploy_to "$host"
  done < <(echo "$MANIFEST" | python3 -c "
import json, sys
m = json.load(sys.stdin)
for i in m['instances']:
    print(i['public_ip'])
")
fi
