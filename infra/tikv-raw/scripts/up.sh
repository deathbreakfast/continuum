#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
docker compose --profile tikv-raw-minimal up -d
echo "TiKV raw minimal starting — waiting for PD + store bootstrap"
for _ in $(seq 1 60); do
  if curl -sf http://127.0.0.1:2379/pd/api/v1/stores 2>/dev/null | grep -q '"state_name":"Up"'; then
    echo "TiKV store is Up"
    exit 0
  fi
  sleep 2
done
echo "warning: TiKV store not Up yet — bench may need a short wait" >&2
