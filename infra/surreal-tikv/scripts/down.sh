#!/usr/bin/env bash
# Tear down all TiKV + SurrealDB lab containers.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

docker compose --profile tikv-minimal --profile tikv-ha-3 --profile tikv-scale-5 \
  --profile surreal-2n --profile surreal-4n down -v

echo "Stack stopped"
