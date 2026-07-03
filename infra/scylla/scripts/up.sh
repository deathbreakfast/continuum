#!/usr/bin/env bash
# Start Scylla lab stack. Default profile: scylla-1
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PROFILE="${1:-scylla-1}"
cd "$ROOT"
docker compose --profile "$PROFILE" up -d
echo "Scylla profile=$PROFILE starting — wait ~60s for CQL on :9042"
