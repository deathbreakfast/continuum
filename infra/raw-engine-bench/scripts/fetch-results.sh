#!/usr/bin/env bash
# Fetch logs from EC2, parse summaries, write results/*.json.
# Usage: fetch-results.sh [--update-experiments]
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
# shellcheck disable=SC1091
source "$ROOT/lib/common.sh"

UPDATE_MD=false
if [[ "${1:-}" == "--update-experiments" ]]; then
  UPDATE_MD=true
fi

DEST="$ROOT/results"
mkdir -p "$DEST"
PARSE="$ROOT/scripts/parse-stress-log.sh"

for role in scylla tikv; do
  host="$(manifest_host "$role")"
  echo ">>> fetch $role from $host"
  for id in "${role}-a" "${role}-b"; do
    scp_from "$host" "~/${REMOTE_DIR}/logs/${id}.log" "$DEST/${id}.log" 2>/dev/null || true
  done
  scp_from "$host" "~/${REMOTE_DIR}/run.log" "$DEST/${role}-run.log" 2>/dev/null || true
done

for id in scylla-a scylla-b; do
  [[ -f "$DEST/${id}.log" ]] || continue
  bash "$PARSE" cassandra-stress "$DEST/${id}.log" >"$DEST/${id}.json"
  echo "parsed $id → $DEST/${id}.json"
done
for id in tikv-a tikv-b; do
  [[ -f "$DEST/${id}.log" ]] || continue
  bash "$PARSE" ycsb "$DEST/${id}.log" >"$DEST/${id}.json"
  echo "parsed $id → $DEST/${id}.json"
done

if $UPDATE_MD; then
  bash "$ROOT/scripts/update-experiments-md.sh"
fi

echo "Results in $DEST"
