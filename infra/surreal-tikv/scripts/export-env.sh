#!/usr/bin/env bash
# Print bench env vars for the active preset (source in shell: source scripts/export-env.sh tikv-minimal)
set -euo pipefail

PRESET="${1:-${CONTINUUM_BENCH_TIKV_PRESET:-tikv-minimal}}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ENV_FILE="$ROOT/presets/${PRESET}.env"

if [[ ! -f "$ENV_FILE" ]]; then
  echo "unknown preset: $PRESET" >&2
  exit 1
fi

# shellcheck disable=SC1090
source "$ENV_FILE"

export CONTINUUM_BENCH_TIKV_TOPOLOGY
export CONTINUUM_BENCH_TIKV_PD_ENDPOINT
export CONTINUUM_BENCH_SURREAL_URL
export CONTINUUM_BENCH_SURREAL_INSTANCES
export CONTINUUM_BENCH_SURREAL_USER
export CONTINUUM_BENCH_SURREAL_PASS

echo "export CONTINUUM_BENCH_TIKV_TOPOLOGY=$CONTINUUM_BENCH_TIKV_TOPOLOGY"
echo "export CONTINUUM_BENCH_TIKV_PD_ENDPOINT=$CONTINUUM_BENCH_TIKV_PD_ENDPOINT"
echo "export CONTINUUM_BENCH_SURREAL_URL=$CONTINUUM_BENCH_SURREAL_URL"
echo "export CONTINUUM_BENCH_SURREAL_INSTANCES=$CONTINUUM_BENCH_SURREAL_INSTANCES"
echo "export CONTINUUM_BENCH_SURREAL_USER=${CONTINUUM_BENCH_SURREAL_USER:-root}"
echo "export CONTINUUM_BENCH_SURREAL_PASS=${CONTINUUM_BENCH_SURREAL_PASS:-root}"
