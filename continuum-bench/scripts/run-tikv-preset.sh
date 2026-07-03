#!/usr/bin/env bash
# Run surreal-tikv benchmark matrix for one Docker Compose preset on colocated hardware.
#
# Usage: run-tikv-preset.sh <preset> <hardware> [--skip-c6]
# Example: run-tikv-preset.sh tikv-minimal aws-t4g-medium --skip-c6
#
# Budget colocated (4 GiB): only tikv-minimal is viable. Do NOT use tikv-ha-3,
# tikv-scale-5, or surreal-* presets on t3.medium/t4g.medium — use multi-EC2 infra
# (infra/surreal-tikv-aws/, Phase 4) for topology/count sweeps.
set -euo pipefail

if [[ $# -lt 2 ]]; then
  echo "usage: $0 <preset> <hardware> [--skip-c6]" >&2
  exit 1
fi

PRESET="$1"
HARDWARE="$2"
shift 2

SKIP_C6=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --skip-c6) SKIP_C6="--skip-experiments bm-c6" ;;
    *) echo "unknown arg: $1" >&2; exit 1 ;;
  esac
  shift
done

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
BENCH="${CONTINUUM_BENCH_BIN:-cargo run --release -p continuum-bench --}"

export CONTINUUM_BENCH_REPORTS_DIR="${CONTINUUM_BENCH_REPORTS_DIR:-$ROOT/profiling/continuum-bench/reports}"

echo "TiKV preset=$PRESET hardware=$HARDWARE reports=$CONTINUUM_BENCH_REPORTS_DIR"

"$ROOT/infra/surreal-tikv/scripts/down.sh"
"$ROOT/infra/surreal-tikv/scripts/up.sh" "$PRESET"
eval "$("$ROOT/infra/surreal-tikv/scripts/export-env.sh" "$PRESET")"

export CONTINUUM_BENCH_SURREAL_HARDWARE="$HARDWARE"
export CONTINUUM_BENCH_TIKV_HARDWARE="$HARDWARE"

eval "$BENCH matrix --subset tikv-lab-colocated \
  --hardware \"$HARDWARE\" \
  --tikv-topology \"$PRESET\" \
  --skip-existing \
  $SKIP_C6"

echo "TIKV_PRESET_DONE preset=$PRESET hardware=$HARDWARE"
