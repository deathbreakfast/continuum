#!/usr/bin/env bash
# SCP reports from colocated fleet back to workspace.
# Usage: fetch-reports.sh [manifest-name]
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

MANIFEST_NAME="${1:-native-colocated}"
DEST="${CONTINUUM_BENCH_REPORTS_DIR:-$REPO_ROOT/profiling/continuum-bench/reports}"
mkdir -p "$DEST"

MANIFEST="$(manifest_read "$MANIFEST_NAME")"
while IFS= read -r host; do
  [[ -z "$host" ]] && continue
  echo "Fetching reports from $host"
  scp_from "$host" "~/continuum-bench/reports/*.json" "$DEST/" 2>/dev/null || true
done < <(echo "$MANIFEST" | python3 -c "
import json, sys
for i in json.load(sys.stdin)['instances']:
    print(i['public_ip'])
")

echo "Reports in $DEST"
