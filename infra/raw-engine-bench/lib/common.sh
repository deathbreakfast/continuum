#!/usr/bin/env bash
# Shared helpers for raw-engine-bench (sources native-aws SSH + manifest).
set -euo pipefail

RAW_BENCH_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
NATIVE_AWS_ROOT="$(cd "$RAW_BENCH_ROOT/../native-aws" && pwd)"
export CONTINUUM_NATIVE_AWS_ROOT="$NATIVE_AWS_ROOT"

# shellcheck disable=SC1091
source "$NATIVE_AWS_ROOT/config/defaults.env"
# shellcheck disable=SC1091
source "$NATIVE_AWS_ROOT/lib/manifest.sh"
# shellcheck disable=SC1091
source "$NATIVE_AWS_ROOT/lib/ssh.sh"

manifest_host() {
  local role="$1"
  local manifest
  manifest="$(manifest_read native-colocated)"
  echo "$manifest" | python3 -c "
import json, sys
m = json.load(sys.stdin)
print(next(i['public_ip'] for i in m['instances'] if i['role'] == sys.argv[1]))
" "$role"
}

REMOTE_DIR="${RAW_ENGINE_BENCH_REMOTE_DIR:-raw-engine-bench}"
