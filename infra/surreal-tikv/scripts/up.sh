#!/usr/bin/env bash
# Start a TiKV + SurrealDB lab preset and wait for health.
set -euo pipefail

PRESET="${1:-tikv-minimal}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"

cd "$ROOT"

echo "Starting preset: $PRESET"
docker compose --profile "$PRESET" up -d

echo "Waiting for PD health..."
for _ in $(seq 1 60); do
  if curl -sf "http://127.0.0.1:2379/health" >/dev/null 2>&1; then
    echo "PD ready"
    break
  fi
  sleep 2
done

if [[ "$PRESET" != "tikv-scale-5" ]]; then
  echo "Waiting for Surreal health..."
  for _ in $(seq 1 90); do
    if curl -sf "http://127.0.0.1:8000/health" >/dev/null 2>&1; then
      echo "Surreal ready"
      exit 0
    fi
    sleep 2
  done
  echo "ERROR: Surreal health check timed out — check docker logs (tikv0/surreal)" >&2
  exit 1
fi
